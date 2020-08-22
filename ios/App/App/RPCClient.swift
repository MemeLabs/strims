//
//  ContentView.swift
//  App
//
//  Created by Slugalisk on 8/20/20.
//  Copyright © 2020 MemeLabs. All rights reserved.
//

import Foundation
import SwiftProtobuf
import PromiseKit

//enum RPCClientError: Error {
//    case runtimeError(String)
//}

struct RPCClientError: Error {
    let message: String

    init(_ message: String) {
        self.message = message
    }

    public var localizedDescription: String {
        return message
    }
}

struct RPCEvent: OptionSet {
    let rawValue: Int
    
    static let data    = RPCEvent(rawValue: 1 << 0)
    static let close    = RPCEvent(rawValue: 1 << 1)
    static let requestError  = RPCEvent(rawValue: 1 << 2)
    static let responseError  = RPCEvent(rawValue: 1 << 3)
}

class RPCResponseStream<T: Message> {
    public let close: () -> Void;
    public var delegate: (T?, RPCEvent) -> Void = {_, _ in}
    
    public init(_ close: @escaping () -> Void) {
        self.close = close
    }
}

extension OutputStream {
    var data: Data? {
        return self.property(forKey: .dataWrittenToMemoryStreamKey) as? Data
    }
}

class RPCClient {
    private static let callbackMethod = "_CALLBACK"
    private static let cancelMethod   = "_CANCEL"
    private static let anyURLPrefix = "strims.gg/"
    
    private var nextCallID: UInt64 = 0;
    private var callbacks: Dictionary<UInt64, (PBCall) -> Void> = Dictionary()
    private var g: SwiftBridge
    
    init() {
        self.g = SwiftBridge()
        self.g.onData = {(_ b: Data?) -> Void in
            self.handleCallback(b)
        }
    }
    
    private func handleCallback(_ b: Data?) {
        let stream = InputStream(data: b!)
        stream.open()
        
        do {
            let call = try BinaryDelimited.parse(messageType: PBCall.self, from: stream)
            
            let json = try call.jsonString()
            print("received rpc data: \(json)")
            
            let callback = self.callbacks[call.parentID]
            callback?(call)
        } catch {
            print("error: \(error)")
        }
        
        stream.close()
    }
    
    private func getNextCallID() -> UInt64 {
        self.nextCallID += 1
        return self.nextCallID
    }
    
    private func call<T: Message>(_ method: String, _ arg: T, _ callID: UInt64, _ parentID: UInt64 = 0) throws {
        let arg = try Google_Protobuf_Any(message: arg, typePrefix: RPCClient.anyURLPrefix)
        let call = PBCall.with {
            $0.id = callID
            $0.parentID = parentID
            $0.method = method
            $0.argument = arg
        }
        
        let stream = OutputStream.toMemory()
        stream.open()
        try BinaryDelimited.serialize(message: call, to: stream)
        stream.close()
        
        self.g.write(stream.data!)
    }
    
    public func call<T: Message>(_ method: String, _ arg: T) {
        DispatchQueue.global(qos: .default).async {
            do {
                try self.call(method, arg, self.getNextCallID())
            } catch {}
        }
    }
    
    public func callStreaming<T: Message, R: Message>(_ method: String, _ arg: T) -> RPCResponseStream<R> {
        let callID = self.getNextCallID()
    
        let stream = RPCResponseStream<R>({
            self.callbacks.removeValue(forKey: callID)
            do {
                try self.call(RPCClient.cancelMethod, PBCancel(), self.getNextCallID(), callID)
            } catch {}
        })
        
        self.callbacks[callID] = {(_ call: PBCall) -> Void in
            do {
                let messageType = Google_Protobuf_Any.messageType(forTypeURL: call.argument.typeURL)
                switch (messageType) {
                case is R.Type:
                    let arg = try R(serializedData: call.argument.value)
                    stream.delegate(arg, RPCEvent.data)
                case is PBClose.Type:
                    self.callbacks.removeValue(forKey: callID)
                    stream.delegate(nil, RPCEvent.close)
                case is PBError.Type:
                    fallthrough
                default:
                    self.callbacks.removeValue(forKey: callID)
                    stream.delegate(nil, RPCEvent.responseError)
                }
            } catch {
                stream.delegate(nil, RPCEvent.responseError)
            }
        }
        
        DispatchQueue.global(qos: .default).async {
            do {
                try self.call(method, arg, callID)
            } catch {
                self.callbacks.removeValue(forKey: callID)
                stream.delegate(nil, RPCEvent.requestError)
            }
        }
        
        return stream
    }

    public func callUnary<T: Message, R: Message>(_ method: String, _ arg: T) -> Promise<R> {
        Promise { seal in
            let callID = self.getNextCallID()

            let timer = Timer.scheduledTimer(withTimeInterval: 5.0, repeats: false) { timer in
                self.callbacks.removeValue(forKey: callID)
                seal.reject(RPCClientError("call timeout"))
            }
            
            self.callbacks[callID] = {(_ call: PBCall) -> Void in
                self.callbacks.removeValue(forKey: callID)
                timer.invalidate()
                                
                do {
                    let messageType = Google_Protobuf_Any.messageType(forTypeURL: call.argument.typeURL)
                    switch (messageType) {
                    case is R.Type:
                        let arg = try R(serializedData: call.argument.value)
                        seal.resolve(arg, nil)
                    case is PBError.Type:
                        let error = try PBError(serializedData: call.argument.value)
                        seal.reject(RPCClientError(error.message))
                    default:
                        print("unexpected response type: \(call.argument.typeURL)")
                        seal.reject(RPCClientError("unexpected response type: \(call.argument.typeURL)"))
                    }
                } catch {
                    seal.reject(error)
                }
            }
            
            DispatchQueue.global(qos: .default).async {
                do {
                    try self.call(method, arg, callID)
                } catch {
                    self.callbacks.removeValue(forKey: callID)
                    seal.reject(error)
                }
            }
        }
    }
}

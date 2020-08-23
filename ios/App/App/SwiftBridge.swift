//
//  SwiftBridge.swift
//  App
//
//  Created by Slugalisk on 8/20/20.
//  Copyright © 2020 MemeLabs. All rights reserved.
//

import UIKit
import Bridge

class SwiftBridge: NSObject, BridgeSwiftSideProtocol {
    public var onData: (_: Data?) -> Void = {_ in}
    public var g: BridgeGoSide?
    public var error: NSError?
    
    override init() {
        super.init()
        self.g = BridgeNewGoSide(self, &self.error)
    }
    
    public func write(_ b: Data?) throws {
        if let error = self.error {
            throw error
        }
        try self.g!.write(b)
    }
    
    public func emitError(_ msg: String?) {
        print("error: " + msg!)
    }
    
    public func emitData(_ b: Data?) {
        self.onData(b)
    }
}

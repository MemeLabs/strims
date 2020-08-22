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
    
    override init() {
        super.init()
        self.g = BridgeNewGoSide(self)!
    }
    
    public func write(_ b: Data?) {
        self.g!.write(b)
    }
    
    public func emitError(_ msg: String?) {
        print("error: " + msg!)
    }
    
    public func emitData(_ b: Data?) {
        print("received bytes: " + String(b!.count))
        self.onData(b)
    }
}

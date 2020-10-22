// swift-format-ignore-file
//
//  BrokerProxyClient.swift
//  App
//
//  Copyright © 2020 MemeLabs. All rights reserved.
//

import Foundation
import PromiseKit

class BrokerProxyClient: RPCClient {
  public func open(_ arg: PBBrokerProxyRequest = PBBrokerProxyRequest()) throws -> RPCResponseStream<PBBrokerProxyEvent> {
    return try self.callStreaming("BrokerProxy/Open", arg)
  }
  public func sendKeys(_ arg: PBBrokerProxySendKeysRequest = PBBrokerProxySendKeysRequest()) -> Promise<PBBrokerProxySendKeysResponse> {
    return self.callUnary("BrokerProxy/SendKeys", arg)
  }
  public func receiveKeys(_ arg: PBBrokerProxyReceiveKeysRequest = PBBrokerProxyReceiveKeysRequest()) -> Promise<PBBrokerProxyReceiveKeysResponse> {
    return self.callUnary("BrokerProxy/ReceiveKeys", arg)
  }
  public func data(_ arg: PBBrokerProxyDataRequest = PBBrokerProxyDataRequest()) -> Promise<PBBrokerProxyDataResponse> {
    return self.callUnary("BrokerProxy/Data", arg)
  }
  
}

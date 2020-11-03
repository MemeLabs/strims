// swift-format-ignore-file
//
//  BrokerProxyClient.swift
//  App
//
//  Copyright © 2020 MemeLabs. All rights reserved.
//

import Foundation
import PromiseKit

class BrokerProxyClient {
  private var client: RPCClient

  init(_ client: RPCClient) {
    self.client = client
  }

  public func open(_ arg: PBBrokerProxyRequest = PBBrokerProxyRequest()) throws -> RPCResponseStream<PBBrokerProxyEvent> {
    return try self.client.callStreaming("BrokerProxy/Open", arg)
  }
  public func sendKeys(_ arg: PBBrokerProxySendKeysRequest = PBBrokerProxySendKeysRequest()) -> Promise<PBBrokerProxySendKeysResponse> {
    return self.client.callUnary("BrokerProxy/SendKeys", arg)
  }
  public func receiveKeys(_ arg: PBBrokerProxyReceiveKeysRequest = PBBrokerProxyReceiveKeysRequest()) -> Promise<PBBrokerProxyReceiveKeysResponse> {
    return self.client.callUnary("BrokerProxy/ReceiveKeys", arg)
  }
  public func data(_ arg: PBBrokerProxyDataRequest = PBBrokerProxyDataRequest()) -> Promise<PBBrokerProxyDataResponse> {
    return self.client.callUnary("BrokerProxy/Data", arg)
  }
  public func close(_ arg: PBBrokerProxyCloseRequest = PBBrokerProxyCloseRequest()) -> Promise<PBBrokerProxyCloseResponse> {
    return self.client.callUnary("BrokerProxy/Close", arg)
  }
  
}

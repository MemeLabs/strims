// swift-format-ignore-file
//
//  BootstrapPeerClient.swift
//  App
//
//  Copyright © 2020 MemeLabs. All rights reserved.
//

import Foundation
import PromiseKit

class BootstrapPeerClient {
  private var client: RPCClient

  init(_ client: RPCClient) {
    self.client = client
  }

  public func getPublishEnabled(_ arg: PBBootstrapPeerGetPublishEnabledRequest = PBBootstrapPeerGetPublishEnabledRequest()) -> Promise<PBBootstrapPeerGetPublishEnabledResponse> {
    return self.client.callUnary("BootstrapPeer/GetPublishEnabled", arg)
  }
  public func listNetworks(_ arg: PBBootstrapPeerListNetworksRequest = PBBootstrapPeerListNetworksRequest()) -> Promise<PBBootstrapPeerListNetworksResponse> {
    return self.client.callUnary("BootstrapPeer/ListNetworks", arg)
  }
  public func publish(_ arg: PBBootstrapPeerPublishRequest = PBBootstrapPeerPublishRequest()) -> Promise<PBBootstrapPeerPublishResponse> {
    return self.client.callUnary("BootstrapPeer/Publish", arg)
  }
  
}

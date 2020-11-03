// swift-format-ignore-file
//
//  BootstrapClient.swift
//  App
//
//  Copyright © 2020 MemeLabs. All rights reserved.
//

import Foundation
import PromiseKit

class BootstrapClient {
  private var client: RPCClient

  init(_ client: RPCClient) {
    self.client = client
  }

  public func createClient(_ arg: PBCreateBootstrapClientRequest = PBCreateBootstrapClientRequest()) -> Promise<PBCreateBootstrapClientResponse> {
    return self.client.callUnary("Bootstrap/CreateClient", arg)
  }
  public func updateClient(_ arg: PBUpdateBootstrapClientRequest = PBUpdateBootstrapClientRequest()) -> Promise<PBUpdateBootstrapClientResponse> {
    return self.client.callUnary("Bootstrap/UpdateClient", arg)
  }
  public func deleteClient(_ arg: PBDeleteBootstrapClientRequest = PBDeleteBootstrapClientRequest()) -> Promise<PBDeleteBootstrapClientResponse> {
    return self.client.callUnary("Bootstrap/DeleteClient", arg)
  }
  public func getClient(_ arg: PBGetBootstrapClientRequest = PBGetBootstrapClientRequest()) -> Promise<PBGetBootstrapClientResponse> {
    return self.client.callUnary("Bootstrap/GetClient", arg)
  }
  public func listClients(_ arg: PBListBootstrapClientsRequest = PBListBootstrapClientsRequest()) -> Promise<PBListBootstrapClientsResponse> {
    return self.client.callUnary("Bootstrap/ListClients", arg)
  }
  public func listPeers(_ arg: PBListBootstrapPeersRequest = PBListBootstrapPeersRequest()) -> Promise<PBListBootstrapPeersResponse> {
    return self.client.callUnary("Bootstrap/ListPeers", arg)
  }
  public func publishNetworkToPeer(_ arg: PBPublishNetworkToBootstrapPeerRequest = PBPublishNetworkToBootstrapPeerRequest()) -> Promise<PBPublishNetworkToBootstrapPeerResponse> {
    return self.client.callUnary("Bootstrap/PublishNetworkToPeer", arg)
  }
  
}

// swift-format-ignore-file
//
//  NetworkPeerClient.swift
//  App
//
//  Copyright © 2020 MemeLabs. All rights reserved.
//

import Foundation
import PromiseKit

class NetworkPeerClient {
  private var client: RPCClient

  init(_ client: RPCClient) {
    self.client = client
  }

  public func negotiate(_ arg: PBNetworkPeerNegotiateRequest = PBNetworkPeerNegotiateRequest()) -> Promise<PBNetworkPeerNegotiateResponse> {
    return self.client.callUnary("NetworkPeer/Negotiate", arg)
  }
  public func open(_ arg: PBNetworkPeerOpenRequest = PBNetworkPeerOpenRequest()) -> Promise<PBNetworkPeerOpenResponse> {
    return self.client.callUnary("NetworkPeer/Open", arg)
  }
  public func close(_ arg: PBNetworkPeerCloseRequest = PBNetworkPeerCloseRequest()) -> Promise<PBNetworkPeerCloseResponse> {
    return self.client.callUnary("NetworkPeer/Close", arg)
  }
  public func updateCertificate(_ arg: PBNetworkPeerUpdateCertificateRequest = PBNetworkPeerUpdateCertificateRequest()) -> Promise<PBNetworkPeerUpdateCertificateResponse> {
    return self.client.callUnary("NetworkPeer/UpdateCertificate", arg)
  }
  
}

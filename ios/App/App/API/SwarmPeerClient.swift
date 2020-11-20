// swift-format-ignore-file
//
//  SwarmPeerClient.swift
//  App
//
//  Copyright © 2020 MemeLabs. All rights reserved.
//

import Foundation
import PromiseKit

class SwarmPeerClient {
  private var client: RPCClient

  init(_ client: RPCClient) {
    self.client = client
  }

  public func announceSwarm(_ arg: PBSwarmPeerAnnounceSwarmRequest = PBSwarmPeerAnnounceSwarmRequest()) -> Promise<PBSwarmPeerAnnounceSwarmResponse> {
    return self.client.callUnary("SwarmPeer/AnnounceSwarm", arg)
  }
  
}

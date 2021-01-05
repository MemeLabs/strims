// swift-format-ignore-file
//
//  TransferPeerClient.swift
//  App
//
//  Copyright © 2020 MemeLabs. All rights reserved.
//

import Foundation
import PromiseKit

class TransferPeerClient {
  private var client: RPCClient

  init(_ client: RPCClient) {
    self.client = client
  }

  public func announceSwarm(_ arg: PBTransferPeerAnnounceSwarmRequest = PBTransferPeerAnnounceSwarmRequest()) -> Promise<PBTransferPeerAnnounceSwarmResponse> {
    return self.client.callUnary("TransferPeer/AnnounceSwarm", arg)
  }
  
}

// swift-format-ignore-file
//
//  VideoIngressShareClient.swift
//  App
//
//  Copyright © 2020 MemeLabs. All rights reserved.
//

import Foundation
import PromiseKit

class VideoIngressShareClient {
  private var client: RPCClient

  init(_ client: RPCClient) {
    self.client = client
  }

  public func createChannel(_ arg: PBVideoIngressShareCreateChannelRequest = PBVideoIngressShareCreateChannelRequest()) -> Promise<PBVideoIngressShareCreateChannelResponse> {
    return self.client.callUnary("VideoIngressShare/CreateChannel", arg)
  }
  public func updateChannel(_ arg: PBVideoIngressShareUpdateChannelRequest = PBVideoIngressShareUpdateChannelRequest()) -> Promise<PBVideoIngressShareUpdateChannelResponse> {
    return self.client.callUnary("VideoIngressShare/UpdateChannel", arg)
  }
  public func deleteChannel(_ arg: PBVideoIngressShareDeleteChannelRequest = PBVideoIngressShareDeleteChannelRequest()) -> Promise<PBVideoIngressShareDeleteChannelResponse> {
    return self.client.callUnary("VideoIngressShare/DeleteChannel", arg)
  }
  
}

// swift-format-ignore-file
//
//  VideoChannelClient.swift
//  App
//
//  Copyright © 2020 MemeLabs. All rights reserved.
//

import Foundation
import PromiseKit

class VideoChannelClient {
  private var client: RPCClient

  init(_ client: RPCClient) {
    self.client = client
  }

  public func list(_ arg: PBVideoChannelListRequest = PBVideoChannelListRequest()) -> Promise<PBVideoChannelListResponse> {
    return self.client.callUnary("VideoChannel/List", arg)
  }
  public func create(_ arg: PBVideoChannelCreateRequest = PBVideoChannelCreateRequest()) -> Promise<PBVideoChannelCreateResponse> {
    return self.client.callUnary("VideoChannel/Create", arg)
  }
  public func update(_ arg: PBVideoChannelUpdateRequest = PBVideoChannelUpdateRequest()) -> Promise<PBVideoChannelUpdateResponse> {
    return self.client.callUnary("VideoChannel/Update", arg)
  }
  public func delete(_ arg: PBVideoChannelDeleteRequest = PBVideoChannelDeleteRequest()) -> Promise<PBVideoChannelDeleteResponse> {
    return self.client.callUnary("VideoChannel/Delete", arg)
  }
  
}

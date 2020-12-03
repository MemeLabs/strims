// swift-format-ignore-file
//
//  DirectoryClient.swift
//  App
//
//  Copyright © 2020 MemeLabs. All rights reserved.
//

import Foundation
import PromiseKit

class DirectoryClient {
  private var client: RPCClient

  init(_ client: RPCClient) {
    self.client = client
  }

  public func publish(_ arg: PBDirectoryPublishRequest = PBDirectoryPublishRequest()) -> Promise<PBDirectoryPublishResponse> {
    return self.client.callUnary("Directory/Publish", arg)
  }
  public func unpublish(_ arg: PBDirectoryUnpublishRequest = PBDirectoryUnpublishRequest()) -> Promise<PBDirectoryUnpublishResponse> {
    return self.client.callUnary("Directory/Unpublish", arg)
  }
  public func join(_ arg: PBDirectoryJoinRequest = PBDirectoryJoinRequest()) -> Promise<PBDirectoryJoinResponse> {
    return self.client.callUnary("Directory/Join", arg)
  }
  public func part(_ arg: PBDirectoryPartRequest = PBDirectoryPartRequest()) -> Promise<PBDirectoryPartResponse> {
    return self.client.callUnary("Directory/Part", arg)
  }
  public func ping(_ arg: PBDirectoryPingRequest = PBDirectoryPingRequest()) -> Promise<PBDirectoryPingResponse> {
    return self.client.callUnary("Directory/Ping", arg)
  }
  
}

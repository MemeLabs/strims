// swift-format-ignore-file
//
//  ProfileClient.swift
//  App
//
//  Copyright © 2020 MemeLabs. All rights reserved.
//

import Foundation
import PromiseKit

class ProfileClient {
  private var client: RPCClient

  init(_ client: RPCClient) {
    self.client = client
  }

  public func create(_ arg: PBCreateProfileRequest = PBCreateProfileRequest()) -> Promise<PBCreateProfileResponse> {
    return self.client.callUnary("Profile/Create", arg)
  }
  public func load(_ arg: PBLoadProfileRequest = PBLoadProfileRequest()) -> Promise<PBLoadProfileResponse> {
    return self.client.callUnary("Profile/Load", arg)
  }
  public func get(_ arg: PBGetProfileRequest = PBGetProfileRequest()) -> Promise<PBGetProfileResponse> {
    return self.client.callUnary("Profile/Get", arg)
  }
  public func update(_ arg: PBUpdateProfileRequest = PBUpdateProfileRequest()) -> Promise<PBUpdateProfileResponse> {
    return self.client.callUnary("Profile/Update", arg)
  }
  public func delete(_ arg: PBDeleteProfileRequest = PBDeleteProfileRequest()) -> Promise<PBDeleteProfileResponse> {
    return self.client.callUnary("Profile/Delete", arg)
  }
  public func list(_ arg: PBListProfilesRequest = PBListProfilesRequest()) -> Promise<PBListProfilesResponse> {
    return self.client.callUnary("Profile/List", arg)
  }
  public func loadSession(_ arg: PBLoadSessionRequest = PBLoadSessionRequest()) -> Promise<PBLoadSessionResponse> {
    return self.client.callUnary("Profile/LoadSession", arg)
  }
  
}

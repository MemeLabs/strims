// swift-format-ignore-file
//
//  NetworkClient.swift
//  App
//
//  Copyright © 2020 MemeLabs. All rights reserved.
//

import Foundation
import PromiseKit

class NetworkClient {
  private var client: RPCClient

  init(_ client: RPCClient) {
    self.client = client
  }

  public func create(_ arg: PBCreateNetworkRequest = PBCreateNetworkRequest()) -> Promise<PBCreateNetworkResponse> {
    return self.client.callUnary("Network/Create", arg)
  }
  public func update(_ arg: PBUpdateNetworkRequest = PBUpdateNetworkRequest()) -> Promise<PBUpdateNetworkResponse> {
    return self.client.callUnary("Network/Update", arg)
  }
  public func delete(_ arg: PBDeleteNetworkRequest = PBDeleteNetworkRequest()) -> Promise<PBDeleteNetworkResponse> {
    return self.client.callUnary("Network/Delete", arg)
  }
  public func get(_ arg: PBGetNetworkRequest = PBGetNetworkRequest()) -> Promise<PBGetNetworkResponse> {
    return self.client.callUnary("Network/Get", arg)
  }
  public func list(_ arg: PBListNetworksRequest = PBListNetworksRequest()) -> Promise<PBListNetworksResponse> {
    return self.client.callUnary("Network/List", arg)
  }
  public func createInvitation(_ arg: PBCreateNetworkInvitationRequest = PBCreateNetworkInvitationRequest()) -> Promise<PBCreateNetworkInvitationResponse> {
    return self.client.callUnary("Network/CreateInvitation", arg)
  }
  public func createFromInvitation(_ arg: PBCreateNetworkFromInvitationRequest = PBCreateNetworkFromInvitationRequest()) -> Promise<PBCreateNetworkFromInvitationResponse> {
    return self.client.callUnary("Network/CreateFromInvitation", arg)
  }
  public func startVPN(_ arg: PBStartVPNRequest = PBStartVPNRequest()) throws -> RPCResponseStream<PBNetworkEvent> {
    return try self.client.callStreaming("Network/StartVPN", arg)
  }
  public func stopVPN(_ arg: PBStopVPNRequest = PBStopVPNRequest()) -> Promise<PBStopVPNResponse> {
    return self.client.callUnary("Network/StopVPN", arg)
  }
  public func getDirectoryEvents(_ arg: PBGetDirectoryEventsRequest = PBGetDirectoryEventsRequest()) throws -> RPCResponseStream<PBDirectoryEvent> {
    return try self.client.callStreaming("Network/GetDirectoryEvents", arg)
  }
  public func testDirectoryPublish(_ arg: PBTestDirectoryPublishRequest = PBTestDirectoryPublishRequest()) -> Promise<PBTestDirectoryPublishResponse> {
    return self.client.callUnary("Network/TestDirectoryPublish", arg)
  }
  
}

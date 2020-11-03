// swift-format-ignore-file
//
//  ChatClient.swift
//  App
//
//  Copyright © 2020 MemeLabs. All rights reserved.
//

import Foundation
import PromiseKit

class ChatClient {
  private var client: RPCClient

  init(_ client: RPCClient) {
    self.client = client
  }

  public func createServer(_ arg: PBCreateChatServerRequest = PBCreateChatServerRequest()) -> Promise<PBCreateChatServerResponse> {
    return self.client.callUnary("Chat/CreateServer", arg)
  }
  public func updateServer(_ arg: PBUpdateChatServerRequest = PBUpdateChatServerRequest()) -> Promise<PBUpdateChatServerResponse> {
    return self.client.callUnary("Chat/UpdateServer", arg)
  }
  public func deleteServer(_ arg: PBDeleteChatServerRequest = PBDeleteChatServerRequest()) -> Promise<PBDeleteChatServerResponse> {
    return self.client.callUnary("Chat/DeleteServer", arg)
  }
  public func getServer(_ arg: PBGetChatServerRequest = PBGetChatServerRequest()) -> Promise<PBGetChatServerResponse> {
    return self.client.callUnary("Chat/GetServer", arg)
  }
  public func listServers(_ arg: PBListChatServersRequest = PBListChatServersRequest()) -> Promise<PBListChatServersResponse> {
    return self.client.callUnary("Chat/ListServers", arg)
  }
  public func openServer(_ arg: PBOpenChatServerRequest = PBOpenChatServerRequest()) throws -> RPCResponseStream<PBChatServerEvent> {
    return try self.client.callStreaming("Chat/OpenServer", arg)
  }
  public func openClient(_ arg: PBOpenChatClientRequest = PBOpenChatClientRequest()) throws -> RPCResponseStream<PBChatClientEvent> {
    return try self.client.callStreaming("Chat/OpenClient", arg)
  }
  public func callClient(_ arg: PBCallChatClientRequest = PBCallChatClientRequest()) -> Promise<PBCallChatClientResponse> {
    return self.client.callUnary("Chat/CallClient", arg)
  }
  
}

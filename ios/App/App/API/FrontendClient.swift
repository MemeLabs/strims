// swift-format-ignore-file
//
//  VideoClient.swift
//  App
//
//  Copyright © 2020 MemeLabs. All rights reserved.
//

class FrontendClient {
  private(set) public var bootstrap: BootstrapClient
  private(set) public var chat: ChatClient
  private(set) public var debug: DebugClient
  private(set) public var network: NetworkClient
  private(set) public var profile: ProfileClient
  private(set) public var video: VideoClient

  init(_ client: RPCClient = RPCClient()) {
    self.bootstrap = BootstrapClient(client)
    self.chat = ChatClient(client)
    self.debug = DebugClient(client)
    self.network = NetworkClient(client)
    self.profile = ProfileClient(client)
    self.video = VideoClient(client)
  }
}

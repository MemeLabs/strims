// swift-format-ignore-file
//
//  DirectoryFrontendClient.swift
//  App
//
//  Copyright © 2020 MemeLabs. All rights reserved.
//

import Foundation
import PromiseKit

class DirectoryFrontendClient {
  private var client: RPCClient

  init(_ client: RPCClient) {
    self.client = client
  }

  public func open(_ arg: PBDirectoryFrontendOpenRequest = PBDirectoryFrontendOpenRequest()) throws -> RPCResponseStream<PBDirectoryFrontendOpenResponse> {
    return try self.client.callStreaming("DirectoryFrontend/Open", arg)
  }
  public func test(_ arg: PBDirectoryFrontendTestRequest = PBDirectoryFrontendTestRequest()) -> Promise<PBDirectoryFrontendTestResponse> {
    return self.client.callUnary("DirectoryFrontend/Test", arg)
  }
  
}

// swift-format-ignore-file
//
//  RPCTestClient.swift
//  App
//
//  Copyright © 2020 MemeLabs. All rights reserved.
//

import Foundation
import PromiseKit

class RPCTestClient {
  private var client: RPCClient

  init(_ client: RPCClient) {
    self.client = client
  }

  public func callUnary(_ arg: PBRPCCallUnaryRequest = PBRPCCallUnaryRequest()) -> Promise<PBRPCCallUnaryResponse> {
    return self.client.callUnary("RPCTest/CallUnary", arg)
  }
  public func callStream(_ arg: PBRPCCallStreamRequest = PBRPCCallStreamRequest()) throws -> RPCResponseStream<PBRPCCallStreamResponse> {
    return try self.client.callStreaming("RPCTest/CallStream", arg)
  }
  
}

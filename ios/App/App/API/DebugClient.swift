// swift-format-ignore-file
//
//  DebugClient.swift
//  App
//
//  Copyright © 2020 MemeLabs. All rights reserved.
//

import Foundation
import PromiseKit

class DebugClient {
  private var client: RPCClient

  init(_ client: RPCClient) {
    self.client = client
  }

  public func pProf(_ arg: PBPProfRequest = PBPProfRequest()) -> Promise<PBPProfResponse> {
    return self.client.callUnary("Debug/PProf", arg)
  }
  public func readMetrics(_ arg: PBReadMetricsRequest = PBReadMetricsRequest()) -> Promise<PBReadMetricsResponse> {
    return self.client.callUnary("Debug/ReadMetrics", arg)
  }
  
}

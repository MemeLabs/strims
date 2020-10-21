// swift-format-ignore-file
//
//  DebugClient.swift
//  App
//
//  Copyright © 2020 MemeLabs. All rights reserved.
//

import Foundation
import PromiseKit

class DebugClient: RPCClient {
  public func pProf(_ arg: PBPProfRequest = PBPProfRequest()) -> Promise<PBPProfResponse> {
    return self.callUnary("Debug/PProf", arg)
  }
  public func readMetrics(_ arg: PBReadMetricsRequest = PBReadMetricsRequest()) -> Promise<PBReadMetricsResponse> {
    return self.callUnary("Debug/ReadMetrics", arg)
  }
  
}

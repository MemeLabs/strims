// swift-format-ignore-file
//
//  FundingClient.swift
//  App
//
//  Copyright © 2020 MemeLabs. All rights reserved.
//

import Foundation
import PromiseKit

class FundingClient {
  private var client: RPCClient

  init(_ client: RPCClient) {
    self.client = client
  }

  public func test(_ arg: PBFundingTestRequest = PBFundingTestRequest()) -> Promise<PBFundingTestResponse> {
    return self.client.callUnary("Funding/Test", arg)
  }
  
}

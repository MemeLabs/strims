// swift-format-ignore-file
//
//  CAPeerClient.swift
//  App
//
//  Copyright © 2020 MemeLabs. All rights reserved.
//

import Foundation
import PromiseKit

class CAPeerClient {
  private var client: RPCClient

  init(_ client: RPCClient) {
    self.client = client
  }

  public func renew(_ arg: PBCAPeerRenewRequest = PBCAPeerRenewRequest()) -> Promise<PBCAPeerRenewResponse> {
    return self.client.callUnary("CAPeer/Renew", arg)
  }
  
}

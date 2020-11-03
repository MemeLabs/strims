// swift-format-ignore-file
//
//  CAClient.swift
//  App
//
//  Copyright © 2020 MemeLabs. All rights reserved.
//

import Foundation
import PromiseKit

class CAClient {
  private var client: RPCClient

  init(_ client: RPCClient) {
    self.client = client
  }

  public func renew(_ arg: PBCARenewRequest = PBCARenewRequest()) -> Promise<PBCARenewResponse> {
    return self.client.callUnary("CA/Renew", arg)
  }
  
}

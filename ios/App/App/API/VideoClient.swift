// swift-format-ignore-file
//
//  VideoClient.swift
//  App
//
//  Copyright © 2020 MemeLabs. All rights reserved.
//

import Foundation
import PromiseKit

class VideoClient {
  private var client: RPCClient

  init(_ client: RPCClient) {
    self.client = client
  }

  public func openClient(_ arg: PBOpenVideoClientRequest = PBOpenVideoClientRequest()) throws -> RPCResponseStream<PBVideoClientEvent> {
    return try self.client.callStreaming("Video/OpenClient", arg)
  }
  public func openServer(_ arg: PBOpenVideoServerRequest = PBOpenVideoServerRequest()) -> Promise<PBVideoServerOpenResponse> {
    return self.client.callUnary("Video/OpenServer", arg)
  }
  public func writeToServer(_ arg: PBWriteToVideoServerRequest = PBWriteToVideoServerRequest()) -> Promise<PBWriteToVideoServerResponse> {
    return self.client.callUnary("Video/WriteToServer", arg)
  }
  public func publishSwarm(_ arg: PBPublishSwarmRequest = PBPublishSwarmRequest()) -> Promise<PBPublishSwarmResponse> {
    return self.client.callUnary("Video/PublishSwarm", arg)
  }
  public func startRTMPIngress(_ arg: PBStartRTMPIngressRequest = PBStartRTMPIngressRequest()) -> Promise<PBStartRTMPIngressResponse> {
    return self.client.callUnary("Video/StartRTMPIngress", arg)
  }
  public func startHLSEgress(_ arg: PBStartHLSEgressRequest = PBStartHLSEgressRequest()) -> Promise<PBStartHLSEgressResponse> {
    return self.client.callUnary("Video/StartHLSEgress", arg)
  }
  public func stopHLSEgress(_ arg: PBStopHLSEgressRequest = PBStopHLSEgressRequest()) -> Promise<PBStopHLSEgressResponse> {
    return self.client.callUnary("Video/StopHLSEgress", arg)
  }
  
}

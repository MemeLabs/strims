// swift-format-ignore-file
//
//  VideoClient.swift
//  App
//
//  Copyright © 2020 MemeLabs. All rights reserved.
//

import Foundation
import PromiseKit

class VideoClient: RPCClient {
  public func openClient(_ arg: PBOpenVideoClientRequest = PBOpenVideoClientRequest()) throws -> RPCResponseStream<PBVideoClientEvent> {
    return try self.callStreaming("Video/OpenClient", arg)
  }
  public func openServer(_ arg: PBOpenVideoServerRequest = PBOpenVideoServerRequest()) -> Promise<PBVideoServerOpenResponse> {
    return self.callUnary("Video/OpenServer", arg)
  }
  public func writeToServer(_ arg: PBWriteToVideoServerRequest = PBWriteToVideoServerRequest()) -> Promise<PBWriteToVideoServerResponse> {
    return self.callUnary("Video/WriteToServer", arg)
  }
  public func publishSwarm(_ arg: PBPublishSwarmRequest = PBPublishSwarmRequest()) -> Promise<PBPublishSwarmResponse> {
    return self.callUnary("Video/PublishSwarm", arg)
  }
  public func startRTMPIngress(_ arg: PBStartRTMPIngressRequest = PBStartRTMPIngressRequest()) -> Promise<PBStartRTMPIngressResponse> {
    return self.callUnary("Video/StartRTMPIngress", arg)
  }
  public func startHLSEgress(_ arg: PBStartHLSEgressRequest = PBStartHLSEgressRequest()) -> Promise<PBStartHLSEgressResponse> {
    return self.callUnary("Video/StartHLSEgress", arg)
  }
  public func stopHLSEgress(_ arg: PBStopHLSEgressRequest = PBStopHLSEgressRequest()) -> Promise<PBStopHLSEgressResponse> {
    return self.callUnary("Video/StopHLSEgress", arg)
  }
  
}

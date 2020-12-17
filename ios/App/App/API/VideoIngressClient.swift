// swift-format-ignore-file
//
//  VideoIngressClient.swift
//  App
//
//  Copyright © 2020 MemeLabs. All rights reserved.
//

import Foundation
import PromiseKit

class VideoIngressClient {
  private var client: RPCClient

  init(_ client: RPCClient) {
    self.client = client
  }

  public func isSupported(_ arg: PBVideoIngressIsSupportedRequest = PBVideoIngressIsSupportedRequest()) -> Promise<PBVideoIngressIsSupportedResponse> {
    return self.client.callUnary("VideoIngress/IsSupported", arg)
  }
  public func getConfig(_ arg: PBVideoIngressGetConfigRequest = PBVideoIngressGetConfigRequest()) -> Promise<PBVideoIngressGetConfigResponse> {
    return self.client.callUnary("VideoIngress/GetConfig", arg)
  }
  public func setConfig(_ arg: PBVideoIngressSetConfigRequest = PBVideoIngressSetConfigRequest()) -> Promise<PBVideoIngressSetConfigResponse> {
    return self.client.callUnary("VideoIngress/SetConfig", arg)
  }
  public func listStreams(_ arg: PBVideoIngressListStreamsRequest = PBVideoIngressListStreamsRequest()) -> Promise<PBVideoIngressListStreamsResponse> {
    return self.client.callUnary("VideoIngress/ListStreams", arg)
  }
  public func listChannels(_ arg: PBVideoIngressListChannelsRequest = PBVideoIngressListChannelsRequest()) -> Promise<PBVideoIngressListChannelsResponse> {
    return self.client.callUnary("VideoIngress/ListChannels", arg)
  }
  public func createChannel(_ arg: PBVideoIngressCreateChannelRequest = PBVideoIngressCreateChannelRequest()) -> Promise<PBVideoIngressCreateChannelResponse> {
    return self.client.callUnary("VideoIngress/CreateChannel", arg)
  }
  public func updateChannel(_ arg: PBVideoIngressUpdateChannelRequest = PBVideoIngressUpdateChannelRequest()) -> Promise<PBVideoIngressUpdateChannelResponse> {
    return self.client.callUnary("VideoIngress/UpdateChannel", arg)
  }
  public func deleteChannel(_ arg: PBVideoIngressDeleteChannelRequest = PBVideoIngressDeleteChannelRequest()) -> Promise<PBVideoIngressDeleteChannelResponse> {
    return self.client.callUnary("VideoIngress/DeleteChannel", arg)
  }
  public func getChannelURL(_ arg: PBVideoIngressGetChannelURLRequest = PBVideoIngressGetChannelURLRequest()) -> Promise<PBVideoIngressGetChannelURLResponse> {
    return self.client.callUnary("VideoIngress/GetChannelURL", arg)
  }
  
}

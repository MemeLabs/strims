import { Readable, Writable } from "stream";

import { RPCHost } from "@memelabs/protobuf/lib/rpc/host";

import { ChatClient } from "./strims/chat/v1/chat_rpc";
import { DebugClient } from "./strims/debug/v1/debug_rpc";
import { FundingClient as FundingServiceClient } from "./strims/funding/v1/funding_rpc";
import { BootstrapFrontendClient } from "./strims/network/v1/bootstrap/bootstrap_rpc";
import { DirectoryFrontendClient } from "./strims/network/v1/directory_rpc";
import { NetworkServiceClient } from "./strims/network/v1/network_rpc";
import { ProfileServiceClient } from "./strims/profile/v1/profile_rpc";
import { CaptureClient as VideoCaptureClient } from "./strims/video/v1/capture_rpc";
import { VideoChannelFrontendClient } from "./strims/video/v1/channel_rpc";
import { EgressClient as VideoEgressClient } from "./strims/video/v1/egress_rpc";
import { VideoIngressClient } from "./strims/video/v1/ingress_rpc";

export class FrontendClient {
  public bootstrap: BootstrapFrontendClient;
  public chat: ChatClient;
  public debug: DebugClient;
  public directory: DirectoryFrontendClient;
  public network: NetworkServiceClient;
  public profile: ProfileServiceClient;
  public videoCapture: VideoCaptureClient;
  public videoChannel: VideoChannelFrontendClient;
  public videoEgress: VideoEgressClient;
  public videoIngress: VideoIngressClient;

  constructor(w: Writable, r: Readable) {
    const host = new RPCHost(w, r);
    this.bootstrap = new BootstrapFrontendClient(host);
    this.chat = new ChatClient(host);
    this.debug = new DebugClient(host);
    this.directory = new DirectoryFrontendClient(host);
    this.network = new NetworkServiceClient(host);
    this.profile = new ProfileServiceClient(host);
    this.videoCapture = new VideoCaptureClient(host);
    this.videoChannel = new VideoChannelFrontendClient(host);
    this.videoEgress = new VideoEgressClient(host);
    this.videoIngress = new VideoIngressClient(host);
  }
}

export class FundingClient {
  public funding: FundingServiceClient;

  constructor(w: Writable, r: Readable) {
    const host = new RPCHost(w, r);
    this.funding = new FundingServiceClient(host);
  }
}

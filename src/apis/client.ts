// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Readable, Writable } from "stream";

import Host, { type HostOptions } from "@memelabs/protobuf/lib/rpc/host";

import { AuthFrontendClient } from "./strims/auth/v1/auth_rpc.pb";
import { AutoseedFrontendClient } from "./strims/autoseed/v1/autoseed_rpc.pb";
import { ChatFrontendClient, ChatServerFrontendClient } from "./strims/chat/v1/chat_rpc.pb";
import { DebugClient } from "./strims/debug/v1/debug_rpc.pb";
import { DevToolsClient as DevToolsServiceClient } from "./strims/devtools/v1/devtools_rpc.pb";
import { CapConnClient } from "./strims/devtools/v1/ppspp/capconn_rpc.pb";
import { BootstrapFrontendClient } from "./strims/network/v1/bootstrap/bootstrap_rpc.pb";
import { DirectoryFrontendClient } from "./strims/network/v1/directory/directory_rpc.pb";
import { InviteLinkClient } from "./strims/network/v1/invite/invite_rpc.pb";
import { NetworkFrontendClient } from "./strims/network/v1/network_rpc.pb";
import { NotificationFrontendClient } from "./strims/notification/v1/notification_rpc.pb";
import { ProfileFrontendClient } from "./strims/profile/v1/profile_rpc.pb";
import { ReplicationFrontendClient } from "./strims/replication/v1/replication_rpc.pb";
import { CaptureClient as VideoCaptureClient } from "./strims/video/v1/capture_rpc.pb";
import { VideoChannelFrontendClient } from "./strims/video/v1/channel_rpc.pb";
import { EgressClient as VideoEgressClient } from "./strims/video/v1/egress_rpc.pb";
import { HLSEgressClient } from "./strims/video/v1/hls_egress_rpc.pb";
import { VideoIngressClient } from "./strims/video/v1/ingress_rpc.pb";
import { VNICFrontendClient } from "./strims/vnic/v1/vnic_rpc.pb";

// The node serves the UI's connection with strims-rpc's Config::frontend(),
// whose Calls run to 10 MiB, past Host's 512 KiB default.
const frontendHostOptions: HostOptions = { maxMessageBytes: 10 * 1024 * 1024 };

export class FrontendClient {
  public auth: AuthFrontendClient;
  public autoseed: AutoseedFrontendClient;
  public bootstrap: BootstrapFrontendClient;
  public chat: ChatFrontendClient;
  public chatServer: ChatServerFrontendClient;
  public debug: DebugClient;
  public directory: DirectoryFrontendClient;
  public network: NetworkFrontendClient;
  public notification: NotificationFrontendClient;
  public profile: ProfileFrontendClient;
  public replication: ReplicationFrontendClient;
  public videoCapture: VideoCaptureClient;
  public videoChannel: VideoChannelFrontendClient;
  public videoEgress: VideoEgressClient;
  public videoIngress: VideoIngressClient;
  public hlsEgress: HLSEgressClient;
  public vnic: VNICFrontendClient;

  constructor(w: Writable, r: Readable) {
    const host = new Host(w, r, undefined, frontendHostOptions);
    this.auth = new AuthFrontendClient(host);
    this.autoseed = new AutoseedFrontendClient(host);
    this.bootstrap = new BootstrapFrontendClient(host);
    this.chat = new ChatFrontendClient(host);
    this.chatServer = new ChatServerFrontendClient(host);
    this.debug = new DebugClient(host);
    this.directory = new DirectoryFrontendClient(host);
    this.network = new NetworkFrontendClient(host);
    this.notification = new NotificationFrontendClient(host);
    this.profile = new ProfileFrontendClient(host);
    this.replication = new ReplicationFrontendClient(host);
    this.videoCapture = new VideoCaptureClient(host);
    this.videoChannel = new VideoChannelFrontendClient(host);
    this.videoEgress = new VideoEgressClient(host);
    this.videoIngress = new VideoIngressClient(host);
    this.hlsEgress = new HLSEgressClient(host);
    this.vnic = new VNICFrontendClient(host);
  }
}

export class DevToolsClient {
  public devTools: DevToolsServiceClient;
  public ppsppCapConn: CapConnClient;

  constructor(w: Writable, r: Readable) {
    const host = new Host(w, r, undefined, frontendHostOptions);
    this.devTools = new DevToolsServiceClient(host);
    this.ppsppCapConn = new CapConnClient(host);
  }
}

export class InviteClient {
  public inviteLink: InviteLinkClient;

  constructor(w: Writable, r: Readable) {
    const host = new Host(w, r);
    this.inviteLink = new InviteLinkClient(host);
  }
}

export type Client = FrontendClient | DevToolsClient | InviteClient;

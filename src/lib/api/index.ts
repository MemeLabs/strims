import { Readable, Writable } from "stream";

import { RPCHost } from "../rpc/host";
import Bootstrap from "./bootstrapClient";
import Chat from "./chatClient";
import Debug from "./debugClient";
import Funding from "./fundingClient";
import Network from "./networkClient";
import Profile from "./profileClient";
import VideoChannel from "./videoChannelClient";
import Video from "./videoClient";
import VideoIngress from "./videoIngressClient";

export class FrontendClient {
  public bootstrap: Bootstrap;
  public debug: Debug;
  public network: Network;
  public profile: Profile;
  public chat: Chat;
  public video: Video;
  public videoIngress: VideoIngress;
  public videoChannel: VideoChannel;

  constructor(w: Writable, r: Readable) {
    const host = new RPCHost(w, r);
    this.bootstrap = new Bootstrap(host);
    this.debug = new Debug(host);
    this.network = new Network(host);
    this.profile = new Profile(host);
    this.chat = new Chat(host);
    this.video = new Video(host);
    this.videoIngress = new VideoIngress(host);
    this.videoChannel = new VideoChannel(host);
  }
}

export class FundingClient {
  public funding: Funding;

  constructor(w: Writable, r: Readable) {
    const host = new RPCHost(w, r);
    this.funding = new Funding(host);
  }
}

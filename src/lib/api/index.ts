import { Readable, Writable } from "stream";

import { RPCHost } from "../rpc/host";
import Bootstrap from "./bootstrapClient";
import Chat from "./chatClient";
import Debug from "./debugClient";
import Directory from "./directoryFrontendClient";
import Funding from "./fundingClient";
import Network from "./networkClient";
import Profile from "./profileClient";
import VideoChannel from "./videoChannelClient";
import Video from "./videoClient";
import VideoIngress from "./videoIngressClient";

export class FrontendClient {
  public bootstrap: Bootstrap;
  public chat: Chat;
  public debug: Debug;
  public directory: Directory;
  public network: Network;
  public profile: Profile;
  public video: Video;
  public videoChannel: VideoChannel;
  public videoIngress: VideoIngress;

  constructor(w: Writable, r: Readable) {
    const host = new RPCHost(w, r);
    this.bootstrap = new Bootstrap(host);
    this.chat = new Chat(host);
    this.debug = new Debug(host);
    this.directory = new Directory(host);
    this.network = new Network(host);
    this.profile = new Profile(host);
    this.video = new Video(host);
    this.videoChannel = new VideoChannel(host);
    this.videoIngress = new VideoIngress(host);
  }
}

export class FundingClient {
  public funding: Funding;

  constructor(w: Writable, r: Readable) {
    const host = new RPCHost(w, r);
    this.funding = new Funding(host);
  }
}

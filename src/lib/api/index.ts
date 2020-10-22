import { Readable, Writable } from "stream";

import { RPCHost } from "../rpc/host";
import Bootstrap from "./bootstrapClient";
import Chat from "./chatClient";
import Debug from "./debugClient";
import Network from "./networkClient";
import Profile from "./profileClient";
import Video from "./videoClient";

export default class Client {
  public bootstrap: Bootstrap;
  public debug: Debug;
  public network: Network;
  public profile: Profile;
  public chat: Chat;
  public video: Video;

  constructor(w: Writable, r: Readable) {
    const host = new RPCHost(w, r);
    this.bootstrap = new Bootstrap(host);
    this.debug = new Debug(host);
    this.network = new Network(host);
    this.profile = new Profile(host);
    this.chat = new Chat(host);
    this.video = new Video(host);
  }
}
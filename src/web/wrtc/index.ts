import Peer from "./peer";
import wsSignal from "./ws_signal";

export interface DialOptions {
  signalProtocol: string
  signalAddress: string
  id: string
}

export class Bridge {
  public ondata: (cid: number, n: number) => void;
  public onclose: (cid: number) => void;
  private peers: Map<number, Peer>;
  private data: Uint8Array;
  private nextCID: number;

  constructor() {
    this.peers = new Map();
    this.data = new Uint8Array(32000);
    this.nextCID = 0;
  }

  public write(cid: number, n: number) {
    // console.log("write", n, this.data.subarray(0, n));
    const c = this.peers.get(cid);
    if (c) {
      c.dc.send(this.data.subarray(0, n));
    }
  }

  public close(cid: number) {
    const c = this.peers.get(cid);
    if (c) {
      c.close();
      this.peers.delete(cid);
    }
  }

  // TODO: dial via channel or via signal server...
  public dial({signalProtocol, signalAddress, id}: DialOptions, cb: (id: number) => void): number {
    const {offer, sendAnswer} = wsSignal(`${signalProtocol}://${signalAddress}/${id}`);

    const pc = new RTCPeerConnection({
      iceServers: [
        {
          urls: ["stun:stun.l.google.com:19302"],
        },
      ],
    });

    const cid = this.nextCID++;

    (async () => {
      pc.ondatachannel = ({channel}) => {
        channel.binaryType = "arraybuffer";

        channel.onmessage = (e) => {
          this.data.set(new Uint8Array(e.data));
          this.ondata(cid, e.data.byteLength);
        };

        this.peers.set(cid, new Peer(cid, pc, channel));
        cb(cid);
      };

      pc.oniceconnectionstatechange = () => {
        if (pc.iceConnectionState === "closed") {
          this.onclose(cid);
        }
      };

      await pc.setRemoteDescription(await offer);

      const answer = await pc.createAnswer();
      await pc.setLocalDescription(answer);
      sendAnswer(answer);
    })();

    return;
  }
}

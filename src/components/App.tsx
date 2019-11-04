import * as React from "react";
import VideoPlayer from "./VideoPlayer";

import p2p from "../p2p.go";

const signal = (url: string) => {
  const ws = new WebSocket(url);

  const offer = new Promise<RTCSessionDescription>((resolve, reject) => {
    ws.onmessage = (e) => resolve(new RTCSessionDescription(JSON.parse(e.data)));
    ws.onerror = (e) => reject(e);
  });

  const sendAnswer = (a) => {
    ws.send(JSON.stringify(a));
    ws.close();
  };

  return {offer, sendAnswer};
};

class WRTCPeer {
  public id: number;
  public pc: RTCPeerConnection;
  public dc: RTCDataChannel;

  constructor(id: number, pc: RTCPeerConnection, dc: RTCDataChannel) {
    this.id = id;
    this.pc = pc;
    this.dc = dc;
  }

  public close() {
    this.pc.close();
  }
}

interface DialOptions {
  signalProtocol: string
  signalAddress: string
  id: string
}

class WRTCImpl {
  public ondata: (cid: number, n: number) => void;
  public onclose: (cid: number) => void;
  private peers: Map<number, WRTCPeer>;
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
    const {offer, sendAnswer} = signal(`${signalProtocol}://${signalAddress}/${id}`);

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

        this.peers.set(cid, new WRTCPeer(cid, pc, channel));
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

class Reader {
  public ondata: (n: number) => void;
  public onflush: () => void;
  public data: Uint8Array;

  constructor() {
    this.data = new Uint8Array(4096);
  }
}

const useP2P = (): [Reader, (sid: string) => void] => {
  const [reader] = React.useState(() => new Reader());

  const init = async (sid: string) => {
    p2p.init(sid, new WRTCImpl(), reader);
  };

  return [reader, init];
};

const App = () => {
  const [reader, init] = useP2P();
  const input = React.useRef(null);

  const handleClick = () => {
    if (input.current) {
      init(input.current.value);
    }
  };

  return (
    <div>
      <input ref={input} />
      <button onClick={handleClick}>submit</button>
      <VideoPlayer reader={reader} />
    </div>
  );
};

export default App;

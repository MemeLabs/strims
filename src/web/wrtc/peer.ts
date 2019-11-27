export default class WRTCPeer {
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

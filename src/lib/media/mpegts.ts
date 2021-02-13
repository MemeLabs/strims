/* eslint-disable */
import muxjs from "mux.js";

import { Source } from "./source";

const MIME_TYPE = "video/mp4;codecs=mp4a.40.5,avc1.64001F";

export class Decoder {
  public source: Source;
  private transmuxer: muxjs.mp4.Transmuxer;
  private initSet: boolean;

  public constructor() {
    this.source = new Source(MIME_TYPE);
    this.initTransmuxer();
  }

  private initTransmuxer() {
    this.transmuxer = new muxjs.mp4.Transmuxer();
    this.transmuxer.on("data", this.handleData.bind(this));
  }

  public reset() {
    this.initTransmuxer();
    this.source.reset();
  }

  public write(b: Uint8Array): void {
    this.transmuxer.push(b);
  }

  public flush(): void {
    this.transmuxer.flush();
  }

  private handleData(event) {
    if (event.type === "combined") {
      const b = this.initSet
        ? event.data
        : Buffer.concat([Buffer.from(event.initSegment), Buffer.from(event.data)]);
      this.initSet = true;

      this.source.appendBuffer(b);
    } else {
      console.warn("unhandled event", event.type);
    }
  }
}

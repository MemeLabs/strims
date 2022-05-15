// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Playlist } from "./hls";
import { Source } from "./source";

const MIME_TYPE = "video/mp4;codecs=mp4a.40.5,avc1.64001F";

export class Decoder {
  public source: Source;
  private headerRead: boolean = false;

  public constructor() {
    this.source = new Source(MIME_TYPE);
  }

  public reset(): void {
    this.headerRead = false;
    this.source.reset();
  }

  public write(b: Uint8Array): void {
    if (!this.headerRead) {
      b = b.slice(2);
      this.headerRead = true;
    }

    this.source.appendBuffer(b);
  }

  public flush(): void {
    this.headerRead = false;
  }
}

export class Relay {
  public playlist: Playlist;
  private buffer: ArrayBuffer[] = [];
  private headerRead: boolean = false;
  private initWritten: boolean = false;
  private initLength: number;

  public constructor() {
    this.playlist = new Playlist({ waitForInit: true });
  }

  public reset() {
    this.buffer = [];
    this.headerRead = false;
    this.initWritten = false;
    this.playlist.reset();
  }

  public write(b: Uint8Array): void {
    if (!this.headerRead) {
      this.initLength = (b[0] << 8) | b[1];
      b = b.slice(2);
      this.headerRead = true;
    }

    this.buffer.push(b);
  }

  public flush(): void {
    const initBuffer: ArrayBuffer[] = [];
    for (let n = this.initLength; n > 0; ) {
      const b = this.buffer[0];

      if (n > b.byteLength) {
        n -= b.byteLength;
        initBuffer.push(this.buffer.shift());
        continue;
      }

      initBuffer.push(b.slice(0, n));
      this.buffer[0] = b.slice(n);
      break;
    }

    if (!this.initWritten) {
      this.playlist.setInitSegment("mp4", new Blob(initBuffer, { type: "video/mp4" }));
      this.initWritten = true;
    }
    this.headerRead = false;

    this.playlist.appendSegment("m4s", new Blob(this.buffer, { type: "video/iso.segment" }));
    this.buffer = [];
  }
}

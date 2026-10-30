// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Playlist } from "./hls";
import { hasCodecs } from "./mime";
import { Source } from "./source";

// The type of listings that name no codecs.
const DEFAULT_MIME_TYPE = "video/mp4;codecs=mp4a.40.5,avc1.64001F";

// The SourceBuffer type of a stream listed as `mime`: the listing's own,
// whose codecs come from the stream's decoder configs, when it names them
// and the browser plays them.
export const sourceType = (mime: string): string =>
  hasCodecs(mime) && MediaSource.isTypeSupported(mime) ? mime : DEFAULT_MIME_TYPE;

export class Decoder {
  public source: Source;
  private headerRead: boolean = false;

  public constructor(mimeType: string) {
    this.source = new Source(sourceType(mimeType));
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
  private buffer: Uint8Array<ArrayBuffer>[] = [];
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

    this.buffer.push(b as Uint8Array<ArrayBuffer>);
  }

  public flush(): void {
    const initBuffer: Uint8Array<ArrayBuffer>[] = [];
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

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

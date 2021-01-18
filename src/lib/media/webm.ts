import * as ebml from "ts-ebml";

import { Source } from "./source";
import WorkQueue from "./workqueue";

export const MIME_TYPE = "video/webm;codecs=vp9,opus";

export class Encoder {
  public ondata: (b: Uint8Array) => void;
  public onend: (b: Uint8Array) => void;

  private mediaRecorder: MediaRecorder;
  private fileReader: FileReader;
  private fileReaderTasks: WorkQueue;
  private ebmlDecoder: ebml.Decoder = new ebml.Decoder();
  private ebmlEncoder: ebml.Encoder = new ebml.Encoder();
  private header: Uint8Array;
  private dataWritten: boolean = false;
  private flushInterval: ReturnType<typeof setInterval>;

  public constructor(stream: MediaStream) {
    this.fileReader = new FileReader();
    this.fileReader.onloadend = this.handleLoadEnd.bind(this);
    this.fileReaderTasks = new WorkQueue();

    const options = {
      audioBitsPerSecond: 128000,
      videoBitsPerSecond: 6000000,
      mimeType: MIME_TYPE,
    };
    this.mediaRecorder = new MediaRecorder(stream, options);

    this.mediaRecorder.ondataavailable = this.handleDataAvailable.bind(this);
    // this.mediaRecorder.onerror = (e) => console.log(new Date(), "onerror", e);
    // this.mediaRecorder.onpause = (e) => console.log(new Date(), "onpause", e);
    // this.mediaRecorder.onresume = (e) => console.log(new Date(), "onresume", e);
    // this.mediaRecorder.onstart = (e) => console.log(new Date(), "onstart", e);
    // this.mediaRecorder.onstop = (e) => console.log(new Date(), "onstop", e);

    this.flushInterval = setInterval(() => this.mediaRecorder.requestData(), 50);

    this.mediaRecorder.start();
  }

  public stop(): void {
    this.mediaRecorder.stop();
    this.onend(new Uint8Array());
    clearInterval(this.flushInterval);
  }

  private handleDataAvailable(e: BlobEvent): void {
    if (e.data.size !== 0) {
      this.fileReaderTasks.insert(() => this.fileReader.readAsArrayBuffer(e.data));
    }
  }

  private handleLoadEnd(): void {
    this.fileReaderTasks.runNext();

    const buf = new Uint8Array(this.fileReader.result as ArrayBuffer);
    let elms = this.ebmlDecoder.decode(buf);
    // console.log(elms);
    let clusterIndex = elms.findIndex(({ name }) => name === "Cluster");

    if (this.header === undefined) {
      const header = this.ebmlEncoder.encode(elms.slice(0, clusterIndex));
      const headerBytes = header.byteLength;

      this.header = new Uint8Array(headerBytes + 2);
      this.header[0] = headerBytes;
      this.header[1] = headerBytes >> 8;
      this.header.set(new Uint8Array(header), 2);

      elms = elms.slice(clusterIndex);
      clusterIndex = 0;
    }

    if (clusterIndex === -1) {
      this.ondata(buf);
      return;
    }

    if (clusterIndex !== 0 || this.dataWritten) {
      this.onend(new Uint8Array(this.ebmlEncoder.encode(elms.slice(0, clusterIndex))));
    }

    const cluster = new Uint8Array(this.ebmlEncoder.encode(elms.slice(clusterIndex)));
    const data = new Uint8Array(this.header.byteLength + cluster.byteLength);
    data.set(this.header);
    data.set(cluster, this.header.byteLength);
    this.ondata(data);

    this.dataWritten = true;
  }
}

export class Decoder {
  public source: Source;
  private headerRead: boolean = false;
  private clusterRead: boolean = false;

  public constructor() {
    this.source = new Source(MIME_TYPE);
  }

  public write(b: Uint8Array): void {
    if (!this.headerRead) {
      const headerBytes = (b[1] << 8) + b[0];
      // console.log("truncate 2");
      b = b.slice(2);

      if (this.clusterRead) {
        // console.log("truncate", headerBytes);
        b = b.slice(headerBytes);
      }

      this.headerRead = true;
      this.clusterRead = true;
    }

    this.source.appendBuffer(b);
  }

  public flush(): void {
    this.headerRead = false;
  }
}

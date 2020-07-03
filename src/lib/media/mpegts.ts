import muxjs from "mux.js";

import WorkQueue from "./workqueue";

const MIME_TYPE = "video/mp4;codecs=mp4a.40.5,avc1.64001F";

// TODO: prune output buffer
export class Decoder {
  public readonly mediaSource: MediaSource;
  private sourceBuffer: SourceBuffer;
  private sourceBufferTasks: WorkQueue;
  private transmuxer: muxjs.mp4.Transmuxer;
  private initSet: boolean;

  public constructor() {
    this.sourceBufferTasks = new WorkQueue();
    this.sourceBufferTasks.pause();

    this.mediaSource = new MediaSource();

    this.transmuxer = new muxjs.mp4.Transmuxer();
    this.transmuxer.on("data", this.handleData.bind(this));

    this.mediaSource.onsourceopen = () => {
      this.sourceBuffer = this.mediaSource.addSourceBuffer(MIME_TYPE);
      this.sourceBuffer.onupdateend = this.sourceBufferTasks.runNext.bind(this.sourceBufferTasks);
      // this.sourceBuffer.onerror = (e) => console.log("onerror", e);
      // this.sourceBuffer.onabort = (e) => console.log("onabort", e);

      this.sourceBufferTasks.runNext();
    };
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

      this.sourceBufferTasks.insert(() => this.sourceBuffer.appendBuffer(b));
      this.sourceBufferTasks.insert(() => this.prune());
    } else {
      console.warn("unhandled event", event.type);
    }
  }

  public end(): number {
    const { buffered } = this.sourceBuffer;

    if (buffered.length === 0) {
      return -1;
    }

    return buffered.end(buffered.length - 1);
  }

  private prune() {
    const { buffered } = this.sourceBuffer;

    if (buffered.length === 0) {
      this.sourceBufferTasks.runNext();
      return;
    }

    const start = buffered.start(0);
    const end = buffered.end(buffered.length - 1);
    // console.log({ start, end });

    if (end - start <= 10) {
      this.sourceBufferTasks.runNext();
      return;
    }

    this.sourceBuffer.remove(0, end - 10);
  }
}

import WorkQueue from "./workqueue";

const MIME_TYPE = "video/mp4;codecs=mp4a.40.5,avc1.64001F";

// TODO: prune output buffer
export class Decoder {
  public readonly mediaSource: MediaSource;
  private sourceBuffer: SourceBuffer;
  private sourceBufferTasks: WorkQueue;
  private headerRead: boolean = false;
  private clusterRead: boolean = false;

  public constructor() {
    this.sourceBufferTasks = new WorkQueue();
    this.sourceBufferTasks.pause();

    this.mediaSource = new MediaSource();

    this.mediaSource.onsourceopen = () => {
      this.sourceBuffer = this.mediaSource.addSourceBuffer(MIME_TYPE);
      this.sourceBuffer.onupdateend = this.sourceBufferTasks.runNext.bind(this.sourceBufferTasks);
      // this.sourceBuffer.onerror = (e) => console.log("onerror", e);
      // this.sourceBuffer.onabort = (e) => console.log("onabort", e);

      this.sourceBufferTasks.runNext();
    };
  }

  public write(b: Uint8Array): void {
    if (!this.headerRead) {
      b = b.slice(2);
      this.headerRead = true;
    }

    this.sourceBufferTasks.insert(() => this.sourceBuffer.appendBuffer(b));
    this.sourceBufferTasks.insert(() => this.prune());
  }

  public flush(): void {
    this.headerRead = false;
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

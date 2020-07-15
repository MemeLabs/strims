import WorkQueue from "./workqueue";

export class Source {
  public readonly mediaSource: MediaSource;
  private sourceBuffer: SourceBuffer;
  private sourceBufferTasks: WorkQueue;

  public constructor(type: string) {
    this.sourceBufferTasks = new WorkQueue();
    this.sourceBufferTasks.pause();

    this.mediaSource = new MediaSource();

    this.mediaSource.onsourceopen = () => {
      this.sourceBuffer = this.mediaSource.addSourceBuffer(type);
      this.sourceBuffer.onupdateend = this.sourceBufferTasks.runNext.bind(this.sourceBufferTasks);
      // this.sourceBuffer.onerror = (e) => console.log("onerror", e);
      // this.sourceBuffer.onabort = (e) => console.log("onabort", e);

      this.sourceBufferTasks.runNext();
    };
  }

  public appendBuffer(b: ArrayBufferView | ArrayBuffer) {
    this.sourceBufferTasks.insert(() => this.sourceBuffer.appendBuffer(b));
    this.sourceBufferTasks.insert(() => this.prune());
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

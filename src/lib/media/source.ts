import WorkQueue from "./workqueue";

export class Source {
  public readonly mediaSource: MediaSource;
  private sourceBuffer: SourceBuffer;
  private sourceBufferTasks: WorkQueue;
  private type: string;

  public constructor(type: string) {
    this.sourceBufferTasks = new WorkQueue();
    this.sourceBufferTasks.pause();

    this.type = type;
    this.mediaSource = new MediaSource();

    this.mediaSource.onsourceopen = () => {
      this.initSourceBuffer();
      this.sourceBufferTasks.runNext();
    };
  }

  private initSourceBuffer(): void {
    this.sourceBuffer = this.mediaSource.addSourceBuffer(this.type);
    // eslint-disable-next-line
    this.sourceBuffer.onupdateend = this.sourceBufferTasks.runNext.bind(this.sourceBufferTasks);
    this.sourceBuffer.onerror = (e) => console.log("onerror", e);
    this.sourceBuffer.onabort = (e) => console.log("onabort", e);
  }

  public reset(): void {
    this.sourceBufferTasks.insert(() => {
      this.sourceBufferTasks.reset();

      if (this.sourceBuffer) {
        this.mediaSource.removeSourceBuffer(this.sourceBuffer);
        this.sourceBuffer.onupdateend = undefined;
        this.sourceBuffer.onerror = undefined;
        this.sourceBuffer.onabort = undefined;
        this.sourceBuffer = undefined;
      }

      this.initSourceBuffer();
    });
  }

  public appendBuffer(b: ArrayBufferView | ArrayBuffer): void {
    this.sourceBufferTasks.insert(() => this.sourceBuffer.appendBuffer(b));
    this.sourceBufferTasks.insert(() => this.prune());
  }

  public bounds(): [number, number] {
    const { buffered } = this.sourceBuffer;
    return buffered.length === 0 ? [0, 0] : [buffered.start(0), buffered.end(buffered.length - 1)];
  }

  private prune(): void {
    const [start, end] = this.bounds();

    if (end - start <= 10) {
      this.sourceBufferTasks.runNext();
      return;
    }

    this.sourceBuffer.remove(0, end - 10);
  }
}

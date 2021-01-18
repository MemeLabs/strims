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
      this.sourceBuffer.onerror = (e) => console.log("onerror", e);
      this.sourceBuffer.onabort = (e) => console.log("onabort", e);

      this.sourceBufferTasks.runNext();
    };
  }

  public appendBuffer(b: ArrayBufferView | ArrayBuffer) {
    this.sourceBufferTasks.insert(() => this.sourceBuffer.appendBuffer(b));
    this.sourceBufferTasks.insert(() => this.prune());
  }

  public bounds(): [number, number] {
    const { buffered } = this.sourceBuffer;
    return buffered.length === 0 ? [0, 0] : [buffered.start(0), buffered.end(buffered.length - 1)];
  }

  private prune() {
    const [start, end] = this.bounds();

    if (end - start <= 10) {
      this.sourceBufferTasks.runNext();
      return;
    }

    this.sourceBuffer.remove(0, end - 10);
  }
}

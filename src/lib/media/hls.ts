export interface Options {
  length?: number;
  targetDuration?: number;
  waitForInit?: boolean;
}

const defaultOptions: Options = {
  length: 10,
  targetDuration: 1,
  waitForInit: false,
};

export class Playlist {
  public onReset: (src: string) => void;
  options: Options;
  writer: Writer;

  public constructor(options?: Options) {
    this.options = {
      ...defaultOptions,
      ...options,
    };
    this.reset();
  }

  public reset(): void {
    this.close();
    this.writer = new Writer(this.options);
    this.writer.onReset = (src) => this.onReset?.(src);
  }

  public close(): void {
    if (this.writer) {
      this.writer.close();
    }
  }

  public setInitSegment(ext: string, b: Blob): void {
    this.writer.setInitSegment(ext, b);
  }

  public appendSegment(ext: string, b: Blob): void {
    this.writer.appendSegment(ext, b);
  }

  public bounds(): [number, number] {
    return [0, 0];
  }
}

const enum WriterState {
  NEW,
  READABLE,
  CLOSED,
}

export interface RequestMessage {
  type: "HLS_RELAY_REQUEST";
  url: string;
  port: MessagePort;
}

export interface ResponseMessage {
  body: Blob;
}

class Writer {
  public onReset: (src: string) => void;
  private state: WriterState = WriterState.NEW;
  private cacheName: string;
  private cache: { [key: string]: Blob } = {};
  private sequence: number = 0;
  private initSegment: string;
  private segments: string[] = [];

  public constructor(private options: Options) {
    const cacheId = ((Math.random() * 999) >> 0).toString().padStart(3, "0");
    this.cacheName = `${Date.now()}-${cacheId}`;

    navigator.serviceWorker.addEventListener("message", this.handleWorkerMessage);

    this.initSegment = "";
    this.segments = [];
  }

  public close() {
    navigator.serviceWorker.removeEventListener("message", this.handleWorkerMessage);
    this.state = WriterState.CLOSED;
  }

  private handleWorkerMessage = (e: MessageEvent<RequestMessage>) => {
    const body = this.cache[e.data.url];
    if (e.data?.type === "HLS_RELAY_REQUEST" && body !== undefined) {
      e.data.port.postMessage({ body });
      e.data.port.close();
    }
  };

  private formatUrl(file: string) {
    return `/_hls-relay/${this.cacheName}/${file}`;
  }

  private formatPlaylist() {
    const playlist: string[] = [
      `#EXTM3U`,
      `#EXT-X-VERSION:7`,
      `#EXT-X-MEDIA-SEQUENCE:${this.sequence - this.segments.length}`,
      `#EXT-X-TARGETDURATION:${this.options.targetDuration}`,
    ];
    if (this.initSegment) {
      playlist.push(`#EXT-X-MAP:URI="${this.initSegment}"`);
    }

    for (const url of this.segments) {
      playlist.push(`#EXTINF:${this.options.targetDuration},`);
      playlist.push(url);
    }

    return playlist.join("\n");
  }

  public setInitSegment(ext: string, b: Blob): void {
    const url = this.formatUrl(`init.${ext}`);
    this.cache[url] = b;
    this.initSegment = url;
  }

  public appendSegment(ext: string, b: Blob): void {
    const sequence = this.sequence++;
    const url = this.formatUrl(`${sequence}.${ext}`);
    this.cache[url] = b;
    this.segments.push(url);
    this.prune();
    this.updatePlaylist();
  }

  private prune(): void {
    while (this.segments.length > this.options.length) {
      delete this.cache[this.segments.shift()];
    }
  }

  private updatePlaylist(): void {
    if ((!this.initSegment && this.options.waitForInit) || this.segments.length === 0) {
      return;
    }

    const url = this.formatUrl("playlist.m3u8");
    const playlist = new TextEncoder().encode(this.formatPlaylist());
    this.cache[url] = new Blob([playlist], { type: "application/vnd.apple.mpegurl" });

    if (this.state === WriterState.NEW) {
      this.onReset(url);
      this.state = WriterState.READABLE;
    }
  }
}

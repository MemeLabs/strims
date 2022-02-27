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
    this.writer.onReset = (src: string) => this.onReset?.(src);
  }

  public close(): void {
    if (this.writer) {
      this.writer.close();
    }
  }

  public getUrl(): string {
    return this.writer.getUrl();
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

class Writer {
  public onReset: (src: string) => void;
  private state: WriterState = WriterState.NEW;
  private cacheName: string;
  private cache: Cache;
  private tasks: Promise<any>;
  private sequence: number = 0;
  private initSegment: string;
  private segments: string[] = [];

  public constructor(private options: Options) {
    const cacheId = ((Math.random() * 999) >> 0).toString().padStart(3, "0");
    this.cacheName = `${Date.now()}-${cacheId}`;

    this.tasks = (async () => {
      this.cache = await caches.open(this.cacheName);
    })();
    this.initSegment = "";
    this.segments = [];
  }

  public close() {
    void this.tasks.then(() => caches.delete(this.cacheName));
    this.state = WriterState.CLOSED;
  }

  public getUrl(): string {
    return this.formatUrl("playlist.m3u8");
  }

  private formatUrl(file: string): string {
    return `/_cache/${this.cacheName}/${file}`;
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

  private insertTask(fn: () => any) {
    this.tasks = this.tasks.then(fn);
  }

  public setInitSegment(ext: string, b: Blob): void {
    const url = this.formatUrl(`init.${ext}`);

    this.insertTask(async () => {
      await this.cache.put(new Request(url), new Response(b));
      this.initSegment = url;
    });
  }

  public appendSegment(ext: string, b: Blob): void {
    const sequence = this.sequence++;
    const url = this.formatUrl(`${sequence}.${ext}`);

    this.insertTask(async () => {
      await this.cache.put(new Request(url), new Response(b));
      this.segments.push(url);
      this.prune();
      this.updatePlaylist();
    });
  }

  private prune(): void {
    while (this.segments.length > this.options.length) {
      const url = this.segments.shift();
      this.insertTask(() => this.cache.delete(new Request(url)));
    }
  }

  private updatePlaylist(): void {
    if ((!this.initSegment && this.options.waitForInit) || this.segments.length === 0) {
      return;
    }

    this.insertTask(async () => {
      const playlist = new TextEncoder().encode(this.formatPlaylist());
      const b = new Blob([playlist], { type: "application/vnd.apple.mpegurl" });
      await this.cache.put(new Request(this.getUrl()), new Response(b));
    });

    if (this.state === WriterState.NEW) {
      this.insertTask(() => this.onReset(this.getUrl()));
      this.state = WriterState.READABLE;
    }
  }
}

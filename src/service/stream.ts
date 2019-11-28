// SEE: https://stackoverflow.com/a/52865648

export type BasicCallback = (error: Error | null, value: any) => void;
export interface Readable<T> extends ReadStream<T> {}
export declare class Readable<T> {
  constructor(opts?: ReadableOptions<Readable<T>>);

  public _read?(size: number): void;
  public _destroy?(error: Error | null, callback: BasicCallback): void;
}

export interface ReadStream<T> {
  readable: boolean;
  readonly readableHighWaterMark: number;
  readonly readableLength: number;
  read(size?: number): T;
  setEncoding(encoding: string): this;
  pause(): this;
  resume(): this;
  isPaused(): boolean;
  unpipe<T extends NodeJS.WritableStream>(destination?: T): this;
  unshift(chunk: T): void;
  wrap(oldStream: NodeJS.ReadableStream): this;
  push(chunk: T | null, encoding?: string): boolean;
  destroy(error?: Error): void;

  /**
   * Event emitter
   * The defined events on documents including:
   * 1. close
   * 2. data
   * 3. end
   * 4. readable
   * 5. error
   */
  addListener(event: "close", listener: () => void): this;
  addListener(event: "data", listener: (chunk: T) => void): this;
  addListener(event: "end", listener: () => void): this;
  addListener(event: "readable", listener: () => void): this;
  addListener(event: "error", listener: (err: Error) => void): this;
  addListener(event: string | symbol, listener: (...args: any[]) => void): this;

  emit(event: "close"): boolean;
  emit(event: "data", chunk: T): boolean;
  emit(event: "end"): boolean;
  emit(event: "readable"): boolean;
  emit(event: "error", err: Error): boolean;
  emit(event: string | symbol, ...args: any[]): boolean;

  on(event: "close", listener: () => void): this;
  on(event: "data", listener: (chunk: T) => void): this;
  on(event: "end", listener: () => void): this;
  on(event: "readable", listener: () => void): this;
  on(event: "error", listener: (err: Error) => void): this;
  on(event: string | symbol, listener: (...args: any[]) => void): this;

  once(event: "close", listener: () => void): this;
  once(event: "data", listener: (chunk: T) => void): this;
  once(event: "end", listener: () => void): this;
  once(event: "readable", listener: () => void): this;
  once(event: "error", listener: (err: Error) => void): this;
  once(event: string | symbol, listener: (...args: any[]) => void): this;

  prependListener(event: "close", listener: () => void): this;
  prependListener(event: "data", listener: (chunk: T) => void): this;
  prependListener(event: "end", listener: () => void): this;
  prependListener(event: "readable", listener: () => void): this;
  prependListener(event: "error", listener: (err: Error) => void): this;
  prependListener(event: string | symbol, listener: (...args: any[]) => void): this;

  prependOnceListener(event: "close", listener: () => void): this;
  prependOnceListener(event: "data", listener: (chunk: T) => void): this;
  prependOnceListener(event: "end", listener: () => void): this;
  prependOnceListener(event: "readable", listener: () => void): this;
  prependOnceListener(event: "error", listener: (err: Error) => void): this;
  prependOnceListener(event: string | symbol, listener: (...args: any[]) => void): this;

  removeListener(event: "close", listener: () => void): this;
  removeListener(event: "data", listener: (chunk: T) => void): this;
  removeListener(event: "end", listener: () => void): this;
  removeListener(event: "readable", listener: () => void): this;
  removeListener(event: "error", listener: (err: Error) => void): this;
  removeListener(event: string | symbol, listener: (...args: any[]) => void): this;

  [Symbol.asyncIterator](): AsyncIterableIterator<T>;
}

export interface ReadableOptions<This> {
  highWaterMark?: number;
  encoding?: string;
  objectMode?: boolean;
  read?(this: This, size: number): void;
  destroy?(
      this: This,
      error: Error | null,
      callback: BasicCallback,
      ): void;
}

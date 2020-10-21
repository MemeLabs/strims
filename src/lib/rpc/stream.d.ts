// adapted from @types/node

import * as events from "events";

interface internal extends events.EventEmitter {
  pipe<T extends NodeJS.WritableStream>(destination: T, options?: { end?: boolean }): T;
}

interface Stream<T> extends internal {
  constructor(opts?: ReadableOptions<T>);
}

export interface ReadableOptions<T> {
  highWaterMark?: number;
  encoding?: string;
  objectMode?: boolean;
  read?(this: Readable<T>, size: number): void;
  destroy?(this: Readable<T>, error: Error | null, callback: (error: Error | null) => void): void;
  autoDestroy?: boolean;
}

export interface Readable<T> extends Stream<T> {
  readable: boolean;
  readonly readableHighWaterMark: number;
  readonly readableLength: number;
  readonly readableObjectMode: boolean;
  destroyed: boolean;
  constructor(opts?: ReadableOptions<T>);
  _read(size: number): void;
  read(size?: number): T;
  setEncoding(encoding: string): this;
  pause(): this;
  resume(): this;
  isPaused(): boolean;
  unpipe(destination?: NodeJS.WritableStream): this;
  unshift(chunk: T, encoding?: BufferEncoding): void;
  wrap(oldStream: NodeJS.ReadableStream): this;
  push(chunk: T, encoding?: string): boolean;
  _destroy(error: Error | null, callback: (error?: Error | null) => void): void;
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
  addListener(event: string | symbol, listener: (...args: T[]) => void): this;

  emit(event: "close"): boolean;
  emit(event: "data", chunk: T): boolean;
  emit(event: "end"): boolean;
  emit(event: "readable"): boolean;
  emit(event: "error", err: Error): boolean;
  emit(event: string | symbol, ...args: T[]): boolean;

  on(event: "close", listener: () => void): this;
  on(event: "data", listener: (chunk: T) => void): this;
  on(event: "end", listener: () => void): this;
  on(event: "readable", listener: () => void): this;
  on(event: "error", listener: (err: Error) => void): this;
  on(event: string | symbol, listener: (...args: T[]) => void): this;

  once(event: "close", listener: () => void): this;
  once(event: "data", listener: (chunk: T) => void): this;
  once(event: "end", listener: () => void): this;
  once(event: "readable", listener: () => void): this;
  once(event: "error", listener: (err: Error) => void): this;
  once(event: string | symbol, listener: (...args: T[]) => void): this;

  prependListener(event: "close", listener: () => void): this;
  prependListener(event: "data", listener: (chunk: T) => void): this;
  prependListener(event: "end", listener: () => void): this;
  prependListener(event: "readable", listener: () => void): this;
  prependListener(event: "error", listener: (err: Error) => void): this;
  prependListener(event: string | symbol, listener: (...args: T[]) => void): this;

  prependOnceListener(event: "close", listener: () => void): this;
  prependOnceListener(event: "data", listener: (chunk: T) => void): this;
  prependOnceListener(event: "end", listener: () => void): this;
  prependOnceListener(event: "readable", listener: () => void): this;
  prependOnceListener(event: "error", listener: (err: Error) => void): this;
  prependOnceListener(event: string | symbol, listener: (...args: T[]) => void): this;

  removeListener(event: "close", listener: () => void): this;
  removeListener(event: "data", listener: (chunk: T) => void): this;
  removeListener(event: "end", listener: () => void): this;
  removeListener(event: "readable", listener: () => void): this;
  removeListener(event: "error", listener: (err: Error) => void): this;
  removeListener(event: string | symbol, listener: (...args: T[]) => void): this;

  [Symbol.asyncIterator]<T>(): AsyncIterableIterator<T>;
}

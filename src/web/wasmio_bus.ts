import { EventEmitter } from "events";

// serial bus for communicating with wasm workers
export default class Bus extends EventEmitter {
  private data: Buffer;
  private writeQueue: Promise<any>;
  private onWrite: (n: number) => void;

  constructor(size: number = 65536) {
    super();
    this.data = new Buffer(size);
    this.writeQueue = Promise.resolve(null);
  }

  public emitData(n: number): void {
    this.emit("data", new Buffer(this.data.slice(0, n)));
  }

  public write(data: Buffer) {
    this.writeQueue = this.writeQueue.then(() => {
      this.data.set(data);
      return this.onWrite(data.length);
    });
  }
}

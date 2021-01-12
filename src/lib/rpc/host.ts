import { PassThrough, Readable, Writable } from "stream";

import { Any } from "../../apis/google/protobuf/any";
import { Call, Cancel, Close, Error, Undefined } from "../../apis/strims/rpc/v1/rpc";
import Reader from "../pb/reader";
import Writer from "../pb/writer";
import { anyValueType, registerType, typeName } from "./registry";
import { Readable as GenericReadable } from "./stream";

registerType("strims.rpc.v1.Cancel", Cancel);
registerType("strims.rpc.v1.Close", Close);
registerType("strims.rpc.v1.Error", Error);
registerType("strims.rpc.v1.Undefined", Undefined);

const CALL_TIMEOUT_MS = 5000;

type CallbackHandler = (m: any) => void;

// RPCHost transport agnostic remote procedure utility using protobufs.
export class RPCHost {
  private w: Writable;
  private service: any;
  private callId: bigint;
  private callbacks: Map<bigint, CallbackHandler>;
  private argWriter: Writer;
  private callWriter: Writer;

  constructor(w: Writable, r: Readable, service: any = {}) {
    this.w = w;
    this.service = service;
    this.callId = BigInt(0);
    this.callbacks = new Map();
    this.argWriter = new Writer();
    this.callWriter = new Writer();

    this.createHandler(r);
  }

  public call(method: string, v: any, parentId: bigint = BigInt(0)): Call {
    const ctor = v.constructor;
    const call = new Call({
      id: ++this.callId,
      parentId,
      method,
      argument: new Any({
        typeUrl: `strims.gg/${typeName(ctor)}`,
        value: ctor.encode(v, this.argWriter.reset()).finish(),
      }),
    });

    const w = new Writer(16 * 1024);
    this.w.write(Call.encode(call, this.callWriter.reset().fork()).ldelim().finish());

    return call;
  }

  public expectOne<T>(call: Call, { timeout }: { timeout?: number } = {}): Promise<T> {
    return new Promise((resolve, reject) => {
      const tid = setTimeout(() => {
        reject();
        this.callbacks.delete(call.id);
      }, timeout || CALL_TIMEOUT_MS);

      this.callbacks.set(call.id, (res: any) => {
        clearTimeout(tid);
        this.callbacks.delete(call.id);

        if (res instanceof Error) {
          reject(res);
        } else {
          resolve(res);
        }
      });
    });
  }

  public expectMany<T>(call: Call): GenericReadable<T> {
    const e = new PassThrough({
      objectMode: true,
    });

    e.on("close", () => this.call("CANCEL", new Cancel(), call.id));

    this.callbacks.set(call.id, (r: any) => {
      if (r instanceof Error) {
        this.callbacks.delete(call.id);
        e.emit("error", new Error({ message: r.message }));
      } else if (r instanceof Close) {
        this.callbacks.delete(call.id);
        e.push(null);
      } else {
        e.push(r);
      }
    });

    return e as any;
  }

  private createHandler(r: Readable) {
    r.on("data", (data: ArrayBuffer) => {
      const reader = new Reader(new Uint8Array(data));
      while (reader.pos < reader.len) {
        const call = Call.decode(reader, reader.uint32());
        const arg = anyValueType(call.argument).decode(call.argument.value);

        if (call.parentId) {
          this.handleCallback(call, arg);
        } else {
          this.handleCall(call, arg);
        }
      }
    });
  }

  private handleCallback(call: Call, arg: any) {
    const cb = this.callbacks.get(call.parentId);
    if (!cb) {
      // TODO: send err closed
      return;
    }
    cb(arg);
  }

  private handleCall(call: Call, arg: any) {
    let res;
    try {
      const h = this.service[call.method];
      if (!h) {
        throw new Error({ message: `method not implemented: ${call.method}` });
      }
      res = h(call, arg);
    } catch (e) {
      // TODO: we may not want to expose this to remote hosts...
      res = new Error({ message: e.message });
    }

    if (res instanceof Readable) {
      res.on("data", (d) => this.call("CALLBACK", d, call.id));
      res.on("close", () => this.call("CALLBACK", new Close(), call.id));
    } else if (res instanceof Promise) {
      res.then((d) => this.call("CALLBACK", d, call.id));
      res.catch(({ message }) => {
        const e = new Error({ message });
        this.call("CALLBACK", e, call.id);
      });
    } else if (res === undefined) {
      this.call("CALLBACK", new Undefined(), call.id);
    } else {
      this.call("CALLBACK", res, call.id);
    }
  }
}

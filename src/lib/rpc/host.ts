import { PassThrough, Readable, Writable } from "stream";

import protobuf from "protobufjs/minimal";

import * as pb from "../pb";
import { anyValueType, typeName } from "../pb/registry";
import { Readable as GenericReadable } from "./stream";

const CALL_TIMEOUT_MS = 5000;

type CallbackHandler = (m: protobuf.Message) => void;

// RPCHost transport agnostic remote procedure utility using protobufs.
export class RPCHost {
  private w: Writable;
  private service: any;
  private callId: number;
  private callbacks: Map<number, CallbackHandler>;

  constructor(w: Writable, r: Readable, service: any = {}) {
    this.w = w;
    this.service = service;
    this.callId = 0;
    this.callbacks = new Map();

    this.createHandler(r);
  }

  public call(method: string, v: any, parentId: number = 0): pb.Call {
    const ctor = v.constructor;
    const call = new pb.Call({
      id: ++this.callId,
      parentId,
      method,
      argument: new pb.google.protobuf.Any({
        type_url: `api/${typeName(ctor)}`,
        value: ctor.encode(v).finish(),
      }),
    });

    this.w.write(pb.Call.encodeDelimited(call).finish().slice());

    return call;
  }

  public expectOne<T>(call: pb.Call, { timeout }: { timeout?: number } = {}): Promise<T> {
    return new Promise((resolve, reject) => {
      const tid = setTimeout(() => {
        reject();
        this.callbacks.delete(call.id);
      }, timeout || CALL_TIMEOUT_MS);

      this.callbacks.set(call.id, (res: any) => {
        clearTimeout(tid);
        this.callbacks.delete(call.id);

        if (res instanceof pb.Error) {
          reject(res);
        } else {
          resolve(res);
        }
      });
    });
  }

  public expectMany<T>(call: pb.Call): GenericReadable<T> {
    const e = new PassThrough({
      objectMode: true,
    });

    this.callbacks.set(call.id, (r: any) => {
      if (r instanceof pb.Error) {
        this.callbacks.delete(call.id);
        e.emit("error", new Error(r.message));
      } else if (r instanceof pb.Close) {
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
      const reader = new protobuf.Reader(new Uint8Array(data));
      while (reader.pos < reader.len) {
        const call = pb.Call.decodeDelimited(reader);
        const arg = anyValueType(call.argument).decode(call.argument.value);

        if (call.parentId) {
          this.handleCallback(call, arg);
        } else {
          this.handleCall(call, arg);
        }
      }
    });
  }

  private handleCallback(call: pb.Call, arg: protobuf.Message) {
    const cb = this.callbacks.get(call.parentId);
    if (!cb) {
      // TODO: send err closed
      return;
    }
    cb(arg);
  }

  private handleCall(call: pb.Call, arg: protobuf.Message) {
    let res;
    try {
      const h = this.service[call.method];
      if (!h) {
        throw new Error(`method not implemented: ${call.method}`);
      }
      res = h(call, arg);
    } catch (e) {
      // TODO: we may not want to expose this to remote hosts...
      res = new pb.Error({ message: e.message });
    }

    if (res instanceof Readable) {
      res.on("data", (d) => this.call("callback", d, call.id));
      res.on("close", () => this.call("callback", new pb.Close(), call.id));
    } else if (res instanceof Promise) {
      res.then((d) => this.call("callback", d, call.id));
      res.catch(({ message }) => {
        const e = new pb.Error({ message });
        this.call("callback", e, call.id);
      });
    } else if (res instanceof protobuf.Message) {
      this.call("callback", res, call.id);
    } else if (res === undefined) {
      this.call("callback", new pb.Undefined(), call.id);
    } else {
      throw new Error(`unsupported rpc return value: ${res}`);
    }
  }
}

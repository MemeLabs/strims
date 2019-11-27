import { EventEmitter } from "events";
import * as proto from "google-protobuf";
import * as any_pb from "google-protobuf/google/protobuf/any_pb";
import { Readable, Writable } from "stream";
import * as rpc_pb from "./rpc_pb";

const HEADER_LEN = 4;
const CALL_TIMEOUT = 1000;
const CALLBACK_METHOD = "callback";

type apiMessageTypesEntry = [string, typeof proto.Message];
const apiMessageTypes: apiMessageTypesEntry[] = [];

export function registerType(typeName: string, type: typeof proto.Message) {
  apiMessageTypes.push([typeName, type]);
}

const getApiMessageTypeName = (m: proto.Message) => {
  const mt = apiMessageTypes.find((t) => m instanceof t[1]);
  return mt && mt[0];
};

const getApiMessageType = (n: string): typeof proto.Message => {
  const nt = apiMessageTypes.find((t) => n.endsWith(t[0]));
  return nt && nt[1];
};

registerType("Error", rpc_pb.Error);
registerType("Close", rpc_pb.Close);
registerType("Undefined", rpc_pb.Undefined);

function decodeFixed32(data: Buffer): number {
  return data.readUInt32LE(0);
}

type CallbackHandler = (m: proto.Message) => void;

// RPCHost transport agnostic remote procedure utility using protobufs.
export class RPCHost extends EventEmitter {
  private w: Writable;
  private service: any;
  private callId: number;
  private callbacks: Map<number, CallbackHandler>;

  constructor(w: Writable, r: Readable, service: any = {}) {
    super();
    this.w = w;
    this.service = service;
    this.callId = 0;
    this.callbacks = new Map();

    this.createHandler(r);
  }

  public call(method: string, v: proto.Message, parentId: number = 0): rpc_pb.Call {
    const m = new rpc_pb.Call();
    m.setId(++ this.callId);
    m.setParentId(parentId);
    m.setMethod(method);

    const a = new any_pb.Any();
    a.pack(v.serializeBinary(), getApiMessageTypeName(v));
    m.setArgument(a);

    // TODO: maybe move serialization to expect* so Call can include the
    // expected response kind...
    const b = Buffer.from(m.serializeBinary());
    const l = Buffer.from(new Uint8Array(4));
    l.writeInt32LE(b.length, 0);

    this.w.write(Buffer.concat([l, b]));

    return m;
  }

  public expectOne<T>(m: rpc_pb.Call): Promise<T> {
    return new Promise((resolve, reject) => {
      const tid = setTimeout(() => {
        reject();
        this.callbacks.delete(m.getId());
      }, CALL_TIMEOUT);

      this.callbacks.set(m.getId(), (r: any) => {
        clearTimeout(tid);
        if (r instanceof rpc_pb.Error) {
          reject(r);
        } else {
          resolve(r);
        }
      });
    });
  }

  public expectMany(m: rpc_pb.Call): EventEmitter {
    const e = new EventEmitter();

    this.callbacks.set(m.getId(), (r: any) => {
      if (r instanceof rpc_pb.Error) {
        this.callbacks.delete(m.getId());
        e.emit("error", new Error(r.getMessage()));
      } else if (r instanceof rpc_pb.Close) {
        this.callbacks.delete(m.getId());
        e.emit("close");
      } else {
        e.emit("data", r);
      }
    });

    return e;
  }

  private createHandler(r: Readable) {
    r.on("data", (data: Buffer) => {
      while (data.length !== 0) {
        const size = decodeFixed32(data);
        if (size === 0 || size > data.length) {
          return;
        }

        const reader = new proto.BinaryReader(data.slice(HEADER_LEN, HEADER_LEN + size));
        const msg = new rpc_pb.Call();
        rpc_pb.Call.deserializeBinaryFromReader(msg, reader);
        this.handleCall(msg);

        data = data.slice(HEADER_LEN + size);
      }
    });
  }

  private handleCall(msg: rpc_pb.Call) {
    const argType = getApiMessageType(msg.getArgument().getTypeName());
    const arg = argType.deserializeBinary(msg.getArgument().getValue_asU8());

    if (msg.getParentId()) {
      this.handleCallback(msg, arg);
    } else {
      this.handleMethod(msg, arg);
    }
  }

  private handleCallback(msg: rpc_pb.Call, arg: proto.Message) {
    const cb = this.callbacks.get(msg.getParentId());
    if (!cb) {
      // TODO: send err closed
      return;
    }
    cb(arg);
  }

  private handleMethod(msg: rpc_pb.Call, arg: proto.Message) {
    let res;
    try {
      const h = this.service[msg.getMethod()];
      if (!h) {
        throw new Error(`method not implemented: ${msg.getMethod()}`);
      }
      res = h(msg, arg);
    } catch (e) {
      res = new rpc_pb.Error();
      // TODO: we may not want to expose this to remote hosts...
      res.setMessage(e.message);
    }

    if (res instanceof Readable) {
      res.on("data", (d) => this.call(CALLBACK_METHOD, d, msg.getId()));
      res.on("close", () => this.call(CALLBACK_METHOD, new rpc_pb.Close(), msg.getId()));
    } else if (res instanceof Promise) {
      res.then((d) => this.call(CALLBACK_METHOD, d, msg.getId()));
      res.catch(({message}) => {
        const e = new rpc_pb.Error();
        e.setMessage(message);
        this.call(CALLBACK_METHOD, e, msg.getId());
      });
    } else if (res instanceof proto.Message) {
      this.call(CALLBACK_METHOD, res, msg.getId());
    } else if (res === undefined) {
      this.call(CALLBACK_METHOD, new rpc_pb.Undefined(), msg.getId());
    } else {
      throw new Error(`unsupported rpc return value: ${res}`);
    }
  }
}

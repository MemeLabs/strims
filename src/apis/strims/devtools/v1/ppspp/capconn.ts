import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export type ICapConnLog = {
  peerLogs?: CapConnLog.IPeerLog[];
}

export class CapConnLog {
  peerLogs: CapConnLog.PeerLog[];

  constructor(v?: ICapConnLog) {
    this.peerLogs = v?.peerLogs ? v.peerLogs.map(v => new CapConnLog.PeerLog(v)) : [];
  }

  static encode(m: CapConnLog, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.peerLogs) CapConnLog.PeerLog.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CapConnLog {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CapConnLog();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.peerLogs.push(CapConnLog.PeerLog.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace CapConnLog {
  export type IPeerLog = {
    label?: string;
    events?: CapConnLog.PeerLog.IEvent[];
  }

  export class PeerLog {
    label: string;
    events: CapConnLog.PeerLog.Event[];

    constructor(v?: IPeerLog) {
      this.label = v?.label || "";
      this.events = v?.events ? v.events.map(v => new CapConnLog.PeerLog.Event(v)) : [];
    }

    static encode(m: PeerLog, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.label.length) w.uint32(10).string(m.label);
      for (const v of m.events) CapConnLog.PeerLog.Event.encode(v, w.uint32(18).fork()).ldelim();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): PeerLog {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new PeerLog();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.label = r.string();
          break;
          case 2:
          m.events.push(CapConnLog.PeerLog.Event.decode(r, r.uint32()));
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export namespace PeerLog {
    export type IEvent = {
      code?: CapConnLog.PeerLog.Event.Code;
      timestamp?: bigint;
      messageTypes?: CapConnLog.PeerLog.Event.MessageType[];
      messageAddresses?: bigint[];
    }

    export class Event {
      code: CapConnLog.PeerLog.Event.Code;
      timestamp: bigint;
      messageTypes: CapConnLog.PeerLog.Event.MessageType[];
      messageAddresses: bigint[];

      constructor(v?: IEvent) {
        this.code = v?.code || 0;
        this.timestamp = v?.timestamp || BigInt(0);
        this.messageTypes = v?.messageTypes ? v.messageTypes : [];
        this.messageAddresses = v?.messageAddresses ? v.messageAddresses : [];
      }

      static encode(m: Event, w?: Writer): Writer {
        if (!w) w = new Writer();
        if (m.code) w.uint32(8).uint32(m.code);
        if (m.timestamp) w.uint32(17).sfixed64(m.timestamp);
        m.messageTypes.reduce((w, v) => w.uint32(v), w.uint32(26).fork()).ldelim();
        m.messageAddresses.reduce((w, v) => w.uint64(v), w.uint32(34).fork()).ldelim();
        return w;
      }

      static decode(r: Reader | Uint8Array, length?: number): Event {
        r = r instanceof Reader ? r : new Reader(r);
        const end = length === undefined ? r.len : r.pos + length;
        const m = new Event();
        while (r.pos < end) {
          const tag = r.uint32();
          switch (tag >> 3) {
            case 1:
            m.code = r.uint32();
            break;
            case 2:
            m.timestamp = r.sfixed64();
            break;
            case 3:
            for (const flen = r.uint32(), fend = r.pos + flen; r.pos < fend;) m.messageTypes.push(r.uint32());
            break;
            case 4:
            for (const flen = r.uint32(), fend = r.pos + flen; r.pos < fend;) m.messageAddresses.push(r.uint64());
            break;
            default:
            r.skipType(tag & 7);
            break;
          }
        }
        return m;
      }
    }

    export namespace Event {
      export enum Code {
        EVENT_CODE_INIT = 0,
        EVENT_CODE_WRITE = 1,
        EVENT_CODE_WRITE_ERR = 2,
        EVENT_CODE_FLUSH = 3,
        EVENT_CODE_FLUSH_ERR = 4,
        EVENT_CODE_READ = 5,
        EVENT_CODE_READ_ERR = 6,
      }
      export enum MessageType {
        MESSAGE_TYPE_HANDSHAKE = 0,
        MESSAGE_TYPE_DATA = 1,
        MESSAGE_TYPE_ACK = 2,
        MESSAGE_TYPE_HAVE = 3,
        MESSAGE_TYPE_INTEGRITY = 4,
        MESSAGE_TYPE_SIGNED_INTEGRITY = 5,
        MESSAGE_TYPE_REQUEST = 6,
        MESSAGE_TYPE_CANCEL = 7,
        MESSAGE_TYPE_CHOKE = 8,
        MESSAGE_TYPE_UNCHOKE = 9,
        MESSAGE_TYPE_PING = 10,
        MESSAGE_TYPE_PONG = 11,
        MESSAGE_TYPE_STREAM_REQUEST = 12,
        MESSAGE_TYPE_STREAM_CANCEL = 13,
        MESSAGE_TYPE_STREAM_OPEN = 14,
        MESSAGE_TYPE_STREAM_CLOSE = 15,
        MESSAGE_TYPE_END = 255,
      }
    }

  }

}

export type ICapConnWatchLogsRequest = {
}

export class CapConnWatchLogsRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: ICapConnWatchLogsRequest) {
  }

  static encode(m: CapConnWatchLogsRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CapConnWatchLogsRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new CapConnWatchLogsRequest();
  }
}

export type ICapConnWatchLogsResponse = {
  op?: CapConnWatchLogsResponse.Op;
  name?: string;
}

export class CapConnWatchLogsResponse {
  op: CapConnWatchLogsResponse.Op;
  name: string;

  constructor(v?: ICapConnWatchLogsResponse) {
    this.op = v?.op || 0;
    this.name = v?.name || "";
  }

  static encode(m: CapConnWatchLogsResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.op) w.uint32(8).uint32(m.op);
    if (m.name.length) w.uint32(18).string(m.name);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CapConnWatchLogsResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CapConnWatchLogsResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.op = r.uint32();
        break;
        case 2:
        m.name = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace CapConnWatchLogsResponse {
  export enum Op {
    CREATE = 0,
    REMOVE = 1,
  }
}

export type ICapConnLoadLogRequest = {
  name?: string;
}

export class CapConnLoadLogRequest {
  name: string;

  constructor(v?: ICapConnLoadLogRequest) {
    this.name = v?.name || "";
  }

  static encode(m: CapConnLoadLogRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.name.length) w.uint32(10).string(m.name);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CapConnLoadLogRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CapConnLoadLogRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.name = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICapConnLoadLogResponse = {
  log?: ICapConnLog;
}

export class CapConnLoadLogResponse {
  log: CapConnLog | undefined;

  constructor(v?: ICapConnLoadLogResponse) {
    this.log = v?.log && new CapConnLog(v.log);
  }

  static encode(m: CapConnLoadLogResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.log) CapConnLog.encode(m.log, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CapConnLoadLogResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CapConnLoadLogResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.log = CapConnLog.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}


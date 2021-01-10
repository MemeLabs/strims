import Reader from "../../../../pb/reader";
import Writer from "../../../../pb/writer";


export interface IHashTableMessage {
  body?: HashTableMessage.IBodyOneOf
}

export class HashTableMessage {
  body: HashTableMessage.BodyOneOf;

  constructor(v?: IHashTableMessage) {
    this.body = new HashTableMessage.BodyOneOf(v?.body);
  }

  static encode(m: HashTableMessage, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    switch (m.body.case) {
      case 1:
      HashTableMessage.Publish.encode(m.body.publish, w.uint32(10).fork()).ldelim();
      break;
      case 2:
      HashTableMessage.Unpublish.encode(m.body.unpublish, w.uint32(18).fork()).ldelim();
      break;
      case 3:
      HashTableMessage.GetRequest.encode(m.body.getRequest, w.uint32(26).fork()).ldelim();
      break;
      case 4:
      HashTableMessage.GetResponse.encode(m.body.getResponse, w.uint32(34).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): HashTableMessage {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new HashTableMessage();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.body.publish = HashTableMessage.Publish.decode(r, r.uint32());
        break;
        case 2:
        m.body.unpublish = HashTableMessage.Unpublish.decode(r, r.uint32());
        break;
        case 3:
        m.body.getRequest = HashTableMessage.GetRequest.decode(r, r.uint32());
        break;
        case 4:
        m.body.getResponse = HashTableMessage.GetResponse.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace HashTableMessage {
  export type IBodyOneOf =
  { publish: HashTableMessage.IPublish }
  |{ unpublish: HashTableMessage.IUnpublish }
  |{ getRequest: HashTableMessage.IGetRequest }
  |{ getResponse: HashTableMessage.IGetResponse }
  ;

  export class BodyOneOf {
    private _publish: HashTableMessage.Publish | undefined;
    private _unpublish: HashTableMessage.Unpublish | undefined;
    private _getRequest: HashTableMessage.GetRequest | undefined;
    private _getResponse: HashTableMessage.GetResponse | undefined;
    private _case: BodyCase = 0;

    constructor(v?: IBodyOneOf) {
      if (v && "publish" in v) this.publish = new HashTableMessage.Publish(v.publish);
      if (v && "unpublish" in v) this.unpublish = new HashTableMessage.Unpublish(v.unpublish);
      if (v && "getRequest" in v) this.getRequest = new HashTableMessage.GetRequest(v.getRequest);
      if (v && "getResponse" in v) this.getResponse = new HashTableMessage.GetResponse(v.getResponse);
    }

    public clear() {
      this._publish = undefined;
      this._unpublish = undefined;
      this._getRequest = undefined;
      this._getResponse = undefined;
      this._case = BodyCase.NOT_SET;
    }

    get case(): BodyCase {
      return this._case;
    }

    set publish(v: HashTableMessage.Publish) {
      this.clear();
      this._publish = v;
      this._case = BodyCase.PUBLISH;
    }

    get publish(): HashTableMessage.Publish {
      return this._publish;
    }

    set unpublish(v: HashTableMessage.Unpublish) {
      this.clear();
      this._unpublish = v;
      this._case = BodyCase.UNPUBLISH;
    }

    get unpublish(): HashTableMessage.Unpublish {
      return this._unpublish;
    }

    set getRequest(v: HashTableMessage.GetRequest) {
      this.clear();
      this._getRequest = v;
      this._case = BodyCase.GET_REQUEST;
    }

    get getRequest(): HashTableMessage.GetRequest {
      return this._getRequest;
    }

    set getResponse(v: HashTableMessage.GetResponse) {
      this.clear();
      this._getResponse = v;
      this._case = BodyCase.GET_RESPONSE;
    }

    get getResponse(): HashTableMessage.GetResponse {
      return this._getResponse;
    }
  }

  export enum BodyCase {
    NOT_SET = 0,
    PUBLISH = 1,
    UNPUBLISH = 2,
    GET_REQUEST = 3,
    GET_RESPONSE = 4,
  }

  export interface IRecord {
    key?: Uint8Array;
    salt?: Uint8Array;
    value?: Uint8Array;
    timestamp?: bigint;
    signature?: Uint8Array;
  }

  export class Record {
    key: Uint8Array = new Uint8Array();
    salt: Uint8Array = new Uint8Array();
    value: Uint8Array = new Uint8Array();
    timestamp: bigint = BigInt(0);
    signature: Uint8Array = new Uint8Array();

    constructor(v?: IRecord) {
      this.key = v?.key || new Uint8Array();
      this.salt = v?.salt || new Uint8Array();
      this.value = v?.value || new Uint8Array();
      this.timestamp = v?.timestamp || BigInt(0);
      this.signature = v?.signature || new Uint8Array();
    }

    static encode(m: Record, w?: Writer): Writer {
      if (!w) w = new Writer(1024);
      if (m.key) w.uint32(10).bytes(m.key);
      if (m.salt) w.uint32(18).bytes(m.salt);
      if (m.value) w.uint32(26).bytes(m.value);
      if (m.timestamp) w.uint32(32).int64(m.timestamp);
      if (m.signature) w.uint32(42).bytes(m.signature);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Record {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Record();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.key = r.bytes();
          break;
          case 2:
          m.salt = r.bytes();
          break;
          case 3:
          m.value = r.bytes();
          break;
          case 4:
          m.timestamp = r.int64();
          break;
          case 5:
          m.signature = r.bytes();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface IPublish {
    record?: HashTableMessage.IRecord;
  }

  export class Publish {
    record: HashTableMessage.Record | undefined;

    constructor(v?: IPublish) {
      this.record = v?.record && new HashTableMessage.Record(v.record);
    }

    static encode(m: Publish, w?: Writer): Writer {
      if (!w) w = new Writer(1024);
      if (m.record) HashTableMessage.Record.encode(m.record, w.uint32(10).fork()).ldelim();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Publish {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Publish();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.record = HashTableMessage.Record.decode(r, r.uint32());
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface IUnpublish {
    record?: HashTableMessage.IRecord;
  }

  export class Unpublish {
    record: HashTableMessage.Record | undefined;

    constructor(v?: IUnpublish) {
      this.record = v?.record && new HashTableMessage.Record(v.record);
    }

    static encode(m: Unpublish, w?: Writer): Writer {
      if (!w) w = new Writer(1024);
      if (m.record) HashTableMessage.Record.encode(m.record, w.uint32(10).fork()).ldelim();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Unpublish {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Unpublish();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.record = HashTableMessage.Record.decode(r, r.uint32());
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface IGetRequest {
    requestId?: bigint;
    hash?: Uint8Array;
    ifModifiedSince?: bigint;
  }

  export class GetRequest {
    requestId: bigint = BigInt(0);
    hash: Uint8Array = new Uint8Array();
    ifModifiedSince: bigint = BigInt(0);

    constructor(v?: IGetRequest) {
      this.requestId = v?.requestId || BigInt(0);
      this.hash = v?.hash || new Uint8Array();
      this.ifModifiedSince = v?.ifModifiedSince || BigInt(0);
    }

    static encode(m: GetRequest, w?: Writer): Writer {
      if (!w) w = new Writer(1024);
      if (m.requestId) w.uint32(8).uint64(m.requestId);
      if (m.hash) w.uint32(18).bytes(m.hash);
      if (m.ifModifiedSince) w.uint32(24).int64(m.ifModifiedSince);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): GetRequest {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new GetRequest();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.requestId = r.uint64();
          break;
          case 2:
          m.hash = r.bytes();
          break;
          case 3:
          m.ifModifiedSince = r.int64();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface IGetResponse {
    requestId?: bigint;
    record?: HashTableMessage.IRecord;
  }

  export class GetResponse {
    requestId: bigint = BigInt(0);
    record: HashTableMessage.Record | undefined;

    constructor(v?: IGetResponse) {
      this.requestId = v?.requestId || BigInt(0);
      this.record = v?.record && new HashTableMessage.Record(v.record);
    }

    static encode(m: GetResponse, w?: Writer): Writer {
      if (!w) w = new Writer(1024);
      if (m.requestId) w.uint32(8).uint64(m.requestId);
      if (m.record) HashTableMessage.Record.encode(m.record, w.uint32(18).fork()).ldelim();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): GetResponse {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new GetResponse();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.requestId = r.uint64();
          break;
          case 2:
          m.record = HashTableMessage.Record.decode(r, r.uint32());
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

}


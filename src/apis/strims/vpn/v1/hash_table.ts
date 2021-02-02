import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export interface IHashTableMessage {
  body?: HashTableMessage.IBody
}

export class HashTableMessage {
  body: HashTableMessage.TBody;

  constructor(v?: IHashTableMessage) {
    this.body = new HashTableMessage.Body(v?.body);
  }

  static encode(m: HashTableMessage, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case HashTableMessage.BodyCase.PUBLISH:
      HashTableMessage.Publish.encode(m.body.publish, w.uint32(10).fork()).ldelim();
      break;
      case HashTableMessage.BodyCase.UNPUBLISH:
      HashTableMessage.Unpublish.encode(m.body.unpublish, w.uint32(18).fork()).ldelim();
      break;
      case HashTableMessage.BodyCase.GET_REQUEST:
      HashTableMessage.GetRequest.encode(m.body.getRequest, w.uint32(26).fork()).ldelim();
      break;
      case HashTableMessage.BodyCase.GET_RESPONSE:
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
        m.body = new HashTableMessage.Body({ publish: HashTableMessage.Publish.decode(r, r.uint32()) });
        break;
        case 2:
        m.body = new HashTableMessage.Body({ unpublish: HashTableMessage.Unpublish.decode(r, r.uint32()) });
        break;
        case 3:
        m.body = new HashTableMessage.Body({ getRequest: HashTableMessage.GetRequest.decode(r, r.uint32()) });
        break;
        case 4:
        m.body = new HashTableMessage.Body({ getResponse: HashTableMessage.GetResponse.decode(r, r.uint32()) });
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
  export enum BodyCase {
    NOT_SET = 0,
    PUBLISH = 1,
    UNPUBLISH = 2,
    GET_REQUEST = 3,
    GET_RESPONSE = 4,
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.PUBLISH, publish: HashTableMessage.IPublish }
  |{ case?: BodyCase.UNPUBLISH, unpublish: HashTableMessage.IUnpublish }
  |{ case?: BodyCase.GET_REQUEST, getRequest: HashTableMessage.IGetRequest }
  |{ case?: BodyCase.GET_RESPONSE, getResponse: HashTableMessage.IGetResponse }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.PUBLISH, publish: HashTableMessage.Publish }
  |{ case: BodyCase.UNPUBLISH, unpublish: HashTableMessage.Unpublish }
  |{ case: BodyCase.GET_REQUEST, getRequest: HashTableMessage.GetRequest }
  |{ case: BodyCase.GET_RESPONSE, getResponse: HashTableMessage.GetResponse }
  >;

  class BodyImpl {
    publish: HashTableMessage.Publish;
    unpublish: HashTableMessage.Unpublish;
    getRequest: HashTableMessage.GetRequest;
    getResponse: HashTableMessage.GetResponse;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "publish" in v) {
        this.case = BodyCase.PUBLISH;
        this.publish = new HashTableMessage.Publish(v.publish);
      } else
      if (v && "unpublish" in v) {
        this.case = BodyCase.UNPUBLISH;
        this.unpublish = new HashTableMessage.Unpublish(v.unpublish);
      } else
      if (v && "getRequest" in v) {
        this.case = BodyCase.GET_REQUEST;
        this.getRequest = new HashTableMessage.GetRequest(v.getRequest);
      } else
      if (v && "getResponse" in v) {
        this.case = BodyCase.GET_RESPONSE;
        this.getResponse = new HashTableMessage.GetResponse(v.getResponse);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { publish: HashTableMessage.IPublish } ? { case: BodyCase.PUBLISH, publish: HashTableMessage.Publish } :
    T extends { unpublish: HashTableMessage.IUnpublish } ? { case: BodyCase.UNPUBLISH, unpublish: HashTableMessage.Unpublish } :
    T extends { getRequest: HashTableMessage.IGetRequest } ? { case: BodyCase.GET_REQUEST, getRequest: HashTableMessage.GetRequest } :
    T extends { getResponse: HashTableMessage.IGetResponse } ? { case: BodyCase.GET_RESPONSE, getResponse: HashTableMessage.GetResponse } :
    never
    >;
  };

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
      if (!w) w = new Writer();
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
    record?: HashTableMessage.IRecord | undefined;
  }

  export class Publish {
    record: HashTableMessage.Record | undefined;

    constructor(v?: IPublish) {
      this.record = v?.record && new HashTableMessage.Record(v.record);
    }

    static encode(m: Publish, w?: Writer): Writer {
      if (!w) w = new Writer();
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
    record?: HashTableMessage.IRecord | undefined;
  }

  export class Unpublish {
    record: HashTableMessage.Record | undefined;

    constructor(v?: IUnpublish) {
      this.record = v?.record && new HashTableMessage.Record(v.record);
    }

    static encode(m: Unpublish, w?: Writer): Writer {
      if (!w) w = new Writer();
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
      if (!w) w = new Writer();
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
    record?: HashTableMessage.IRecord | undefined;
  }

  export class GetResponse {
    requestId: bigint = BigInt(0);
    record: HashTableMessage.Record | undefined;

    constructor(v?: IGetResponse) {
      this.requestId = v?.requestId || BigInt(0);
      this.record = v?.record && new HashTableMessage.Record(v.record);
    }

    static encode(m: GetResponse, w?: Writer): Writer {
      if (!w) w = new Writer();
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


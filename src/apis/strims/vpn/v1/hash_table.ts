import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export type IHashTableMessage = {
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
      strims_vpn_v1_HashTableMessage_Publish.encode(m.body.publish, w.uint32(10).fork()).ldelim();
      break;
      case HashTableMessage.BodyCase.UNPUBLISH:
      strims_vpn_v1_HashTableMessage_Unpublish.encode(m.body.unpublish, w.uint32(18).fork()).ldelim();
      break;
      case HashTableMessage.BodyCase.GET_REQUEST:
      strims_vpn_v1_HashTableMessage_GetRequest.encode(m.body.getRequest, w.uint32(26).fork()).ldelim();
      break;
      case HashTableMessage.BodyCase.GET_RESPONSE:
      strims_vpn_v1_HashTableMessage_GetResponse.encode(m.body.getResponse, w.uint32(34).fork()).ldelim();
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
        m.body = new HashTableMessage.Body({ publish: strims_vpn_v1_HashTableMessage_Publish.decode(r, r.uint32()) });
        break;
        case 2:
        m.body = new HashTableMessage.Body({ unpublish: strims_vpn_v1_HashTableMessage_Unpublish.decode(r, r.uint32()) });
        break;
        case 3:
        m.body = new HashTableMessage.Body({ getRequest: strims_vpn_v1_HashTableMessage_GetRequest.decode(r, r.uint32()) });
        break;
        case 4:
        m.body = new HashTableMessage.Body({ getResponse: strims_vpn_v1_HashTableMessage_GetResponse.decode(r, r.uint32()) });
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
  |{ case?: BodyCase.PUBLISH, publish: strims_vpn_v1_HashTableMessage_IPublish }
  |{ case?: BodyCase.UNPUBLISH, unpublish: strims_vpn_v1_HashTableMessage_IUnpublish }
  |{ case?: BodyCase.GET_REQUEST, getRequest: strims_vpn_v1_HashTableMessage_IGetRequest }
  |{ case?: BodyCase.GET_RESPONSE, getResponse: strims_vpn_v1_HashTableMessage_IGetResponse }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.PUBLISH, publish: strims_vpn_v1_HashTableMessage_Publish }
  |{ case: BodyCase.UNPUBLISH, unpublish: strims_vpn_v1_HashTableMessage_Unpublish }
  |{ case: BodyCase.GET_REQUEST, getRequest: strims_vpn_v1_HashTableMessage_GetRequest }
  |{ case: BodyCase.GET_RESPONSE, getResponse: strims_vpn_v1_HashTableMessage_GetResponse }
  >;

  class BodyImpl {
    publish: strims_vpn_v1_HashTableMessage_Publish;
    unpublish: strims_vpn_v1_HashTableMessage_Unpublish;
    getRequest: strims_vpn_v1_HashTableMessage_GetRequest;
    getResponse: strims_vpn_v1_HashTableMessage_GetResponse;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "publish" in v) {
        this.case = BodyCase.PUBLISH;
        this.publish = new strims_vpn_v1_HashTableMessage_Publish(v.publish);
      } else
      if (v && "unpublish" in v) {
        this.case = BodyCase.UNPUBLISH;
        this.unpublish = new strims_vpn_v1_HashTableMessage_Unpublish(v.unpublish);
      } else
      if (v && "getRequest" in v) {
        this.case = BodyCase.GET_REQUEST;
        this.getRequest = new strims_vpn_v1_HashTableMessage_GetRequest(v.getRequest);
      } else
      if (v && "getResponse" in v) {
        this.case = BodyCase.GET_RESPONSE;
        this.getResponse = new strims_vpn_v1_HashTableMessage_GetResponse(v.getResponse);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { publish: strims_vpn_v1_HashTableMessage_IPublish } ? { case: BodyCase.PUBLISH, publish: strims_vpn_v1_HashTableMessage_Publish } :
    T extends { unpublish: strims_vpn_v1_HashTableMessage_IUnpublish } ? { case: BodyCase.UNPUBLISH, unpublish: strims_vpn_v1_HashTableMessage_Unpublish } :
    T extends { getRequest: strims_vpn_v1_HashTableMessage_IGetRequest } ? { case: BodyCase.GET_REQUEST, getRequest: strims_vpn_v1_HashTableMessage_GetRequest } :
    T extends { getResponse: strims_vpn_v1_HashTableMessage_IGetResponse } ? { case: BodyCase.GET_RESPONSE, getResponse: strims_vpn_v1_HashTableMessage_GetResponse } :
    never
    >;
  };

  export type IRecord = {
    key?: Uint8Array;
    salt?: Uint8Array;
    value?: Uint8Array;
    timestamp?: bigint;
    signature?: Uint8Array;
  }

  export class Record {
    key: Uint8Array;
    salt: Uint8Array;
    value: Uint8Array;
    timestamp: bigint;
    signature: Uint8Array;

    constructor(v?: IRecord) {
      this.key = v?.key || new Uint8Array();
      this.salt = v?.salt || new Uint8Array();
      this.value = v?.value || new Uint8Array();
      this.timestamp = v?.timestamp || BigInt(0);
      this.signature = v?.signature || new Uint8Array();
    }

    static encode(m: Record, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.key.length) w.uint32(10).bytes(m.key);
      if (m.salt.length) w.uint32(18).bytes(m.salt);
      if (m.value.length) w.uint32(26).bytes(m.value);
      if (m.timestamp) w.uint32(32).int64(m.timestamp);
      if (m.signature.length) w.uint32(42).bytes(m.signature);
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

  export type IPublish = {
    record?: strims_vpn_v1_HashTableMessage_IRecord;
  }

  export class Publish {
    record: strims_vpn_v1_HashTableMessage_Record | undefined;

    constructor(v?: IPublish) {
      this.record = v?.record && new strims_vpn_v1_HashTableMessage_Record(v.record);
    }

    static encode(m: Publish, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.record) strims_vpn_v1_HashTableMessage_Record.encode(m.record, w.uint32(10).fork()).ldelim();
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
          m.record = strims_vpn_v1_HashTableMessage_Record.decode(r, r.uint32());
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type IUnpublish = {
    record?: strims_vpn_v1_HashTableMessage_IRecord;
  }

  export class Unpublish {
    record: strims_vpn_v1_HashTableMessage_Record | undefined;

    constructor(v?: IUnpublish) {
      this.record = v?.record && new strims_vpn_v1_HashTableMessage_Record(v.record);
    }

    static encode(m: Unpublish, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.record) strims_vpn_v1_HashTableMessage_Record.encode(m.record, w.uint32(10).fork()).ldelim();
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
          m.record = strims_vpn_v1_HashTableMessage_Record.decode(r, r.uint32());
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type IGetRequest = {
    requestId?: bigint;
    hash?: Uint8Array;
    ifModifiedSince?: bigint;
  }

  export class GetRequest {
    requestId: bigint;
    hash: Uint8Array;
    ifModifiedSince: bigint;

    constructor(v?: IGetRequest) {
      this.requestId = v?.requestId || BigInt(0);
      this.hash = v?.hash || new Uint8Array();
      this.ifModifiedSince = v?.ifModifiedSince || BigInt(0);
    }

    static encode(m: GetRequest, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.requestId) w.uint32(8).uint64(m.requestId);
      if (m.hash.length) w.uint32(18).bytes(m.hash);
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

  export type IGetResponse = {
    requestId?: bigint;
    record?: strims_vpn_v1_HashTableMessage_IRecord;
  }

  export class GetResponse {
    requestId: bigint;
    record: strims_vpn_v1_HashTableMessage_Record | undefined;

    constructor(v?: IGetResponse) {
      this.requestId = v?.requestId || BigInt(0);
      this.record = v?.record && new strims_vpn_v1_HashTableMessage_Record(v.record);
    }

    static encode(m: GetResponse, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.requestId) w.uint32(8).uint64(m.requestId);
      if (m.record) strims_vpn_v1_HashTableMessage_Record.encode(m.record, w.uint32(18).fork()).ldelim();
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
          m.record = strims_vpn_v1_HashTableMessage_Record.decode(r, r.uint32());
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

/* @internal */
export const strims_vpn_v1_HashTableMessage = HashTableMessage;
/* @internal */
export type strims_vpn_v1_HashTableMessage = HashTableMessage;
/* @internal */
export type strims_vpn_v1_IHashTableMessage = IHashTableMessage;
/* @internal */
export const strims_vpn_v1_HashTableMessage_Record = HashTableMessage.Record;
/* @internal */
export type strims_vpn_v1_HashTableMessage_Record = HashTableMessage.Record;
/* @internal */
export type strims_vpn_v1_HashTableMessage_IRecord = HashTableMessage.IRecord;
/* @internal */
export const strims_vpn_v1_HashTableMessage_Publish = HashTableMessage.Publish;
/* @internal */
export type strims_vpn_v1_HashTableMessage_Publish = HashTableMessage.Publish;
/* @internal */
export type strims_vpn_v1_HashTableMessage_IPublish = HashTableMessage.IPublish;
/* @internal */
export const strims_vpn_v1_HashTableMessage_Unpublish = HashTableMessage.Unpublish;
/* @internal */
export type strims_vpn_v1_HashTableMessage_Unpublish = HashTableMessage.Unpublish;
/* @internal */
export type strims_vpn_v1_HashTableMessage_IUnpublish = HashTableMessage.IUnpublish;
/* @internal */
export const strims_vpn_v1_HashTableMessage_GetRequest = HashTableMessage.GetRequest;
/* @internal */
export type strims_vpn_v1_HashTableMessage_GetRequest = HashTableMessage.GetRequest;
/* @internal */
export type strims_vpn_v1_HashTableMessage_IGetRequest = HashTableMessage.IGetRequest;
/* @internal */
export const strims_vpn_v1_HashTableMessage_GetResponse = HashTableMessage.GetResponse;
/* @internal */
export type strims_vpn_v1_HashTableMessage_GetResponse = HashTableMessage.GetResponse;
/* @internal */
export type strims_vpn_v1_HashTableMessage_IGetResponse = HashTableMessage.IGetResponse;

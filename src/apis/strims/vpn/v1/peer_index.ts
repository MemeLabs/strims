import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export type IPeerIndexMessage = {
  body?: PeerIndexMessage.IBody
}

export class PeerIndexMessage {
  body: PeerIndexMessage.TBody;

  constructor(v?: IPeerIndexMessage) {
    this.body = new PeerIndexMessage.Body(v?.body);
  }

  static encode(m: PeerIndexMessage, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case PeerIndexMessage.BodyCase.PUBLISH:
      strims_vpn_v1_PeerIndexMessage_Publish.encode(m.body.publish, w.uint32(10).fork()).ldelim();
      break;
      case PeerIndexMessage.BodyCase.UNPUBLISH:
      strims_vpn_v1_PeerIndexMessage_Unpublish.encode(m.body.unpublish, w.uint32(18).fork()).ldelim();
      break;
      case PeerIndexMessage.BodyCase.SEARCH_REQUEST:
      strims_vpn_v1_PeerIndexMessage_SearchRequest.encode(m.body.searchRequest, w.uint32(26).fork()).ldelim();
      break;
      case PeerIndexMessage.BodyCase.SEARCH_RESPONSE:
      strims_vpn_v1_PeerIndexMessage_SearchResponse.encode(m.body.searchResponse, w.uint32(34).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PeerIndexMessage {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new PeerIndexMessage();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.body = new PeerIndexMessage.Body({ publish: strims_vpn_v1_PeerIndexMessage_Publish.decode(r, r.uint32()) });
        break;
        case 2:
        m.body = new PeerIndexMessage.Body({ unpublish: strims_vpn_v1_PeerIndexMessage_Unpublish.decode(r, r.uint32()) });
        break;
        case 3:
        m.body = new PeerIndexMessage.Body({ searchRequest: strims_vpn_v1_PeerIndexMessage_SearchRequest.decode(r, r.uint32()) });
        break;
        case 4:
        m.body = new PeerIndexMessage.Body({ searchResponse: strims_vpn_v1_PeerIndexMessage_SearchResponse.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace PeerIndexMessage {
  export enum BodyCase {
    NOT_SET = 0,
    PUBLISH = 1,
    UNPUBLISH = 2,
    SEARCH_REQUEST = 3,
    SEARCH_RESPONSE = 4,
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.PUBLISH, publish: strims_vpn_v1_PeerIndexMessage_IPublish }
  |{ case?: BodyCase.UNPUBLISH, unpublish: strims_vpn_v1_PeerIndexMessage_IUnpublish }
  |{ case?: BodyCase.SEARCH_REQUEST, searchRequest: strims_vpn_v1_PeerIndexMessage_ISearchRequest }
  |{ case?: BodyCase.SEARCH_RESPONSE, searchResponse: strims_vpn_v1_PeerIndexMessage_ISearchResponse }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.PUBLISH, publish: strims_vpn_v1_PeerIndexMessage_Publish }
  |{ case: BodyCase.UNPUBLISH, unpublish: strims_vpn_v1_PeerIndexMessage_Unpublish }
  |{ case: BodyCase.SEARCH_REQUEST, searchRequest: strims_vpn_v1_PeerIndexMessage_SearchRequest }
  |{ case: BodyCase.SEARCH_RESPONSE, searchResponse: strims_vpn_v1_PeerIndexMessage_SearchResponse }
  >;

  class BodyImpl {
    publish: strims_vpn_v1_PeerIndexMessage_Publish;
    unpublish: strims_vpn_v1_PeerIndexMessage_Unpublish;
    searchRequest: strims_vpn_v1_PeerIndexMessage_SearchRequest;
    searchResponse: strims_vpn_v1_PeerIndexMessage_SearchResponse;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "publish" in v) {
        this.case = BodyCase.PUBLISH;
        this.publish = new strims_vpn_v1_PeerIndexMessage_Publish(v.publish);
      } else
      if (v && "unpublish" in v) {
        this.case = BodyCase.UNPUBLISH;
        this.unpublish = new strims_vpn_v1_PeerIndexMessage_Unpublish(v.unpublish);
      } else
      if (v && "searchRequest" in v) {
        this.case = BodyCase.SEARCH_REQUEST;
        this.searchRequest = new strims_vpn_v1_PeerIndexMessage_SearchRequest(v.searchRequest);
      } else
      if (v && "searchResponse" in v) {
        this.case = BodyCase.SEARCH_RESPONSE;
        this.searchResponse = new strims_vpn_v1_PeerIndexMessage_SearchResponse(v.searchResponse);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { publish: strims_vpn_v1_PeerIndexMessage_IPublish } ? { case: BodyCase.PUBLISH, publish: strims_vpn_v1_PeerIndexMessage_Publish } :
    T extends { unpublish: strims_vpn_v1_PeerIndexMessage_IUnpublish } ? { case: BodyCase.UNPUBLISH, unpublish: strims_vpn_v1_PeerIndexMessage_Unpublish } :
    T extends { searchRequest: strims_vpn_v1_PeerIndexMessage_ISearchRequest } ? { case: BodyCase.SEARCH_REQUEST, searchRequest: strims_vpn_v1_PeerIndexMessage_SearchRequest } :
    T extends { searchResponse: strims_vpn_v1_PeerIndexMessage_ISearchResponse } ? { case: BodyCase.SEARCH_RESPONSE, searchResponse: strims_vpn_v1_PeerIndexMessage_SearchResponse } :
    never
    >;
  };

  export type IRecord = {
    hash?: Uint8Array;
    hostId?: Uint8Array;
    port?: number;
    timestamp?: bigint;
    key?: Uint8Array;
    signature?: Uint8Array;
  }

  export class Record {
    hash: Uint8Array;
    hostId: Uint8Array;
    port: number;
    timestamp: bigint;
    key: Uint8Array;
    signature: Uint8Array;

    constructor(v?: IRecord) {
      this.hash = v?.hash || new Uint8Array();
      this.hostId = v?.hostId || new Uint8Array();
      this.port = v?.port || 0;
      this.timestamp = v?.timestamp || BigInt(0);
      this.key = v?.key || new Uint8Array();
      this.signature = v?.signature || new Uint8Array();
    }

    static encode(m: Record, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.hash.length) w.uint32(10).bytes(m.hash);
      if (m.hostId.length) w.uint32(18).bytes(m.hostId);
      if (m.port) w.uint32(24).uint32(m.port);
      if (m.timestamp) w.uint32(32).int64(m.timestamp);
      if (m.key.length) w.uint32(80010).bytes(m.key);
      if (m.signature.length) w.uint32(80018).bytes(m.signature);
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
          m.hash = r.bytes();
          break;
          case 2:
          m.hostId = r.bytes();
          break;
          case 3:
          m.port = r.uint32();
          break;
          case 4:
          m.timestamp = r.int64();
          break;
          case 10001:
          m.key = r.bytes();
          break;
          case 10002:
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
    record?: strims_vpn_v1_PeerIndexMessage_IRecord;
  }

  export class Publish {
    record: strims_vpn_v1_PeerIndexMessage_Record | undefined;

    constructor(v?: IPublish) {
      this.record = v?.record && new strims_vpn_v1_PeerIndexMessage_Record(v.record);
    }

    static encode(m: Publish, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.record) strims_vpn_v1_PeerIndexMessage_Record.encode(m.record, w.uint32(10).fork()).ldelim();
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
          m.record = strims_vpn_v1_PeerIndexMessage_Record.decode(r, r.uint32());
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
    record?: strims_vpn_v1_PeerIndexMessage_IRecord;
  }

  export class Unpublish {
    record: strims_vpn_v1_PeerIndexMessage_Record | undefined;

    constructor(v?: IUnpublish) {
      this.record = v?.record && new strims_vpn_v1_PeerIndexMessage_Record(v.record);
    }

    static encode(m: Unpublish, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.record) strims_vpn_v1_PeerIndexMessage_Record.encode(m.record, w.uint32(10).fork()).ldelim();
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
          m.record = strims_vpn_v1_PeerIndexMessage_Record.decode(r, r.uint32());
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type ISearchRequest = {
    requestId?: bigint;
    hash?: Uint8Array;
  }

  export class SearchRequest {
    requestId: bigint;
    hash: Uint8Array;

    constructor(v?: ISearchRequest) {
      this.requestId = v?.requestId || BigInt(0);
      this.hash = v?.hash || new Uint8Array();
    }

    static encode(m: SearchRequest, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.requestId) w.uint32(8).uint64(m.requestId);
      if (m.hash.length) w.uint32(18).bytes(m.hash);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): SearchRequest {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new SearchRequest();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.requestId = r.uint64();
          break;
          case 2:
          m.hash = r.bytes();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type ISearchResponse = {
    requestId?: bigint;
    records?: strims_vpn_v1_PeerIndexMessage_IRecord[];
  }

  export class SearchResponse {
    requestId: bigint;
    records: strims_vpn_v1_PeerIndexMessage_Record[];

    constructor(v?: ISearchResponse) {
      this.requestId = v?.requestId || BigInt(0);
      this.records = v?.records ? v.records.map(v => new strims_vpn_v1_PeerIndexMessage_Record(v)) : [];
    }

    static encode(m: SearchResponse, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.requestId) w.uint32(8).uint64(m.requestId);
      for (const v of m.records) strims_vpn_v1_PeerIndexMessage_Record.encode(v, w.uint32(18).fork()).ldelim();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): SearchResponse {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new SearchResponse();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.requestId = r.uint64();
          break;
          case 2:
          m.records.push(strims_vpn_v1_PeerIndexMessage_Record.decode(r, r.uint32()));
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
export const strims_vpn_v1_PeerIndexMessage = PeerIndexMessage;
/* @internal */
export type strims_vpn_v1_PeerIndexMessage = PeerIndexMessage;
/* @internal */
export type strims_vpn_v1_IPeerIndexMessage = IPeerIndexMessage;
/* @internal */
export const strims_vpn_v1_PeerIndexMessage_Record = PeerIndexMessage.Record;
/* @internal */
export type strims_vpn_v1_PeerIndexMessage_Record = PeerIndexMessage.Record;
/* @internal */
export type strims_vpn_v1_PeerIndexMessage_IRecord = PeerIndexMessage.IRecord;
/* @internal */
export const strims_vpn_v1_PeerIndexMessage_Publish = PeerIndexMessage.Publish;
/* @internal */
export type strims_vpn_v1_PeerIndexMessage_Publish = PeerIndexMessage.Publish;
/* @internal */
export type strims_vpn_v1_PeerIndexMessage_IPublish = PeerIndexMessage.IPublish;
/* @internal */
export const strims_vpn_v1_PeerIndexMessage_Unpublish = PeerIndexMessage.Unpublish;
/* @internal */
export type strims_vpn_v1_PeerIndexMessage_Unpublish = PeerIndexMessage.Unpublish;
/* @internal */
export type strims_vpn_v1_PeerIndexMessage_IUnpublish = PeerIndexMessage.IUnpublish;
/* @internal */
export const strims_vpn_v1_PeerIndexMessage_SearchRequest = PeerIndexMessage.SearchRequest;
/* @internal */
export type strims_vpn_v1_PeerIndexMessage_SearchRequest = PeerIndexMessage.SearchRequest;
/* @internal */
export type strims_vpn_v1_PeerIndexMessage_ISearchRequest = PeerIndexMessage.ISearchRequest;
/* @internal */
export const strims_vpn_v1_PeerIndexMessage_SearchResponse = PeerIndexMessage.SearchResponse;
/* @internal */
export type strims_vpn_v1_PeerIndexMessage_SearchResponse = PeerIndexMessage.SearchResponse;
/* @internal */
export type strims_vpn_v1_PeerIndexMessage_ISearchResponse = PeerIndexMessage.ISearchResponse;

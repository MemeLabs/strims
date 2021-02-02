import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export interface IPeerIndexMessage {
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
      PeerIndexMessage.Publish.encode(m.body.publish, w.uint32(10).fork()).ldelim();
      break;
      case PeerIndexMessage.BodyCase.UNPUBLISH:
      PeerIndexMessage.Unpublish.encode(m.body.unpublish, w.uint32(18).fork()).ldelim();
      break;
      case PeerIndexMessage.BodyCase.SEARCH_REQUEST:
      PeerIndexMessage.SearchRequest.encode(m.body.searchRequest, w.uint32(26).fork()).ldelim();
      break;
      case PeerIndexMessage.BodyCase.SEARCH_RESPONSE:
      PeerIndexMessage.SearchResponse.encode(m.body.searchResponse, w.uint32(34).fork()).ldelim();
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
        m.body = new PeerIndexMessage.Body({ publish: PeerIndexMessage.Publish.decode(r, r.uint32()) });
        break;
        case 2:
        m.body = new PeerIndexMessage.Body({ unpublish: PeerIndexMessage.Unpublish.decode(r, r.uint32()) });
        break;
        case 3:
        m.body = new PeerIndexMessage.Body({ searchRequest: PeerIndexMessage.SearchRequest.decode(r, r.uint32()) });
        break;
        case 4:
        m.body = new PeerIndexMessage.Body({ searchResponse: PeerIndexMessage.SearchResponse.decode(r, r.uint32()) });
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
  |{ case?: BodyCase.PUBLISH, publish: PeerIndexMessage.IPublish }
  |{ case?: BodyCase.UNPUBLISH, unpublish: PeerIndexMessage.IUnpublish }
  |{ case?: BodyCase.SEARCH_REQUEST, searchRequest: PeerIndexMessage.ISearchRequest }
  |{ case?: BodyCase.SEARCH_RESPONSE, searchResponse: PeerIndexMessage.ISearchResponse }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.PUBLISH, publish: PeerIndexMessage.Publish }
  |{ case: BodyCase.UNPUBLISH, unpublish: PeerIndexMessage.Unpublish }
  |{ case: BodyCase.SEARCH_REQUEST, searchRequest: PeerIndexMessage.SearchRequest }
  |{ case: BodyCase.SEARCH_RESPONSE, searchResponse: PeerIndexMessage.SearchResponse }
  >;

  class BodyImpl {
    publish: PeerIndexMessage.Publish;
    unpublish: PeerIndexMessage.Unpublish;
    searchRequest: PeerIndexMessage.SearchRequest;
    searchResponse: PeerIndexMessage.SearchResponse;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "publish" in v) {
        this.case = BodyCase.PUBLISH;
        this.publish = new PeerIndexMessage.Publish(v.publish);
      } else
      if (v && "unpublish" in v) {
        this.case = BodyCase.UNPUBLISH;
        this.unpublish = new PeerIndexMessage.Unpublish(v.unpublish);
      } else
      if (v && "searchRequest" in v) {
        this.case = BodyCase.SEARCH_REQUEST;
        this.searchRequest = new PeerIndexMessage.SearchRequest(v.searchRequest);
      } else
      if (v && "searchResponse" in v) {
        this.case = BodyCase.SEARCH_RESPONSE;
        this.searchResponse = new PeerIndexMessage.SearchResponse(v.searchResponse);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { publish: PeerIndexMessage.IPublish } ? { case: BodyCase.PUBLISH, publish: PeerIndexMessage.Publish } :
    T extends { unpublish: PeerIndexMessage.IUnpublish } ? { case: BodyCase.UNPUBLISH, unpublish: PeerIndexMessage.Unpublish } :
    T extends { searchRequest: PeerIndexMessage.ISearchRequest } ? { case: BodyCase.SEARCH_REQUEST, searchRequest: PeerIndexMessage.SearchRequest } :
    T extends { searchResponse: PeerIndexMessage.ISearchResponse } ? { case: BodyCase.SEARCH_RESPONSE, searchResponse: PeerIndexMessage.SearchResponse } :
    never
    >;
  };

  export interface IRecord {
    hash?: Uint8Array;
    key?: Uint8Array;
    hostId?: Uint8Array;
    port?: number;
    timestamp?: bigint;
    signature?: Uint8Array;
  }

  export class Record {
    hash: Uint8Array = new Uint8Array();
    key: Uint8Array = new Uint8Array();
    hostId: Uint8Array = new Uint8Array();
    port: number = 0;
    timestamp: bigint = BigInt(0);
    signature: Uint8Array = new Uint8Array();

    constructor(v?: IRecord) {
      this.hash = v?.hash || new Uint8Array();
      this.key = v?.key || new Uint8Array();
      this.hostId = v?.hostId || new Uint8Array();
      this.port = v?.port || 0;
      this.timestamp = v?.timestamp || BigInt(0);
      this.signature = v?.signature || new Uint8Array();
    }

    static encode(m: Record, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.hash) w.uint32(10).bytes(m.hash);
      if (m.key) w.uint32(18).bytes(m.key);
      if (m.hostId) w.uint32(26).bytes(m.hostId);
      if (m.port) w.uint32(32).uint32(m.port);
      if (m.timestamp) w.uint32(40).int64(m.timestamp);
      if (m.signature) w.uint32(50).bytes(m.signature);
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
          m.key = r.bytes();
          break;
          case 3:
          m.hostId = r.bytes();
          break;
          case 4:
          m.port = r.uint32();
          break;
          case 5:
          m.timestamp = r.int64();
          break;
          case 6:
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
    record?: PeerIndexMessage.IRecord | undefined;
  }

  export class Publish {
    record: PeerIndexMessage.Record | undefined;

    constructor(v?: IPublish) {
      this.record = v?.record && new PeerIndexMessage.Record(v.record);
    }

    static encode(m: Publish, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.record) PeerIndexMessage.Record.encode(m.record, w.uint32(10).fork()).ldelim();
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
          m.record = PeerIndexMessage.Record.decode(r, r.uint32());
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
    record?: PeerIndexMessage.IRecord | undefined;
  }

  export class Unpublish {
    record: PeerIndexMessage.Record | undefined;

    constructor(v?: IUnpublish) {
      this.record = v?.record && new PeerIndexMessage.Record(v.record);
    }

    static encode(m: Unpublish, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.record) PeerIndexMessage.Record.encode(m.record, w.uint32(10).fork()).ldelim();
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
          m.record = PeerIndexMessage.Record.decode(r, r.uint32());
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface ISearchRequest {
    requestId?: bigint;
    hash?: Uint8Array;
  }

  export class SearchRequest {
    requestId: bigint = BigInt(0);
    hash: Uint8Array = new Uint8Array();

    constructor(v?: ISearchRequest) {
      this.requestId = v?.requestId || BigInt(0);
      this.hash = v?.hash || new Uint8Array();
    }

    static encode(m: SearchRequest, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.requestId) w.uint32(8).uint64(m.requestId);
      if (m.hash) w.uint32(18).bytes(m.hash);
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

  export interface ISearchResponse {
    requestId?: bigint;
    records?: PeerIndexMessage.IRecord[];
  }

  export class SearchResponse {
    requestId: bigint = BigInt(0);
    records: PeerIndexMessage.Record[] = [];

    constructor(v?: ISearchResponse) {
      this.requestId = v?.requestId || BigInt(0);
      if (v?.records) this.records = v.records.map(v => new PeerIndexMessage.Record(v));
    }

    static encode(m: SearchResponse, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.requestId) w.uint32(8).uint64(m.requestId);
      for (const v of m.records) PeerIndexMessage.Record.encode(v, w.uint32(18).fork()).ldelim();
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
          m.records.push(PeerIndexMessage.Record.decode(r, r.uint32()));
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


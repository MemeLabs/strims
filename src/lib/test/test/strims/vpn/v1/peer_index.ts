import Reader from "../../../../pb/reader";
import Writer from "../../../../pb/writer";


export interface IPeerIndexMessage {
  body?: PeerIndexMessage.IBodyOneOf
}

export class PeerIndexMessage {
  body: PeerIndexMessage.BodyOneOf;

  constructor(v?: IPeerIndexMessage) {
    this.body = new PeerIndexMessage.BodyOneOf(v?.body);
  }

  static encode(m: PeerIndexMessage, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    switch (m.body.case) {
      case 1:
      PeerIndexMessage.Publish.encode(m.body.publish, w.uint32(10).fork()).ldelim();
      break;
      case 2:
      PeerIndexMessage.Unpublish.encode(m.body.unpublish, w.uint32(18).fork()).ldelim();
      break;
      case 3:
      PeerIndexMessage.SearchRequest.encode(m.body.searchRequest, w.uint32(26).fork()).ldelim();
      break;
      case 4:
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
        m.body.publish = PeerIndexMessage.Publish.decode(r, r.uint32());
        break;
        case 2:
        m.body.unpublish = PeerIndexMessage.Unpublish.decode(r, r.uint32());
        break;
        case 3:
        m.body.searchRequest = PeerIndexMessage.SearchRequest.decode(r, r.uint32());
        break;
        case 4:
        m.body.searchResponse = PeerIndexMessage.SearchResponse.decode(r, r.uint32());
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
  export type IBodyOneOf =
  { publish: PeerIndexMessage.IPublish }
  |{ unpublish: PeerIndexMessage.IUnpublish }
  |{ searchRequest: PeerIndexMessage.ISearchRequest }
  |{ searchResponse: PeerIndexMessage.ISearchResponse }
  ;

  export class BodyOneOf {
    private _publish: PeerIndexMessage.Publish | undefined;
    private _unpublish: PeerIndexMessage.Unpublish | undefined;
    private _searchRequest: PeerIndexMessage.SearchRequest | undefined;
    private _searchResponse: PeerIndexMessage.SearchResponse | undefined;
    private _case: BodyCase = 0;

    constructor(v?: IBodyOneOf) {
      if (v && "publish" in v) this.publish = new PeerIndexMessage.Publish(v.publish);
      if (v && "unpublish" in v) this.unpublish = new PeerIndexMessage.Unpublish(v.unpublish);
      if (v && "searchRequest" in v) this.searchRequest = new PeerIndexMessage.SearchRequest(v.searchRequest);
      if (v && "searchResponse" in v) this.searchResponse = new PeerIndexMessage.SearchResponse(v.searchResponse);
    }

    public clear() {
      this._publish = undefined;
      this._unpublish = undefined;
      this._searchRequest = undefined;
      this._searchResponse = undefined;
      this._case = BodyCase.NOT_SET;
    }

    get case(): BodyCase {
      return this._case;
    }

    set publish(v: PeerIndexMessage.Publish) {
      this.clear();
      this._publish = v;
      this._case = BodyCase.PUBLISH;
    }

    get publish(): PeerIndexMessage.Publish {
      return this._publish;
    }

    set unpublish(v: PeerIndexMessage.Unpublish) {
      this.clear();
      this._unpublish = v;
      this._case = BodyCase.UNPUBLISH;
    }

    get unpublish(): PeerIndexMessage.Unpublish {
      return this._unpublish;
    }

    set searchRequest(v: PeerIndexMessage.SearchRequest) {
      this.clear();
      this._searchRequest = v;
      this._case = BodyCase.SEARCH_REQUEST;
    }

    get searchRequest(): PeerIndexMessage.SearchRequest {
      return this._searchRequest;
    }

    set searchResponse(v: PeerIndexMessage.SearchResponse) {
      this.clear();
      this._searchResponse = v;
      this._case = BodyCase.SEARCH_RESPONSE;
    }

    get searchResponse(): PeerIndexMessage.SearchResponse {
      return this._searchResponse;
    }
  }

  export enum BodyCase {
    NOT_SET = 0,
    PUBLISH = 1,
    UNPUBLISH = 2,
    SEARCH_REQUEST = 3,
    SEARCH_RESPONSE = 4,
  }

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
      if (!w) w = new Writer(1024);
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
    record?: PeerIndexMessage.IRecord;
  }

  export class Publish {
    record: PeerIndexMessage.Record | undefined;

    constructor(v?: IPublish) {
      this.record = v?.record && new PeerIndexMessage.Record(v.record);
    }

    static encode(m: Publish, w?: Writer): Writer {
      if (!w) w = new Writer(1024);
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
    record?: PeerIndexMessage.IRecord;
  }

  export class Unpublish {
    record: PeerIndexMessage.Record | undefined;

    constructor(v?: IUnpublish) {
      this.record = v?.record && new PeerIndexMessage.Record(v.record);
    }

    static encode(m: Unpublish, w?: Writer): Writer {
      if (!w) w = new Writer(1024);
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
      if (!w) w = new Writer(1024);
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
      if (!w) w = new Writer(1024);
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


// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export type IPeerIndexMessage = {
  body?: PeerIndexMessage_IBody
}

export class PeerIndexMessage {
  body: PeerIndexMessage_TBody;

  constructor(v?: IPeerIndexMessage) {
    this.body = v?.body ? new PeerIndexMessage_Body(v.body) : new PeerIndexMessage_Body();
  }

  static encode(m: PeerIndexMessage, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case PeerIndexMessage_BodyCase.PUBLISH:
      PeerIndexMessage_Publish.encode(m.body.publish, w.uint32(10).fork()).ldelim();
      break;
      case PeerIndexMessage_BodyCase.UNPUBLISH:
      PeerIndexMessage_Unpublish.encode(m.body.unpublish, w.uint32(18).fork()).ldelim();
      break;
      case PeerIndexMessage_BodyCase.SEARCH_REQUEST:
      PeerIndexMessage_SearchRequest.encode(m.body.searchRequest, w.uint32(26).fork()).ldelim();
      break;
      case PeerIndexMessage_BodyCase.SEARCH_RESPONSE:
      PeerIndexMessage_SearchResponse.encode(m.body.searchResponse, w.uint32(34).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PeerIndexMessage {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new PeerIndexMessage();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.body = new PeerIndexMessage_Body({ publish: PeerIndexMessage_Publish.decode(r, r.uint32()) });
        break;
        case 2:
        m.body = new PeerIndexMessage_Body({ unpublish: PeerIndexMessage_Unpublish.decode(r, r.uint32()) });
        break;
        case 3:
        m.body = new PeerIndexMessage_Body({ searchRequest: PeerIndexMessage_SearchRequest.decode(r, r.uint32()) });
        break;
        case 4:
        m.body = new PeerIndexMessage_Body({ searchResponse: PeerIndexMessage_SearchResponse.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export enum PeerIndexMessage_BodyCase {
  NOT_SET = 0,
  PUBLISH = 1,
  UNPUBLISH = 2,
  SEARCH_REQUEST = 3,
  SEARCH_RESPONSE = 4,
}

export type PeerIndexMessage_IBody =
{ case?: PeerIndexMessage_BodyCase.NOT_SET }
|{ case?: PeerIndexMessage_BodyCase.PUBLISH, publish: PeerIndexMessage_IPublish }
|{ case?: PeerIndexMessage_BodyCase.UNPUBLISH, unpublish: PeerIndexMessage_IUnpublish }
|{ case?: PeerIndexMessage_BodyCase.SEARCH_REQUEST, searchRequest: PeerIndexMessage_ISearchRequest }
|{ case?: PeerIndexMessage_BodyCase.SEARCH_RESPONSE, searchResponse: PeerIndexMessage_ISearchResponse }
;

export type PeerIndexMessage_TBody = Readonly<
{ case: PeerIndexMessage_BodyCase.NOT_SET }
|{ case: PeerIndexMessage_BodyCase.PUBLISH, publish: PeerIndexMessage_Publish }
|{ case: PeerIndexMessage_BodyCase.UNPUBLISH, unpublish: PeerIndexMessage_Unpublish }
|{ case: PeerIndexMessage_BodyCase.SEARCH_REQUEST, searchRequest: PeerIndexMessage_SearchRequest }
|{ case: PeerIndexMessage_BodyCase.SEARCH_RESPONSE, searchResponse: PeerIndexMessage_SearchResponse }
>;

class PeerIndexMessage_BodyImpl {
  declare publish?: PeerIndexMessage_Publish;
  declare unpublish?: PeerIndexMessage_Unpublish;
  declare searchRequest?: PeerIndexMessage_SearchRequest;
  declare searchResponse?: PeerIndexMessage_SearchResponse;
  case: PeerIndexMessage_BodyCase = PeerIndexMessage_BodyCase.NOT_SET;

  constructor(v?: PeerIndexMessage_IBody) {
    if (v && "publish" in v) {
      this.case = PeerIndexMessage_BodyCase.PUBLISH;
      this.publish = new PeerIndexMessage_Publish(v.publish);
    } else
    if (v && "unpublish" in v) {
      this.case = PeerIndexMessage_BodyCase.UNPUBLISH;
      this.unpublish = new PeerIndexMessage_Unpublish(v.unpublish);
    } else
    if (v && "searchRequest" in v) {
      this.case = PeerIndexMessage_BodyCase.SEARCH_REQUEST;
      this.searchRequest = new PeerIndexMessage_SearchRequest(v.searchRequest);
    } else
    if (v && "searchResponse" in v) {
      this.case = PeerIndexMessage_BodyCase.SEARCH_RESPONSE;
      this.searchResponse = new PeerIndexMessage_SearchResponse(v.searchResponse);
    }
  }
}

export const PeerIndexMessage_Body = PeerIndexMessage_BodyImpl as {
  new (): Readonly<{ case: PeerIndexMessage_BodyCase.NOT_SET }>;
  new <T extends PeerIndexMessage_IBody>(v: T): Readonly<
  T extends { publish: PeerIndexMessage_IPublish } ? { case: PeerIndexMessage_BodyCase.PUBLISH, publish: PeerIndexMessage_Publish } :
  T extends { unpublish: PeerIndexMessage_IUnpublish } ? { case: PeerIndexMessage_BodyCase.UNPUBLISH, unpublish: PeerIndexMessage_Unpublish } :
  T extends { searchRequest: PeerIndexMessage_ISearchRequest } ? { case: PeerIndexMessage_BodyCase.SEARCH_REQUEST, searchRequest: PeerIndexMessage_SearchRequest } :
  T extends { searchResponse: PeerIndexMessage_ISearchResponse } ? { case: PeerIndexMessage_BodyCase.SEARCH_RESPONSE, searchResponse: PeerIndexMessage_SearchResponse } :
  never
  >;
};

/**
 * What a record states. Unpublish marks a record that withdraws the
 * host's earlier ones for the hash.
 */
export type PeerIndexMessage_IRecordBody = {
  hash?: Uint8Array;
  hostId?: Uint8Array;
  port?: number;
  timestamp?: bigint;
  unpublish?: boolean;
}

/**
 * What a record states. Unpublish marks a record that withdraws the
 * host's earlier ones for the hash.
 */
export class PeerIndexMessage_RecordBody {
  hash: Uint8Array;
  hostId: Uint8Array;
  port: number;
  timestamp: bigint;
  unpublish: boolean;

  constructor(v?: PeerIndexMessage_IRecordBody) {
    this.hash = v?.hash || new Uint8Array();
    this.hostId = v?.hostId || new Uint8Array();
    this.port = v?.port || 0;
    this.timestamp = v?.timestamp || BigInt(0);
    this.unpublish = v?.unpublish || false;
  }

  static encode(m: PeerIndexMessage_RecordBody, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.hash.length) w.uint32(10).bytes(m.hash);
    if (m.hostId.length) w.uint32(18).bytes(m.hostId);
    if (m.port) w.uint32(24).uint32(m.port);
    if (m.timestamp) w.uint32(32).int64(m.timestamp);
    if (m.unpublish) w.uint32(40).bool(m.unpublish);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PeerIndexMessage_RecordBody {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new PeerIndexMessage_RecordBody();
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
        case 5:
        m.unpublish = r.bool();
        break;
        default:
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

/**
 * A RecordBody's encoding, signed by its host in the
 * vpn-peer-index-record domain.
 */
export type PeerIndexMessage_IRecord = {
  body?: Uint8Array;
  signature?: Uint8Array;
}

/**
 * A RecordBody's encoding, signed by its host in the
 * vpn-peer-index-record domain.
 */
export class PeerIndexMessage_Record {
  body: Uint8Array;
  signature: Uint8Array;

  constructor(v?: PeerIndexMessage_IRecord) {
    this.body = v?.body || new Uint8Array();
    this.signature = v?.signature || new Uint8Array();
  }

  static encode(m: PeerIndexMessage_Record, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.body.length) w.uint32(10).bytes(m.body);
    if (m.signature.length) w.uint32(18).bytes(m.signature);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PeerIndexMessage_Record {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new PeerIndexMessage_Record();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.body = r.bytes();
        break;
        case 2:
        m.signature = r.bytes();
        break;
        default:
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type PeerIndexMessage_IPublish = {
  record?: PeerIndexMessage_IRecord;
}

export class PeerIndexMessage_Publish {
  record: PeerIndexMessage_Record | undefined;

  constructor(v?: PeerIndexMessage_IPublish) {
    this.record = v?.record && new PeerIndexMessage_Record(v.record);
  }

  static encode(m: PeerIndexMessage_Publish, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.record) PeerIndexMessage_Record.encode(m.record, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PeerIndexMessage_Publish {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new PeerIndexMessage_Publish();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.record = PeerIndexMessage_Record.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type PeerIndexMessage_IUnpublish = {
  record?: PeerIndexMessage_IRecord;
}

export class PeerIndexMessage_Unpublish {
  record: PeerIndexMessage_Record | undefined;

  constructor(v?: PeerIndexMessage_IUnpublish) {
    this.record = v?.record && new PeerIndexMessage_Record(v.record);
  }

  static encode(m: PeerIndexMessage_Unpublish, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.record) PeerIndexMessage_Record.encode(m.record, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PeerIndexMessage_Unpublish {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new PeerIndexMessage_Unpublish();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.record = PeerIndexMessage_Record.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type PeerIndexMessage_ISearchRequest = {
  requestId?: bigint;
  hash?: Uint8Array;
}

export class PeerIndexMessage_SearchRequest {
  requestId: bigint;
  hash: Uint8Array;

  constructor(v?: PeerIndexMessage_ISearchRequest) {
    this.requestId = v?.requestId || BigInt(0);
    this.hash = v?.hash || new Uint8Array();
  }

  static encode(m: PeerIndexMessage_SearchRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.requestId) w.uint32(8).uint64(m.requestId);
    if (m.hash.length) w.uint32(18).bytes(m.hash);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PeerIndexMessage_SearchRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new PeerIndexMessage_SearchRequest();
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type PeerIndexMessage_ISearchResponse = {
  requestId?: bigint;
  records?: PeerIndexMessage_IRecord[];
}

export class PeerIndexMessage_SearchResponse {
  requestId: bigint;
  records: PeerIndexMessage_Record[];

  constructor(v?: PeerIndexMessage_ISearchResponse) {
    this.requestId = v?.requestId || BigInt(0);
    this.records = v?.records ? v.records.map(v => new PeerIndexMessage_Record(v)) : [];
  }

  static encode(m: PeerIndexMessage_SearchResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.requestId) w.uint32(8).uint64(m.requestId);
    for (const v of m.records) PeerIndexMessage_Record.encode(v, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PeerIndexMessage_SearchResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new PeerIndexMessage_SearchResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.requestId = r.uint64();
        break;
        case 2:
        m.records.push(PeerIndexMessage_Record.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}


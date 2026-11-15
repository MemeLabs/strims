// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export type IHashTableMessage = {
  body?: HashTableMessage_IBody
}

export class HashTableMessage {
  body: HashTableMessage_TBody;

  constructor(v?: IHashTableMessage) {
    this.body = v?.body ? new HashTableMessage_Body(v.body) : new HashTableMessage_Body();
  }

  static encode(m: HashTableMessage, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case HashTableMessage_BodyCase.PUBLISH:
      HashTableMessage_Publish.encode(m.body.publish, w.uint32(10).fork()).ldelim();
      break;
      case HashTableMessage_BodyCase.UNPUBLISH:
      HashTableMessage_Unpublish.encode(m.body.unpublish, w.uint32(18).fork()).ldelim();
      break;
      case HashTableMessage_BodyCase.GET_REQUEST:
      HashTableMessage_GetRequest.encode(m.body.getRequest, w.uint32(26).fork()).ldelim();
      break;
      case HashTableMessage_BodyCase.GET_RESPONSE:
      HashTableMessage_GetResponse.encode(m.body.getResponse, w.uint32(34).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): HashTableMessage {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new HashTableMessage();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.body = new HashTableMessage_Body({ publish: HashTableMessage_Publish.decode(r, r.uint32()) });
        break;
        case 2:
        m.body = new HashTableMessage_Body({ unpublish: HashTableMessage_Unpublish.decode(r, r.uint32()) });
        break;
        case 3:
        m.body = new HashTableMessage_Body({ getRequest: HashTableMessage_GetRequest.decode(r, r.uint32()) });
        break;
        case 4:
        m.body = new HashTableMessage_Body({ getResponse: HashTableMessage_GetResponse.decode(r, r.uint32()) });
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

export enum HashTableMessage_BodyCase {
  NOT_SET = 0,
  PUBLISH = 1,
  UNPUBLISH = 2,
  GET_REQUEST = 3,
  GET_RESPONSE = 4,
}

export type HashTableMessage_IBody =
{ case?: HashTableMessage_BodyCase.NOT_SET }
|{ case?: HashTableMessage_BodyCase.PUBLISH, publish: HashTableMessage_IPublish }
|{ case?: HashTableMessage_BodyCase.UNPUBLISH, unpublish: HashTableMessage_IUnpublish }
|{ case?: HashTableMessage_BodyCase.GET_REQUEST, getRequest: HashTableMessage_IGetRequest }
|{ case?: HashTableMessage_BodyCase.GET_RESPONSE, getResponse: HashTableMessage_IGetResponse }
;

export type HashTableMessage_TBody = Readonly<
{ case: HashTableMessage_BodyCase.NOT_SET }
|{ case: HashTableMessage_BodyCase.PUBLISH, publish: HashTableMessage_Publish }
|{ case: HashTableMessage_BodyCase.UNPUBLISH, unpublish: HashTableMessage_Unpublish }
|{ case: HashTableMessage_BodyCase.GET_REQUEST, getRequest: HashTableMessage_GetRequest }
|{ case: HashTableMessage_BodyCase.GET_RESPONSE, getResponse: HashTableMessage_GetResponse }
>;

class HashTableMessage_BodyImpl {
  declare publish?: HashTableMessage_Publish;
  declare unpublish?: HashTableMessage_Unpublish;
  declare getRequest?: HashTableMessage_GetRequest;
  declare getResponse?: HashTableMessage_GetResponse;
  case: HashTableMessage_BodyCase = HashTableMessage_BodyCase.NOT_SET;

  constructor(v?: HashTableMessage_IBody) {
    if (v && "publish" in v) {
      this.case = HashTableMessage_BodyCase.PUBLISH;
      this.publish = new HashTableMessage_Publish(v.publish);
    } else
    if (v && "unpublish" in v) {
      this.case = HashTableMessage_BodyCase.UNPUBLISH;
      this.unpublish = new HashTableMessage_Unpublish(v.unpublish);
    } else
    if (v && "getRequest" in v) {
      this.case = HashTableMessage_BodyCase.GET_REQUEST;
      this.getRequest = new HashTableMessage_GetRequest(v.getRequest);
    } else
    if (v && "getResponse" in v) {
      this.case = HashTableMessage_BodyCase.GET_RESPONSE;
      this.getResponse = new HashTableMessage_GetResponse(v.getResponse);
    }
  }
}

export const HashTableMessage_Body = HashTableMessage_BodyImpl as {
  new (): Readonly<{ case: HashTableMessage_BodyCase.NOT_SET }>;
  new <T extends HashTableMessage_IBody>(v: T): Readonly<
  T extends { publish: HashTableMessage_IPublish } ? { case: HashTableMessage_BodyCase.PUBLISH, publish: HashTableMessage_Publish } :
  T extends { unpublish: HashTableMessage_IUnpublish } ? { case: HashTableMessage_BodyCase.UNPUBLISH, unpublish: HashTableMessage_Unpublish } :
  T extends { getRequest: HashTableMessage_IGetRequest } ? { case: HashTableMessage_BodyCase.GET_REQUEST, getRequest: HashTableMessage_GetRequest } :
  T extends { getResponse: HashTableMessage_IGetResponse } ? { case: HashTableMessage_BodyCase.GET_RESPONSE, getResponse: HashTableMessage_GetResponse } :
  never
  >;
};

/**
 * What a record states. Unpublish marks a record that withdraws the
 * value published under (key, salt).
 */
export type HashTableMessage_IRecordBody = {
  key?: Uint8Array;
  salt?: Uint8Array;
  value?: Uint8Array;
  timestamp?: bigint;
  unpublish?: boolean;
}

/**
 * What a record states. Unpublish marks a record that withdraws the
 * value published under (key, salt).
 */
export class HashTableMessage_RecordBody {
  key: Uint8Array;
  salt: Uint8Array;
  value: Uint8Array;
  timestamp: bigint;
  unpublish: boolean;

  constructor(v?: HashTableMessage_IRecordBody) {
    this.key = v?.key || new Uint8Array();
    this.salt = v?.salt || new Uint8Array();
    this.value = v?.value || new Uint8Array();
    this.timestamp = v?.timestamp || BigInt(0);
    this.unpublish = v?.unpublish || false;
  }

  static encode(m: HashTableMessage_RecordBody, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.key.length) w.uint32(10).bytes(m.key);
    if (m.salt.length) w.uint32(18).bytes(m.salt);
    if (m.value.length) w.uint32(26).bytes(m.value);
    if (m.timestamp) w.uint32(32).int64(m.timestamp);
    if (m.unpublish) w.uint32(40).bool(m.unpublish);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): HashTableMessage_RecordBody {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new HashTableMessage_RecordBody();
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
 * A RecordBody's encoding, signed by its key in the
 * vpn-hash-table-record domain.
 */
export type HashTableMessage_IRecord = {
  body?: Uint8Array;
  signature?: Uint8Array;
}

/**
 * A RecordBody's encoding, signed by its key in the
 * vpn-hash-table-record domain.
 */
export class HashTableMessage_Record {
  body: Uint8Array;
  signature: Uint8Array;

  constructor(v?: HashTableMessage_IRecord) {
    this.body = v?.body || new Uint8Array();
    this.signature = v?.signature || new Uint8Array();
  }

  static encode(m: HashTableMessage_Record, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.body.length) w.uint32(10).bytes(m.body);
    if (m.signature.length) w.uint32(18).bytes(m.signature);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): HashTableMessage_Record {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new HashTableMessage_Record();
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

export type HashTableMessage_IPublish = {
  record?: HashTableMessage_IRecord;
}

export class HashTableMessage_Publish {
  record: HashTableMessage_Record | undefined;

  constructor(v?: HashTableMessage_IPublish) {
    this.record = v?.record && new HashTableMessage_Record(v.record);
  }

  static encode(m: HashTableMessage_Publish, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.record) HashTableMessage_Record.encode(m.record, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): HashTableMessage_Publish {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new HashTableMessage_Publish();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.record = HashTableMessage_Record.decode(r, r.uint32());
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

export type HashTableMessage_IUnpublish = {
  record?: HashTableMessage_IRecord;
}

export class HashTableMessage_Unpublish {
  record: HashTableMessage_Record | undefined;

  constructor(v?: HashTableMessage_IUnpublish) {
    this.record = v?.record && new HashTableMessage_Record(v.record);
  }

  static encode(m: HashTableMessage_Unpublish, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.record) HashTableMessage_Record.encode(m.record, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): HashTableMessage_Unpublish {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new HashTableMessage_Unpublish();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.record = HashTableMessage_Record.decode(r, r.uint32());
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

export type HashTableMessage_IGetRequest = {
  requestId?: bigint;
  hash?: Uint8Array;
  ifModifiedSince?: bigint;
}

export class HashTableMessage_GetRequest {
  requestId: bigint;
  hash: Uint8Array;
  ifModifiedSince: bigint;

  constructor(v?: HashTableMessage_IGetRequest) {
    this.requestId = v?.requestId || BigInt(0);
    this.hash = v?.hash || new Uint8Array();
    this.ifModifiedSince = v?.ifModifiedSince || BigInt(0);
  }

  static encode(m: HashTableMessage_GetRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.requestId) w.uint32(8).uint64(m.requestId);
    if (m.hash.length) w.uint32(18).bytes(m.hash);
    if (m.ifModifiedSince) w.uint32(24).int64(m.ifModifiedSince);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): HashTableMessage_GetRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new HashTableMessage_GetRequest();
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type HashTableMessage_IGetResponse = {
  requestId?: bigint;
  record?: HashTableMessage_IRecord;
}

export class HashTableMessage_GetResponse {
  requestId: bigint;
  record: HashTableMessage_Record | undefined;

  constructor(v?: HashTableMessage_IGetResponse) {
    this.requestId = v?.requestId || BigInt(0);
    this.record = v?.record && new HashTableMessage_Record(v.record);
  }

  static encode(m: HashTableMessage_GetResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.requestId) w.uint32(8).uint64(m.requestId);
    if (m.record) HashTableMessage_Record.encode(m.record, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): HashTableMessage_GetResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new HashTableMessage_GetResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.requestId = r.uint64();
        break;
        case 2:
        m.record = HashTableMessage_Record.decode(r, r.uint32());
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


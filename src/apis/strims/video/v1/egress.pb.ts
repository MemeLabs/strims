// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export type IEgressOpenStreamRequest = {
  swarmUri?: string;
  networkKeys?: Uint8Array[];
  /** The MIME type of the stream's listing, which names its format. */
  mimeType?: string;
}

export class EgressOpenStreamRequest {
  swarmUri: string;
  networkKeys: Uint8Array[];
  /** The MIME type of the stream's listing, which names its format. */
  mimeType: string;

  constructor(v?: IEgressOpenStreamRequest) {
    this.swarmUri = v?.swarmUri || "";
    this.networkKeys = v?.networkKeys ? v.networkKeys : [];
    this.mimeType = v?.mimeType || "";
  }

  static encode(m: EgressOpenStreamRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.swarmUri.length) w.uint32(10).string(m.swarmUri);
    for (const v of m.networkKeys) w.uint32(18).bytes(v);
    if (m.mimeType.length) w.uint32(26).string(m.mimeType);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): EgressOpenStreamRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new EgressOpenStreamRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.swarmUri = r.string();
        break;
        case 2:
        m.networkKeys.push(r.bytes())
        break;
        case 3:
        m.mimeType = r.string();
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

export type IEgressOpenStreamResponse = {
  body?: EgressOpenStreamResponse_IBody
}

export class EgressOpenStreamResponse {
  body: EgressOpenStreamResponse_TBody;

  constructor(v?: IEgressOpenStreamResponse) {
    this.body = v?.body ? new EgressOpenStreamResponse_Body(v.body) : new EgressOpenStreamResponse_Body();
  }

  static encode(m: EgressOpenStreamResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case EgressOpenStreamResponse_BodyCase.OPEN:
      EgressOpenStreamResponse_Open.encode(m.body.open, w.uint32(10).fork()).ldelim();
      break;
      case EgressOpenStreamResponse_BodyCase.DATA:
      EgressOpenStreamResponse_Data.encode(m.body.data, w.uint32(18).fork()).ldelim();
      break;
      case EgressOpenStreamResponse_BodyCase.ERROR:
      EgressOpenStreamResponse_Error.encode(m.body.error, w.uint32(26).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): EgressOpenStreamResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new EgressOpenStreamResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.body = new EgressOpenStreamResponse_Body({ open: EgressOpenStreamResponse_Open.decode(r, r.uint32()) });
        break;
        case 2:
        m.body = new EgressOpenStreamResponse_Body({ data: EgressOpenStreamResponse_Data.decode(r, r.uint32()) });
        break;
        case 3:
        m.body = new EgressOpenStreamResponse_Body({ error: EgressOpenStreamResponse_Error.decode(r, r.uint32()) });
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

export enum EgressOpenStreamResponse_BodyCase {
  NOT_SET = 0,
  OPEN = 1,
  DATA = 2,
  ERROR = 3,
}

export type EgressOpenStreamResponse_IBody =
{ case?: EgressOpenStreamResponse_BodyCase.NOT_SET }
|{ case?: EgressOpenStreamResponse_BodyCase.OPEN, open: EgressOpenStreamResponse_IOpen }
|{ case?: EgressOpenStreamResponse_BodyCase.DATA, data: EgressOpenStreamResponse_IData }
|{ case?: EgressOpenStreamResponse_BodyCase.ERROR, error: EgressOpenStreamResponse_IError }
;

export type EgressOpenStreamResponse_TBody = Readonly<
{ case: EgressOpenStreamResponse_BodyCase.NOT_SET }
|{ case: EgressOpenStreamResponse_BodyCase.OPEN, open: EgressOpenStreamResponse_Open }
|{ case: EgressOpenStreamResponse_BodyCase.DATA, data: EgressOpenStreamResponse_Data }
|{ case: EgressOpenStreamResponse_BodyCase.ERROR, error: EgressOpenStreamResponse_Error }
>;

class EgressOpenStreamResponse_BodyImpl {
  declare open?: EgressOpenStreamResponse_Open;
  declare data?: EgressOpenStreamResponse_Data;
  declare error?: EgressOpenStreamResponse_Error;
  case: EgressOpenStreamResponse_BodyCase = EgressOpenStreamResponse_BodyCase.NOT_SET;

  constructor(v?: EgressOpenStreamResponse_IBody) {
    if (v && "open" in v) {
      this.case = EgressOpenStreamResponse_BodyCase.OPEN;
      this.open = new EgressOpenStreamResponse_Open(v.open);
    } else
    if (v && "data" in v) {
      this.case = EgressOpenStreamResponse_BodyCase.DATA;
      this.data = new EgressOpenStreamResponse_Data(v.data);
    } else
    if (v && "error" in v) {
      this.case = EgressOpenStreamResponse_BodyCase.ERROR;
      this.error = new EgressOpenStreamResponse_Error(v.error);
    }
  }
}

export const EgressOpenStreamResponse_Body = EgressOpenStreamResponse_BodyImpl as {
  new (): Readonly<{ case: EgressOpenStreamResponse_BodyCase.NOT_SET }>;
  new <T extends EgressOpenStreamResponse_IBody>(v: T): Readonly<
  T extends { open: EgressOpenStreamResponse_IOpen } ? { case: EgressOpenStreamResponse_BodyCase.OPEN, open: EgressOpenStreamResponse_Open } :
  T extends { data: EgressOpenStreamResponse_IData } ? { case: EgressOpenStreamResponse_BodyCase.DATA, data: EgressOpenStreamResponse_Data } :
  T extends { error: EgressOpenStreamResponse_IError } ? { case: EgressOpenStreamResponse_BodyCase.ERROR, error: EgressOpenStreamResponse_Error } :
  never
  >;
};

export type EgressOpenStreamResponse_IOpen = {
  transferId?: Uint8Array;
}

export class EgressOpenStreamResponse_Open {
  transferId: Uint8Array;

  constructor(v?: EgressOpenStreamResponse_IOpen) {
    this.transferId = v?.transferId || new Uint8Array();
  }

  static encode(m: EgressOpenStreamResponse_Open, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.transferId.length) w.uint32(10).bytes(m.transferId);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): EgressOpenStreamResponse_Open {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new EgressOpenStreamResponse_Open();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.transferId = r.bytes();
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

export type EgressOpenStreamResponse_IData = {
  data?: Uint8Array;
  segmentEnd?: boolean;
  discontinuity?: boolean;
}

export class EgressOpenStreamResponse_Data {
  data: Uint8Array;
  segmentEnd: boolean;
  discontinuity: boolean;

  constructor(v?: EgressOpenStreamResponse_IData) {
    this.data = v?.data || new Uint8Array();
    this.segmentEnd = v?.segmentEnd || false;
    this.discontinuity = v?.discontinuity || false;
  }

  static encode(m: EgressOpenStreamResponse_Data, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.data.length) w.uint32(10).bytes(m.data);
    if (m.segmentEnd) w.uint32(16).bool(m.segmentEnd);
    if (m.discontinuity) w.uint32(24).bool(m.discontinuity);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): EgressOpenStreamResponse_Data {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new EgressOpenStreamResponse_Data();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.data = r.bytes();
        break;
        case 2:
        m.segmentEnd = r.bool();
        break;
        case 3:
        m.discontinuity = r.bool();
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

export type EgressOpenStreamResponse_IError = {
  message?: string;
}

export class EgressOpenStreamResponse_Error {
  message: string;

  constructor(v?: EgressOpenStreamResponse_IError) {
    this.message = v?.message || "";
  }

  static encode(m: EgressOpenStreamResponse_Error, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.message.length) w.uint32(10).string(m.message);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): EgressOpenStreamResponse_Error {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new EgressOpenStreamResponse_Error();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.message = r.string();
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


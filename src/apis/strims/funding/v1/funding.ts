import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export type IFundingTestRequest = {
  name?: string;
}

export class FundingTestRequest {
  name: string;

  constructor(v?: IFundingTestRequest) {
    this.name = v?.name || "";
  }

  static encode(m: FundingTestRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.name.length) w.uint32(10).string(m.name);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FundingTestRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new FundingTestRequest();
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

export type IFundingTestResponse = {
  message?: string;
}

export class FundingTestResponse {
  message: string;

  constructor(v?: IFundingTestResponse) {
    this.message = v?.message || "";
  }

  static encode(m: FundingTestResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.message.length) w.uint32(10).string(m.message);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FundingTestResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new FundingTestResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.message = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

/* @internal */
export const strims_funding_v1_FundingTestRequest = FundingTestRequest;
/* @internal */
export type strims_funding_v1_FundingTestRequest = FundingTestRequest;
/* @internal */
export type strims_funding_v1_IFundingTestRequest = IFundingTestRequest;
/* @internal */
export const strims_funding_v1_FundingTestResponse = FundingTestResponse;
/* @internal */
export type strims_funding_v1_FundingTestResponse = FundingTestResponse;
/* @internal */
export type strims_funding_v1_IFundingTestResponse = IFundingTestResponse;

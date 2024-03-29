import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export type IDevToolsTestRequest = {
  name?: string;
}

export class DevToolsTestRequest {
  name: string;

  constructor(v?: IDevToolsTestRequest) {
    this.name = v?.name || "";
  }

  static encode(m: DevToolsTestRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.name.length) w.uint32(10).string(m.name);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DevToolsTestRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DevToolsTestRequest();
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

export type IDevToolsTestResponse = {
  message?: string;
}

export class DevToolsTestResponse {
  message: string;

  constructor(v?: IDevToolsTestResponse) {
    this.message = v?.message || "";
  }

  static encode(m: DevToolsTestResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.message.length) w.uint32(10).string(m.message);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DevToolsTestResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DevToolsTestResponse();
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
export const strims_devtools_v1_DevToolsTestRequest = DevToolsTestRequest;
/* @internal */
export type strims_devtools_v1_DevToolsTestRequest = DevToolsTestRequest;
/* @internal */
export type strims_devtools_v1_IDevToolsTestRequest = IDevToolsTestRequest;
/* @internal */
export const strims_devtools_v1_DevToolsTestResponse = DevToolsTestResponse;
/* @internal */
export type strims_devtools_v1_DevToolsTestResponse = DevToolsTestResponse;
/* @internal */
export type strims_devtools_v1_IDevToolsTestResponse = IDevToolsTestResponse;

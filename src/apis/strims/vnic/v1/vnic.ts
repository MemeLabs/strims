import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  strims_type_Certificate,
  strims_type_ICertificate,
} from "../../type/certificate";

export type ILinkDescription = {
  interface?: string;
  description?: string;
}

export class LinkDescription {
  interface: string;
  description: string;

  constructor(v?: ILinkDescription) {
    this.interface = v?.interface || "";
    this.description = v?.description || "";
  }

  static encode(m: LinkDescription, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.interface.length) w.uint32(10).string(m.interface);
    if (m.description.length) w.uint32(18).string(m.description);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): LinkDescription {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new LinkDescription();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.interface = r.string();
        break;
        case 2:
        m.description = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ITCPMuxInit = {
  protocolVersion?: number;
  peerKey?: Uint8Array;
}

export class TCPMuxInit {
  protocolVersion: number;
  peerKey: Uint8Array;

  constructor(v?: ITCPMuxInit) {
    this.protocolVersion = v?.protocolVersion || 0;
    this.peerKey = v?.peerKey || new Uint8Array();
  }

  static encode(m: TCPMuxInit, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.protocolVersion) w.uint32(8).uint32(m.protocolVersion);
    if (m.peerKey.length) w.uint32(18).bytes(m.peerKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): TCPMuxInit {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new TCPMuxInit();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.protocolVersion = r.uint32();
        break;
        case 2:
        m.peerKey = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IAESLinkInit = {
  protocolVersion?: number;
  key?: Uint8Array;
  iv?: Uint8Array;
}

export class AESLinkInit {
  protocolVersion: number;
  key: Uint8Array;
  iv: Uint8Array;

  constructor(v?: IAESLinkInit) {
    this.protocolVersion = v?.protocolVersion || 0;
    this.key = v?.key || new Uint8Array();
    this.iv = v?.iv || new Uint8Array();
  }

  static encode(m: AESLinkInit, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.protocolVersion) w.uint32(8).uint32(m.protocolVersion);
    if (m.key.length) w.uint32(18).bytes(m.key);
    if (m.iv.length) w.uint32(26).bytes(m.iv);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): AESLinkInit {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new AESLinkInit();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.protocolVersion = r.uint32();
        break;
        case 2:
        m.key = r.bytes();
        break;
        case 3:
        m.iv = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IPeerInit = {
  protocolVersion?: number;
  certificate?: strims_type_ICertificate;
  nodePlatform?: string;
  nodeVersion?: string;
}

export class PeerInit {
  protocolVersion: number;
  certificate: strims_type_Certificate | undefined;
  nodePlatform: string;
  nodeVersion: string;

  constructor(v?: IPeerInit) {
    this.protocolVersion = v?.protocolVersion || 0;
    this.certificate = v?.certificate && new strims_type_Certificate(v.certificate);
    this.nodePlatform = v?.nodePlatform || "";
    this.nodeVersion = v?.nodeVersion || "";
  }

  static encode(m: PeerInit, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.protocolVersion) w.uint32(8).uint32(m.protocolVersion);
    if (m.certificate) strims_type_Certificate.encode(m.certificate, w.uint32(18).fork()).ldelim();
    if (m.nodePlatform.length) w.uint32(26).string(m.nodePlatform);
    if (m.nodeVersion.length) w.uint32(34).string(m.nodeVersion);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PeerInit {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new PeerInit();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.protocolVersion = r.uint32();
        break;
        case 2:
        m.certificate = strims_type_Certificate.decode(r, r.uint32());
        break;
        case 3:
        m.nodePlatform = r.string();
        break;
        case 4:
        m.nodeVersion = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IConfig = {
  maxUploadBytesPerSecond?: bigint;
  maxPeers?: number;
}

export class Config {
  maxUploadBytesPerSecond: bigint;
  maxPeers: number;

  constructor(v?: IConfig) {
    this.maxUploadBytesPerSecond = v?.maxUploadBytesPerSecond || BigInt(0);
    this.maxPeers = v?.maxPeers || 0;
  }

  static encode(m: Config, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.maxUploadBytesPerSecond) w.uint32(8).uint64(m.maxUploadBytesPerSecond);
    if (m.maxPeers) w.uint32(16).uint32(m.maxPeers);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Config {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Config();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.maxUploadBytesPerSecond = r.uint64();
        break;
        case 2:
        m.maxPeers = r.uint32();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IGetConfigRequest = Record<string, any>;

export class GetConfigRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IGetConfigRequest) {
  }

  static encode(m: GetConfigRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetConfigRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new GetConfigRequest();
  }
}

export type IGetConfigResponse = {
  config?: strims_vnic_v1_IConfig;
}

export class GetConfigResponse {
  config: strims_vnic_v1_Config | undefined;

  constructor(v?: IGetConfigResponse) {
    this.config = v?.config && new strims_vnic_v1_Config(v.config);
  }

  static encode(m: GetConfigResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.config) strims_vnic_v1_Config.encode(m.config, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetConfigResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetConfigResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.config = strims_vnic_v1_Config.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ISetConfigRequest = {
  config?: strims_vnic_v1_IConfig;
}

export class SetConfigRequest {
  config: strims_vnic_v1_Config | undefined;

  constructor(v?: ISetConfigRequest) {
    this.config = v?.config && new strims_vnic_v1_Config(v.config);
  }

  static encode(m: SetConfigRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.config) strims_vnic_v1_Config.encode(m.config, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SetConfigRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new SetConfigRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.config = strims_vnic_v1_Config.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ISetConfigResponse = {
  config?: strims_vnic_v1_IConfig;
}

export class SetConfigResponse {
  config: strims_vnic_v1_Config | undefined;

  constructor(v?: ISetConfigResponse) {
    this.config = v?.config && new strims_vnic_v1_Config(v.config);
  }

  static encode(m: SetConfigResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.config) strims_vnic_v1_Config.encode(m.config, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SetConfigResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new SetConfigResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.config = strims_vnic_v1_Config.decode(r, r.uint32());
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
export const strims_vnic_v1_LinkDescription = LinkDescription;
/* @internal */
export type strims_vnic_v1_LinkDescription = LinkDescription;
/* @internal */
export type strims_vnic_v1_ILinkDescription = ILinkDescription;
/* @internal */
export const strims_vnic_v1_TCPMuxInit = TCPMuxInit;
/* @internal */
export type strims_vnic_v1_TCPMuxInit = TCPMuxInit;
/* @internal */
export type strims_vnic_v1_ITCPMuxInit = ITCPMuxInit;
/* @internal */
export const strims_vnic_v1_AESLinkInit = AESLinkInit;
/* @internal */
export type strims_vnic_v1_AESLinkInit = AESLinkInit;
/* @internal */
export type strims_vnic_v1_IAESLinkInit = IAESLinkInit;
/* @internal */
export const strims_vnic_v1_PeerInit = PeerInit;
/* @internal */
export type strims_vnic_v1_PeerInit = PeerInit;
/* @internal */
export type strims_vnic_v1_IPeerInit = IPeerInit;
/* @internal */
export const strims_vnic_v1_Config = Config;
/* @internal */
export type strims_vnic_v1_Config = Config;
/* @internal */
export type strims_vnic_v1_IConfig = IConfig;
/* @internal */
export const strims_vnic_v1_GetConfigRequest = GetConfigRequest;
/* @internal */
export type strims_vnic_v1_GetConfigRequest = GetConfigRequest;
/* @internal */
export type strims_vnic_v1_IGetConfigRequest = IGetConfigRequest;
/* @internal */
export const strims_vnic_v1_GetConfigResponse = GetConfigResponse;
/* @internal */
export type strims_vnic_v1_GetConfigResponse = GetConfigResponse;
/* @internal */
export type strims_vnic_v1_IGetConfigResponse = IGetConfigResponse;
/* @internal */
export const strims_vnic_v1_SetConfigRequest = SetConfigRequest;
/* @internal */
export type strims_vnic_v1_SetConfigRequest = SetConfigRequest;
/* @internal */
export type strims_vnic_v1_ISetConfigRequest = ISetConfigRequest;
/* @internal */
export const strims_vnic_v1_SetConfigResponse = SetConfigResponse;
/* @internal */
export type strims_vnic_v1_SetConfigResponse = SetConfigResponse;
/* @internal */
export type strims_vnic_v1_ISetConfigResponse = ISetConfigResponse;

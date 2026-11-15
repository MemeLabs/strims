// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  Certificate as strims_type_Certificate,
  type ICertificate as strims_type_ICertificate,
} from "../../../type/certificate.pb";
import {
  VersionVector as strims_dao_v1_VersionVector,
  type IVersionVector as strims_dao_v1_IVersionVector,
} from "../../../dao/v1/dao.pb";

export type IConfig = {
  enablePublishing?: boolean;
}

export class Config {
  enablePublishing: boolean;

  constructor(v?: IConfig) {
    this.enablePublishing = v?.enablePublishing || false;
  }

  static encode(m: Config, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.enablePublishing) w.uint32(8).bool(m.enablePublishing);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Config {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Config();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.enablePublishing = r.bool();
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

export type IGetConfigRequest = object;

export class GetConfigRequest {

  constructor(v?: IGetConfigRequest) {
    void v; // an empty message has no fields to set
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
  config?: IConfig;
}

export class GetConfigResponse {
  config: Config | undefined;

  constructor(v?: IGetConfigResponse) {
    this.config = v?.config && new Config(v.config);
  }

  static encode(m: GetConfigResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.config) Config.encode(m.config, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetConfigResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new GetConfigResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.config = Config.decode(r, r.uint32());
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

export type ISetConfigRequest = {
  config?: IConfig;
}

export class SetConfigRequest {
  config: Config | undefined;

  constructor(v?: ISetConfigRequest) {
    this.config = v?.config && new Config(v.config);
  }

  static encode(m: SetConfigRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.config) Config.encode(m.config, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SetConfigRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new SetConfigRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.config = Config.decode(r, r.uint32());
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

export type ISetConfigResponse = {
  config?: IConfig;
}

export class SetConfigResponse {
  config: Config | undefined;

  constructor(v?: ISetConfigResponse) {
    this.config = v?.config && new Config(v.config);
  }

  static encode(m: SetConfigResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.config) Config.encode(m.config, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SetConfigResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new SetConfigResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.config = Config.decode(r, r.uint32());
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

export type IBootstrapClient = {
  id?: bigint;
  version?: strims_dao_v1_IVersionVector;
  clientOptions?: BootstrapClient_IClientOptions
}

export class BootstrapClient {
  id: bigint;
  version: strims_dao_v1_VersionVector | undefined;
  clientOptions: BootstrapClient_TClientOptions;

  constructor(v?: IBootstrapClient) {
    this.id = v?.id || BigInt(0);
    this.version = v?.version && new strims_dao_v1_VersionVector(v.version);
    this.clientOptions = v?.clientOptions ? new BootstrapClient_ClientOptions(v.clientOptions) : new BootstrapClient_ClientOptions();
  }

  static encode(m: BootstrapClient, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.version) strims_dao_v1_VersionVector.encode(m.version, w.uint32(26).fork()).ldelim();
    switch (m.clientOptions.case) {
      case BootstrapClient_ClientOptionsCase.WEBSOCKET_OPTIONS:
      BootstrapClientWebSocketOptions.encode(m.clientOptions.websocketOptions, w.uint32(18).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BootstrapClient {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new BootstrapClient();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 3:
        m.version = strims_dao_v1_VersionVector.decode(r, r.uint32());
        break;
        case 2:
        m.clientOptions = new BootstrapClient_ClientOptions({ websocketOptions: BootstrapClientWebSocketOptions.decode(r, r.uint32()) });
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

export enum BootstrapClient_ClientOptionsCase {
  NOT_SET = 0,
  WEBSOCKET_OPTIONS = 2,
}

export type BootstrapClient_IClientOptions =
{ case?: BootstrapClient_ClientOptionsCase.NOT_SET }
|{ case?: BootstrapClient_ClientOptionsCase.WEBSOCKET_OPTIONS, websocketOptions: IBootstrapClientWebSocketOptions }
;

export type BootstrapClient_TClientOptions = Readonly<
{ case: BootstrapClient_ClientOptionsCase.NOT_SET }
|{ case: BootstrapClient_ClientOptionsCase.WEBSOCKET_OPTIONS, websocketOptions: BootstrapClientWebSocketOptions }
>;

class BootstrapClient_ClientOptionsImpl {
  declare websocketOptions?: BootstrapClientWebSocketOptions;
  case: BootstrapClient_ClientOptionsCase = BootstrapClient_ClientOptionsCase.NOT_SET;

  constructor(v?: BootstrapClient_IClientOptions) {
    if (v && "websocketOptions" in v) {
      this.case = BootstrapClient_ClientOptionsCase.WEBSOCKET_OPTIONS;
      this.websocketOptions = new BootstrapClientWebSocketOptions(v.websocketOptions);
    }
  }
}

export const BootstrapClient_ClientOptions = BootstrapClient_ClientOptionsImpl as {
  new (): Readonly<{ case: BootstrapClient_ClientOptionsCase.NOT_SET }>;
  new <T extends BootstrapClient_IClientOptions>(v: T): Readonly<
  T extends { websocketOptions: IBootstrapClientWebSocketOptions } ? { case: BootstrapClient_ClientOptionsCase.WEBSOCKET_OPTIONS, websocketOptions: BootstrapClientWebSocketOptions } :
  never
  >;
};

export type IBootstrapClientWebSocketOptions = {
  url?: string;
  insecureSkipVerifyTls?: boolean;
}

export class BootstrapClientWebSocketOptions {
  url: string;
  insecureSkipVerifyTls: boolean;

  constructor(v?: IBootstrapClientWebSocketOptions) {
    this.url = v?.url || "";
    this.insecureSkipVerifyTls = v?.insecureSkipVerifyTls || false;
  }

  static encode(m: BootstrapClientWebSocketOptions, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.url.length) w.uint32(10).string(m.url);
    if (m.insecureSkipVerifyTls) w.uint32(16).bool(m.insecureSkipVerifyTls);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BootstrapClientWebSocketOptions {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new BootstrapClientWebSocketOptions();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.url = r.string();
        break;
        case 2:
        m.insecureSkipVerifyTls = r.bool();
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

export type ICreateBootstrapClientRequest = {
  clientOptions?: CreateBootstrapClientRequest_IClientOptions
}

export class CreateBootstrapClientRequest {
  clientOptions: CreateBootstrapClientRequest_TClientOptions;

  constructor(v?: ICreateBootstrapClientRequest) {
    this.clientOptions = v?.clientOptions ? new CreateBootstrapClientRequest_ClientOptions(v.clientOptions) : new CreateBootstrapClientRequest_ClientOptions();
  }

  static encode(m: CreateBootstrapClientRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.clientOptions.case) {
      case CreateBootstrapClientRequest_ClientOptionsCase.WEBSOCKET_OPTIONS:
      BootstrapClientWebSocketOptions.encode(m.clientOptions.websocketOptions, w.uint32(10).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateBootstrapClientRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new CreateBootstrapClientRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.clientOptions = new CreateBootstrapClientRequest_ClientOptions({ websocketOptions: BootstrapClientWebSocketOptions.decode(r, r.uint32()) });
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

export enum CreateBootstrapClientRequest_ClientOptionsCase {
  NOT_SET = 0,
  WEBSOCKET_OPTIONS = 1,
}

export type CreateBootstrapClientRequest_IClientOptions =
{ case?: CreateBootstrapClientRequest_ClientOptionsCase.NOT_SET }
|{ case?: CreateBootstrapClientRequest_ClientOptionsCase.WEBSOCKET_OPTIONS, websocketOptions: IBootstrapClientWebSocketOptions }
;

export type CreateBootstrapClientRequest_TClientOptions = Readonly<
{ case: CreateBootstrapClientRequest_ClientOptionsCase.NOT_SET }
|{ case: CreateBootstrapClientRequest_ClientOptionsCase.WEBSOCKET_OPTIONS, websocketOptions: BootstrapClientWebSocketOptions }
>;

class CreateBootstrapClientRequest_ClientOptionsImpl {
  declare websocketOptions?: BootstrapClientWebSocketOptions;
  case: CreateBootstrapClientRequest_ClientOptionsCase = CreateBootstrapClientRequest_ClientOptionsCase.NOT_SET;

  constructor(v?: CreateBootstrapClientRequest_IClientOptions) {
    if (v && "websocketOptions" in v) {
      this.case = CreateBootstrapClientRequest_ClientOptionsCase.WEBSOCKET_OPTIONS;
      this.websocketOptions = new BootstrapClientWebSocketOptions(v.websocketOptions);
    }
  }
}

export const CreateBootstrapClientRequest_ClientOptions = CreateBootstrapClientRequest_ClientOptionsImpl as {
  new (): Readonly<{ case: CreateBootstrapClientRequest_ClientOptionsCase.NOT_SET }>;
  new <T extends CreateBootstrapClientRequest_IClientOptions>(v: T): Readonly<
  T extends { websocketOptions: IBootstrapClientWebSocketOptions } ? { case: CreateBootstrapClientRequest_ClientOptionsCase.WEBSOCKET_OPTIONS, websocketOptions: BootstrapClientWebSocketOptions } :
  never
  >;
};

export type ICreateBootstrapClientResponse = {
  bootstrapClient?: IBootstrapClient;
}

export class CreateBootstrapClientResponse {
  bootstrapClient: BootstrapClient | undefined;

  constructor(v?: ICreateBootstrapClientResponse) {
    this.bootstrapClient = v?.bootstrapClient && new BootstrapClient(v.bootstrapClient);
  }

  static encode(m: CreateBootstrapClientResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.bootstrapClient) BootstrapClient.encode(m.bootstrapClient, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateBootstrapClientResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new CreateBootstrapClientResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.bootstrapClient = BootstrapClient.decode(r, r.uint32());
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

export type IUpdateBootstrapClientRequest = {
  id?: bigint;
  clientOptions?: UpdateBootstrapClientRequest_IClientOptions
}

export class UpdateBootstrapClientRequest {
  id: bigint;
  clientOptions: UpdateBootstrapClientRequest_TClientOptions;

  constructor(v?: IUpdateBootstrapClientRequest) {
    this.id = v?.id || BigInt(0);
    this.clientOptions = v?.clientOptions ? new UpdateBootstrapClientRequest_ClientOptions(v.clientOptions) : new UpdateBootstrapClientRequest_ClientOptions();
  }

  static encode(m: UpdateBootstrapClientRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    switch (m.clientOptions.case) {
      case UpdateBootstrapClientRequest_ClientOptionsCase.WEBSOCKET_OPTIONS:
      BootstrapClientWebSocketOptions.encode(m.clientOptions.websocketOptions, w.uint32(18).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateBootstrapClientRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UpdateBootstrapClientRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.clientOptions = new UpdateBootstrapClientRequest_ClientOptions({ websocketOptions: BootstrapClientWebSocketOptions.decode(r, r.uint32()) });
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

export enum UpdateBootstrapClientRequest_ClientOptionsCase {
  NOT_SET = 0,
  WEBSOCKET_OPTIONS = 2,
}

export type UpdateBootstrapClientRequest_IClientOptions =
{ case?: UpdateBootstrapClientRequest_ClientOptionsCase.NOT_SET }
|{ case?: UpdateBootstrapClientRequest_ClientOptionsCase.WEBSOCKET_OPTIONS, websocketOptions: IBootstrapClientWebSocketOptions }
;

export type UpdateBootstrapClientRequest_TClientOptions = Readonly<
{ case: UpdateBootstrapClientRequest_ClientOptionsCase.NOT_SET }
|{ case: UpdateBootstrapClientRequest_ClientOptionsCase.WEBSOCKET_OPTIONS, websocketOptions: BootstrapClientWebSocketOptions }
>;

class UpdateBootstrapClientRequest_ClientOptionsImpl {
  declare websocketOptions?: BootstrapClientWebSocketOptions;
  case: UpdateBootstrapClientRequest_ClientOptionsCase = UpdateBootstrapClientRequest_ClientOptionsCase.NOT_SET;

  constructor(v?: UpdateBootstrapClientRequest_IClientOptions) {
    if (v && "websocketOptions" in v) {
      this.case = UpdateBootstrapClientRequest_ClientOptionsCase.WEBSOCKET_OPTIONS;
      this.websocketOptions = new BootstrapClientWebSocketOptions(v.websocketOptions);
    }
  }
}

export const UpdateBootstrapClientRequest_ClientOptions = UpdateBootstrapClientRequest_ClientOptionsImpl as {
  new (): Readonly<{ case: UpdateBootstrapClientRequest_ClientOptionsCase.NOT_SET }>;
  new <T extends UpdateBootstrapClientRequest_IClientOptions>(v: T): Readonly<
  T extends { websocketOptions: IBootstrapClientWebSocketOptions } ? { case: UpdateBootstrapClientRequest_ClientOptionsCase.WEBSOCKET_OPTIONS, websocketOptions: BootstrapClientWebSocketOptions } :
  never
  >;
};

export type IUpdateBootstrapClientResponse = {
  bootstrapClient?: IBootstrapClient;
}

export class UpdateBootstrapClientResponse {
  bootstrapClient: BootstrapClient | undefined;

  constructor(v?: IUpdateBootstrapClientResponse) {
    this.bootstrapClient = v?.bootstrapClient && new BootstrapClient(v.bootstrapClient);
  }

  static encode(m: UpdateBootstrapClientResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.bootstrapClient) BootstrapClient.encode(m.bootstrapClient, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateBootstrapClientResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UpdateBootstrapClientResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.bootstrapClient = BootstrapClient.decode(r, r.uint32());
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

export type IDeleteBootstrapClientRequest = {
  id?: bigint;
}

export class DeleteBootstrapClientRequest {
  id: bigint;

  constructor(v?: IDeleteBootstrapClientRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: DeleteBootstrapClientRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeleteBootstrapClientRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new DeleteBootstrapClientRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
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

export type IDeleteBootstrapClientResponse = object;

export class DeleteBootstrapClientResponse {

  constructor(v?: IDeleteBootstrapClientResponse) {
    void v; // an empty message has no fields to set
  }

  static encode(m: DeleteBootstrapClientResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeleteBootstrapClientResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new DeleteBootstrapClientResponse();
  }
}

export type IGetBootstrapClientRequest = {
  id?: bigint;
}

export class GetBootstrapClientRequest {
  id: bigint;

  constructor(v?: IGetBootstrapClientRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: GetBootstrapClientRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetBootstrapClientRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new GetBootstrapClientRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
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

export type IGetBootstrapClientResponse = {
  bootstrapClient?: IBootstrapClient;
}

export class GetBootstrapClientResponse {
  bootstrapClient: BootstrapClient | undefined;

  constructor(v?: IGetBootstrapClientResponse) {
    this.bootstrapClient = v?.bootstrapClient && new BootstrapClient(v.bootstrapClient);
  }

  static encode(m: GetBootstrapClientResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.bootstrapClient) BootstrapClient.encode(m.bootstrapClient, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetBootstrapClientResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new GetBootstrapClientResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.bootstrapClient = BootstrapClient.decode(r, r.uint32());
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

export type IListBootstrapClientsRequest = object;

export class ListBootstrapClientsRequest {

  constructor(v?: IListBootstrapClientsRequest) {
    void v; // an empty message has no fields to set
  }

  static encode(m: ListBootstrapClientsRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListBootstrapClientsRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new ListBootstrapClientsRequest();
  }
}

export type IListBootstrapClientsResponse = {
  bootstrapClients?: IBootstrapClient[];
}

export class ListBootstrapClientsResponse {
  bootstrapClients: BootstrapClient[];

  constructor(v?: IListBootstrapClientsResponse) {
    this.bootstrapClients = v?.bootstrapClients ? v.bootstrapClients.map(v => new BootstrapClient(v)) : [];
  }

  static encode(m: ListBootstrapClientsResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.bootstrapClients) BootstrapClient.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListBootstrapClientsResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ListBootstrapClientsResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.bootstrapClients.push(BootstrapClient.decode(r, r.uint32()));
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

export type IListBootstrapPeersRequest = object;

export class ListBootstrapPeersRequest {

  constructor(v?: IListBootstrapPeersRequest) {
    void v; // an empty message has no fields to set
  }

  static encode(m: ListBootstrapPeersRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListBootstrapPeersRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new ListBootstrapPeersRequest();
  }
}

export type IListBootstrapPeersResponse = {
  peers?: IBootstrapPeer[];
}

export class ListBootstrapPeersResponse {
  peers: BootstrapPeer[];

  constructor(v?: IListBootstrapPeersResponse) {
    this.peers = v?.peers ? v.peers.map(v => new BootstrapPeer(v)) : [];
  }

  static encode(m: ListBootstrapPeersResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.peers) BootstrapPeer.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListBootstrapPeersResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ListBootstrapPeersResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.peers.push(BootstrapPeer.decode(r, r.uint32()));
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

export type IBootstrapPeer = {
  peerId?: bigint;
  label?: string;
}

export class BootstrapPeer {
  peerId: bigint;
  label: string;

  constructor(v?: IBootstrapPeer) {
    this.peerId = v?.peerId || BigInt(0);
    this.label = v?.label || "";
  }

  static encode(m: BootstrapPeer, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.peerId) w.uint32(8).uint64(m.peerId);
    if (m.label.length) w.uint32(18).string(m.label);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BootstrapPeer {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new BootstrapPeer();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.peerId = r.uint64();
        break;
        case 2:
        m.label = r.string();
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

export type IBootstrapServiceMessage = {
  body?: BootstrapServiceMessage_IBody
}

export class BootstrapServiceMessage {
  body: BootstrapServiceMessage_TBody;

  constructor(v?: IBootstrapServiceMessage) {
    this.body = v?.body ? new BootstrapServiceMessage_Body(v.body) : new BootstrapServiceMessage_Body();
  }

  static encode(m: BootstrapServiceMessage, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case BootstrapServiceMessage_BodyCase.BROKER_OFFER:
      BootstrapServiceMessage_BrokerOffer.encode(m.body.brokerOffer, w.uint32(10).fork()).ldelim();
      break;
      case BootstrapServiceMessage_BodyCase.PUBLISH_REQUEST:
      BootstrapServiceMessage_PublishRequest.encode(m.body.publishRequest, w.uint32(18).fork()).ldelim();
      break;
      case BootstrapServiceMessage_BodyCase.PUBLISH_RESPONSE:
      BootstrapServiceMessage_PublishResponse.encode(m.body.publishResponse, w.uint32(26).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BootstrapServiceMessage {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new BootstrapServiceMessage();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.body = new BootstrapServiceMessage_Body({ brokerOffer: BootstrapServiceMessage_BrokerOffer.decode(r, r.uint32()) });
        break;
        case 2:
        m.body = new BootstrapServiceMessage_Body({ publishRequest: BootstrapServiceMessage_PublishRequest.decode(r, r.uint32()) });
        break;
        case 3:
        m.body = new BootstrapServiceMessage_Body({ publishResponse: BootstrapServiceMessage_PublishResponse.decode(r, r.uint32()) });
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

export enum BootstrapServiceMessage_BodyCase {
  NOT_SET = 0,
  BROKER_OFFER = 1,
  PUBLISH_REQUEST = 2,
  PUBLISH_RESPONSE = 3,
}

export type BootstrapServiceMessage_IBody =
{ case?: BootstrapServiceMessage_BodyCase.NOT_SET }
|{ case?: BootstrapServiceMessage_BodyCase.BROKER_OFFER, brokerOffer: BootstrapServiceMessage_IBrokerOffer }
|{ case?: BootstrapServiceMessage_BodyCase.PUBLISH_REQUEST, publishRequest: BootstrapServiceMessage_IPublishRequest }
|{ case?: BootstrapServiceMessage_BodyCase.PUBLISH_RESPONSE, publishResponse: BootstrapServiceMessage_IPublishResponse }
;

export type BootstrapServiceMessage_TBody = Readonly<
{ case: BootstrapServiceMessage_BodyCase.NOT_SET }
|{ case: BootstrapServiceMessage_BodyCase.BROKER_OFFER, brokerOffer: BootstrapServiceMessage_BrokerOffer }
|{ case: BootstrapServiceMessage_BodyCase.PUBLISH_REQUEST, publishRequest: BootstrapServiceMessage_PublishRequest }
|{ case: BootstrapServiceMessage_BodyCase.PUBLISH_RESPONSE, publishResponse: BootstrapServiceMessage_PublishResponse }
>;

class BootstrapServiceMessage_BodyImpl {
  declare brokerOffer?: BootstrapServiceMessage_BrokerOffer;
  declare publishRequest?: BootstrapServiceMessage_PublishRequest;
  declare publishResponse?: BootstrapServiceMessage_PublishResponse;
  case: BootstrapServiceMessage_BodyCase = BootstrapServiceMessage_BodyCase.NOT_SET;

  constructor(v?: BootstrapServiceMessage_IBody) {
    if (v && "brokerOffer" in v) {
      this.case = BootstrapServiceMessage_BodyCase.BROKER_OFFER;
      this.brokerOffer = new BootstrapServiceMessage_BrokerOffer(v.brokerOffer);
    } else
    if (v && "publishRequest" in v) {
      this.case = BootstrapServiceMessage_BodyCase.PUBLISH_REQUEST;
      this.publishRequest = new BootstrapServiceMessage_PublishRequest(v.publishRequest);
    } else
    if (v && "publishResponse" in v) {
      this.case = BootstrapServiceMessage_BodyCase.PUBLISH_RESPONSE;
      this.publishResponse = new BootstrapServiceMessage_PublishResponse(v.publishResponse);
    }
  }
}

export const BootstrapServiceMessage_Body = BootstrapServiceMessage_BodyImpl as {
  new (): Readonly<{ case: BootstrapServiceMessage_BodyCase.NOT_SET }>;
  new <T extends BootstrapServiceMessage_IBody>(v: T): Readonly<
  T extends { brokerOffer: BootstrapServiceMessage_IBrokerOffer } ? { case: BootstrapServiceMessage_BodyCase.BROKER_OFFER, brokerOffer: BootstrapServiceMessage_BrokerOffer } :
  T extends { publishRequest: BootstrapServiceMessage_IPublishRequest } ? { case: BootstrapServiceMessage_BodyCase.PUBLISH_REQUEST, publishRequest: BootstrapServiceMessage_PublishRequest } :
  T extends { publishResponse: BootstrapServiceMessage_IPublishResponse } ? { case: BootstrapServiceMessage_BodyCase.PUBLISH_RESPONSE, publishResponse: BootstrapServiceMessage_PublishResponse } :
  never
  >;
};

export type BootstrapServiceMessage_IBrokerOffer = object;

export class BootstrapServiceMessage_BrokerOffer {

  constructor(v?: BootstrapServiceMessage_IBrokerOffer) {
    void v; // an empty message has no fields to set
  }

  static encode(m: BootstrapServiceMessage_BrokerOffer, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BootstrapServiceMessage_BrokerOffer {
    if (r instanceof Reader && length) r.skip(length);
    return new BootstrapServiceMessage_BrokerOffer();
  }
}

export type BootstrapServiceMessage_IPublishRequest = {
  name?: string;
  certificate?: strims_type_ICertificate;
}

export class BootstrapServiceMessage_PublishRequest {
  name: string;
  certificate: strims_type_Certificate | undefined;

  constructor(v?: BootstrapServiceMessage_IPublishRequest) {
    this.name = v?.name || "";
    this.certificate = v?.certificate && new strims_type_Certificate(v.certificate);
  }

  static encode(m: BootstrapServiceMessage_PublishRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.name.length) w.uint32(10).string(m.name);
    if (m.certificate) strims_type_Certificate.encode(m.certificate, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BootstrapServiceMessage_PublishRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new BootstrapServiceMessage_PublishRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.name = r.string();
        break;
        case 2:
        m.certificate = strims_type_Certificate.decode(r, r.uint32());
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

export type BootstrapServiceMessage_IPublishResponse = {
  body?: BootstrapServiceMessage_PublishResponse_IBody
}

export class BootstrapServiceMessage_PublishResponse {
  body: BootstrapServiceMessage_PublishResponse_TBody;

  constructor(v?: BootstrapServiceMessage_IPublishResponse) {
    this.body = v?.body ? new BootstrapServiceMessage_PublishResponse_Body(v.body) : new BootstrapServiceMessage_PublishResponse_Body();
  }

  static encode(m: BootstrapServiceMessage_PublishResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case BootstrapServiceMessage_PublishResponse_BodyCase.ERROR:
      w.uint32(10).string(m.body.error);
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): BootstrapServiceMessage_PublishResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new BootstrapServiceMessage_PublishResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.body = new BootstrapServiceMessage_PublishResponse_Body({ error: r.string() });
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

export enum BootstrapServiceMessage_PublishResponse_BodyCase {
  NOT_SET = 0,
  ERROR = 1,
}

export type BootstrapServiceMessage_PublishResponse_IBody =
{ case?: BootstrapServiceMessage_PublishResponse_BodyCase.NOT_SET }
|{ case?: BootstrapServiceMessage_PublishResponse_BodyCase.ERROR, error: string }
;

export type BootstrapServiceMessage_PublishResponse_TBody = Readonly<
{ case: BootstrapServiceMessage_PublishResponse_BodyCase.NOT_SET }
|{ case: BootstrapServiceMessage_PublishResponse_BodyCase.ERROR, error: string }
>;

class BootstrapServiceMessage_PublishResponse_BodyImpl {
  declare error?: string;
  case: BootstrapServiceMessage_PublishResponse_BodyCase = BootstrapServiceMessage_PublishResponse_BodyCase.NOT_SET;

  constructor(v?: BootstrapServiceMessage_PublishResponse_IBody) {
    if (v && "error" in v) {
      this.case = BootstrapServiceMessage_PublishResponse_BodyCase.ERROR;
      this.error = v.error;
    }
  }
}

export const BootstrapServiceMessage_PublishResponse_Body = BootstrapServiceMessage_PublishResponse_BodyImpl as {
  new (): Readonly<{ case: BootstrapServiceMessage_PublishResponse_BodyCase.NOT_SET }>;
  new <T extends BootstrapServiceMessage_PublishResponse_IBody>(v: T): Readonly<
  T extends { error: string } ? { case: BootstrapServiceMessage_PublishResponse_BodyCase.ERROR, error: string } :
  never
  >;
};

export type IPublishNetworkToBootstrapPeerRequest = {
  peerId?: bigint;
  networkId?: bigint;
}

export class PublishNetworkToBootstrapPeerRequest {
  peerId: bigint;
  networkId: bigint;

  constructor(v?: IPublishNetworkToBootstrapPeerRequest) {
    this.peerId = v?.peerId || BigInt(0);
    this.networkId = v?.networkId || BigInt(0);
  }

  static encode(m: PublishNetworkToBootstrapPeerRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.peerId) w.uint32(8).uint64(m.peerId);
    if (m.networkId) w.uint32(16).uint64(m.networkId);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PublishNetworkToBootstrapPeerRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new PublishNetworkToBootstrapPeerRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.peerId = r.uint64();
        break;
        case 2:
        m.networkId = r.uint64();
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

export type IPublishNetworkToBootstrapPeerResponse = object;

export class PublishNetworkToBootstrapPeerResponse {

  constructor(v?: IPublishNetworkToBootstrapPeerResponse) {
    void v; // an empty message has no fields to set
  }

  static encode(m: PublishNetworkToBootstrapPeerResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PublishNetworkToBootstrapPeerResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new PublishNetworkToBootstrapPeerResponse();
  }
}


// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  Key as strims_type_Key,
  type IKey as strims_type_IKey,
} from "../../type/key.pb";
import {
  Image as strims_type_Image,
  type IImage as strims_type_IImage,
} from "../../type/image.pb";
import {
  Certificate as strims_type_Certificate,
  type ICertificate as strims_type_ICertificate,
} from "../../type/certificate.pb";
import {
  BootstrapClient as strims_network_v1_bootstrap_BootstrapClient,
  type IBootstrapClient as strims_network_v1_bootstrap_IBootstrapClient,
} from "./bootstrap/bootstrap.pb";
import {
  ServerConfig as strims_network_v1_directory_ServerConfig,
  type IServerConfig as strims_network_v1_directory_IServerConfig,
} from "./directory/directory.pb";
import type {
  ErrorCode as strims_network_v1_errors_ErrorCode,
} from "./errors/errors.pb";
import {
  VersionVector as strims_dao_v1_VersionVector,
  type IVersionVector as strims_dao_v1_IVersionVector,
} from "../../dao/v1/dao.pb";

export type ICreateServerRequest = {
  name?: string;
  icon?: strims_type_IImage;
  alias?: string;
}

export class CreateServerRequest {
  name: string;
  icon: strims_type_Image | undefined;
  alias: string;

  constructor(v?: ICreateServerRequest) {
    this.name = v?.name || "";
    this.icon = v?.icon && new strims_type_Image(v.icon);
    this.alias = v?.alias || "";
  }

  static encode(m: CreateServerRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.name.length) w.uint32(10).string(m.name);
    if (m.icon) strims_type_Image.encode(m.icon, w.uint32(18).fork()).ldelim();
    if (m.alias.length) w.uint32(26).string(m.alias);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateServerRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new CreateServerRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.name = r.string();
        break;
        case 2:
        m.icon = strims_type_Image.decode(r, r.uint32());
        break;
        case 3:
        m.alias = r.string();
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

export type ICreateServerResponse = {
  network?: INetwork;
}

export class CreateServerResponse {
  network: Network | undefined;

  constructor(v?: ICreateServerResponse) {
    this.network = v?.network && new Network(v.network);
  }

  static encode(m: CreateServerResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.network) Network.encode(m.network, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateServerResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new CreateServerResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.network = Network.decode(r, r.uint32());
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

export type IUpdateServerConfigRequest = {
  networkId?: bigint;
  serverConfig?: IServerConfig;
}

export class UpdateServerConfigRequest {
  networkId: bigint;
  serverConfig: ServerConfig | undefined;

  constructor(v?: IUpdateServerConfigRequest) {
    this.networkId = v?.networkId || BigInt(0);
    this.serverConfig = v?.serverConfig && new ServerConfig(v.serverConfig);
  }

  static encode(m: UpdateServerConfigRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkId) w.uint32(8).uint64(m.networkId);
    if (m.serverConfig) ServerConfig.encode(m.serverConfig, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateServerConfigRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UpdateServerConfigRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkId = r.uint64();
        break;
        case 2:
        m.serverConfig = ServerConfig.decode(r, r.uint32());
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

export type IUpdateServerConfigResponse = {
  network?: INetwork;
}

export class UpdateServerConfigResponse {
  network: Network | undefined;

  constructor(v?: IUpdateServerConfigResponse) {
    this.network = v?.network && new Network(v.network);
  }

  static encode(m: UpdateServerConfigResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.network) Network.encode(m.network, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateServerConfigResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UpdateServerConfigResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.network = Network.decode(r, r.uint32());
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

export type IDeleteNetworkRequest = {
  id?: bigint;
}

export class DeleteNetworkRequest {
  id: bigint;

  constructor(v?: IDeleteNetworkRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: DeleteNetworkRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeleteNetworkRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new DeleteNetworkRequest();
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

export type IDeleteNetworkResponse = object;

export class DeleteNetworkResponse {

  constructor(v?: IDeleteNetworkResponse) {
    void v; // an empty message has no fields to set
  }

  static encode(m: DeleteNetworkResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeleteNetworkResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new DeleteNetworkResponse();
  }
}

export type IGetNetworkRequest = {
  id?: bigint;
}

export class GetNetworkRequest {
  id: bigint;

  constructor(v?: IGetNetworkRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: GetNetworkRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetNetworkRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new GetNetworkRequest();
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

export type IGetNetworkResponse = {
  network?: INetwork;
}

export class GetNetworkResponse {
  network: Network | undefined;

  constructor(v?: IGetNetworkResponse) {
    this.network = v?.network && new Network(v.network);
  }

  static encode(m: GetNetworkResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.network) Network.encode(m.network, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetNetworkResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new GetNetworkResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.network = Network.decode(r, r.uint32());
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

export type IListNetworksRequest = object;

export class ListNetworksRequest {

  constructor(v?: IListNetworksRequest) {
    void v; // an empty message has no fields to set
  }

  static encode(m: ListNetworksRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListNetworksRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new ListNetworksRequest();
  }
}

export type IListNetworksResponse = {
  networks?: INetwork[];
}

export class ListNetworksResponse {
  networks: Network[];

  constructor(v?: IListNetworksResponse) {
    this.networks = v?.networks ? v.networks.map(v => new Network(v)) : [];
  }

  static encode(m: ListNetworksResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.networks) Network.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListNetworksResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ListNetworksResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networks.push(Network.decode(r, r.uint32()));
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

export type IServerConfig = {
  name?: string;
  key?: strims_type_IKey;
  rootCertTtlSecs?: bigint;
  peerCertTtlSecs?: bigint;
  directory?: strims_network_v1_directory_IServerConfig;
  icon?: strims_type_IImage;
}

export class ServerConfig {
  name: string;
  key: strims_type_Key | undefined;
  rootCertTtlSecs: bigint;
  peerCertTtlSecs: bigint;
  directory: strims_network_v1_directory_ServerConfig | undefined;
  icon: strims_type_Image | undefined;

  constructor(v?: IServerConfig) {
    this.name = v?.name || "";
    this.key = v?.key && new strims_type_Key(v.key);
    this.rootCertTtlSecs = v?.rootCertTtlSecs || BigInt(0);
    this.peerCertTtlSecs = v?.peerCertTtlSecs || BigInt(0);
    this.directory = v?.directory && new strims_network_v1_directory_ServerConfig(v.directory);
    this.icon = v?.icon && new strims_type_Image(v.icon);
  }

  static encode(m: ServerConfig, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.name.length) w.uint32(18).string(m.name);
    if (m.key) strims_type_Key.encode(m.key, w.uint32(26).fork()).ldelim();
    if (m.rootCertTtlSecs) w.uint32(32).uint64(m.rootCertTtlSecs);
    if (m.peerCertTtlSecs) w.uint32(40).uint64(m.peerCertTtlSecs);
    if (m.directory) strims_network_v1_directory_ServerConfig.encode(m.directory, w.uint32(50).fork()).ldelim();
    if (m.icon) strims_type_Image.encode(m.icon, w.uint32(58).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ServerConfig {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ServerConfig();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 2:
        m.name = r.string();
        break;
        case 3:
        m.key = strims_type_Key.decode(r, r.uint32());
        break;
        case 4:
        m.rootCertTtlSecs = r.uint64();
        break;
        case 5:
        m.peerCertTtlSecs = r.uint64();
        break;
        case 6:
        m.directory = strims_network_v1_directory_ServerConfig.decode(r, r.uint32());
        break;
        case 7:
        m.icon = strims_type_Image.decode(r, r.uint32());
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

export type INetwork = {
  id?: bigint;
  version?: strims_dao_v1_IVersionVector;
  certificate?: strims_type_ICertificate;
  alias?: string;
  serverConfig?: IServerConfig;
  certificateRenewalError?: strims_network_v1_errors_ErrorCode;
}

export class Network {
  id: bigint;
  version: strims_dao_v1_VersionVector | undefined;
  certificate: strims_type_Certificate | undefined;
  alias: string;
  serverConfig: ServerConfig | undefined;
  certificateRenewalError: strims_network_v1_errors_ErrorCode;

  constructor(v?: INetwork) {
    this.id = v?.id || BigInt(0);
    this.version = v?.version && new strims_dao_v1_VersionVector(v.version);
    this.certificate = v?.certificate && new strims_type_Certificate(v.certificate);
    this.alias = v?.alias || "";
    this.serverConfig = v?.serverConfig && new ServerConfig(v.serverConfig);
    this.certificateRenewalError = v?.certificateRenewalError || 0;
  }

  static encode(m: Network, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.version) strims_dao_v1_VersionVector.encode(m.version, w.uint32(58).fork()).ldelim();
    if (m.certificate) strims_type_Certificate.encode(m.certificate, w.uint32(18).fork()).ldelim();
    if (m.alias.length) w.uint32(34).string(m.alias);
    if (m.serverConfig) ServerConfig.encode(m.serverConfig, w.uint32(42).fork()).ldelim();
    if (m.certificateRenewalError) w.uint32(48).uint32(m.certificateRenewalError);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Network {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Network();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 7:
        m.version = strims_dao_v1_VersionVector.decode(r, r.uint32());
        break;
        case 2:
        m.certificate = strims_type_Certificate.decode(r, r.uint32());
        break;
        case 4:
        m.alias = r.string();
        break;
        case 5:
        m.serverConfig = ServerConfig.decode(r, r.uint32());
        break;
        case 6:
        m.certificateRenewalError = r.uint32();
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

export type IPeer = {
  id?: bigint;
  networkId?: bigint;
  publicKey?: Uint8Array;
  inviterPeerId?: bigint;
  inviteQuota?: number;
  createdAt?: bigint;
  isAdmin?: boolean;
  isBanned?: boolean;
  alias?: string;
  aliasChangedAt?: bigint;
}

export class Peer {
  id: bigint;
  networkId: bigint;
  publicKey: Uint8Array;
  inviterPeerId: bigint;
  inviteQuota: number;
  createdAt: bigint;
  isAdmin: boolean;
  isBanned: boolean;
  alias: string;
  aliasChangedAt: bigint;

  constructor(v?: IPeer) {
    this.id = v?.id || BigInt(0);
    this.networkId = v?.networkId || BigInt(0);
    this.publicKey = v?.publicKey || new Uint8Array();
    this.inviterPeerId = v?.inviterPeerId || BigInt(0);
    this.inviteQuota = v?.inviteQuota || 0;
    this.createdAt = v?.createdAt || BigInt(0);
    this.isAdmin = v?.isAdmin || false;
    this.isBanned = v?.isBanned || false;
    this.alias = v?.alias || "";
    this.aliasChangedAt = v?.aliasChangedAt || BigInt(0);
  }

  static encode(m: Peer, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.networkId) w.uint32(16).uint64(m.networkId);
    if (m.publicKey.length) w.uint32(26).bytes(m.publicKey);
    if (m.inviterPeerId) w.uint32(32).uint64(m.inviterPeerId);
    if (m.inviteQuota) w.uint32(40).uint32(m.inviteQuota);
    if (m.createdAt) w.uint32(48).int64(m.createdAt);
    if (m.isAdmin) w.uint32(56).bool(m.isAdmin);
    if (m.isBanned) w.uint32(64).bool(m.isBanned);
    if (m.alias.length) w.uint32(74).string(m.alias);
    if (m.aliasChangedAt) w.uint32(80).int64(m.aliasChangedAt);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Peer {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Peer();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.networkId = r.uint64();
        break;
        case 3:
        m.publicKey = r.bytes();
        break;
        case 4:
        m.inviterPeerId = r.uint64();
        break;
        case 5:
        m.inviteQuota = r.uint32();
        break;
        case 6:
        m.createdAt = r.int64();
        break;
        case 7:
        m.isAdmin = r.bool();
        break;
        case 8:
        m.isBanned = r.bool();
        break;
        case 9:
        m.alias = r.string();
        break;
        case 10:
        m.aliasChangedAt = r.int64();
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

export type IAliasReservation = {
  id?: bigint;
  networkId?: bigint;
  alias?: string;
  peerKey?: Uint8Array;
  reservedUntil?: bigint;
}

export class AliasReservation {
  id: bigint;
  networkId: bigint;
  alias: string;
  peerKey: Uint8Array;
  reservedUntil: bigint;

  constructor(v?: IAliasReservation) {
    this.id = v?.id || BigInt(0);
    this.networkId = v?.networkId || BigInt(0);
    this.alias = v?.alias || "";
    this.peerKey = v?.peerKey || new Uint8Array();
    this.reservedUntil = v?.reservedUntil || BigInt(0);
  }

  static encode(m: AliasReservation, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.networkId) w.uint32(16).uint64(m.networkId);
    if (m.alias.length) w.uint32(26).string(m.alias);
    if (m.peerKey.length) w.uint32(34).bytes(m.peerKey);
    if (m.reservedUntil) w.uint32(40).int64(m.reservedUntil);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): AliasReservation {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new AliasReservation();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.networkId = r.uint64();
        break;
        case 3:
        m.alias = r.string();
        break;
        case 4:
        m.peerKey = r.bytes();
        break;
        case 5:
        m.reservedUntil = r.int64();
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

export type ICreateInvitationRequest = {
  networkId?: bigint;
  bootstrapClientId?: bigint;
}

export class CreateInvitationRequest {
  networkId: bigint;
  bootstrapClientId: bigint;

  constructor(v?: ICreateInvitationRequest) {
    this.networkId = v?.networkId || BigInt(0);
    this.bootstrapClientId = v?.bootstrapClientId || BigInt(0);
  }

  static encode(m: CreateInvitationRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkId) w.uint32(8).uint64(m.networkId);
    if (m.bootstrapClientId) w.uint32(16).uint64(m.bootstrapClientId);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateInvitationRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new CreateInvitationRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkId = r.uint64();
        break;
        case 2:
        m.bootstrapClientId = r.uint64();
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

export type ICreateInvitationResponse = {
  invitation?: IInvitation;
}

export class CreateInvitationResponse {
  invitation: Invitation | undefined;

  constructor(v?: ICreateInvitationResponse) {
    this.invitation = v?.invitation && new Invitation(v.invitation);
  }

  static encode(m: CreateInvitationResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.invitation) Invitation.encode(m.invitation, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateInvitationResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new CreateInvitationResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.invitation = Invitation.decode(r, r.uint32());
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

export type IInvitation = {
  version?: number;
  data?: Uint8Array;
}

export class Invitation {
  version: number;
  data: Uint8Array;

  constructor(v?: IInvitation) {
    this.version = v?.version || 0;
    this.data = v?.data || new Uint8Array();
  }

  static encode(m: Invitation, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.version) w.uint32(8).uint32(m.version);
    if (m.data.length) w.uint32(18).bytes(m.data);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Invitation {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Invitation();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.version = r.uint32();
        break;
        case 2:
        m.data = r.bytes();
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

export type IInvitationV0 = {
  key?: strims_type_IKey;
  certificate?: strims_type_ICertificate;
  networkName?: string;
  bootstrapClients?: strims_network_v1_bootstrap_IBootstrapClient[];
}

export class InvitationV0 {
  key: strims_type_Key | undefined;
  certificate: strims_type_Certificate | undefined;
  networkName: string;
  bootstrapClients: strims_network_v1_bootstrap_BootstrapClient[];

  constructor(v?: IInvitationV0) {
    this.key = v?.key && new strims_type_Key(v.key);
    this.certificate = v?.certificate && new strims_type_Certificate(v.certificate);
    this.networkName = v?.networkName || "";
    this.bootstrapClients = v?.bootstrapClients ? v.bootstrapClients.map(v => new strims_network_v1_bootstrap_BootstrapClient(v)) : [];
  }

  static encode(m: InvitationV0, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.key) strims_type_Key.encode(m.key, w.uint32(10).fork()).ldelim();
    if (m.certificate) strims_type_Certificate.encode(m.certificate, w.uint32(18).fork()).ldelim();
    if (m.networkName.length) w.uint32(34).string(m.networkName);
    for (const v of m.bootstrapClients) strims_network_v1_bootstrap_BootstrapClient.encode(v, w.uint32(42).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): InvitationV0 {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new InvitationV0();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.key = strims_type_Key.decode(r, r.uint32());
        break;
        case 2:
        m.certificate = strims_type_Certificate.decode(r, r.uint32());
        break;
        case 4:
        m.networkName = r.string();
        break;
        case 5:
        m.bootstrapClients.push(strims_network_v1_bootstrap_BootstrapClient.decode(r, r.uint32()));
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

export type ICreateNetworkFromInvitationRequest = {
  alias?: string;
  invitation?: CreateNetworkFromInvitationRequest_IInvitation
}

export class CreateNetworkFromInvitationRequest {
  alias: string;
  invitation: CreateNetworkFromInvitationRequest_TInvitation;

  constructor(v?: ICreateNetworkFromInvitationRequest) {
    this.alias = v?.alias || "";
    this.invitation = v?.invitation ? new CreateNetworkFromInvitationRequest_Invitation(v.invitation) : new CreateNetworkFromInvitationRequest_Invitation();
  }

  static encode(m: CreateNetworkFromInvitationRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.alias.length) w.uint32(10).string(m.alias);
    switch (m.invitation.case) {
      case CreateNetworkFromInvitationRequest_InvitationCase.INVITATION_B64:
      w.uint32(8010).string(m.invitation.invitationB64);
      break;
      case CreateNetworkFromInvitationRequest_InvitationCase.INVITATION_BYTES:
      w.uint32(8018).bytes(m.invitation.invitationBytes);
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateNetworkFromInvitationRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new CreateNetworkFromInvitationRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.alias = r.string();
        break;
        case 1001:
        m.invitation = new CreateNetworkFromInvitationRequest_Invitation({ invitationB64: r.string() });
        break;
        case 1002:
        m.invitation = new CreateNetworkFromInvitationRequest_Invitation({ invitationBytes: r.bytes() });
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

export enum CreateNetworkFromInvitationRequest_InvitationCase {
  NOT_SET = 0,
  INVITATION_B64 = 1001,
  INVITATION_BYTES = 1002,
}

export type CreateNetworkFromInvitationRequest_IInvitation =
{ case?: CreateNetworkFromInvitationRequest_InvitationCase.NOT_SET }
|{ case?: CreateNetworkFromInvitationRequest_InvitationCase.INVITATION_B64, invitationB64: string }
|{ case?: CreateNetworkFromInvitationRequest_InvitationCase.INVITATION_BYTES, invitationBytes: Uint8Array }
;

export type CreateNetworkFromInvitationRequest_TInvitation = Readonly<
{ case: CreateNetworkFromInvitationRequest_InvitationCase.NOT_SET }
|{ case: CreateNetworkFromInvitationRequest_InvitationCase.INVITATION_B64, invitationB64: string }
|{ case: CreateNetworkFromInvitationRequest_InvitationCase.INVITATION_BYTES, invitationBytes: Uint8Array }
>;

class CreateNetworkFromInvitationRequest_InvitationImpl {
  declare invitationB64?: string;
  declare invitationBytes?: Uint8Array;
  case: CreateNetworkFromInvitationRequest_InvitationCase = CreateNetworkFromInvitationRequest_InvitationCase.NOT_SET;

  constructor(v?: CreateNetworkFromInvitationRequest_IInvitation) {
    if (v && "invitationB64" in v) {
      this.case = CreateNetworkFromInvitationRequest_InvitationCase.INVITATION_B64;
      this.invitationB64 = v.invitationB64;
    } else
    if (v && "invitationBytes" in v) {
      this.case = CreateNetworkFromInvitationRequest_InvitationCase.INVITATION_BYTES;
      this.invitationBytes = v.invitationBytes;
    }
  }
}

export const CreateNetworkFromInvitationRequest_Invitation = CreateNetworkFromInvitationRequest_InvitationImpl as {
  new (): Readonly<{ case: CreateNetworkFromInvitationRequest_InvitationCase.NOT_SET }>;
  new <T extends CreateNetworkFromInvitationRequest_IInvitation>(v: T): Readonly<
  T extends { invitationB64: string } ? { case: CreateNetworkFromInvitationRequest_InvitationCase.INVITATION_B64, invitationB64: string } :
  T extends { invitationBytes: Uint8Array } ? { case: CreateNetworkFromInvitationRequest_InvitationCase.INVITATION_BYTES, invitationBytes: Uint8Array } :
  never
  >;
};

export type ICreateNetworkFromInvitationResponse = {
  network?: INetwork;
}

export class CreateNetworkFromInvitationResponse {
  network: Network | undefined;

  constructor(v?: ICreateNetworkFromInvitationResponse) {
    this.network = v?.network && new Network(v.network);
  }

  static encode(m: CreateNetworkFromInvitationResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.network) Network.encode(m.network, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateNetworkFromInvitationResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new CreateNetworkFromInvitationResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.network = Network.decode(r, r.uint32());
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

export type INetworkEvent = {
  body?: NetworkEvent_IBody
}

export class NetworkEvent {
  body: NetworkEvent_TBody;

  constructor(v?: INetworkEvent) {
    this.body = v?.body ? new NetworkEvent_Body(v.body) : new NetworkEvent_Body();
  }

  static encode(m: NetworkEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case NetworkEvent_BodyCase.NETWORK_START:
      NetworkEvent_NetworkStart.encode(m.body.networkStart, w.uint32(8010).fork()).ldelim();
      break;
      case NetworkEvent_BodyCase.NETWORK_STOP:
      NetworkEvent_NetworkStop.encode(m.body.networkStop, w.uint32(8018).fork()).ldelim();
      break;
      case NetworkEvent_BodyCase.NETWORK_PEER_COUNT_UPDATE:
      NetworkEvent_NetworkPeerCountUpdate.encode(m.body.networkPeerCountUpdate, w.uint32(8026).fork()).ldelim();
      break;
      case NetworkEvent_BodyCase.UI_CONFIG_UPDATE:
      UIConfig.encode(m.body.uiConfigUpdate, w.uint32(8034).fork()).ldelim();
      break;
      case NetworkEvent_BodyCase.NETWORK_UPDATE:
      Network.encode(m.body.networkUpdate, w.uint32(8042).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): NetworkEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new NetworkEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1001:
        m.body = new NetworkEvent_Body({ networkStart: NetworkEvent_NetworkStart.decode(r, r.uint32()) });
        break;
        case 1002:
        m.body = new NetworkEvent_Body({ networkStop: NetworkEvent_NetworkStop.decode(r, r.uint32()) });
        break;
        case 1003:
        m.body = new NetworkEvent_Body({ networkPeerCountUpdate: NetworkEvent_NetworkPeerCountUpdate.decode(r, r.uint32()) });
        break;
        case 1004:
        m.body = new NetworkEvent_Body({ uiConfigUpdate: UIConfig.decode(r, r.uint32()) });
        break;
        case 1005:
        m.body = new NetworkEvent_Body({ networkUpdate: Network.decode(r, r.uint32()) });
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

export enum NetworkEvent_BodyCase {
  NOT_SET = 0,
  NETWORK_START = 1001,
  NETWORK_STOP = 1002,
  NETWORK_PEER_COUNT_UPDATE = 1003,
  UI_CONFIG_UPDATE = 1004,
  NETWORK_UPDATE = 1005,
}

export type NetworkEvent_IBody =
{ case?: NetworkEvent_BodyCase.NOT_SET }
|{ case?: NetworkEvent_BodyCase.NETWORK_START, networkStart: NetworkEvent_INetworkStart }
|{ case?: NetworkEvent_BodyCase.NETWORK_STOP, networkStop: NetworkEvent_INetworkStop }
|{ case?: NetworkEvent_BodyCase.NETWORK_PEER_COUNT_UPDATE, networkPeerCountUpdate: NetworkEvent_INetworkPeerCountUpdate }
|{ case?: NetworkEvent_BodyCase.UI_CONFIG_UPDATE, uiConfigUpdate: IUIConfig }
|{ case?: NetworkEvent_BodyCase.NETWORK_UPDATE, networkUpdate: INetwork }
;

export type NetworkEvent_TBody = Readonly<
{ case: NetworkEvent_BodyCase.NOT_SET }
|{ case: NetworkEvent_BodyCase.NETWORK_START, networkStart: NetworkEvent_NetworkStart }
|{ case: NetworkEvent_BodyCase.NETWORK_STOP, networkStop: NetworkEvent_NetworkStop }
|{ case: NetworkEvent_BodyCase.NETWORK_PEER_COUNT_UPDATE, networkPeerCountUpdate: NetworkEvent_NetworkPeerCountUpdate }
|{ case: NetworkEvent_BodyCase.UI_CONFIG_UPDATE, uiConfigUpdate: UIConfig }
|{ case: NetworkEvent_BodyCase.NETWORK_UPDATE, networkUpdate: Network }
>;

class NetworkEvent_BodyImpl {
  declare networkStart?: NetworkEvent_NetworkStart;
  declare networkStop?: NetworkEvent_NetworkStop;
  declare networkPeerCountUpdate?: NetworkEvent_NetworkPeerCountUpdate;
  declare uiConfigUpdate?: UIConfig;
  declare networkUpdate?: Network;
  case: NetworkEvent_BodyCase = NetworkEvent_BodyCase.NOT_SET;

  constructor(v?: NetworkEvent_IBody) {
    if (v && "networkStart" in v) {
      this.case = NetworkEvent_BodyCase.NETWORK_START;
      this.networkStart = new NetworkEvent_NetworkStart(v.networkStart);
    } else
    if (v && "networkStop" in v) {
      this.case = NetworkEvent_BodyCase.NETWORK_STOP;
      this.networkStop = new NetworkEvent_NetworkStop(v.networkStop);
    } else
    if (v && "networkPeerCountUpdate" in v) {
      this.case = NetworkEvent_BodyCase.NETWORK_PEER_COUNT_UPDATE;
      this.networkPeerCountUpdate = new NetworkEvent_NetworkPeerCountUpdate(v.networkPeerCountUpdate);
    } else
    if (v && "uiConfigUpdate" in v) {
      this.case = NetworkEvent_BodyCase.UI_CONFIG_UPDATE;
      this.uiConfigUpdate = new UIConfig(v.uiConfigUpdate);
    } else
    if (v && "networkUpdate" in v) {
      this.case = NetworkEvent_BodyCase.NETWORK_UPDATE;
      this.networkUpdate = new Network(v.networkUpdate);
    }
  }
}

export const NetworkEvent_Body = NetworkEvent_BodyImpl as {
  new (): Readonly<{ case: NetworkEvent_BodyCase.NOT_SET }>;
  new <T extends NetworkEvent_IBody>(v: T): Readonly<
  T extends { networkStart: NetworkEvent_INetworkStart } ? { case: NetworkEvent_BodyCase.NETWORK_START, networkStart: NetworkEvent_NetworkStart } :
  T extends { networkStop: NetworkEvent_INetworkStop } ? { case: NetworkEvent_BodyCase.NETWORK_STOP, networkStop: NetworkEvent_NetworkStop } :
  T extends { networkPeerCountUpdate: NetworkEvent_INetworkPeerCountUpdate } ? { case: NetworkEvent_BodyCase.NETWORK_PEER_COUNT_UPDATE, networkPeerCountUpdate: NetworkEvent_NetworkPeerCountUpdate } :
  T extends { uiConfigUpdate: IUIConfig } ? { case: NetworkEvent_BodyCase.UI_CONFIG_UPDATE, uiConfigUpdate: UIConfig } :
  T extends { networkUpdate: INetwork } ? { case: NetworkEvent_BodyCase.NETWORK_UPDATE, networkUpdate: Network } :
  never
  >;
};

export type NetworkEvent_INetworkStart = {
  network?: INetwork;
  peerCount?: number;
}

export class NetworkEvent_NetworkStart {
  network: Network | undefined;
  peerCount: number;

  constructor(v?: NetworkEvent_INetworkStart) {
    this.network = v?.network && new Network(v.network);
    this.peerCount = v?.peerCount || 0;
  }

  static encode(m: NetworkEvent_NetworkStart, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.network) Network.encode(m.network, w.uint32(10).fork()).ldelim();
    if (m.peerCount) w.uint32(16).uint32(m.peerCount);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): NetworkEvent_NetworkStart {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new NetworkEvent_NetworkStart();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.network = Network.decode(r, r.uint32());
        break;
        case 2:
        m.peerCount = r.uint32();
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

export type NetworkEvent_INetworkStop = {
  networkId?: bigint;
}

export class NetworkEvent_NetworkStop {
  networkId: bigint;

  constructor(v?: NetworkEvent_INetworkStop) {
    this.networkId = v?.networkId || BigInt(0);
  }

  static encode(m: NetworkEvent_NetworkStop, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkId) w.uint32(8).uint64(m.networkId);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): NetworkEvent_NetworkStop {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new NetworkEvent_NetworkStop();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
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

export type NetworkEvent_INetworkPeerCountUpdate = {
  networkId?: bigint;
  peerCount?: number;
}

export class NetworkEvent_NetworkPeerCountUpdate {
  networkId: bigint;
  peerCount: number;

  constructor(v?: NetworkEvent_INetworkPeerCountUpdate) {
    this.networkId = v?.networkId || BigInt(0);
    this.peerCount = v?.peerCount || 0;
  }

  static encode(m: NetworkEvent_NetworkPeerCountUpdate, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkId) w.uint32(8).uint64(m.networkId);
    if (m.peerCount) w.uint32(16).uint32(m.peerCount);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): NetworkEvent_NetworkPeerCountUpdate {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new NetworkEvent_NetworkPeerCountUpdate();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkId = r.uint64();
        break;
        case 2:
        m.peerCount = r.uint32();
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

export type IUIConfig = {
  networkDisplayOrder?: bigint[];
}

export class UIConfig {
  networkDisplayOrder: bigint[];

  constructor(v?: IUIConfig) {
    this.networkDisplayOrder = v?.networkDisplayOrder ? v.networkDisplayOrder : [];
  }

  static encode(m: UIConfig, w?: Writer): Writer {
    if (!w) w = new Writer();
    m.networkDisplayOrder.reduce((w, v) => w.uint64(v), w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UIConfig {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UIConfig();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        for (const flen = r.uint32(), fend = r.pos + flen; r.pos < fend;) m.networkDisplayOrder.push(r.uint64());
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

export type IWatchNetworksRequest = object;

export class WatchNetworksRequest {

  constructor(v?: IWatchNetworksRequest) {
    void v; // an empty message has no fields to set
  }

  static encode(m: WatchNetworksRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WatchNetworksRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new WatchNetworksRequest();
  }
}

export type IWatchNetworksResponse = {
  event?: INetworkEvent;
}

export class WatchNetworksResponse {
  event: NetworkEvent | undefined;

  constructor(v?: IWatchNetworksResponse) {
    this.event = v?.event && new NetworkEvent(v.event);
  }

  static encode(m: WatchNetworksResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.event) NetworkEvent.encode(m.event, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WatchNetworksResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new WatchNetworksResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.event = NetworkEvent.decode(r, r.uint32());
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

export type IUpdateDisplayOrderRequest = {
  networkIds?: bigint[];
}

export class UpdateDisplayOrderRequest {
  networkIds: bigint[];

  constructor(v?: IUpdateDisplayOrderRequest) {
    this.networkIds = v?.networkIds ? v.networkIds : [];
  }

  static encode(m: UpdateDisplayOrderRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    m.networkIds.reduce((w, v) => w.uint64(v), w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateDisplayOrderRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UpdateDisplayOrderRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        for (const flen = r.uint32(), fend = r.pos + flen; r.pos < fend;) m.networkIds.push(r.uint64());
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

export type IUpdateDisplayOrderResponse = object;

export class UpdateDisplayOrderResponse {

  constructor(v?: IUpdateDisplayOrderResponse) {
    void v; // an empty message has no fields to set
  }

  static encode(m: UpdateDisplayOrderResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateDisplayOrderResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new UpdateDisplayOrderResponse();
  }
}

export type IUpdateAliasRequest = {
  id?: bigint;
  alias?: string;
}

export class UpdateAliasRequest {
  id: bigint;
  alias: string;

  constructor(v?: IUpdateAliasRequest) {
    this.id = v?.id || BigInt(0);
    this.alias = v?.alias || "";
  }

  static encode(m: UpdateAliasRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.alias.length) w.uint32(18).string(m.alias);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateAliasRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UpdateAliasRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.alias = r.string();
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

export type IUpdateAliasResponse = {
  network?: INetwork;
}

export class UpdateAliasResponse {
  network: Network | undefined;

  constructor(v?: IUpdateAliasResponse) {
    this.network = v?.network && new Network(v.network);
  }

  static encode(m: UpdateAliasResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.network) Network.encode(m.network, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateAliasResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UpdateAliasResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.network = Network.decode(r, r.uint32());
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

export type IGetUIConfigRequest = object;

export class GetUIConfigRequest {

  constructor(v?: IGetUIConfigRequest) {
    void v; // an empty message has no fields to set
  }

  static encode(m: GetUIConfigRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetUIConfigRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new GetUIConfigRequest();
  }
}

export type IGetUIConfigResponse = {
  config?: IUIConfig;
}

export class GetUIConfigResponse {
  config: UIConfig | undefined;

  constructor(v?: IGetUIConfigResponse) {
    this.config = v?.config && new UIConfig(v.config);
  }

  static encode(m: GetUIConfigResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.config) UIConfig.encode(m.config, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetUIConfigResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new GetUIConfigResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.config = UIConfig.decode(r, r.uint32());
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

export type IListPeersRequest = {
  networkId?: bigint;
}

export class ListPeersRequest {
  networkId: bigint;

  constructor(v?: IListPeersRequest) {
    this.networkId = v?.networkId || BigInt(0);
  }

  static encode(m: ListPeersRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkId) w.uint32(8).uint64(m.networkId);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListPeersRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ListPeersRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
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

export type IListPeersResponse = {
  peers?: IPeer[];
}

export class ListPeersResponse {
  peers: Peer[];

  constructor(v?: IListPeersResponse) {
    this.peers = v?.peers ? v.peers.map(v => new Peer(v)) : [];
  }

  static encode(m: ListPeersResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.peers) Peer.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListPeersResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ListPeersResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.peers.push(Peer.decode(r, r.uint32()));
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

export type IGrantPeerInvitationRequest = {
  id?: bigint;
  count?: number;
}

export class GrantPeerInvitationRequest {
  id: bigint;
  count: number;

  constructor(v?: IGrantPeerInvitationRequest) {
    this.id = v?.id || BigInt(0);
    this.count = v?.count || 0;
  }

  static encode(m: GrantPeerInvitationRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.count) w.uint32(16).uint32(m.count);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GrantPeerInvitationRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new GrantPeerInvitationRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.count = r.uint32();
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

export type IGrantPeerInvitationResponse = {
  peer?: IPeer;
}

export class GrantPeerInvitationResponse {
  peer: Peer | undefined;

  constructor(v?: IGrantPeerInvitationResponse) {
    this.peer = v?.peer && new Peer(v.peer);
  }

  static encode(m: GrantPeerInvitationResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.peer) Peer.encode(m.peer, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GrantPeerInvitationResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new GrantPeerInvitationResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.peer = Peer.decode(r, r.uint32());
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

export type ITogglePeerBanRequest = {
  id?: bigint;
  value?: boolean;
}

export class TogglePeerBanRequest {
  id: bigint;
  value: boolean;

  constructor(v?: ITogglePeerBanRequest) {
    this.id = v?.id || BigInt(0);
    this.value = v?.value || false;
  }

  static encode(m: TogglePeerBanRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.value) w.uint32(16).bool(m.value);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): TogglePeerBanRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new TogglePeerBanRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.value = r.bool();
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

export type ITogglePeerBanResponse = {
  peer?: IPeer;
}

export class TogglePeerBanResponse {
  peer: Peer | undefined;

  constructor(v?: ITogglePeerBanResponse) {
    this.peer = v?.peer && new Peer(v.peer);
  }

  static encode(m: TogglePeerBanResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.peer) Peer.encode(m.peer, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): TogglePeerBanResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new TogglePeerBanResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.peer = Peer.decode(r, r.uint32());
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

export type IResetPeerRenameCooldownRequest = {
  id?: bigint;
}

export class ResetPeerRenameCooldownRequest {
  id: bigint;

  constructor(v?: IResetPeerRenameCooldownRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: ResetPeerRenameCooldownRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ResetPeerRenameCooldownRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ResetPeerRenameCooldownRequest();
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

export type IResetPeerRenameCooldownResponse = {
  peer?: IPeer;
}

export class ResetPeerRenameCooldownResponse {
  peer: Peer | undefined;

  constructor(v?: IResetPeerRenameCooldownResponse) {
    this.peer = v?.peer && new Peer(v.peer);
  }

  static encode(m: ResetPeerRenameCooldownResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.peer) Peer.encode(m.peer, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ResetPeerRenameCooldownResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ResetPeerRenameCooldownResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.peer = Peer.decode(r, r.uint32());
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

export type IDeletePeerRequest = {
  id?: bigint;
}

export class DeletePeerRequest {
  id: bigint;

  constructor(v?: IDeletePeerRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: DeletePeerRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeletePeerRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new DeletePeerRequest();
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

export type IDeletePeerResponse = object;

export class DeletePeerResponse {

  constructor(v?: IDeletePeerResponse) {
    void v; // an empty message has no fields to set
  }

  static encode(m: DeletePeerResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeletePeerResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new DeletePeerResponse();
  }
}

export type IListAliasReservationsRequest = {
  networkId?: bigint;
}

export class ListAliasReservationsRequest {
  networkId: bigint;

  constructor(v?: IListAliasReservationsRequest) {
    this.networkId = v?.networkId || BigInt(0);
  }

  static encode(m: ListAliasReservationsRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkId) w.uint32(8).uint64(m.networkId);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListAliasReservationsRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ListAliasReservationsRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
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

export type IListAliasReservationsResponse = {
  aliasReservations?: IAliasReservation[];
}

export class ListAliasReservationsResponse {
  aliasReservations: AliasReservation[];

  constructor(v?: IListAliasReservationsResponse) {
    this.aliasReservations = v?.aliasReservations ? v.aliasReservations.map(v => new AliasReservation(v)) : [];
  }

  static encode(m: ListAliasReservationsResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.aliasReservations) AliasReservation.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListAliasReservationsResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ListAliasReservationsResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.aliasReservations.push(AliasReservation.decode(r, r.uint32()));
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

export type IResetAliasReservationCooldownRequest = {
  id?: bigint;
}

export class ResetAliasReservationCooldownRequest {
  id: bigint;

  constructor(v?: IResetAliasReservationCooldownRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: ResetAliasReservationCooldownRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ResetAliasReservationCooldownRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ResetAliasReservationCooldownRequest();
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

export type IResetAliasReservationCooldownResponse = object;

export class ResetAliasReservationCooldownResponse {

  constructor(v?: IResetAliasReservationCooldownResponse) {
    void v; // an empty message has no fields to set
  }

  static encode(m: ResetAliasReservationCooldownResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ResetAliasReservationCooldownResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new ResetAliasReservationCooldownResponse();
  }
}


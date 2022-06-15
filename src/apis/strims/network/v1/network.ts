import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  Key as strims_type_Key,
  IKey as strims_type_IKey,
} from "../../type/key";
import {
  Certificate as strims_type_Certificate,
  ICertificate as strims_type_ICertificate,
} from "../../type/certificate";
import {
  BootstrapClient as strims_network_v1_bootstrap_BootstrapClient,
  IBootstrapClient as strims_network_v1_bootstrap_IBootstrapClient,
} from "./bootstrap/bootstrap";
import {
  ServerConfig as strims_network_v1_directory_ServerConfig,
  IServerConfig as strims_network_v1_directory_IServerConfig,
} from "./directory/directory";
import {
  ErrorCode as strims_network_v1_errors_ErrorCode,
} from "./errors/errors";

export type INetworkIcon = {
  data?: Uint8Array;
  type?: string;
}

export class NetworkIcon {
  data: Uint8Array;
  type: string;

  constructor(v?: INetworkIcon) {
    this.data = v?.data || new Uint8Array();
    this.type = v?.type || "";
  }

  static encode(m: NetworkIcon, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.data.length) w.uint32(10).bytes(m.data);
    if (m.type.length) w.uint32(18).string(m.type);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): NetworkIcon {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new NetworkIcon();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.data = r.bytes();
        break;
        case 2:
        m.type = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICreateServerRequest = {
  name?: string;
  icon?: INetworkIcon;
  alias?: string;
}

export class CreateServerRequest {
  name: string;
  icon: NetworkIcon | undefined;
  alias: string;

  constructor(v?: ICreateServerRequest) {
    this.name = v?.name || "";
    this.icon = v?.icon && new NetworkIcon(v.icon);
    this.alias = v?.alias || "";
  }

  static encode(m: CreateServerRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.name.length) w.uint32(10).string(m.name);
    if (m.icon) NetworkIcon.encode(m.icon, w.uint32(18).fork()).ldelim();
    if (m.alias.length) w.uint32(26).string(m.alias);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateServerRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreateServerRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.name = r.string();
        break;
        case 2:
        m.icon = NetworkIcon.decode(r, r.uint32());
        break;
        case 3:
        m.alias = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreateServerResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.network = Network.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
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
    const end = length === undefined ? r.len : r.pos + length;
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
        r.skipType(tag & 7);
        break;
      }
    }
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UpdateServerConfigResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.network = Network.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DeleteNetworkRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IDeleteNetworkResponse = Record<string, any>;

export class DeleteNetworkResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IDeleteNetworkResponse) {
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetNetworkRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetNetworkResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.network = Network.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IListNetworksRequest = Record<string, any>;

export class ListNetworksRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IListNetworksRequest) {
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ListNetworksResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networks.push(Network.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IServerConfig = {
  name?: string;
  key?: strims_type_IKey;
  rootCertTtlSecs?: bigint;
  peerCertTtlSecs?: bigint;
  directory?: strims_network_v1_directory_IServerConfig;
}

export class ServerConfig {
  name: string;
  key: strims_type_Key | undefined;
  rootCertTtlSecs: bigint;
  peerCertTtlSecs: bigint;
  directory: strims_network_v1_directory_ServerConfig | undefined;

  constructor(v?: IServerConfig) {
    this.name = v?.name || "";
    this.key = v?.key && new strims_type_Key(v.key);
    this.rootCertTtlSecs = v?.rootCertTtlSecs || BigInt(0);
    this.peerCertTtlSecs = v?.peerCertTtlSecs || BigInt(0);
    this.directory = v?.directory && new strims_network_v1_directory_ServerConfig(v.directory);
  }

  static encode(m: ServerConfig, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.name.length) w.uint32(18).string(m.name);
    if (m.key) strims_type_Key.encode(m.key, w.uint32(26).fork()).ldelim();
    if (m.rootCertTtlSecs) w.uint32(32).uint64(m.rootCertTtlSecs);
    if (m.peerCertTtlSecs) w.uint32(40).uint64(m.peerCertTtlSecs);
    if (m.directory) strims_network_v1_directory_ServerConfig.encode(m.directory, w.uint32(50).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ServerConfig {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
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
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type INetwork = {
  id?: bigint;
  certificate?: strims_type_ICertificate;
  icon?: INetworkIcon;
  alias?: string;
  serverConfig?: IServerConfig;
  certificateRenewalError?: strims_network_v1_errors_ErrorCode;
}

export class Network {
  id: bigint;
  certificate: strims_type_Certificate | undefined;
  icon: NetworkIcon | undefined;
  alias: string;
  serverConfig: ServerConfig | undefined;
  certificateRenewalError: strims_network_v1_errors_ErrorCode;

  constructor(v?: INetwork) {
    this.id = v?.id || BigInt(0);
    this.certificate = v?.certificate && new strims_type_Certificate(v.certificate);
    this.icon = v?.icon && new NetworkIcon(v.icon);
    this.alias = v?.alias || "";
    this.serverConfig = v?.serverConfig && new ServerConfig(v.serverConfig);
    this.certificateRenewalError = v?.certificateRenewalError || 0;
  }

  static encode(m: Network, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.certificate) strims_type_Certificate.encode(m.certificate, w.uint32(18).fork()).ldelim();
    if (m.icon) NetworkIcon.encode(m.icon, w.uint32(26).fork()).ldelim();
    if (m.alias.length) w.uint32(34).string(m.alias);
    if (m.serverConfig) ServerConfig.encode(m.serverConfig, w.uint32(42).fork()).ldelim();
    if (m.certificateRenewalError) w.uint32(48).uint32(m.certificateRenewalError);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Network {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Network();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.certificate = strims_type_Certificate.decode(r, r.uint32());
        break;
        case 3:
        m.icon = NetworkIcon.decode(r, r.uint32());
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
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IPeer = {
  id?: bigint;
  networkId?: bigint;
  publicKey?: Uint8Array;
  inviterPeerId?: bigint;
  inviteQuota?: number;
}

export class Peer {
  id: bigint;
  networkId: bigint;
  publicKey: Uint8Array;
  inviterPeerId: bigint;
  inviteQuota: number;

  constructor(v?: IPeer) {
    this.id = v?.id || BigInt(0);
    this.networkId = v?.networkId || BigInt(0);
    this.publicKey = v?.publicKey || new Uint8Array();
    this.inviterPeerId = v?.inviterPeerId || BigInt(0);
    this.inviteQuota = v?.inviteQuota || 0;
  }

  static encode(m: Peer, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.networkId) w.uint32(16).uint64(m.networkId);
    if (m.publicKey.length) w.uint32(26).bytes(m.publicKey);
    if (m.inviterPeerId) w.uint32(32).uint64(m.inviterPeerId);
    if (m.inviteQuota) w.uint32(40).uint32(m.inviteQuota);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Peer {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
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
        default:
        r.skipType(tag & 7);
        break;
      }
    }
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
    const end = length === undefined ? r.len : r.pos + length;
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
        r.skipType(tag & 7);
        break;
      }
    }
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreateInvitationResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.invitation = Invitation.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
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
    const end = length === undefined ? r.len : r.pos + length;
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
        r.skipType(tag & 7);
        break;
      }
    }
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
    const end = length === undefined ? r.len : r.pos + length;
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
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICreateNetworkFromInvitationRequest = {
  alias?: string;
  invitation?: CreateNetworkFromInvitationRequest.IInvitation
}

export class CreateNetworkFromInvitationRequest {
  alias: string;
  invitation: CreateNetworkFromInvitationRequest.TInvitation;

  constructor(v?: ICreateNetworkFromInvitationRequest) {
    this.alias = v?.alias || "";
    this.invitation = new CreateNetworkFromInvitationRequest.Invitation(v?.invitation);
  }

  static encode(m: CreateNetworkFromInvitationRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.alias.length) w.uint32(10).string(m.alias);
    switch (m.invitation.case) {
      case CreateNetworkFromInvitationRequest.InvitationCase.INVITATION_B64:
      w.uint32(8010).string(m.invitation.invitationB64);
      break;
      case CreateNetworkFromInvitationRequest.InvitationCase.INVITATION_BYTES:
      w.uint32(8018).bytes(m.invitation.invitationBytes);
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateNetworkFromInvitationRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreateNetworkFromInvitationRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.alias = r.string();
        break;
        case 1001:
        m.invitation = new CreateNetworkFromInvitationRequest.Invitation({ invitationB64: r.string() });
        break;
        case 1002:
        m.invitation = new CreateNetworkFromInvitationRequest.Invitation({ invitationBytes: r.bytes() });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace CreateNetworkFromInvitationRequest {
  export enum InvitationCase {
    NOT_SET = 0,
    INVITATION_B64 = 1001,
    INVITATION_BYTES = 1002,
  }

  export type IInvitation =
  { case?: InvitationCase.NOT_SET }
  |{ case?: InvitationCase.INVITATION_B64, invitationB64: string }
  |{ case?: InvitationCase.INVITATION_BYTES, invitationBytes: Uint8Array }
  ;

  export type TInvitation = Readonly<
  { case: InvitationCase.NOT_SET }
  |{ case: InvitationCase.INVITATION_B64, invitationB64: string }
  |{ case: InvitationCase.INVITATION_BYTES, invitationBytes: Uint8Array }
  >;

  class InvitationImpl {
    invitationB64: string;
    invitationBytes: Uint8Array;
    case: InvitationCase = InvitationCase.NOT_SET;

    constructor(v?: IInvitation) {
      if (v && "invitationB64" in v) {
        this.case = InvitationCase.INVITATION_B64;
        this.invitationB64 = v.invitationB64;
      } else
      if (v && "invitationBytes" in v) {
        this.case = InvitationCase.INVITATION_BYTES;
        this.invitationBytes = v.invitationBytes;
      }
    }
  }

  export const Invitation = InvitationImpl as {
    new (): Readonly<{ case: InvitationCase.NOT_SET }>;
    new <T extends IInvitation>(v: T): Readonly<
    T extends { invitationB64: string } ? { case: InvitationCase.INVITATION_B64, invitationB64: string } :
    T extends { invitationBytes: Uint8Array } ? { case: InvitationCase.INVITATION_BYTES, invitationBytes: Uint8Array } :
    never
    >;
  };

}

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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreateNetworkFromInvitationResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.network = Network.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type INetworkEvent = {
  body?: NetworkEvent.IBody
}

export class NetworkEvent {
  body: NetworkEvent.TBody;

  constructor(v?: INetworkEvent) {
    this.body = new NetworkEvent.Body(v?.body);
  }

  static encode(m: NetworkEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case NetworkEvent.BodyCase.NETWORK_START:
      NetworkEvent.NetworkStart.encode(m.body.networkStart, w.uint32(8010).fork()).ldelim();
      break;
      case NetworkEvent.BodyCase.NETWORK_STOP:
      NetworkEvent.NetworkStop.encode(m.body.networkStop, w.uint32(8018).fork()).ldelim();
      break;
      case NetworkEvent.BodyCase.NETWORK_PEER_COUNT_UPDATE:
      NetworkEvent.NetworkPeerCountUpdate.encode(m.body.networkPeerCountUpdate, w.uint32(8026).fork()).ldelim();
      break;
      case NetworkEvent.BodyCase.UI_CONFIG_UPDATE:
      UIConfig.encode(m.body.uiConfigUpdate, w.uint32(8034).fork()).ldelim();
      break;
      case NetworkEvent.BodyCase.NETWORK_UPDATE:
      Network.encode(m.body.networkUpdate, w.uint32(8042).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): NetworkEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new NetworkEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1001:
        m.body = new NetworkEvent.Body({ networkStart: NetworkEvent.NetworkStart.decode(r, r.uint32()) });
        break;
        case 1002:
        m.body = new NetworkEvent.Body({ networkStop: NetworkEvent.NetworkStop.decode(r, r.uint32()) });
        break;
        case 1003:
        m.body = new NetworkEvent.Body({ networkPeerCountUpdate: NetworkEvent.NetworkPeerCountUpdate.decode(r, r.uint32()) });
        break;
        case 1004:
        m.body = new NetworkEvent.Body({ uiConfigUpdate: UIConfig.decode(r, r.uint32()) });
        break;
        case 1005:
        m.body = new NetworkEvent.Body({ networkUpdate: Network.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace NetworkEvent {
  export enum BodyCase {
    NOT_SET = 0,
    NETWORK_START = 1001,
    NETWORK_STOP = 1002,
    NETWORK_PEER_COUNT_UPDATE = 1003,
    UI_CONFIG_UPDATE = 1004,
    NETWORK_UPDATE = 1005,
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.NETWORK_START, networkStart: NetworkEvent.INetworkStart }
  |{ case?: BodyCase.NETWORK_STOP, networkStop: NetworkEvent.INetworkStop }
  |{ case?: BodyCase.NETWORK_PEER_COUNT_UPDATE, networkPeerCountUpdate: NetworkEvent.INetworkPeerCountUpdate }
  |{ case?: BodyCase.UI_CONFIG_UPDATE, uiConfigUpdate: IUIConfig }
  |{ case?: BodyCase.NETWORK_UPDATE, networkUpdate: INetwork }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.NETWORK_START, networkStart: NetworkEvent.NetworkStart }
  |{ case: BodyCase.NETWORK_STOP, networkStop: NetworkEvent.NetworkStop }
  |{ case: BodyCase.NETWORK_PEER_COUNT_UPDATE, networkPeerCountUpdate: NetworkEvent.NetworkPeerCountUpdate }
  |{ case: BodyCase.UI_CONFIG_UPDATE, uiConfigUpdate: UIConfig }
  |{ case: BodyCase.NETWORK_UPDATE, networkUpdate: Network }
  >;

  class BodyImpl {
    networkStart: NetworkEvent.NetworkStart;
    networkStop: NetworkEvent.NetworkStop;
    networkPeerCountUpdate: NetworkEvent.NetworkPeerCountUpdate;
    uiConfigUpdate: UIConfig;
    networkUpdate: Network;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "networkStart" in v) {
        this.case = BodyCase.NETWORK_START;
        this.networkStart = new NetworkEvent.NetworkStart(v.networkStart);
      } else
      if (v && "networkStop" in v) {
        this.case = BodyCase.NETWORK_STOP;
        this.networkStop = new NetworkEvent.NetworkStop(v.networkStop);
      } else
      if (v && "networkPeerCountUpdate" in v) {
        this.case = BodyCase.NETWORK_PEER_COUNT_UPDATE;
        this.networkPeerCountUpdate = new NetworkEvent.NetworkPeerCountUpdate(v.networkPeerCountUpdate);
      } else
      if (v && "uiConfigUpdate" in v) {
        this.case = BodyCase.UI_CONFIG_UPDATE;
        this.uiConfigUpdate = new UIConfig(v.uiConfigUpdate);
      } else
      if (v && "networkUpdate" in v) {
        this.case = BodyCase.NETWORK_UPDATE;
        this.networkUpdate = new Network(v.networkUpdate);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { networkStart: NetworkEvent.INetworkStart } ? { case: BodyCase.NETWORK_START, networkStart: NetworkEvent.NetworkStart } :
    T extends { networkStop: NetworkEvent.INetworkStop } ? { case: BodyCase.NETWORK_STOP, networkStop: NetworkEvent.NetworkStop } :
    T extends { networkPeerCountUpdate: NetworkEvent.INetworkPeerCountUpdate } ? { case: BodyCase.NETWORK_PEER_COUNT_UPDATE, networkPeerCountUpdate: NetworkEvent.NetworkPeerCountUpdate } :
    T extends { uiConfigUpdate: IUIConfig } ? { case: BodyCase.UI_CONFIG_UPDATE, uiConfigUpdate: UIConfig } :
    T extends { networkUpdate: INetwork } ? { case: BodyCase.NETWORK_UPDATE, networkUpdate: Network } :
    never
    >;
  };

  export type INetworkStart = {
    network?: INetwork;
    peerCount?: number;
  }

  export class NetworkStart {
    network: Network | undefined;
    peerCount: number;

    constructor(v?: INetworkStart) {
      this.network = v?.network && new Network(v.network);
      this.peerCount = v?.peerCount || 0;
    }

    static encode(m: NetworkStart, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.network) Network.encode(m.network, w.uint32(10).fork()).ldelim();
      if (m.peerCount) w.uint32(16).uint32(m.peerCount);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): NetworkStart {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new NetworkStart();
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
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type INetworkStop = {
    networkId?: bigint;
  }

  export class NetworkStop {
    networkId: bigint;

    constructor(v?: INetworkStop) {
      this.networkId = v?.networkId || BigInt(0);
    }

    static encode(m: NetworkStop, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.networkId) w.uint32(8).uint64(m.networkId);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): NetworkStop {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new NetworkStop();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.networkId = r.uint64();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type INetworkPeerCountUpdate = {
    networkId?: bigint;
    peerCount?: number;
  }

  export class NetworkPeerCountUpdate {
    networkId: bigint;
    peerCount: number;

    constructor(v?: INetworkPeerCountUpdate) {
      this.networkId = v?.networkId || BigInt(0);
      this.peerCount = v?.peerCount || 0;
    }

    static encode(m: NetworkPeerCountUpdate, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.networkId) w.uint32(8).uint64(m.networkId);
      if (m.peerCount) w.uint32(16).uint32(m.peerCount);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): NetworkPeerCountUpdate {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new NetworkPeerCountUpdate();
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
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UIConfig();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        for (const flen = r.uint32(), fend = r.pos + flen; r.pos < fend;) m.networkDisplayOrder.push(r.uint64());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IWatchNetworksRequest = Record<string, any>;

export class WatchNetworksRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IWatchNetworksRequest) {
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new WatchNetworksResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.event = NetworkEvent.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UpdateDisplayOrderRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        for (const flen = r.uint32(), fend = r.pos + flen; r.pos < fend;) m.networkIds.push(r.uint64());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IUpdateDisplayOrderResponse = Record<string, any>;

export class UpdateDisplayOrderResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IUpdateDisplayOrderResponse) {
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
    const end = length === undefined ? r.len : r.pos + length;
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
        r.skipType(tag & 7);
        break;
      }
    }
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UpdateAliasResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.network = Network.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IGetUIConfigRequest = Record<string, any>;

export class GetUIConfigRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IGetUIConfigRequest) {
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetUIConfigResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.config = UIConfig.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}


import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  strims_type_Key,
  strims_type_IKey,
} from "../../type/key";
import {
  strims_type_Image,
  strims_type_IImage,
} from "../../type/image";
import {
  strims_type_Certificate,
  strims_type_ICertificate,
} from "../../type/certificate";
import {
  strims_network_v1_bootstrap_BootstrapClient,
  strims_network_v1_bootstrap_IBootstrapClient,
} from "./bootstrap/bootstrap";
import {
  strims_network_v1_directory_ServerConfig,
  strims_network_v1_directory_IServerConfig,
} from "./directory/directory";
import {
  strims_network_v1_errors_ErrorCode,
} from "./errors/errors";

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
    const end = length === undefined ? r.len : r.pos + length;
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
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICreateServerResponse = {
  network?: strims_network_v1_INetwork;
}

export class CreateServerResponse {
  network: strims_network_v1_Network | undefined;

  constructor(v?: ICreateServerResponse) {
    this.network = v?.network && new strims_network_v1_Network(v.network);
  }

  static encode(m: CreateServerResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.network) strims_network_v1_Network.encode(m.network, w.uint32(10).fork()).ldelim();
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
        m.network = strims_network_v1_Network.decode(r, r.uint32());
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
  serverConfig?: strims_network_v1_IServerConfig;
}

export class UpdateServerConfigRequest {
  networkId: bigint;
  serverConfig: strims_network_v1_ServerConfig | undefined;

  constructor(v?: IUpdateServerConfigRequest) {
    this.networkId = v?.networkId || BigInt(0);
    this.serverConfig = v?.serverConfig && new strims_network_v1_ServerConfig(v.serverConfig);
  }

  static encode(m: UpdateServerConfigRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkId) w.uint32(8).uint64(m.networkId);
    if (m.serverConfig) strims_network_v1_ServerConfig.encode(m.serverConfig, w.uint32(18).fork()).ldelim();
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
        m.serverConfig = strims_network_v1_ServerConfig.decode(r, r.uint32());
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
  network?: strims_network_v1_INetwork;
}

export class UpdateServerConfigResponse {
  network: strims_network_v1_Network | undefined;

  constructor(v?: IUpdateServerConfigResponse) {
    this.network = v?.network && new strims_network_v1_Network(v.network);
  }

  static encode(m: UpdateServerConfigResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.network) strims_network_v1_Network.encode(m.network, w.uint32(10).fork()).ldelim();
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
        m.network = strims_network_v1_Network.decode(r, r.uint32());
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
  network?: strims_network_v1_INetwork;
}

export class GetNetworkResponse {
  network: strims_network_v1_Network | undefined;

  constructor(v?: IGetNetworkResponse) {
    this.network = v?.network && new strims_network_v1_Network(v.network);
  }

  static encode(m: GetNetworkResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.network) strims_network_v1_Network.encode(m.network, w.uint32(10).fork()).ldelim();
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
        m.network = strims_network_v1_Network.decode(r, r.uint32());
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
  networks?: strims_network_v1_INetwork[];
}

export class ListNetworksResponse {
  networks: strims_network_v1_Network[];

  constructor(v?: IListNetworksResponse) {
    this.networks = v?.networks ? v.networks.map(v => new strims_network_v1_Network(v)) : [];
  }

  static encode(m: ListNetworksResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.networks) strims_network_v1_Network.encode(v, w.uint32(10).fork()).ldelim();
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
        m.networks.push(strims_network_v1_Network.decode(r, r.uint32()));
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
        case 7:
        m.icon = strims_type_Image.decode(r, r.uint32());
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
  alias?: string;
  serverConfig?: strims_network_v1_IServerConfig;
  certificateRenewalError?: strims_network_v1_errors_ErrorCode;
}

export class Network {
  id: bigint;
  certificate: strims_type_Certificate | undefined;
  alias: string;
  serverConfig: strims_network_v1_ServerConfig | undefined;
  certificateRenewalError: strims_network_v1_errors_ErrorCode;

  constructor(v?: INetwork) {
    this.id = v?.id || BigInt(0);
    this.certificate = v?.certificate && new strims_type_Certificate(v.certificate);
    this.alias = v?.alias || "";
    this.serverConfig = v?.serverConfig && new strims_network_v1_ServerConfig(v.serverConfig);
    this.certificateRenewalError = v?.certificateRenewalError || 0;
  }

  static encode(m: Network, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.certificate) strims_type_Certificate.encode(m.certificate, w.uint32(18).fork()).ldelim();
    if (m.alias.length) w.uint32(34).string(m.alias);
    if (m.serverConfig) strims_network_v1_ServerConfig.encode(m.serverConfig, w.uint32(42).fork()).ldelim();
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
        case 4:
        m.alias = r.string();
        break;
        case 5:
        m.serverConfig = strims_network_v1_ServerConfig.decode(r, r.uint32());
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
  invitation?: strims_network_v1_IInvitation;
}

export class CreateInvitationResponse {
  invitation: strims_network_v1_Invitation | undefined;

  constructor(v?: ICreateInvitationResponse) {
    this.invitation = v?.invitation && new strims_network_v1_Invitation(v.invitation);
  }

  static encode(m: CreateInvitationResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.invitation) strims_network_v1_Invitation.encode(m.invitation, w.uint32(10).fork()).ldelim();
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
        m.invitation = strims_network_v1_Invitation.decode(r, r.uint32());
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
  network?: strims_network_v1_INetwork;
}

export class CreateNetworkFromInvitationResponse {
  network: strims_network_v1_Network | undefined;

  constructor(v?: ICreateNetworkFromInvitationResponse) {
    this.network = v?.network && new strims_network_v1_Network(v.network);
  }

  static encode(m: CreateNetworkFromInvitationResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.network) strims_network_v1_Network.encode(m.network, w.uint32(10).fork()).ldelim();
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
        m.network = strims_network_v1_Network.decode(r, r.uint32());
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
      strims_network_v1_NetworkEvent_NetworkStart.encode(m.body.networkStart, w.uint32(8010).fork()).ldelim();
      break;
      case NetworkEvent.BodyCase.NETWORK_STOP:
      strims_network_v1_NetworkEvent_NetworkStop.encode(m.body.networkStop, w.uint32(8018).fork()).ldelim();
      break;
      case NetworkEvent.BodyCase.NETWORK_PEER_COUNT_UPDATE:
      strims_network_v1_NetworkEvent_NetworkPeerCountUpdate.encode(m.body.networkPeerCountUpdate, w.uint32(8026).fork()).ldelim();
      break;
      case NetworkEvent.BodyCase.UI_CONFIG_UPDATE:
      strims_network_v1_UIConfig.encode(m.body.uiConfigUpdate, w.uint32(8034).fork()).ldelim();
      break;
      case NetworkEvent.BodyCase.NETWORK_UPDATE:
      strims_network_v1_Network.encode(m.body.networkUpdate, w.uint32(8042).fork()).ldelim();
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
        m.body = new NetworkEvent.Body({ networkStart: strims_network_v1_NetworkEvent_NetworkStart.decode(r, r.uint32()) });
        break;
        case 1002:
        m.body = new NetworkEvent.Body({ networkStop: strims_network_v1_NetworkEvent_NetworkStop.decode(r, r.uint32()) });
        break;
        case 1003:
        m.body = new NetworkEvent.Body({ networkPeerCountUpdate: strims_network_v1_NetworkEvent_NetworkPeerCountUpdate.decode(r, r.uint32()) });
        break;
        case 1004:
        m.body = new NetworkEvent.Body({ uiConfigUpdate: strims_network_v1_UIConfig.decode(r, r.uint32()) });
        break;
        case 1005:
        m.body = new NetworkEvent.Body({ networkUpdate: strims_network_v1_Network.decode(r, r.uint32()) });
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
  |{ case?: BodyCase.NETWORK_START, networkStart: strims_network_v1_NetworkEvent_INetworkStart }
  |{ case?: BodyCase.NETWORK_STOP, networkStop: strims_network_v1_NetworkEvent_INetworkStop }
  |{ case?: BodyCase.NETWORK_PEER_COUNT_UPDATE, networkPeerCountUpdate: strims_network_v1_NetworkEvent_INetworkPeerCountUpdate }
  |{ case?: BodyCase.UI_CONFIG_UPDATE, uiConfigUpdate: strims_network_v1_IUIConfig }
  |{ case?: BodyCase.NETWORK_UPDATE, networkUpdate: strims_network_v1_INetwork }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.NETWORK_START, networkStart: strims_network_v1_NetworkEvent_NetworkStart }
  |{ case: BodyCase.NETWORK_STOP, networkStop: strims_network_v1_NetworkEvent_NetworkStop }
  |{ case: BodyCase.NETWORK_PEER_COUNT_UPDATE, networkPeerCountUpdate: strims_network_v1_NetworkEvent_NetworkPeerCountUpdate }
  |{ case: BodyCase.UI_CONFIG_UPDATE, uiConfigUpdate: strims_network_v1_UIConfig }
  |{ case: BodyCase.NETWORK_UPDATE, networkUpdate: strims_network_v1_Network }
  >;

  class BodyImpl {
    networkStart: strims_network_v1_NetworkEvent_NetworkStart;
    networkStop: strims_network_v1_NetworkEvent_NetworkStop;
    networkPeerCountUpdate: strims_network_v1_NetworkEvent_NetworkPeerCountUpdate;
    uiConfigUpdate: strims_network_v1_UIConfig;
    networkUpdate: strims_network_v1_Network;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "networkStart" in v) {
        this.case = BodyCase.NETWORK_START;
        this.networkStart = new strims_network_v1_NetworkEvent_NetworkStart(v.networkStart);
      } else
      if (v && "networkStop" in v) {
        this.case = BodyCase.NETWORK_STOP;
        this.networkStop = new strims_network_v1_NetworkEvent_NetworkStop(v.networkStop);
      } else
      if (v && "networkPeerCountUpdate" in v) {
        this.case = BodyCase.NETWORK_PEER_COUNT_UPDATE;
        this.networkPeerCountUpdate = new strims_network_v1_NetworkEvent_NetworkPeerCountUpdate(v.networkPeerCountUpdate);
      } else
      if (v && "uiConfigUpdate" in v) {
        this.case = BodyCase.UI_CONFIG_UPDATE;
        this.uiConfigUpdate = new strims_network_v1_UIConfig(v.uiConfigUpdate);
      } else
      if (v && "networkUpdate" in v) {
        this.case = BodyCase.NETWORK_UPDATE;
        this.networkUpdate = new strims_network_v1_Network(v.networkUpdate);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { networkStart: strims_network_v1_NetworkEvent_INetworkStart } ? { case: BodyCase.NETWORK_START, networkStart: strims_network_v1_NetworkEvent_NetworkStart } :
    T extends { networkStop: strims_network_v1_NetworkEvent_INetworkStop } ? { case: BodyCase.NETWORK_STOP, networkStop: strims_network_v1_NetworkEvent_NetworkStop } :
    T extends { networkPeerCountUpdate: strims_network_v1_NetworkEvent_INetworkPeerCountUpdate } ? { case: BodyCase.NETWORK_PEER_COUNT_UPDATE, networkPeerCountUpdate: strims_network_v1_NetworkEvent_NetworkPeerCountUpdate } :
    T extends { uiConfigUpdate: strims_network_v1_IUIConfig } ? { case: BodyCase.UI_CONFIG_UPDATE, uiConfigUpdate: strims_network_v1_UIConfig } :
    T extends { networkUpdate: strims_network_v1_INetwork } ? { case: BodyCase.NETWORK_UPDATE, networkUpdate: strims_network_v1_Network } :
    never
    >;
  };

  export type INetworkStart = {
    network?: strims_network_v1_INetwork;
    peerCount?: number;
  }

  export class NetworkStart {
    network: strims_network_v1_Network | undefined;
    peerCount: number;

    constructor(v?: INetworkStart) {
      this.network = v?.network && new strims_network_v1_Network(v.network);
      this.peerCount = v?.peerCount || 0;
    }

    static encode(m: NetworkStart, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.network) strims_network_v1_Network.encode(m.network, w.uint32(10).fork()).ldelim();
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
          m.network = strims_network_v1_Network.decode(r, r.uint32());
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
  event?: strims_network_v1_INetworkEvent;
}

export class WatchNetworksResponse {
  event: strims_network_v1_NetworkEvent | undefined;

  constructor(v?: IWatchNetworksResponse) {
    this.event = v?.event && new strims_network_v1_NetworkEvent(v.event);
  }

  static encode(m: WatchNetworksResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.event) strims_network_v1_NetworkEvent.encode(m.event, w.uint32(10).fork()).ldelim();
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
        m.event = strims_network_v1_NetworkEvent.decode(r, r.uint32());
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
  network?: strims_network_v1_INetwork;
}

export class UpdateAliasResponse {
  network: strims_network_v1_Network | undefined;

  constructor(v?: IUpdateAliasResponse) {
    this.network = v?.network && new strims_network_v1_Network(v.network);
  }

  static encode(m: UpdateAliasResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.network) strims_network_v1_Network.encode(m.network, w.uint32(10).fork()).ldelim();
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
        m.network = strims_network_v1_Network.decode(r, r.uint32());
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
  config?: strims_network_v1_IUIConfig;
}

export class GetUIConfigResponse {
  config: strims_network_v1_UIConfig | undefined;

  constructor(v?: IGetUIConfigResponse) {
    this.config = v?.config && new strims_network_v1_UIConfig(v.config);
  }

  static encode(m: GetUIConfigResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.config) strims_network_v1_UIConfig.encode(m.config, w.uint32(10).fork()).ldelim();
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
        m.config = strims_network_v1_UIConfig.decode(r, r.uint32());
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
export const strims_network_v1_CreateServerRequest = CreateServerRequest;
/* @internal */
export type strims_network_v1_CreateServerRequest = CreateServerRequest;
/* @internal */
export type strims_network_v1_ICreateServerRequest = ICreateServerRequest;
/* @internal */
export const strims_network_v1_CreateServerResponse = CreateServerResponse;
/* @internal */
export type strims_network_v1_CreateServerResponse = CreateServerResponse;
/* @internal */
export type strims_network_v1_ICreateServerResponse = ICreateServerResponse;
/* @internal */
export const strims_network_v1_UpdateServerConfigRequest = UpdateServerConfigRequest;
/* @internal */
export type strims_network_v1_UpdateServerConfigRequest = UpdateServerConfigRequest;
/* @internal */
export type strims_network_v1_IUpdateServerConfigRequest = IUpdateServerConfigRequest;
/* @internal */
export const strims_network_v1_UpdateServerConfigResponse = UpdateServerConfigResponse;
/* @internal */
export type strims_network_v1_UpdateServerConfigResponse = UpdateServerConfigResponse;
/* @internal */
export type strims_network_v1_IUpdateServerConfigResponse = IUpdateServerConfigResponse;
/* @internal */
export const strims_network_v1_DeleteNetworkRequest = DeleteNetworkRequest;
/* @internal */
export type strims_network_v1_DeleteNetworkRequest = DeleteNetworkRequest;
/* @internal */
export type strims_network_v1_IDeleteNetworkRequest = IDeleteNetworkRequest;
/* @internal */
export const strims_network_v1_DeleteNetworkResponse = DeleteNetworkResponse;
/* @internal */
export type strims_network_v1_DeleteNetworkResponse = DeleteNetworkResponse;
/* @internal */
export type strims_network_v1_IDeleteNetworkResponse = IDeleteNetworkResponse;
/* @internal */
export const strims_network_v1_GetNetworkRequest = GetNetworkRequest;
/* @internal */
export type strims_network_v1_GetNetworkRequest = GetNetworkRequest;
/* @internal */
export type strims_network_v1_IGetNetworkRequest = IGetNetworkRequest;
/* @internal */
export const strims_network_v1_GetNetworkResponse = GetNetworkResponse;
/* @internal */
export type strims_network_v1_GetNetworkResponse = GetNetworkResponse;
/* @internal */
export type strims_network_v1_IGetNetworkResponse = IGetNetworkResponse;
/* @internal */
export const strims_network_v1_ListNetworksRequest = ListNetworksRequest;
/* @internal */
export type strims_network_v1_ListNetworksRequest = ListNetworksRequest;
/* @internal */
export type strims_network_v1_IListNetworksRequest = IListNetworksRequest;
/* @internal */
export const strims_network_v1_ListNetworksResponse = ListNetworksResponse;
/* @internal */
export type strims_network_v1_ListNetworksResponse = ListNetworksResponse;
/* @internal */
export type strims_network_v1_IListNetworksResponse = IListNetworksResponse;
/* @internal */
export const strims_network_v1_ServerConfig = ServerConfig;
/* @internal */
export type strims_network_v1_ServerConfig = ServerConfig;
/* @internal */
export type strims_network_v1_IServerConfig = IServerConfig;
/* @internal */
export const strims_network_v1_Network = Network;
/* @internal */
export type strims_network_v1_Network = Network;
/* @internal */
export type strims_network_v1_INetwork = INetwork;
/* @internal */
export const strims_network_v1_Peer = Peer;
/* @internal */
export type strims_network_v1_Peer = Peer;
/* @internal */
export type strims_network_v1_IPeer = IPeer;
/* @internal */
export const strims_network_v1_CreateInvitationRequest = CreateInvitationRequest;
/* @internal */
export type strims_network_v1_CreateInvitationRequest = CreateInvitationRequest;
/* @internal */
export type strims_network_v1_ICreateInvitationRequest = ICreateInvitationRequest;
/* @internal */
export const strims_network_v1_CreateInvitationResponse = CreateInvitationResponse;
/* @internal */
export type strims_network_v1_CreateInvitationResponse = CreateInvitationResponse;
/* @internal */
export type strims_network_v1_ICreateInvitationResponse = ICreateInvitationResponse;
/* @internal */
export const strims_network_v1_Invitation = Invitation;
/* @internal */
export type strims_network_v1_Invitation = Invitation;
/* @internal */
export type strims_network_v1_IInvitation = IInvitation;
/* @internal */
export const strims_network_v1_InvitationV0 = InvitationV0;
/* @internal */
export type strims_network_v1_InvitationV0 = InvitationV0;
/* @internal */
export type strims_network_v1_IInvitationV0 = IInvitationV0;
/* @internal */
export const strims_network_v1_CreateNetworkFromInvitationRequest = CreateNetworkFromInvitationRequest;
/* @internal */
export type strims_network_v1_CreateNetworkFromInvitationRequest = CreateNetworkFromInvitationRequest;
/* @internal */
export type strims_network_v1_ICreateNetworkFromInvitationRequest = ICreateNetworkFromInvitationRequest;
/* @internal */
export const strims_network_v1_CreateNetworkFromInvitationResponse = CreateNetworkFromInvitationResponse;
/* @internal */
export type strims_network_v1_CreateNetworkFromInvitationResponse = CreateNetworkFromInvitationResponse;
/* @internal */
export type strims_network_v1_ICreateNetworkFromInvitationResponse = ICreateNetworkFromInvitationResponse;
/* @internal */
export const strims_network_v1_NetworkEvent = NetworkEvent;
/* @internal */
export type strims_network_v1_NetworkEvent = NetworkEvent;
/* @internal */
export type strims_network_v1_INetworkEvent = INetworkEvent;
/* @internal */
export const strims_network_v1_UIConfig = UIConfig;
/* @internal */
export type strims_network_v1_UIConfig = UIConfig;
/* @internal */
export type strims_network_v1_IUIConfig = IUIConfig;
/* @internal */
export const strims_network_v1_WatchNetworksRequest = WatchNetworksRequest;
/* @internal */
export type strims_network_v1_WatchNetworksRequest = WatchNetworksRequest;
/* @internal */
export type strims_network_v1_IWatchNetworksRequest = IWatchNetworksRequest;
/* @internal */
export const strims_network_v1_WatchNetworksResponse = WatchNetworksResponse;
/* @internal */
export type strims_network_v1_WatchNetworksResponse = WatchNetworksResponse;
/* @internal */
export type strims_network_v1_IWatchNetworksResponse = IWatchNetworksResponse;
/* @internal */
export const strims_network_v1_UpdateDisplayOrderRequest = UpdateDisplayOrderRequest;
/* @internal */
export type strims_network_v1_UpdateDisplayOrderRequest = UpdateDisplayOrderRequest;
/* @internal */
export type strims_network_v1_IUpdateDisplayOrderRequest = IUpdateDisplayOrderRequest;
/* @internal */
export const strims_network_v1_UpdateDisplayOrderResponse = UpdateDisplayOrderResponse;
/* @internal */
export type strims_network_v1_UpdateDisplayOrderResponse = UpdateDisplayOrderResponse;
/* @internal */
export type strims_network_v1_IUpdateDisplayOrderResponse = IUpdateDisplayOrderResponse;
/* @internal */
export const strims_network_v1_UpdateAliasRequest = UpdateAliasRequest;
/* @internal */
export type strims_network_v1_UpdateAliasRequest = UpdateAliasRequest;
/* @internal */
export type strims_network_v1_IUpdateAliasRequest = IUpdateAliasRequest;
/* @internal */
export const strims_network_v1_UpdateAliasResponse = UpdateAliasResponse;
/* @internal */
export type strims_network_v1_UpdateAliasResponse = UpdateAliasResponse;
/* @internal */
export type strims_network_v1_IUpdateAliasResponse = IUpdateAliasResponse;
/* @internal */
export const strims_network_v1_GetUIConfigRequest = GetUIConfigRequest;
/* @internal */
export type strims_network_v1_GetUIConfigRequest = GetUIConfigRequest;
/* @internal */
export type strims_network_v1_IGetUIConfigRequest = IGetUIConfigRequest;
/* @internal */
export const strims_network_v1_GetUIConfigResponse = GetUIConfigResponse;
/* @internal */
export type strims_network_v1_GetUIConfigResponse = GetUIConfigResponse;
/* @internal */
export type strims_network_v1_IGetUIConfigResponse = IGetUIConfigResponse;
/* @internal */
export const strims_network_v1_NetworkEvent_NetworkStart = NetworkEvent.NetworkStart;
/* @internal */
export type strims_network_v1_NetworkEvent_NetworkStart = NetworkEvent.NetworkStart;
/* @internal */
export type strims_network_v1_NetworkEvent_INetworkStart = NetworkEvent.INetworkStart;
/* @internal */
export const strims_network_v1_NetworkEvent_NetworkStop = NetworkEvent.NetworkStop;
/* @internal */
export type strims_network_v1_NetworkEvent_NetworkStop = NetworkEvent.NetworkStop;
/* @internal */
export type strims_network_v1_NetworkEvent_INetworkStop = NetworkEvent.INetworkStop;
/* @internal */
export const strims_network_v1_NetworkEvent_NetworkPeerCountUpdate = NetworkEvent.NetworkPeerCountUpdate;
/* @internal */
export type strims_network_v1_NetworkEvent_NetworkPeerCountUpdate = NetworkEvent.NetworkPeerCountUpdate;
/* @internal */
export type strims_network_v1_NetworkEvent_INetworkPeerCountUpdate = NetworkEvent.INetworkPeerCountUpdate;

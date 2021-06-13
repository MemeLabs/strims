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
    if (m.data) w.uint32(10).bytes(m.data);
    if (m.type) w.uint32(18).string(m.type);
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

export type ICreateNetworkRequest = {
  name?: string;
  icon?: INetworkIcon;
}

export class CreateNetworkRequest {
  name: string;
  icon: NetworkIcon | undefined;

  constructor(v?: ICreateNetworkRequest) {
    this.name = v?.name || "";
    this.icon = v?.icon && new NetworkIcon(v.icon);
  }

  static encode(m: CreateNetworkRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.name) w.uint32(10).string(m.name);
    if (m.icon) NetworkIcon.encode(m.icon, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateNetworkRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreateNetworkRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.name = r.string();
        break;
        case 2:
        m.icon = NetworkIcon.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICreateNetworkResponse = {
  network?: INetwork;
}

export class CreateNetworkResponse {
  network: Network | undefined;

  constructor(v?: ICreateNetworkResponse) {
    this.network = v?.network && new Network(v.network);
  }

  static encode(m: CreateNetworkResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.network) Network.encode(m.network, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateNetworkResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreateNetworkResponse();
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

export type IUpdateNetworkRequest = {
  id?: bigint;
  name?: string;
}

export class UpdateNetworkRequest {
  id: bigint;
  name: string;

  constructor(v?: IUpdateNetworkRequest) {
    this.id = v?.id || BigInt(0);
    this.name = v?.name || "";
  }

  static encode(m: UpdateNetworkRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.name) w.uint32(18).string(m.name);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateNetworkRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UpdateNetworkRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
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

export type IUpdateNetworkResponse = {
  network?: INetwork;
}

export class UpdateNetworkResponse {
  network: Network | undefined;

  constructor(v?: IUpdateNetworkResponse) {
    this.network = v?.network && new Network(v.network);
  }

  static encode(m: UpdateNetworkResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.network) Network.encode(m.network, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateNetworkResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UpdateNetworkResponse();
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

export type IDeleteNetworkResponse = {
}

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

export type IListNetworksRequest = {
}

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

export type INetwork = {
  id?: bigint;
  name?: string;
  key?: strims_type_IKey;
  certificate?: strims_type_ICertificate;
  icon?: INetworkIcon;
  altProfileName?: string;
}

export class Network {
  id: bigint;
  name: string;
  key: strims_type_Key | undefined;
  certificate: strims_type_Certificate | undefined;
  icon: NetworkIcon | undefined;
  altProfileName: string;

  constructor(v?: INetwork) {
    this.id = v?.id || BigInt(0);
    this.name = v?.name || "";
    this.key = v?.key && new strims_type_Key(v.key);
    this.certificate = v?.certificate && new strims_type_Certificate(v.certificate);
    this.icon = v?.icon && new NetworkIcon(v.icon);
    this.altProfileName = v?.altProfileName || "";
  }

  static encode(m: Network, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.name) w.uint32(18).string(m.name);
    if (m.key) strims_type_Key.encode(m.key, w.uint32(26).fork()).ldelim();
    if (m.certificate) strims_type_Certificate.encode(m.certificate, w.uint32(34).fork()).ldelim();
    if (m.icon) NetworkIcon.encode(m.icon, w.uint32(42).fork()).ldelim();
    if (m.altProfileName) w.uint32(50).string(m.altProfileName);
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
        m.name = r.string();
        break;
        case 3:
        m.key = strims_type_Key.decode(r, r.uint32());
        break;
        case 4:
        m.certificate = strims_type_Certificate.decode(r, r.uint32());
        break;
        case 5:
        m.icon = NetworkIcon.decode(r, r.uint32());
        break;
        case 6:
        m.altProfileName = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICreateNetworkInvitationRequest = {
  signingKey?: strims_type_IKey;
  signingCert?: strims_type_ICertificate;
  networkName?: string;
}

export class CreateNetworkInvitationRequest {
  signingKey: strims_type_Key | undefined;
  signingCert: strims_type_Certificate | undefined;
  networkName: string;

  constructor(v?: ICreateNetworkInvitationRequest) {
    this.signingKey = v?.signingKey && new strims_type_Key(v.signingKey);
    this.signingCert = v?.signingCert && new strims_type_Certificate(v.signingCert);
    this.networkName = v?.networkName || "";
  }

  static encode(m: CreateNetworkInvitationRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.signingKey) strims_type_Key.encode(m.signingKey, w.uint32(10).fork()).ldelim();
    if (m.signingCert) strims_type_Certificate.encode(m.signingCert, w.uint32(18).fork()).ldelim();
    if (m.networkName) w.uint32(26).string(m.networkName);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateNetworkInvitationRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreateNetworkInvitationRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.signingKey = strims_type_Key.decode(r, r.uint32());
        break;
        case 2:
        m.signingCert = strims_type_Certificate.decode(r, r.uint32());
        break;
        case 3:
        m.networkName = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICreateNetworkInvitationResponse = {
  invitation?: IInvitation;
  invitationB64?: string;
  invitationBytes?: Uint8Array;
}

export class CreateNetworkInvitationResponse {
  invitation: Invitation | undefined;
  invitationB64: string;
  invitationBytes: Uint8Array;

  constructor(v?: ICreateNetworkInvitationResponse) {
    this.invitation = v?.invitation && new Invitation(v.invitation);
    this.invitationB64 = v?.invitationB64 || "";
    this.invitationBytes = v?.invitationBytes || new Uint8Array();
  }

  static encode(m: CreateNetworkInvitationResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.invitation) Invitation.encode(m.invitation, w.uint32(10).fork()).ldelim();
    if (m.invitationB64) w.uint32(18).string(m.invitationB64);
    if (m.invitationBytes) w.uint32(26).bytes(m.invitationBytes);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateNetworkInvitationResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreateNetworkInvitationResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.invitation = Invitation.decode(r, r.uint32());
        break;
        case 2:
        m.invitationB64 = r.string();
        break;
        case 3:
        m.invitationBytes = r.bytes();
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
    if (m.data) w.uint32(18).bytes(m.data);
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
}

export class InvitationV0 {
  key: strims_type_Key | undefined;
  certificate: strims_type_Certificate | undefined;
  networkName: string;

  constructor(v?: IInvitationV0) {
    this.key = v?.key && new strims_type_Key(v.key);
    this.certificate = v?.certificate && new strims_type_Certificate(v.certificate);
    this.networkName = v?.networkName || "";
  }

  static encode(m: InvitationV0, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.key) strims_type_Key.encode(m.key, w.uint32(10).fork()).ldelim();
    if (m.certificate) strims_type_Certificate.encode(m.certificate, w.uint32(18).fork()).ldelim();
    if (m.networkName) w.uint32(34).string(m.networkName);
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
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICreateNetworkFromInvitationRequest = {
  invitation?: CreateNetworkFromInvitationRequest.IInvitation
}

export class CreateNetworkFromInvitationRequest {
  invitation: CreateNetworkFromInvitationRequest.TInvitation;

  constructor(v?: ICreateNetworkFromInvitationRequest) {
    this.invitation = new CreateNetworkFromInvitationRequest.Invitation(v?.invitation);
  }

  static encode(m: CreateNetworkFromInvitationRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.invitation.case) {
      case CreateNetworkFromInvitationRequest.InvitationCase.INVITATION_B64:
      w.uint32(10).string(m.invitation.invitationB64);
      break;
      case CreateNetworkFromInvitationRequest.InvitationCase.INVITATION_BYTES:
      w.uint32(18).bytes(m.invitation.invitationBytes);
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
        m.invitation = new CreateNetworkFromInvitationRequest.Invitation({ invitationB64: r.string() });
        break;
        case 2:
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
    INVITATION_B64 = 1,
    INVITATION_BYTES = 2,
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
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.NETWORK_START, networkStart: NetworkEvent.INetworkStart }
  |{ case?: BodyCase.NETWORK_STOP, networkStop: NetworkEvent.INetworkStop }
  |{ case?: BodyCase.NETWORK_PEER_COUNT_UPDATE, networkPeerCountUpdate: NetworkEvent.INetworkPeerCountUpdate }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.NETWORK_START, networkStart: NetworkEvent.NetworkStart }
  |{ case: BodyCase.NETWORK_STOP, networkStop: NetworkEvent.NetworkStop }
  |{ case: BodyCase.NETWORK_PEER_COUNT_UPDATE, networkPeerCountUpdate: NetworkEvent.NetworkPeerCountUpdate }
  >;

  class BodyImpl {
    networkStart: NetworkEvent.NetworkStart;
    networkStop: NetworkEvent.NetworkStop;
    networkPeerCountUpdate: NetworkEvent.NetworkPeerCountUpdate;
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
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { networkStart: NetworkEvent.INetworkStart } ? { case: BodyCase.NETWORK_START, networkStart: NetworkEvent.NetworkStart } :
    T extends { networkStop: NetworkEvent.INetworkStop } ? { case: BodyCase.NETWORK_STOP, networkStop: NetworkEvent.NetworkStop } :
    T extends { networkPeerCountUpdate: NetworkEvent.INetworkPeerCountUpdate } ? { case: BodyCase.NETWORK_PEER_COUNT_UPDATE, networkPeerCountUpdate: NetworkEvent.NetworkPeerCountUpdate } :
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

export type IWatchNetworksRequest = {
}

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

export enum KeyUsage {
  KEY_USAGE_UNDEFINED = 0,
  KEY_USAGE_PEER = 1,
  KEY_USAGE_BOOTSTRAP = 2,
  KEY_USAGE_SIGN = 4,
  KEY_USAGE_BROKER = 8,
  KEY_USAGE_ENCIPHERMENT = 16,
}

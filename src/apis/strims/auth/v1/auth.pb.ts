// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  BootstrapClient as strims_network_v1_bootstrap_BootstrapClient,
  type IBootstrapClient as strims_network_v1_bootstrap_IBootstrapClient,
} from "../../network/v1/bootstrap/bootstrap.pb";
import {
  Network as strims_network_v1_Network,
  type INetwork as strims_network_v1_INetwork,
} from "../../network/v1/network.pb";
import {
  Device as strims_profile_v1_Device,
  type IDevice as strims_profile_v1_IDevice,
  Profile as strims_profile_v1_Profile,
  type IProfile as strims_profile_v1_IProfile,
  ProfileID as strims_profile_v1_ProfileID,
  type IProfileID as strims_profile_v1_IProfileID,
  StorageKey as strims_profile_v1_StorageKey,
  type IStorageKey as strims_profile_v1_IStorageKey,
} from "../../profile/v1/profile.pb";

export type ISessionThing = {
  profileId?: bigint;
  profileKey?: Uint8Array;
}

export class SessionThing {
  profileId: bigint;
  profileKey: Uint8Array;

  constructor(v?: ISessionThing) {
    this.profileId = v?.profileId || BigInt(0);
    this.profileKey = v?.profileKey || new Uint8Array();
  }

  static encode(m: SessionThing, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.profileId) w.uint32(8).uint64(m.profileId);
    if (m.profileKey.length) w.uint32(18).bytes(m.profileKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SessionThing {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new SessionThing();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.profileId = r.uint64();
        break;
        case 2:
        m.profileKey = r.bytes();
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

export type ITOTPConfig = {
  secret?: string;
  recoverCodes?: string[];
}

export class TOTPConfig {
  secret: string;
  recoverCodes: string[];

  constructor(v?: ITOTPConfig) {
    this.secret = v?.secret || "";
    this.recoverCodes = v?.recoverCodes ? v.recoverCodes : [];
  }

  static encode(m: TOTPConfig, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.secret.length) w.uint32(10).string(m.secret);
    for (const v of m.recoverCodes) w.uint32(18).string(v);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): TOTPConfig {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new TOTPConfig();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.secret = r.string();
        break;
        case 2:
        m.recoverCodes.push(r.string())
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

export type IServerUserThing = {
  id?: bigint;
  name?: string;
  credentials?: ServerUserThing_ICredentials
}

export class ServerUserThing {
  id: bigint;
  name: string;
  credentials: ServerUserThing_TCredentials;

  constructor(v?: IServerUserThing) {
    this.id = v?.id || BigInt(0);
    this.name = v?.name || "";
    this.credentials = v?.credentials ? new ServerUserThing_Credentials(v.credentials) : new ServerUserThing_Credentials();
  }

  static encode(m: ServerUserThing, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.name.length) w.uint32(18).string(m.name);
    switch (m.credentials.case) {
      case ServerUserThing_CredentialsCase.UNENCRYPTED:
      ServerUserThing_Unencrypted.encode(m.credentials.unencrypted, w.uint32(8010).fork()).ldelim();
      break;
      case ServerUserThing_CredentialsCase.PASSWORD:
      ServerUserThing_Password.encode(m.credentials.password, w.uint32(8018).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ServerUserThing {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ServerUserThing();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.name = r.string();
        break;
        case 1001:
        m.credentials = new ServerUserThing_Credentials({ unencrypted: ServerUserThing_Unencrypted.decode(r, r.uint32()) });
        break;
        case 1002:
        m.credentials = new ServerUserThing_Credentials({ password: ServerUserThing_Password.decode(r, r.uint32()) });
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

export enum ServerUserThing_CredentialsCase {
  NOT_SET = 0,
  UNENCRYPTED = 1001,
  PASSWORD = 1002,
}

export type ServerUserThing_ICredentials =
{ case?: ServerUserThing_CredentialsCase.NOT_SET }
|{ case?: ServerUserThing_CredentialsCase.UNENCRYPTED, unencrypted: ServerUserThing_IUnencrypted }
|{ case?: ServerUserThing_CredentialsCase.PASSWORD, password: ServerUserThing_IPassword }
;

export type ServerUserThing_TCredentials = Readonly<
{ case: ServerUserThing_CredentialsCase.NOT_SET }
|{ case: ServerUserThing_CredentialsCase.UNENCRYPTED, unencrypted: ServerUserThing_Unencrypted }
|{ case: ServerUserThing_CredentialsCase.PASSWORD, password: ServerUserThing_Password }
>;

class ServerUserThing_CredentialsImpl {
  declare unencrypted?: ServerUserThing_Unencrypted;
  declare password?: ServerUserThing_Password;
  case: ServerUserThing_CredentialsCase = ServerUserThing_CredentialsCase.NOT_SET;

  constructor(v?: ServerUserThing_ICredentials) {
    if (v && "unencrypted" in v) {
      this.case = ServerUserThing_CredentialsCase.UNENCRYPTED;
      this.unencrypted = new ServerUserThing_Unencrypted(v.unencrypted);
    } else
    if (v && "password" in v) {
      this.case = ServerUserThing_CredentialsCase.PASSWORD;
      this.password = new ServerUserThing_Password(v.password);
    }
  }
}

export const ServerUserThing_Credentials = ServerUserThing_CredentialsImpl as {
  new (): Readonly<{ case: ServerUserThing_CredentialsCase.NOT_SET }>;
  new <T extends ServerUserThing_ICredentials>(v: T): Readonly<
  T extends { unencrypted: ServerUserThing_IUnencrypted } ? { case: ServerUserThing_CredentialsCase.UNENCRYPTED, unencrypted: ServerUserThing_Unencrypted } :
  T extends { password: ServerUserThing_IPassword } ? { case: ServerUserThing_CredentialsCase.PASSWORD, password: ServerUserThing_Password } :
  never
  >;
};

export type ServerUserThing_IUnencrypted = {
  profileId?: bigint;
  profileKey?: Uint8Array;
}

export class ServerUserThing_Unencrypted {
  profileId: bigint;
  profileKey: Uint8Array;

  constructor(v?: ServerUserThing_IUnencrypted) {
    this.profileId = v?.profileId || BigInt(0);
    this.profileKey = v?.profileKey || new Uint8Array();
  }

  static encode(m: ServerUserThing_Unencrypted, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.profileId) w.uint32(8).uint64(m.profileId);
    if (m.profileKey.length) w.uint32(18).bytes(m.profileKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ServerUserThing_Unencrypted {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ServerUserThing_Unencrypted();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.profileId = r.uint64();
        break;
        case 2:
        m.profileKey = r.bytes();
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

export type ServerUserThing_IPassword = {
  authKey?: strims_profile_v1_IStorageKey;
  totpRequired?: boolean;
  secret?: Uint8Array;
}

export class ServerUserThing_Password {
  authKey: strims_profile_v1_StorageKey | undefined;
  totpRequired: boolean;
  secret: Uint8Array;

  constructor(v?: ServerUserThing_IPassword) {
    this.authKey = v?.authKey && new strims_profile_v1_StorageKey(v.authKey);
    this.totpRequired = v?.totpRequired || false;
    this.secret = v?.secret || new Uint8Array();
  }

  static encode(m: ServerUserThing_Password, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.authKey) strims_profile_v1_StorageKey.encode(m.authKey, w.uint32(10).fork()).ldelim();
    if (m.totpRequired) w.uint32(16).bool(m.totpRequired);
    if (m.secret.length) w.uint32(26).bytes(m.secret);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ServerUserThing_Password {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ServerUserThing_Password();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.authKey = strims_profile_v1_StorageKey.decode(r, r.uint32());
        break;
        case 2:
        m.totpRequired = r.bool();
        break;
        case 3:
        m.secret = r.bytes();
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

export type ServerUserThing_Password_ISecret = {
  profileId?: bigint;
  profileKey?: Uint8Array;
  totp?: ITOTPConfig;
}

export class ServerUserThing_Password_Secret {
  profileId: bigint;
  profileKey: Uint8Array;
  totp: TOTPConfig | undefined;

  constructor(v?: ServerUserThing_Password_ISecret) {
    this.profileId = v?.profileId || BigInt(0);
    this.profileKey = v?.profileKey || new Uint8Array();
    this.totp = v?.totp && new TOTPConfig(v.totp);
  }

  static encode(m: ServerUserThing_Password_Secret, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.profileId) w.uint32(8).uint64(m.profileId);
    if (m.profileKey.length) w.uint32(18).bytes(m.profileKey);
    if (m.totp) TOTPConfig.encode(m.totp, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ServerUserThing_Password_Secret {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ServerUserThing_Password_Secret();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.profileId = r.uint64();
        break;
        case 2:
        m.profileKey = r.bytes();
        break;
        case 3:
        m.totp = TOTPConfig.decode(r, r.uint32());
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

export type ILinkedProfile = {
  id?: bigint;
  name?: string;
  serverAddress?: string;
  credentials?: LinkedProfile_ICredentials
}

export class LinkedProfile {
  id: bigint;
  name: string;
  serverAddress: string;
  credentials: LinkedProfile_TCredentials;

  constructor(v?: ILinkedProfile) {
    this.id = v?.id || BigInt(0);
    this.name = v?.name || "";
    this.serverAddress = v?.serverAddress || "";
    this.credentials = v?.credentials ? new LinkedProfile_Credentials(v.credentials) : new LinkedProfile_Credentials();
  }

  static encode(m: LinkedProfile, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.name.length) w.uint32(18).string(m.name);
    if (m.serverAddress.length) w.uint32(26).string(m.serverAddress);
    switch (m.credentials.case) {
      case LinkedProfile_CredentialsCase.UNENCRYPTED:
      LinkedProfile_Unencrypted.encode(m.credentials.unencrypted, w.uint32(8010).fork()).ldelim();
      break;
      case LinkedProfile_CredentialsCase.PASSWORD:
      LinkedProfile_Password.encode(m.credentials.password, w.uint32(8018).fork()).ldelim();
      break;
      case LinkedProfile_CredentialsCase.TOKEN:
      LinkedProfile_Token.encode(m.credentials.token, w.uint32(8026).fork()).ldelim();
      break;
      case LinkedProfile_CredentialsCase.KEY:
      LinkedProfile_Key.encode(m.credentials.key, w.uint32(8034).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): LinkedProfile {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new LinkedProfile();
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
        m.serverAddress = r.string();
        break;
        case 1001:
        m.credentials = new LinkedProfile_Credentials({ unencrypted: LinkedProfile_Unencrypted.decode(r, r.uint32()) });
        break;
        case 1002:
        m.credentials = new LinkedProfile_Credentials({ password: LinkedProfile_Password.decode(r, r.uint32()) });
        break;
        case 1003:
        m.credentials = new LinkedProfile_Credentials({ token: LinkedProfile_Token.decode(r, r.uint32()) });
        break;
        case 1004:
        m.credentials = new LinkedProfile_Credentials({ key: LinkedProfile_Key.decode(r, r.uint32()) });
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

export enum LinkedProfile_CredentialsCase {
  NOT_SET = 0,
  UNENCRYPTED = 1001,
  PASSWORD = 1002,
  TOKEN = 1003,
  KEY = 1004,
}

export type LinkedProfile_ICredentials =
{ case?: LinkedProfile_CredentialsCase.NOT_SET }
|{ case?: LinkedProfile_CredentialsCase.UNENCRYPTED, unencrypted: LinkedProfile_IUnencrypted }
|{ case?: LinkedProfile_CredentialsCase.PASSWORD, password: LinkedProfile_IPassword }
|{ case?: LinkedProfile_CredentialsCase.TOKEN, token: LinkedProfile_IToken }
|{ case?: LinkedProfile_CredentialsCase.KEY, key: LinkedProfile_IKey }
;

export type LinkedProfile_TCredentials = Readonly<
{ case: LinkedProfile_CredentialsCase.NOT_SET }
|{ case: LinkedProfile_CredentialsCase.UNENCRYPTED, unencrypted: LinkedProfile_Unencrypted }
|{ case: LinkedProfile_CredentialsCase.PASSWORD, password: LinkedProfile_Password }
|{ case: LinkedProfile_CredentialsCase.TOKEN, token: LinkedProfile_Token }
|{ case: LinkedProfile_CredentialsCase.KEY, key: LinkedProfile_Key }
>;

class LinkedProfile_CredentialsImpl {
  declare unencrypted?: LinkedProfile_Unencrypted;
  declare password?: LinkedProfile_Password;
  declare token?: LinkedProfile_Token;
  declare key?: LinkedProfile_Key;
  case: LinkedProfile_CredentialsCase = LinkedProfile_CredentialsCase.NOT_SET;

  constructor(v?: LinkedProfile_ICredentials) {
    if (v && "unencrypted" in v) {
      this.case = LinkedProfile_CredentialsCase.UNENCRYPTED;
      this.unencrypted = new LinkedProfile_Unencrypted(v.unencrypted);
    } else
    if (v && "password" in v) {
      this.case = LinkedProfile_CredentialsCase.PASSWORD;
      this.password = new LinkedProfile_Password(v.password);
    } else
    if (v && "token" in v) {
      this.case = LinkedProfile_CredentialsCase.TOKEN;
      this.token = new LinkedProfile_Token(v.token);
    } else
    if (v && "key" in v) {
      this.case = LinkedProfile_CredentialsCase.KEY;
      this.key = new LinkedProfile_Key(v.key);
    }
  }
}

export const LinkedProfile_Credentials = LinkedProfile_CredentialsImpl as {
  new (): Readonly<{ case: LinkedProfile_CredentialsCase.NOT_SET }>;
  new <T extends LinkedProfile_ICredentials>(v: T): Readonly<
  T extends { unencrypted: LinkedProfile_IUnencrypted } ? { case: LinkedProfile_CredentialsCase.UNENCRYPTED, unencrypted: LinkedProfile_Unencrypted } :
  T extends { password: LinkedProfile_IPassword } ? { case: LinkedProfile_CredentialsCase.PASSWORD, password: LinkedProfile_Password } :
  T extends { token: LinkedProfile_IToken } ? { case: LinkedProfile_CredentialsCase.TOKEN, token: LinkedProfile_Token } :
  T extends { key: LinkedProfile_IKey } ? { case: LinkedProfile_CredentialsCase.KEY, key: LinkedProfile_Key } :
  never
  >;
};

export type LinkedProfile_IUnencrypted = object;

export class LinkedProfile_Unencrypted {

  constructor(v?: LinkedProfile_IUnencrypted) {
    void v; // an empty message has no fields to set
  }

  static encode(m: LinkedProfile_Unencrypted, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): LinkedProfile_Unencrypted {
    if (r instanceof Reader && length) r.skip(length);
    return new LinkedProfile_Unencrypted();
  }
}

export type LinkedProfile_IPassword = {
  totpRequired?: boolean;
}

export class LinkedProfile_Password {
  totpRequired: boolean;

  constructor(v?: LinkedProfile_IPassword) {
    this.totpRequired = v?.totpRequired || false;
  }

  static encode(m: LinkedProfile_Password, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.totpRequired) w.uint32(8).bool(m.totpRequired);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): LinkedProfile_Password {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new LinkedProfile_Password();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.totpRequired = r.bool();
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

export type LinkedProfile_IToken = {
  profileId?: bigint;
  token?: Uint8Array;
  eol?: bigint;
}

export class LinkedProfile_Token {
  profileId: bigint;
  token: Uint8Array;
  eol: bigint;

  constructor(v?: LinkedProfile_IToken) {
    this.profileId = v?.profileId || BigInt(0);
    this.token = v?.token || new Uint8Array();
    this.eol = v?.eol || BigInt(0);
  }

  static encode(m: LinkedProfile_Token, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.profileId) w.uint32(8).uint64(m.profileId);
    if (m.token.length) w.uint32(18).bytes(m.token);
    if (m.eol) w.uint32(24).uint64(m.eol);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): LinkedProfile_Token {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new LinkedProfile_Token();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.profileId = r.uint64();
        break;
        case 2:
        m.token = r.bytes();
        break;
        case 3:
        m.eol = r.uint64();
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

export type LinkedProfile_IKey = {
  profileId?: bigint;
  profileKey?: Uint8Array;
}

export class LinkedProfile_Key {
  profileId: bigint;
  profileKey: Uint8Array;

  constructor(v?: LinkedProfile_IKey) {
    this.profileId = v?.profileId || BigInt(0);
    this.profileKey = v?.profileKey || new Uint8Array();
  }

  static encode(m: LinkedProfile_Key, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.profileId) w.uint32(8).uint64(m.profileId);
    if (m.profileKey.length) w.uint32(18).bytes(m.profileKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): LinkedProfile_Key {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new LinkedProfile_Key();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.profileId = r.uint64();
        break;
        case 2:
        m.profileKey = r.bytes();
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

export type IPairingToken = {
  auth?: IServerUserThing;
  profile?: strims_profile_v1_IProfile;
  networks?: strims_network_v1_INetwork[];
  bootstraps?: strims_network_v1_bootstrap_IBootstrapClient[];
  devices?: strims_profile_v1_IDevice[];
  profileId?: strims_profile_v1_IProfileID;
}

export class PairingToken {
  auth: ServerUserThing | undefined;
  profile: strims_profile_v1_Profile | undefined;
  networks: strims_network_v1_Network[];
  bootstraps: strims_network_v1_bootstrap_BootstrapClient[];
  devices: strims_profile_v1_Device[];
  profileId: strims_profile_v1_ProfileID | undefined;

  constructor(v?: IPairingToken) {
    this.auth = v?.auth && new ServerUserThing(v.auth);
    this.profile = v?.profile && new strims_profile_v1_Profile(v.profile);
    this.networks = v?.networks ? v.networks.map(v => new strims_network_v1_Network(v)) : [];
    this.bootstraps = v?.bootstraps ? v.bootstraps.map(v => new strims_network_v1_bootstrap_BootstrapClient(v)) : [];
    this.devices = v?.devices ? v.devices.map(v => new strims_profile_v1_Device(v)) : [];
    this.profileId = v?.profileId && new strims_profile_v1_ProfileID(v.profileId);
  }

  static encode(m: PairingToken, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.auth) ServerUserThing.encode(m.auth, w.uint32(10).fork()).ldelim();
    if (m.profile) strims_profile_v1_Profile.encode(m.profile, w.uint32(18).fork()).ldelim();
    for (const v of m.networks) strims_network_v1_Network.encode(v, w.uint32(26).fork()).ldelim();
    for (const v of m.bootstraps) strims_network_v1_bootstrap_BootstrapClient.encode(v, w.uint32(34).fork()).ldelim();
    for (const v of m.devices) strims_profile_v1_Device.encode(v, w.uint32(42).fork()).ldelim();
    if (m.profileId) strims_profile_v1_ProfileID.encode(m.profileId, w.uint32(50).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PairingToken {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new PairingToken();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.auth = ServerUserThing.decode(r, r.uint32());
        break;
        case 2:
        m.profile = strims_profile_v1_Profile.decode(r, r.uint32());
        break;
        case 3:
        m.networks.push(strims_network_v1_Network.decode(r, r.uint32()));
        break;
        case 4:
        m.bootstraps.push(strims_network_v1_bootstrap_BootstrapClient.decode(r, r.uint32()));
        break;
        case 5:
        m.devices.push(strims_profile_v1_Device.decode(r, r.uint32()));
        break;
        case 6:
        m.profileId = strims_profile_v1_ProfileID.decode(r, r.uint32());
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

export type ISignInRequest = {
  credentials?: SignInRequest_ICredentials
}

export class SignInRequest {
  credentials: SignInRequest_TCredentials;

  constructor(v?: ISignInRequest) {
    this.credentials = v?.credentials ? new SignInRequest_Credentials(v.credentials) : new SignInRequest_Credentials();
  }

  static encode(m: SignInRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.credentials.case) {
      case SignInRequest_CredentialsCase.PASSWORD:
      SignInRequest_Password.encode(m.credentials.password, w.uint32(8010).fork()).ldelim();
      break;
      case SignInRequest_CredentialsCase.TOKEN:
      SignInRequest_Token.encode(m.credentials.token, w.uint32(8018).fork()).ldelim();
      break;
      case SignInRequest_CredentialsCase.KEY:
      SignInRequest_Key.encode(m.credentials.key, w.uint32(8026).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SignInRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new SignInRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1001:
        m.credentials = new SignInRequest_Credentials({ password: SignInRequest_Password.decode(r, r.uint32()) });
        break;
        case 1002:
        m.credentials = new SignInRequest_Credentials({ token: SignInRequest_Token.decode(r, r.uint32()) });
        break;
        case 1003:
        m.credentials = new SignInRequest_Credentials({ key: SignInRequest_Key.decode(r, r.uint32()) });
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

export enum SignInRequest_CredentialsCase {
  NOT_SET = 0,
  PASSWORD = 1001,
  TOKEN = 1002,
  KEY = 1003,
}

export type SignInRequest_ICredentials =
{ case?: SignInRequest_CredentialsCase.NOT_SET }
|{ case?: SignInRequest_CredentialsCase.PASSWORD, password: SignInRequest_IPassword }
|{ case?: SignInRequest_CredentialsCase.TOKEN, token: SignInRequest_IToken }
|{ case?: SignInRequest_CredentialsCase.KEY, key: SignInRequest_IKey }
;

export type SignInRequest_TCredentials = Readonly<
{ case: SignInRequest_CredentialsCase.NOT_SET }
|{ case: SignInRequest_CredentialsCase.PASSWORD, password: SignInRequest_Password }
|{ case: SignInRequest_CredentialsCase.TOKEN, token: SignInRequest_Token }
|{ case: SignInRequest_CredentialsCase.KEY, key: SignInRequest_Key }
>;

class SignInRequest_CredentialsImpl {
  declare password?: SignInRequest_Password;
  declare token?: SignInRequest_Token;
  declare key?: SignInRequest_Key;
  case: SignInRequest_CredentialsCase = SignInRequest_CredentialsCase.NOT_SET;

  constructor(v?: SignInRequest_ICredentials) {
    if (v && "password" in v) {
      this.case = SignInRequest_CredentialsCase.PASSWORD;
      this.password = new SignInRequest_Password(v.password);
    } else
    if (v && "token" in v) {
      this.case = SignInRequest_CredentialsCase.TOKEN;
      this.token = new SignInRequest_Token(v.token);
    } else
    if (v && "key" in v) {
      this.case = SignInRequest_CredentialsCase.KEY;
      this.key = new SignInRequest_Key(v.key);
    }
  }
}

export const SignInRequest_Credentials = SignInRequest_CredentialsImpl as {
  new (): Readonly<{ case: SignInRequest_CredentialsCase.NOT_SET }>;
  new <T extends SignInRequest_ICredentials>(v: T): Readonly<
  T extends { password: SignInRequest_IPassword } ? { case: SignInRequest_CredentialsCase.PASSWORD, password: SignInRequest_Password } :
  T extends { token: SignInRequest_IToken } ? { case: SignInRequest_CredentialsCase.TOKEN, token: SignInRequest_Token } :
  T extends { key: SignInRequest_IKey } ? { case: SignInRequest_CredentialsCase.KEY, key: SignInRequest_Key } :
  never
  >;
};

export type SignInRequest_IPassword = {
  name?: string;
  password?: string;
  totpPasscode?: string;
  persistSession?: boolean;
  persistLogin?: boolean;
  pairingToken?: IPairingToken;
}

export class SignInRequest_Password {
  name: string;
  password: string;
  totpPasscode: string;
  persistSession: boolean;
  persistLogin: boolean;
  pairingToken: PairingToken | undefined;

  constructor(v?: SignInRequest_IPassword) {
    this.name = v?.name || "";
    this.password = v?.password || "";
    this.totpPasscode = v?.totpPasscode || "";
    this.persistSession = v?.persistSession || false;
    this.persistLogin = v?.persistLogin || false;
    this.pairingToken = v?.pairingToken && new PairingToken(v.pairingToken);
  }

  static encode(m: SignInRequest_Password, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.name.length) w.uint32(10).string(m.name);
    if (m.password.length) w.uint32(18).string(m.password);
    if (m.totpPasscode.length) w.uint32(26).string(m.totpPasscode);
    if (m.persistSession) w.uint32(32).bool(m.persistSession);
    if (m.persistLogin) w.uint32(40).bool(m.persistLogin);
    if (m.pairingToken) PairingToken.encode(m.pairingToken, w.uint32(50).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SignInRequest_Password {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new SignInRequest_Password();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.name = r.string();
        break;
        case 2:
        m.password = r.string();
        break;
        case 3:
        m.totpPasscode = r.string();
        break;
        case 4:
        m.persistSession = r.bool();
        break;
        case 5:
        m.persistLogin = r.bool();
        break;
        case 6:
        m.pairingToken = PairingToken.decode(r, r.uint32());
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

export type SignInRequest_IToken = {
  profileId?: bigint;
  token?: Uint8Array;
  eol?: bigint;
}

export class SignInRequest_Token {
  profileId: bigint;
  token: Uint8Array;
  eol: bigint;

  constructor(v?: SignInRequest_IToken) {
    this.profileId = v?.profileId || BigInt(0);
    this.token = v?.token || new Uint8Array();
    this.eol = v?.eol || BigInt(0);
  }

  static encode(m: SignInRequest_Token, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.profileId) w.uint32(8).uint64(m.profileId);
    if (m.token.length) w.uint32(18).bytes(m.token);
    if (m.eol) w.uint32(24).uint64(m.eol);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SignInRequest_Token {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new SignInRequest_Token();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.profileId = r.uint64();
        break;
        case 2:
        m.token = r.bytes();
        break;
        case 3:
        m.eol = r.uint64();
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

export type SignInRequest_IKey = {
  profileId?: bigint;
  profileKey?: Uint8Array;
}

export class SignInRequest_Key {
  profileId: bigint;
  profileKey: Uint8Array;

  constructor(v?: SignInRequest_IKey) {
    this.profileId = v?.profileId || BigInt(0);
    this.profileKey = v?.profileKey || new Uint8Array();
  }

  static encode(m: SignInRequest_Key, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.profileId) w.uint32(8).uint64(m.profileId);
    if (m.profileKey.length) w.uint32(18).bytes(m.profileKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SignInRequest_Key {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new SignInRequest_Key();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.profileId = r.uint64();
        break;
        case 2:
        m.profileKey = r.bytes();
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

export type ISignInResponse = {
  linkedProfile?: ILinkedProfile;
  profile?: strims_profile_v1_IProfile;
}

export class SignInResponse {
  linkedProfile: LinkedProfile | undefined;
  profile: strims_profile_v1_Profile | undefined;

  constructor(v?: ISignInResponse) {
    this.linkedProfile = v?.linkedProfile && new LinkedProfile(v.linkedProfile);
    this.profile = v?.profile && new strims_profile_v1_Profile(v.profile);
  }

  static encode(m: SignInResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.linkedProfile) LinkedProfile.encode(m.linkedProfile, w.uint32(10).fork()).ldelim();
    if (m.profile) strims_profile_v1_Profile.encode(m.profile, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SignInResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new SignInResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.linkedProfile = LinkedProfile.decode(r, r.uint32());
        break;
        case 2:
        m.profile = strims_profile_v1_Profile.decode(r, r.uint32());
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

export type ISignUpRequest = {
  name?: string;
  password?: string;
  persistSession?: boolean;
  persistLogin?: boolean;
}

export class SignUpRequest {
  name: string;
  password: string;
  persistSession: boolean;
  persistLogin: boolean;

  constructor(v?: ISignUpRequest) {
    this.name = v?.name || "";
    this.password = v?.password || "";
    this.persistSession = v?.persistSession || false;
    this.persistLogin = v?.persistLogin || false;
  }

  static encode(m: SignUpRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.name.length) w.uint32(10).string(m.name);
    if (m.password.length) w.uint32(18).string(m.password);
    if (m.persistSession) w.uint32(32).bool(m.persistSession);
    if (m.persistLogin) w.uint32(40).bool(m.persistLogin);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SignUpRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new SignUpRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.name = r.string();
        break;
        case 2:
        m.password = r.string();
        break;
        case 4:
        m.persistSession = r.bool();
        break;
        case 5:
        m.persistLogin = r.bool();
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

export type ISignUpResponse = {
  linkedProfile?: ILinkedProfile;
  profile?: strims_profile_v1_IProfile;
}

export class SignUpResponse {
  linkedProfile: LinkedProfile | undefined;
  profile: strims_profile_v1_Profile | undefined;

  constructor(v?: ISignUpResponse) {
    this.linkedProfile = v?.linkedProfile && new LinkedProfile(v.linkedProfile);
    this.profile = v?.profile && new strims_profile_v1_Profile(v.profile);
  }

  static encode(m: SignUpResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.linkedProfile) LinkedProfile.encode(m.linkedProfile, w.uint32(10).fork()).ldelim();
    if (m.profile) strims_profile_v1_Profile.encode(m.profile, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SignUpResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new SignUpResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.linkedProfile = LinkedProfile.decode(r, r.uint32());
        break;
        case 2:
        m.profile = strims_profile_v1_Profile.decode(r, r.uint32());
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


import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  strims_network_v1_bootstrap_BootstrapClient,
  strims_network_v1_bootstrap_IBootstrapClient,
} from "../../network/v1/bootstrap/bootstrap";
import {
  strims_network_v1_Network,
  strims_network_v1_INetwork,
} from "../../network/v1/network";
import {
  strims_profile_v1_Device,
  strims_profile_v1_IDevice,
  strims_profile_v1_Profile,
  strims_profile_v1_IProfile,
  strims_profile_v1_ProfileID,
  strims_profile_v1_IProfileID,
  strims_profile_v1_StorageKey,
  strims_profile_v1_IStorageKey,
} from "../../profile/v1/profile";

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
    const end = length === undefined ? r.len : r.pos + length;
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
        r.skipType(tag & 7);
        break;
      }
    }
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
    const end = length === undefined ? r.len : r.pos + length;
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
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IServerUserThing = {
  id?: bigint;
  name?: string;
  credentials?: ServerUserThing.ICredentials
}

export class ServerUserThing {
  id: bigint;
  name: string;
  credentials: ServerUserThing.TCredentials;

  constructor(v?: IServerUserThing) {
    this.id = v?.id || BigInt(0);
    this.name = v?.name || "";
    this.credentials = new ServerUserThing.Credentials(v?.credentials);
  }

  static encode(m: ServerUserThing, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.name.length) w.uint32(18).string(m.name);
    switch (m.credentials.case) {
      case ServerUserThing.CredentialsCase.UNENCRYPTED:
      strims_auth_v1_ServerUserThing_Unencrypted.encode(m.credentials.unencrypted, w.uint32(8010).fork()).ldelim();
      break;
      case ServerUserThing.CredentialsCase.PASSWORD:
      strims_auth_v1_ServerUserThing_Password.encode(m.credentials.password, w.uint32(8018).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ServerUserThing {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
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
        m.credentials = new ServerUserThing.Credentials({ unencrypted: strims_auth_v1_ServerUserThing_Unencrypted.decode(r, r.uint32()) });
        break;
        case 1002:
        m.credentials = new ServerUserThing.Credentials({ password: strims_auth_v1_ServerUserThing_Password.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace ServerUserThing {
  export enum CredentialsCase {
    NOT_SET = 0,
    UNENCRYPTED = 1001,
    PASSWORD = 1002,
  }

  export type ICredentials =
  { case?: CredentialsCase.NOT_SET }
  |{ case?: CredentialsCase.UNENCRYPTED, unencrypted: strims_auth_v1_ServerUserThing_IUnencrypted }
  |{ case?: CredentialsCase.PASSWORD, password: strims_auth_v1_ServerUserThing_IPassword }
  ;

  export type TCredentials = Readonly<
  { case: CredentialsCase.NOT_SET }
  |{ case: CredentialsCase.UNENCRYPTED, unencrypted: strims_auth_v1_ServerUserThing_Unencrypted }
  |{ case: CredentialsCase.PASSWORD, password: strims_auth_v1_ServerUserThing_Password }
  >;

  class CredentialsImpl {
    unencrypted: strims_auth_v1_ServerUserThing_Unencrypted;
    password: strims_auth_v1_ServerUserThing_Password;
    case: CredentialsCase = CredentialsCase.NOT_SET;

    constructor(v?: ICredentials) {
      if (v && "unencrypted" in v) {
        this.case = CredentialsCase.UNENCRYPTED;
        this.unencrypted = new strims_auth_v1_ServerUserThing_Unencrypted(v.unencrypted);
      } else
      if (v && "password" in v) {
        this.case = CredentialsCase.PASSWORD;
        this.password = new strims_auth_v1_ServerUserThing_Password(v.password);
      }
    }
  }

  export const Credentials = CredentialsImpl as {
    new (): Readonly<{ case: CredentialsCase.NOT_SET }>;
    new <T extends ICredentials>(v: T): Readonly<
    T extends { unencrypted: strims_auth_v1_ServerUserThing_IUnencrypted } ? { case: CredentialsCase.UNENCRYPTED, unencrypted: strims_auth_v1_ServerUserThing_Unencrypted } :
    T extends { password: strims_auth_v1_ServerUserThing_IPassword } ? { case: CredentialsCase.PASSWORD, password: strims_auth_v1_ServerUserThing_Password } :
    never
    >;
  };

  export type IUnencrypted = {
    profileId?: bigint;
    profileKey?: Uint8Array;
  }

  export class Unencrypted {
    profileId: bigint;
    profileKey: Uint8Array;

    constructor(v?: IUnencrypted) {
      this.profileId = v?.profileId || BigInt(0);
      this.profileKey = v?.profileKey || new Uint8Array();
    }

    static encode(m: Unencrypted, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.profileId) w.uint32(8).uint64(m.profileId);
      if (m.profileKey.length) w.uint32(18).bytes(m.profileKey);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Unencrypted {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Unencrypted();
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
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type IPassword = {
    authKey?: strims_profile_v1_IStorageKey;
    totpRequired?: boolean;
    secret?: Uint8Array;
  }

  export class Password {
    authKey: strims_profile_v1_StorageKey | undefined;
    totpRequired: boolean;
    secret: Uint8Array;

    constructor(v?: IPassword) {
      this.authKey = v?.authKey && new strims_profile_v1_StorageKey(v.authKey);
      this.totpRequired = v?.totpRequired || false;
      this.secret = v?.secret || new Uint8Array();
    }

    static encode(m: Password, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.authKey) strims_profile_v1_StorageKey.encode(m.authKey, w.uint32(10).fork()).ldelim();
      if (m.totpRequired) w.uint32(16).bool(m.totpRequired);
      if (m.secret.length) w.uint32(26).bytes(m.secret);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Password {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Password();
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
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export namespace Password {
    export type ISecret = {
      profileId?: bigint;
      profileKey?: Uint8Array;
      totp?: strims_auth_v1_ITOTPConfig;
    }

    export class Secret {
      profileId: bigint;
      profileKey: Uint8Array;
      totp: strims_auth_v1_TOTPConfig | undefined;

      constructor(v?: ISecret) {
        this.profileId = v?.profileId || BigInt(0);
        this.profileKey = v?.profileKey || new Uint8Array();
        this.totp = v?.totp && new strims_auth_v1_TOTPConfig(v.totp);
      }

      static encode(m: Secret, w?: Writer): Writer {
        if (!w) w = new Writer();
        if (m.profileId) w.uint32(8).uint64(m.profileId);
        if (m.profileKey.length) w.uint32(18).bytes(m.profileKey);
        if (m.totp) strims_auth_v1_TOTPConfig.encode(m.totp, w.uint32(26).fork()).ldelim();
        return w;
      }

      static decode(r: Reader | Uint8Array, length?: number): Secret {
        r = r instanceof Reader ? r : new Reader(r);
        const end = length === undefined ? r.len : r.pos + length;
        const m = new Secret();
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
            m.totp = strims_auth_v1_TOTPConfig.decode(r, r.uint32());
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

}

export type ILinkedProfile = {
  id?: bigint;
  name?: string;
  serverAddress?: string;
  credentials?: LinkedProfile.ICredentials
}

export class LinkedProfile {
  id: bigint;
  name: string;
  serverAddress: string;
  credentials: LinkedProfile.TCredentials;

  constructor(v?: ILinkedProfile) {
    this.id = v?.id || BigInt(0);
    this.name = v?.name || "";
    this.serverAddress = v?.serverAddress || "";
    this.credentials = new LinkedProfile.Credentials(v?.credentials);
  }

  static encode(m: LinkedProfile, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.name.length) w.uint32(18).string(m.name);
    if (m.serverAddress.length) w.uint32(26).string(m.serverAddress);
    switch (m.credentials.case) {
      case LinkedProfile.CredentialsCase.UNENCRYPTED:
      strims_auth_v1_LinkedProfile_Unencrypted.encode(m.credentials.unencrypted, w.uint32(8010).fork()).ldelim();
      break;
      case LinkedProfile.CredentialsCase.PASSWORD:
      strims_auth_v1_LinkedProfile_Password.encode(m.credentials.password, w.uint32(8018).fork()).ldelim();
      break;
      case LinkedProfile.CredentialsCase.TOKEN:
      strims_auth_v1_LinkedProfile_Token.encode(m.credentials.token, w.uint32(8026).fork()).ldelim();
      break;
      case LinkedProfile.CredentialsCase.KEY:
      strims_auth_v1_LinkedProfile_Key.encode(m.credentials.key, w.uint32(8034).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): LinkedProfile {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
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
        m.credentials = new LinkedProfile.Credentials({ unencrypted: strims_auth_v1_LinkedProfile_Unencrypted.decode(r, r.uint32()) });
        break;
        case 1002:
        m.credentials = new LinkedProfile.Credentials({ password: strims_auth_v1_LinkedProfile_Password.decode(r, r.uint32()) });
        break;
        case 1003:
        m.credentials = new LinkedProfile.Credentials({ token: strims_auth_v1_LinkedProfile_Token.decode(r, r.uint32()) });
        break;
        case 1004:
        m.credentials = new LinkedProfile.Credentials({ key: strims_auth_v1_LinkedProfile_Key.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace LinkedProfile {
  export enum CredentialsCase {
    NOT_SET = 0,
    UNENCRYPTED = 1001,
    PASSWORD = 1002,
    TOKEN = 1003,
    KEY = 1004,
  }

  export type ICredentials =
  { case?: CredentialsCase.NOT_SET }
  |{ case?: CredentialsCase.UNENCRYPTED, unencrypted: strims_auth_v1_LinkedProfile_IUnencrypted }
  |{ case?: CredentialsCase.PASSWORD, password: strims_auth_v1_LinkedProfile_IPassword }
  |{ case?: CredentialsCase.TOKEN, token: strims_auth_v1_LinkedProfile_IToken }
  |{ case?: CredentialsCase.KEY, key: strims_auth_v1_LinkedProfile_IKey }
  ;

  export type TCredentials = Readonly<
  { case: CredentialsCase.NOT_SET }
  |{ case: CredentialsCase.UNENCRYPTED, unencrypted: strims_auth_v1_LinkedProfile_Unencrypted }
  |{ case: CredentialsCase.PASSWORD, password: strims_auth_v1_LinkedProfile_Password }
  |{ case: CredentialsCase.TOKEN, token: strims_auth_v1_LinkedProfile_Token }
  |{ case: CredentialsCase.KEY, key: strims_auth_v1_LinkedProfile_Key }
  >;

  class CredentialsImpl {
    unencrypted: strims_auth_v1_LinkedProfile_Unencrypted;
    password: strims_auth_v1_LinkedProfile_Password;
    token: strims_auth_v1_LinkedProfile_Token;
    key: strims_auth_v1_LinkedProfile_Key;
    case: CredentialsCase = CredentialsCase.NOT_SET;

    constructor(v?: ICredentials) {
      if (v && "unencrypted" in v) {
        this.case = CredentialsCase.UNENCRYPTED;
        this.unencrypted = new strims_auth_v1_LinkedProfile_Unencrypted(v.unencrypted);
      } else
      if (v && "password" in v) {
        this.case = CredentialsCase.PASSWORD;
        this.password = new strims_auth_v1_LinkedProfile_Password(v.password);
      } else
      if (v && "token" in v) {
        this.case = CredentialsCase.TOKEN;
        this.token = new strims_auth_v1_LinkedProfile_Token(v.token);
      } else
      if (v && "key" in v) {
        this.case = CredentialsCase.KEY;
        this.key = new strims_auth_v1_LinkedProfile_Key(v.key);
      }
    }
  }

  export const Credentials = CredentialsImpl as {
    new (): Readonly<{ case: CredentialsCase.NOT_SET }>;
    new <T extends ICredentials>(v: T): Readonly<
    T extends { unencrypted: strims_auth_v1_LinkedProfile_IUnencrypted } ? { case: CredentialsCase.UNENCRYPTED, unencrypted: strims_auth_v1_LinkedProfile_Unencrypted } :
    T extends { password: strims_auth_v1_LinkedProfile_IPassword } ? { case: CredentialsCase.PASSWORD, password: strims_auth_v1_LinkedProfile_Password } :
    T extends { token: strims_auth_v1_LinkedProfile_IToken } ? { case: CredentialsCase.TOKEN, token: strims_auth_v1_LinkedProfile_Token } :
    T extends { key: strims_auth_v1_LinkedProfile_IKey } ? { case: CredentialsCase.KEY, key: strims_auth_v1_LinkedProfile_Key } :
    never
    >;
  };

  export type IUnencrypted = Record<string, any>;

  export class Unencrypted {

    // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
    constructor(v?: IUnencrypted) {
    }

    static encode(m: Unencrypted, w?: Writer): Writer {
      if (!w) w = new Writer();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Unencrypted {
      if (r instanceof Reader && length) r.skip(length);
      return new Unencrypted();
    }
  }

  export type IPassword = {
    totpRequired?: boolean;
  }

  export class Password {
    totpRequired: boolean;

    constructor(v?: IPassword) {
      this.totpRequired = v?.totpRequired || false;
    }

    static encode(m: Password, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.totpRequired) w.uint32(8).bool(m.totpRequired);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Password {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Password();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.totpRequired = r.bool();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type IToken = {
    profileId?: bigint;
    token?: Uint8Array;
    eol?: bigint;
  }

  export class Token {
    profileId: bigint;
    token: Uint8Array;
    eol: bigint;

    constructor(v?: IToken) {
      this.profileId = v?.profileId || BigInt(0);
      this.token = v?.token || new Uint8Array();
      this.eol = v?.eol || BigInt(0);
    }

    static encode(m: Token, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.profileId) w.uint32(8).uint64(m.profileId);
      if (m.token.length) w.uint32(18).bytes(m.token);
      if (m.eol) w.uint32(24).uint64(m.eol);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Token {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Token();
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
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type IKey = {
    profileId?: bigint;
    profileKey?: Uint8Array;
  }

  export class Key {
    profileId: bigint;
    profileKey: Uint8Array;

    constructor(v?: IKey) {
      this.profileId = v?.profileId || BigInt(0);
      this.profileKey = v?.profileKey || new Uint8Array();
    }

    static encode(m: Key, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.profileId) w.uint32(8).uint64(m.profileId);
      if (m.profileKey.length) w.uint32(18).bytes(m.profileKey);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Key {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Key();
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
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

}

export type IPairingToken = {
  auth?: strims_auth_v1_IServerUserThing;
  profile?: strims_profile_v1_IProfile;
  networks?: strims_network_v1_INetwork[];
  bootstraps?: strims_network_v1_bootstrap_IBootstrapClient[];
  devices?: strims_profile_v1_IDevice[];
  profileId?: strims_profile_v1_IProfileID;
}

export class PairingToken {
  auth: strims_auth_v1_ServerUserThing | undefined;
  profile: strims_profile_v1_Profile | undefined;
  networks: strims_network_v1_Network[];
  bootstraps: strims_network_v1_bootstrap_BootstrapClient[];
  devices: strims_profile_v1_Device[];
  profileId: strims_profile_v1_ProfileID | undefined;

  constructor(v?: IPairingToken) {
    this.auth = v?.auth && new strims_auth_v1_ServerUserThing(v.auth);
    this.profile = v?.profile && new strims_profile_v1_Profile(v.profile);
    this.networks = v?.networks ? v.networks.map(v => new strims_network_v1_Network(v)) : [];
    this.bootstraps = v?.bootstraps ? v.bootstraps.map(v => new strims_network_v1_bootstrap_BootstrapClient(v)) : [];
    this.devices = v?.devices ? v.devices.map(v => new strims_profile_v1_Device(v)) : [];
    this.profileId = v?.profileId && new strims_profile_v1_ProfileID(v.profileId);
  }

  static encode(m: PairingToken, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.auth) strims_auth_v1_ServerUserThing.encode(m.auth, w.uint32(10).fork()).ldelim();
    if (m.profile) strims_profile_v1_Profile.encode(m.profile, w.uint32(18).fork()).ldelim();
    for (const v of m.networks) strims_network_v1_Network.encode(v, w.uint32(26).fork()).ldelim();
    for (const v of m.bootstraps) strims_network_v1_bootstrap_BootstrapClient.encode(v, w.uint32(34).fork()).ldelim();
    for (const v of m.devices) strims_profile_v1_Device.encode(v, w.uint32(42).fork()).ldelim();
    if (m.profileId) strims_profile_v1_ProfileID.encode(m.profileId, w.uint32(50).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PairingToken {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new PairingToken();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.auth = strims_auth_v1_ServerUserThing.decode(r, r.uint32());
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
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ISignInRequest = {
  credentials?: SignInRequest.ICredentials
}

export class SignInRequest {
  credentials: SignInRequest.TCredentials;

  constructor(v?: ISignInRequest) {
    this.credentials = new SignInRequest.Credentials(v?.credentials);
  }

  static encode(m: SignInRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.credentials.case) {
      case SignInRequest.CredentialsCase.PASSWORD:
      strims_auth_v1_SignInRequest_Password.encode(m.credentials.password, w.uint32(8010).fork()).ldelim();
      break;
      case SignInRequest.CredentialsCase.TOKEN:
      strims_auth_v1_SignInRequest_Token.encode(m.credentials.token, w.uint32(8018).fork()).ldelim();
      break;
      case SignInRequest.CredentialsCase.KEY:
      strims_auth_v1_SignInRequest_Key.encode(m.credentials.key, w.uint32(8026).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SignInRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new SignInRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1001:
        m.credentials = new SignInRequest.Credentials({ password: strims_auth_v1_SignInRequest_Password.decode(r, r.uint32()) });
        break;
        case 1002:
        m.credentials = new SignInRequest.Credentials({ token: strims_auth_v1_SignInRequest_Token.decode(r, r.uint32()) });
        break;
        case 1003:
        m.credentials = new SignInRequest.Credentials({ key: strims_auth_v1_SignInRequest_Key.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace SignInRequest {
  export enum CredentialsCase {
    NOT_SET = 0,
    PASSWORD = 1001,
    TOKEN = 1002,
    KEY = 1003,
  }

  export type ICredentials =
  { case?: CredentialsCase.NOT_SET }
  |{ case?: CredentialsCase.PASSWORD, password: strims_auth_v1_SignInRequest_IPassword }
  |{ case?: CredentialsCase.TOKEN, token: strims_auth_v1_SignInRequest_IToken }
  |{ case?: CredentialsCase.KEY, key: strims_auth_v1_SignInRequest_IKey }
  ;

  export type TCredentials = Readonly<
  { case: CredentialsCase.NOT_SET }
  |{ case: CredentialsCase.PASSWORD, password: strims_auth_v1_SignInRequest_Password }
  |{ case: CredentialsCase.TOKEN, token: strims_auth_v1_SignInRequest_Token }
  |{ case: CredentialsCase.KEY, key: strims_auth_v1_SignInRequest_Key }
  >;

  class CredentialsImpl {
    password: strims_auth_v1_SignInRequest_Password;
    token: strims_auth_v1_SignInRequest_Token;
    key: strims_auth_v1_SignInRequest_Key;
    case: CredentialsCase = CredentialsCase.NOT_SET;

    constructor(v?: ICredentials) {
      if (v && "password" in v) {
        this.case = CredentialsCase.PASSWORD;
        this.password = new strims_auth_v1_SignInRequest_Password(v.password);
      } else
      if (v && "token" in v) {
        this.case = CredentialsCase.TOKEN;
        this.token = new strims_auth_v1_SignInRequest_Token(v.token);
      } else
      if (v && "key" in v) {
        this.case = CredentialsCase.KEY;
        this.key = new strims_auth_v1_SignInRequest_Key(v.key);
      }
    }
  }

  export const Credentials = CredentialsImpl as {
    new (): Readonly<{ case: CredentialsCase.NOT_SET }>;
    new <T extends ICredentials>(v: T): Readonly<
    T extends { password: strims_auth_v1_SignInRequest_IPassword } ? { case: CredentialsCase.PASSWORD, password: strims_auth_v1_SignInRequest_Password } :
    T extends { token: strims_auth_v1_SignInRequest_IToken } ? { case: CredentialsCase.TOKEN, token: strims_auth_v1_SignInRequest_Token } :
    T extends { key: strims_auth_v1_SignInRequest_IKey } ? { case: CredentialsCase.KEY, key: strims_auth_v1_SignInRequest_Key } :
    never
    >;
  };

  export type IPassword = {
    name?: string;
    password?: string;
    totpPasscode?: string;
    persistSession?: boolean;
    persistLogin?: boolean;
    pairingToken?: strims_auth_v1_IPairingToken;
  }

  export class Password {
    name: string;
    password: string;
    totpPasscode: string;
    persistSession: boolean;
    persistLogin: boolean;
    pairingToken: strims_auth_v1_PairingToken | undefined;

    constructor(v?: IPassword) {
      this.name = v?.name || "";
      this.password = v?.password || "";
      this.totpPasscode = v?.totpPasscode || "";
      this.persistSession = v?.persistSession || false;
      this.persistLogin = v?.persistLogin || false;
      this.pairingToken = v?.pairingToken && new strims_auth_v1_PairingToken(v.pairingToken);
    }

    static encode(m: Password, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.name.length) w.uint32(10).string(m.name);
      if (m.password.length) w.uint32(18).string(m.password);
      if (m.totpPasscode.length) w.uint32(26).string(m.totpPasscode);
      if (m.persistSession) w.uint32(32).bool(m.persistSession);
      if (m.persistLogin) w.uint32(40).bool(m.persistLogin);
      if (m.pairingToken) strims_auth_v1_PairingToken.encode(m.pairingToken, w.uint32(50).fork()).ldelim();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Password {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Password();
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
          m.pairingToken = strims_auth_v1_PairingToken.decode(r, r.uint32());
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type IToken = {
    profileId?: bigint;
    token?: Uint8Array;
    eol?: bigint;
  }

  export class Token {
    profileId: bigint;
    token: Uint8Array;
    eol: bigint;

    constructor(v?: IToken) {
      this.profileId = v?.profileId || BigInt(0);
      this.token = v?.token || new Uint8Array();
      this.eol = v?.eol || BigInt(0);
    }

    static encode(m: Token, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.profileId) w.uint32(8).uint64(m.profileId);
      if (m.token.length) w.uint32(18).bytes(m.token);
      if (m.eol) w.uint32(24).uint64(m.eol);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Token {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Token();
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
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type IKey = {
    profileId?: bigint;
    profileKey?: Uint8Array;
  }

  export class Key {
    profileId: bigint;
    profileKey: Uint8Array;

    constructor(v?: IKey) {
      this.profileId = v?.profileId || BigInt(0);
      this.profileKey = v?.profileKey || new Uint8Array();
    }

    static encode(m: Key, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.profileId) w.uint32(8).uint64(m.profileId);
      if (m.profileKey.length) w.uint32(18).bytes(m.profileKey);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Key {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Key();
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
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

}

export type ISignInResponse = {
  linkedProfile?: strims_auth_v1_ILinkedProfile;
  profile?: strims_profile_v1_IProfile;
}

export class SignInResponse {
  linkedProfile: strims_auth_v1_LinkedProfile | undefined;
  profile: strims_profile_v1_Profile | undefined;

  constructor(v?: ISignInResponse) {
    this.linkedProfile = v?.linkedProfile && new strims_auth_v1_LinkedProfile(v.linkedProfile);
    this.profile = v?.profile && new strims_profile_v1_Profile(v.profile);
  }

  static encode(m: SignInResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.linkedProfile) strims_auth_v1_LinkedProfile.encode(m.linkedProfile, w.uint32(10).fork()).ldelim();
    if (m.profile) strims_profile_v1_Profile.encode(m.profile, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SignInResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new SignInResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.linkedProfile = strims_auth_v1_LinkedProfile.decode(r, r.uint32());
        break;
        case 2:
        m.profile = strims_profile_v1_Profile.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
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
    const end = length === undefined ? r.len : r.pos + length;
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
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ISignUpResponse = {
  linkedProfile?: strims_auth_v1_ILinkedProfile;
  profile?: strims_profile_v1_IProfile;
}

export class SignUpResponse {
  linkedProfile: strims_auth_v1_LinkedProfile | undefined;
  profile: strims_profile_v1_Profile | undefined;

  constructor(v?: ISignUpResponse) {
    this.linkedProfile = v?.linkedProfile && new strims_auth_v1_LinkedProfile(v.linkedProfile);
    this.profile = v?.profile && new strims_profile_v1_Profile(v.profile);
  }

  static encode(m: SignUpResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.linkedProfile) strims_auth_v1_LinkedProfile.encode(m.linkedProfile, w.uint32(10).fork()).ldelim();
    if (m.profile) strims_profile_v1_Profile.encode(m.profile, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SignUpResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new SignUpResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.linkedProfile = strims_auth_v1_LinkedProfile.decode(r, r.uint32());
        break;
        case 2:
        m.profile = strims_profile_v1_Profile.decode(r, r.uint32());
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
export const strims_auth_v1_SessionThing = SessionThing;
/* @internal */
export type strims_auth_v1_SessionThing = SessionThing;
/* @internal */
export type strims_auth_v1_ISessionThing = ISessionThing;
/* @internal */
export const strims_auth_v1_TOTPConfig = TOTPConfig;
/* @internal */
export type strims_auth_v1_TOTPConfig = TOTPConfig;
/* @internal */
export type strims_auth_v1_ITOTPConfig = ITOTPConfig;
/* @internal */
export const strims_auth_v1_ServerUserThing = ServerUserThing;
/* @internal */
export type strims_auth_v1_ServerUserThing = ServerUserThing;
/* @internal */
export type strims_auth_v1_IServerUserThing = IServerUserThing;
/* @internal */
export const strims_auth_v1_LinkedProfile = LinkedProfile;
/* @internal */
export type strims_auth_v1_LinkedProfile = LinkedProfile;
/* @internal */
export type strims_auth_v1_ILinkedProfile = ILinkedProfile;
/* @internal */
export const strims_auth_v1_PairingToken = PairingToken;
/* @internal */
export type strims_auth_v1_PairingToken = PairingToken;
/* @internal */
export type strims_auth_v1_IPairingToken = IPairingToken;
/* @internal */
export const strims_auth_v1_SignInRequest = SignInRequest;
/* @internal */
export type strims_auth_v1_SignInRequest = SignInRequest;
/* @internal */
export type strims_auth_v1_ISignInRequest = ISignInRequest;
/* @internal */
export const strims_auth_v1_SignInResponse = SignInResponse;
/* @internal */
export type strims_auth_v1_SignInResponse = SignInResponse;
/* @internal */
export type strims_auth_v1_ISignInResponse = ISignInResponse;
/* @internal */
export const strims_auth_v1_SignUpRequest = SignUpRequest;
/* @internal */
export type strims_auth_v1_SignUpRequest = SignUpRequest;
/* @internal */
export type strims_auth_v1_ISignUpRequest = ISignUpRequest;
/* @internal */
export const strims_auth_v1_SignUpResponse = SignUpResponse;
/* @internal */
export type strims_auth_v1_SignUpResponse = SignUpResponse;
/* @internal */
export type strims_auth_v1_ISignUpResponse = ISignUpResponse;
/* @internal */
export const strims_auth_v1_ServerUserThing_Unencrypted = ServerUserThing.Unencrypted;
/* @internal */
export type strims_auth_v1_ServerUserThing_Unencrypted = ServerUserThing.Unencrypted;
/* @internal */
export type strims_auth_v1_ServerUserThing_IUnencrypted = ServerUserThing.IUnencrypted;
/* @internal */
export const strims_auth_v1_ServerUserThing_Password = ServerUserThing.Password;
/* @internal */
export type strims_auth_v1_ServerUserThing_Password = ServerUserThing.Password;
/* @internal */
export type strims_auth_v1_ServerUserThing_IPassword = ServerUserThing.IPassword;
/* @internal */
export const strims_auth_v1_ServerUserThing_Password_Secret = ServerUserThing.Password.Secret;
/* @internal */
export type strims_auth_v1_ServerUserThing_Password_Secret = ServerUserThing.Password.Secret;
/* @internal */
export type strims_auth_v1_ServerUserThing_Password_ISecret = ServerUserThing.Password.ISecret;
/* @internal */
export const strims_auth_v1_LinkedProfile_Unencrypted = LinkedProfile.Unencrypted;
/* @internal */
export type strims_auth_v1_LinkedProfile_Unencrypted = LinkedProfile.Unencrypted;
/* @internal */
export type strims_auth_v1_LinkedProfile_IUnencrypted = LinkedProfile.IUnencrypted;
/* @internal */
export const strims_auth_v1_LinkedProfile_Password = LinkedProfile.Password;
/* @internal */
export type strims_auth_v1_LinkedProfile_Password = LinkedProfile.Password;
/* @internal */
export type strims_auth_v1_LinkedProfile_IPassword = LinkedProfile.IPassword;
/* @internal */
export const strims_auth_v1_LinkedProfile_Token = LinkedProfile.Token;
/* @internal */
export type strims_auth_v1_LinkedProfile_Token = LinkedProfile.Token;
/* @internal */
export type strims_auth_v1_LinkedProfile_IToken = LinkedProfile.IToken;
/* @internal */
export const strims_auth_v1_LinkedProfile_Key = LinkedProfile.Key;
/* @internal */
export type strims_auth_v1_LinkedProfile_Key = LinkedProfile.Key;
/* @internal */
export type strims_auth_v1_LinkedProfile_IKey = LinkedProfile.IKey;
/* @internal */
export const strims_auth_v1_SignInRequest_Password = SignInRequest.Password;
/* @internal */
export type strims_auth_v1_SignInRequest_Password = SignInRequest.Password;
/* @internal */
export type strims_auth_v1_SignInRequest_IPassword = SignInRequest.IPassword;
/* @internal */
export const strims_auth_v1_SignInRequest_Token = SignInRequest.Token;
/* @internal */
export type strims_auth_v1_SignInRequest_Token = SignInRequest.Token;
/* @internal */
export type strims_auth_v1_SignInRequest_IToken = SignInRequest.IToken;
/* @internal */
export const strims_auth_v1_SignInRequest_Key = SignInRequest.Key;
/* @internal */
export type strims_auth_v1_SignInRequest_Key = SignInRequest.Key;
/* @internal */
export type strims_auth_v1_SignInRequest_IKey = SignInRequest.IKey;

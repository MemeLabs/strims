import Reader from "../../../pb/reader";
import Writer from "../../../pb/writer";

import {
  Key as strims_type_Key,
  IKey as strims_type_IKey,
} from "../type/key";

export interface ICreateProfileRequest {
  name?: string;
  password?: string;
}

export class CreateProfileRequest {
  name: string = "";
  password: string = "";

  constructor(v?: ICreateProfileRequest) {
    this.name = v?.name || "";
    this.password = v?.password || "";
  }

  static encode(m: CreateProfileRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.name) w.uint32(10).string(m.name);
    if (m.password) w.uint32(18).string(m.password);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateProfileRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreateProfileRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.name = r.string();
        break;
        case 2:
        m.password = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface ICreateProfileResponse {
  sessionId?: string;
  profile?: IProfile | undefined;
}

export class CreateProfileResponse {
  sessionId: string = "";
  profile: Profile | undefined;

  constructor(v?: ICreateProfileResponse) {
    this.sessionId = v?.sessionId || "";
    this.profile = v?.profile && new Profile(v.profile);
  }

  static encode(m: CreateProfileResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.sessionId) w.uint32(10).string(m.sessionId);
    if (m.profile) Profile.encode(m.profile, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateProfileResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreateProfileResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.sessionId = r.string();
        break;
        case 2:
        m.profile = Profile.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IUpdateProfileRequest {
  name?: string;
  password?: string;
}

export class UpdateProfileRequest {
  name: string = "";
  password: string = "";

  constructor(v?: IUpdateProfileRequest) {
    this.name = v?.name || "";
    this.password = v?.password || "";
  }

  static encode(m: UpdateProfileRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.name) w.uint32(10).string(m.name);
    if (m.password) w.uint32(18).string(m.password);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateProfileRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UpdateProfileRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.name = r.string();
        break;
        case 2:
        m.password = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IUpdateProfileResponse {
  profile?: IProfile | undefined;
}

export class UpdateProfileResponse {
  profile: Profile | undefined;

  constructor(v?: IUpdateProfileResponse) {
    this.profile = v?.profile && new Profile(v.profile);
  }

  static encode(m: UpdateProfileResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.profile) Profile.encode(m.profile, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateProfileResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UpdateProfileResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.profile = Profile.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IDeleteProfileRequest {
  id?: bigint;
}

export class DeleteProfileRequest {
  id: bigint = BigInt(0);

  constructor(v?: IDeleteProfileRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: DeleteProfileRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeleteProfileRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DeleteProfileRequest();
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

export interface IDeleteProfileResponse {
}

export class DeleteProfileResponse {

  constructor(v?: IDeleteProfileResponse) {
    // noop
  }

  static encode(m: DeleteProfileResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeleteProfileResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new DeleteProfileResponse();
  }
}

export interface ILoadProfileRequest {
  id?: bigint;
  name?: string;
  password?: string;
}

export class LoadProfileRequest {
  id: bigint = BigInt(0);
  name: string = "";
  password: string = "";

  constructor(v?: ILoadProfileRequest) {
    this.id = v?.id || BigInt(0);
    this.name = v?.name || "";
    this.password = v?.password || "";
  }

  static encode(m: LoadProfileRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.name) w.uint32(18).string(m.name);
    if (m.password) w.uint32(26).string(m.password);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): LoadProfileRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new LoadProfileRequest();
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
        m.password = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface ILoadProfileResponse {
  sessionId?: string;
  profile?: IProfile | undefined;
}

export class LoadProfileResponse {
  sessionId: string = "";
  profile: Profile | undefined;

  constructor(v?: ILoadProfileResponse) {
    this.sessionId = v?.sessionId || "";
    this.profile = v?.profile && new Profile(v.profile);
  }

  static encode(m: LoadProfileResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.sessionId) w.uint32(10).string(m.sessionId);
    if (m.profile) Profile.encode(m.profile, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): LoadProfileResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new LoadProfileResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.sessionId = r.string();
        break;
        case 2:
        m.profile = Profile.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IGetProfileRequest {
  sessionId?: string;
}

export class GetProfileRequest {
  sessionId: string = "";

  constructor(v?: IGetProfileRequest) {
    this.sessionId = v?.sessionId || "";
  }

  static encode(m: GetProfileRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.sessionId) w.uint32(10).string(m.sessionId);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetProfileRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetProfileRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.sessionId = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IGetProfileResponse {
  profile?: IProfile | undefined;
}

export class GetProfileResponse {
  profile: Profile | undefined;

  constructor(v?: IGetProfileResponse) {
    this.profile = v?.profile && new Profile(v.profile);
  }

  static encode(m: GetProfileResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.profile) Profile.encode(m.profile, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetProfileResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetProfileResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 2:
        m.profile = Profile.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IListProfilesRequest {
}

export class ListProfilesRequest {

  constructor(v?: IListProfilesRequest) {
    // noop
  }

  static encode(m: ListProfilesRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListProfilesRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new ListProfilesRequest();
  }
}

export interface IListProfilesResponse {
  profiles?: IProfileSummary[];
}

export class ListProfilesResponse {
  profiles: ProfileSummary[] = [];

  constructor(v?: IListProfilesResponse) {
    if (v?.profiles) this.profiles = v.profiles.map(v => new ProfileSummary(v));
  }

  static encode(m: ListProfilesResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    for (const v of m.profiles) ProfileSummary.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListProfilesResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ListProfilesResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.profiles.push(ProfileSummary.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface ILoadSessionRequest {
  sessionId?: string;
}

export class LoadSessionRequest {
  sessionId: string = "";

  constructor(v?: ILoadSessionRequest) {
    this.sessionId = v?.sessionId || "";
  }

  static encode(m: LoadSessionRequest, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.sessionId) w.uint32(10).string(m.sessionId);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): LoadSessionRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new LoadSessionRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.sessionId = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface ILoadSessionResponse {
  sessionId?: string;
  profile?: IProfile | undefined;
}

export class LoadSessionResponse {
  sessionId: string = "";
  profile: Profile | undefined;

  constructor(v?: ILoadSessionResponse) {
    this.sessionId = v?.sessionId || "";
    this.profile = v?.profile && new Profile(v.profile);
  }

  static encode(m: LoadSessionResponse, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.sessionId) w.uint32(10).string(m.sessionId);
    if (m.profile) Profile.encode(m.profile, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): LoadSessionResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new LoadSessionResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.sessionId = r.string();
        break;
        case 2:
        m.profile = Profile.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IStorageKey {
  kdfType?: KDFType;
  kdfOptions?: StorageKey.IKdfOptionsOneOf
}

export class StorageKey {
  kdfType: KDFType = 0;
  kdfOptions: StorageKey.TKdfOptionsOneOf;

  constructor(v?: IStorageKey) {
    this.kdfType = v?.kdfType || 0;
    this.kdfOptions = new StorageKey.KdfOptionsOneOf(v?.kdfOptions);
  }

  static encode(m: StorageKey, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.kdfType) w.uint32(8).uint32(m.kdfType);
    switch (m.kdfOptions.case) {
      case StorageKey.KdfOptionsCase.PBKDF2_OPTIONS:
      StorageKey.PBKDF2Options.encode(m.kdfOptions.pbkdf2Options, w.uint32(18).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): StorageKey {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new StorageKey();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.kdfType = r.uint32();
        break;
        case 2:
        m.kdfOptions = new StorageKey.KdfOptionsOneOf({ pbkdf2Options: StorageKey.PBKDF2Options.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace StorageKey {
  export enum KdfOptionsCase {
    NOT_SET = 0,
    PBKDF2_OPTIONS = 2,
  }

  export type IKdfOptionsOneOf =
  { case?: KdfOptionsCase.NOT_SET }
  |{ case?: KdfOptionsCase.PBKDF2_OPTIONS, pbkdf2Options: StorageKey.IPBKDF2Options }
  ;

  export type TKdfOptionsOneOf = Readonly<
  { case: KdfOptionsCase.NOT_SET }
  |{ case: KdfOptionsCase.PBKDF2_OPTIONS, pbkdf2Options: StorageKey.PBKDF2Options }
  >;

  class KdfOptionsOneOfImpl {
    pbkdf2Options: StorageKey.PBKDF2Options;
    case: KdfOptionsCase = KdfOptionsCase.NOT_SET;

    constructor(v?: IKdfOptionsOneOf) {
      if (v && "pbkdf2Options" in v) {
        this.case = KdfOptionsCase.PBKDF2_OPTIONS;
        this.pbkdf2Options = new StorageKey.PBKDF2Options(v.pbkdf2Options);
      }
    }
  }

  export const KdfOptionsOneOf = KdfOptionsOneOfImpl as {
    new (): Readonly<{ case: KdfOptionsCase.NOT_SET }>;
    new <T extends IKdfOptionsOneOf>(v: T): Readonly<
    T extends { pbkdf2Options: StorageKey.IPBKDF2Options } ? { case: KdfOptionsCase.PBKDF2_OPTIONS, pbkdf2Options: StorageKey.PBKDF2Options } :
    never
    >;
  };

  export interface IPBKDF2Options {
    iterations?: number;
    keySize?: number;
    salt?: Uint8Array;
  }

  export class PBKDF2Options {
    iterations: number = 0;
    keySize: number = 0;
    salt: Uint8Array = new Uint8Array();

    constructor(v?: IPBKDF2Options) {
      this.iterations = v?.iterations || 0;
      this.keySize = v?.keySize || 0;
      this.salt = v?.salt || new Uint8Array();
    }

    static encode(m: PBKDF2Options, w?: Writer): Writer {
      if (!w) w = new Writer(1024);
      if (m.iterations) w.uint32(8).uint32(m.iterations);
      if (m.keySize) w.uint32(16).uint32(m.keySize);
      if (m.salt) w.uint32(26).bytes(m.salt);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): PBKDF2Options {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new PBKDF2Options();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.iterations = r.uint32();
          break;
          case 2:
          m.keySize = r.uint32();
          break;
          case 3:
          m.salt = r.bytes();
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

export interface IProfile {
  id?: bigint;
  name?: string;
  secret?: Uint8Array;
  key?: strims_type_IKey | undefined;
}

export class Profile {
  id: bigint = BigInt(0);
  name: string = "";
  secret: Uint8Array = new Uint8Array();
  key: strims_type_Key | undefined;

  constructor(v?: IProfile) {
    this.id = v?.id || BigInt(0);
    this.name = v?.name || "";
    this.secret = v?.secret || new Uint8Array();
    this.key = v?.key && new strims_type_Key(v.key);
  }

  static encode(m: Profile, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.name) w.uint32(18).string(m.name);
    if (m.secret) w.uint32(26).bytes(m.secret);
    if (m.key) strims_type_Key.encode(m.key, w.uint32(34).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Profile {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Profile();
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
        m.secret = r.bytes();
        break;
        case 4:
        m.key = strims_type_Key.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IProfileSummary {
  id?: bigint;
  name?: string;
}

export class ProfileSummary {
  id: bigint = BigInt(0);
  name: string = "";

  constructor(v?: IProfileSummary) {
    this.id = v?.id || BigInt(0);
    this.name = v?.name || "";
  }

  static encode(m: ProfileSummary, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.name) w.uint32(18).string(m.name);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ProfileSummary {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ProfileSummary();
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

export interface IProfileID {
  nextId?: bigint;
}

export class ProfileID {
  nextId: bigint = BigInt(0);

  constructor(v?: IProfileID) {
    this.nextId = v?.nextId || BigInt(0);
  }

  static encode(m: ProfileID, w?: Writer): Writer {
    if (!w) w = new Writer(1024);
    if (m.nextId) w.uint32(8).uint64(m.nextId);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ProfileID {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ProfileID();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.nextId = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export enum KDFType {
  KDF_TYPE_UNDEFINED = 0,
  KDF_TYPE_PBKDF2_SHA256 = 1,
}

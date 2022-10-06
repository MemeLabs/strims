import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  strims_type_Key,
  strims_type_IKey,
} from "../../type/key";
import {
  strims_dao_v1_VersionVector,
  strims_dao_v1_IVersionVector,
} from "../../dao/v1/dao";

export type IUpdateProfileRequest = {
  name?: string;
  password?: string;
}

export class UpdateProfileRequest {
  name: string;
  password: string;

  constructor(v?: IUpdateProfileRequest) {
    this.name = v?.name || "";
    this.password = v?.password || "";
  }

  static encode(m: UpdateProfileRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.name.length) w.uint32(10).string(m.name);
    if (m.password.length) w.uint32(18).string(m.password);
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

export type IUpdateProfileResponse = {
  profile?: strims_profile_v1_IProfile;
}

export class UpdateProfileResponse {
  profile: strims_profile_v1_Profile | undefined;

  constructor(v?: IUpdateProfileResponse) {
    this.profile = v?.profile && new strims_profile_v1_Profile(v.profile);
  }

  static encode(m: UpdateProfileResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.profile) strims_profile_v1_Profile.encode(m.profile, w.uint32(10).fork()).ldelim();
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

export type IGetProfileRequest = Record<string, any>;

export class GetProfileRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IGetProfileRequest) {
  }

  static encode(m: GetProfileRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetProfileRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new GetProfileRequest();
  }
}

export type IGetProfileResponse = {
  profile?: strims_profile_v1_IProfile;
}

export class GetProfileResponse {
  profile: strims_profile_v1_Profile | undefined;

  constructor(v?: IGetProfileResponse) {
    this.profile = v?.profile && new strims_profile_v1_Profile(v.profile);
  }

  static encode(m: GetProfileResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.profile) strims_profile_v1_Profile.encode(m.profile, w.uint32(18).fork()).ldelim();
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

export type IStorageKey = {
  kdfType?: strims_profile_v1_KDFType;
  kdfOptions?: StorageKey.IKdfOptions
}

export class StorageKey {
  kdfType: strims_profile_v1_KDFType;
  kdfOptions: StorageKey.TKdfOptions;

  constructor(v?: IStorageKey) {
    this.kdfType = v?.kdfType || 0;
    this.kdfOptions = new StorageKey.KdfOptions(v?.kdfOptions);
  }

  static encode(m: StorageKey, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.kdfType) w.uint32(8).uint32(m.kdfType);
    switch (m.kdfOptions.case) {
      case StorageKey.KdfOptionsCase.PBKDF2_OPTIONS:
      strims_profile_v1_StorageKey_PBKDF2Options.encode(m.kdfOptions.pbkdf2Options, w.uint32(18).fork()).ldelim();
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
        m.kdfOptions = new StorageKey.KdfOptions({ pbkdf2Options: strims_profile_v1_StorageKey_PBKDF2Options.decode(r, r.uint32()) });
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

  export type IKdfOptions =
  { case?: KdfOptionsCase.NOT_SET }
  |{ case?: KdfOptionsCase.PBKDF2_OPTIONS, pbkdf2Options: strims_profile_v1_StorageKey_IPBKDF2Options }
  ;

  export type TKdfOptions = Readonly<
  { case: KdfOptionsCase.NOT_SET }
  |{ case: KdfOptionsCase.PBKDF2_OPTIONS, pbkdf2Options: strims_profile_v1_StorageKey_PBKDF2Options }
  >;

  class KdfOptionsImpl {
    pbkdf2Options: strims_profile_v1_StorageKey_PBKDF2Options;
    case: KdfOptionsCase = KdfOptionsCase.NOT_SET;

    constructor(v?: IKdfOptions) {
      if (v && "pbkdf2Options" in v) {
        this.case = KdfOptionsCase.PBKDF2_OPTIONS;
        this.pbkdf2Options = new strims_profile_v1_StorageKey_PBKDF2Options(v.pbkdf2Options);
      }
    }
  }

  export const KdfOptions = KdfOptionsImpl as {
    new (): Readonly<{ case: KdfOptionsCase.NOT_SET }>;
    new <T extends IKdfOptions>(v: T): Readonly<
    T extends { pbkdf2Options: strims_profile_v1_StorageKey_IPBKDF2Options } ? { case: KdfOptionsCase.PBKDF2_OPTIONS, pbkdf2Options: strims_profile_v1_StorageKey_PBKDF2Options } :
    never
    >;
  };

  export type IPBKDF2Options = {
    iterations?: number;
    keySize?: number;
    salt?: Uint8Array;
  }

  export class PBKDF2Options {
    iterations: number;
    keySize: number;
    salt: Uint8Array;

    constructor(v?: IPBKDF2Options) {
      this.iterations = v?.iterations || 0;
      this.keySize = v?.keySize || 0;
      this.salt = v?.salt || new Uint8Array();
    }

    static encode(m: PBKDF2Options, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.iterations) w.uint32(8).uint32(m.iterations);
      if (m.keySize) w.uint32(16).uint32(m.keySize);
      if (m.salt.length) w.uint32(26).bytes(m.salt);
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

export type IDevice = {
  id?: bigint;
  version?: strims_dao_v1_IVersionVector;
  device?: string;
  os?: string;
  lastLogin?: bigint;
}

export class Device {
  id: bigint;
  version: strims_dao_v1_VersionVector | undefined;
  device: string;
  os: string;
  lastLogin: bigint;

  constructor(v?: IDevice) {
    this.id = v?.id || BigInt(0);
    this.version = v?.version && new strims_dao_v1_VersionVector(v.version);
    this.device = v?.device || "";
    this.os = v?.os || "";
    this.lastLogin = v?.lastLogin || BigInt(0);
  }

  static encode(m: Device, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.version) strims_dao_v1_VersionVector.encode(m.version, w.uint32(18).fork()).ldelim();
    if (m.device.length) w.uint32(26).string(m.device);
    if (m.os.length) w.uint32(34).string(m.os);
    if (m.lastLogin) w.uint32(40).int64(m.lastLogin);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Device {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Device();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.version = strims_dao_v1_VersionVector.decode(r, r.uint32());
        break;
        case 3:
        m.device = r.string();
        break;
        case 4:
        m.os = r.string();
        break;
        case 5:
        m.lastLogin = r.int64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IProfile = {
  id?: bigint;
  name?: string;
  secret?: Uint8Array;
  key?: strims_type_IKey;
  deviceId?: bigint;
}

export class Profile {
  id: bigint;
  name: string;
  secret: Uint8Array;
  key: strims_type_Key | undefined;
  deviceId: bigint;

  constructor(v?: IProfile) {
    this.id = v?.id || BigInt(0);
    this.name = v?.name || "";
    this.secret = v?.secret || new Uint8Array();
    this.key = v?.key && new strims_type_Key(v.key);
    this.deviceId = v?.deviceId || BigInt(0);
  }

  static encode(m: Profile, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.name.length) w.uint32(18).string(m.name);
    if (m.secret.length) w.uint32(26).bytes(m.secret);
    if (m.key) strims_type_Key.encode(m.key, w.uint32(34).fork()).ldelim();
    if (m.deviceId) w.uint32(40).uint64(m.deviceId);
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
        case 5:
        m.deviceId = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IProfileID = {
  nextId?: bigint;
  lastId?: bigint;
  nextRange?: strims_profile_v1_IProfileID;
}

export class ProfileID {
  nextId: bigint;
  lastId: bigint;
  nextRange: strims_profile_v1_ProfileID | undefined;

  constructor(v?: IProfileID) {
    this.nextId = v?.nextId || BigInt(0);
    this.lastId = v?.lastId || BigInt(0);
    this.nextRange = v?.nextRange && new strims_profile_v1_ProfileID(v.nextRange);
  }

  static encode(m: ProfileID, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.nextId) w.uint32(8).uint64(m.nextId);
    if (m.lastId) w.uint32(16).uint64(m.lastId);
    if (m.nextRange) strims_profile_v1_ProfileID.encode(m.nextRange, w.uint32(26).fork()).ldelim();
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
        case 2:
        m.lastId = r.uint64();
        break;
        case 3:
        m.nextRange = strims_profile_v1_ProfileID.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IDeleteDeviceRequest = {
  id?: bigint;
}

export class DeleteDeviceRequest {
  id: bigint;

  constructor(v?: IDeleteDeviceRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: DeleteDeviceRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeleteDeviceRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DeleteDeviceRequest();
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

export type IDeleteDeviceResponse = Record<string, any>;

export class DeleteDeviceResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IDeleteDeviceResponse) {
  }

  static encode(m: DeleteDeviceResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeleteDeviceResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new DeleteDeviceResponse();
  }
}

export type IGetDeviceRequest = {
  id?: bigint;
}

export class GetDeviceRequest {
  id: bigint;

  constructor(v?: IGetDeviceRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: GetDeviceRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetDeviceRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetDeviceRequest();
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

export type IGetDeviceResponse = {
  device?: strims_profile_v1_IDevice;
}

export class GetDeviceResponse {
  device: strims_profile_v1_Device | undefined;

  constructor(v?: IGetDeviceResponse) {
    this.device = v?.device && new strims_profile_v1_Device(v.device);
  }

  static encode(m: GetDeviceResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.device) strims_profile_v1_Device.encode(m.device, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetDeviceResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetDeviceResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.device = strims_profile_v1_Device.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IListDevicesRequest = Record<string, any>;

export class ListDevicesRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IListDevicesRequest) {
  }

  static encode(m: ListDevicesRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListDevicesRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new ListDevicesRequest();
  }
}

export type IListDevicesResponse = {
  devices?: strims_profile_v1_IDevice[];
  currentDevice?: strims_profile_v1_IDevice;
}

export class ListDevicesResponse {
  devices: strims_profile_v1_Device[];
  currentDevice: strims_profile_v1_Device | undefined;

  constructor(v?: IListDevicesResponse) {
    this.devices = v?.devices ? v.devices.map(v => new strims_profile_v1_Device(v)) : [];
    this.currentDevice = v?.currentDevice && new strims_profile_v1_Device(v.currentDevice);
  }

  static encode(m: ListDevicesResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.devices) strims_profile_v1_Device.encode(v, w.uint32(10).fork()).ldelim();
    if (m.currentDevice) strims_profile_v1_Device.encode(m.currentDevice, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListDevicesResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ListDevicesResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.devices.push(strims_profile_v1_Device.decode(r, r.uint32()));
        break;
        case 2:
        m.currentDevice = strims_profile_v1_Device.decode(r, r.uint32());
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
/* @internal */
export const strims_profile_v1_UpdateProfileRequest = UpdateProfileRequest;
/* @internal */
export type strims_profile_v1_UpdateProfileRequest = UpdateProfileRequest;
/* @internal */
export type strims_profile_v1_IUpdateProfileRequest = IUpdateProfileRequest;
/* @internal */
export const strims_profile_v1_UpdateProfileResponse = UpdateProfileResponse;
/* @internal */
export type strims_profile_v1_UpdateProfileResponse = UpdateProfileResponse;
/* @internal */
export type strims_profile_v1_IUpdateProfileResponse = IUpdateProfileResponse;
/* @internal */
export const strims_profile_v1_GetProfileRequest = GetProfileRequest;
/* @internal */
export type strims_profile_v1_GetProfileRequest = GetProfileRequest;
/* @internal */
export type strims_profile_v1_IGetProfileRequest = IGetProfileRequest;
/* @internal */
export const strims_profile_v1_GetProfileResponse = GetProfileResponse;
/* @internal */
export type strims_profile_v1_GetProfileResponse = GetProfileResponse;
/* @internal */
export type strims_profile_v1_IGetProfileResponse = IGetProfileResponse;
/* @internal */
export const strims_profile_v1_StorageKey = StorageKey;
/* @internal */
export type strims_profile_v1_StorageKey = StorageKey;
/* @internal */
export type strims_profile_v1_IStorageKey = IStorageKey;
/* @internal */
export const strims_profile_v1_Device = Device;
/* @internal */
export type strims_profile_v1_Device = Device;
/* @internal */
export type strims_profile_v1_IDevice = IDevice;
/* @internal */
export const strims_profile_v1_Profile = Profile;
/* @internal */
export type strims_profile_v1_Profile = Profile;
/* @internal */
export type strims_profile_v1_IProfile = IProfile;
/* @internal */
export const strims_profile_v1_ProfileID = ProfileID;
/* @internal */
export type strims_profile_v1_ProfileID = ProfileID;
/* @internal */
export type strims_profile_v1_IProfileID = IProfileID;
/* @internal */
export const strims_profile_v1_DeleteDeviceRequest = DeleteDeviceRequest;
/* @internal */
export type strims_profile_v1_DeleteDeviceRequest = DeleteDeviceRequest;
/* @internal */
export type strims_profile_v1_IDeleteDeviceRequest = IDeleteDeviceRequest;
/* @internal */
export const strims_profile_v1_DeleteDeviceResponse = DeleteDeviceResponse;
/* @internal */
export type strims_profile_v1_DeleteDeviceResponse = DeleteDeviceResponse;
/* @internal */
export type strims_profile_v1_IDeleteDeviceResponse = IDeleteDeviceResponse;
/* @internal */
export const strims_profile_v1_GetDeviceRequest = GetDeviceRequest;
/* @internal */
export type strims_profile_v1_GetDeviceRequest = GetDeviceRequest;
/* @internal */
export type strims_profile_v1_IGetDeviceRequest = IGetDeviceRequest;
/* @internal */
export const strims_profile_v1_GetDeviceResponse = GetDeviceResponse;
/* @internal */
export type strims_profile_v1_GetDeviceResponse = GetDeviceResponse;
/* @internal */
export type strims_profile_v1_IGetDeviceResponse = IGetDeviceResponse;
/* @internal */
export const strims_profile_v1_ListDevicesRequest = ListDevicesRequest;
/* @internal */
export type strims_profile_v1_ListDevicesRequest = ListDevicesRequest;
/* @internal */
export type strims_profile_v1_IListDevicesRequest = IListDevicesRequest;
/* @internal */
export const strims_profile_v1_ListDevicesResponse = ListDevicesResponse;
/* @internal */
export type strims_profile_v1_ListDevicesResponse = ListDevicesResponse;
/* @internal */
export type strims_profile_v1_IListDevicesResponse = IListDevicesResponse;
/* @internal */
export const strims_profile_v1_StorageKey_PBKDF2Options = StorageKey.PBKDF2Options;
/* @internal */
export type strims_profile_v1_StorageKey_PBKDF2Options = StorageKey.PBKDF2Options;
/* @internal */
export type strims_profile_v1_StorageKey_IPBKDF2Options = StorageKey.IPBKDF2Options;
/* @internal */
export const strims_profile_v1_KDFType = KDFType;
/* @internal */
export type strims_profile_v1_KDFType = KDFType;

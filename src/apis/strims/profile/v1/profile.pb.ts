// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  Key as strims_type_Key,
  type IKey as strims_type_IKey,
} from "../../type/key.pb";
import {
  VersionVector as strims_dao_v1_VersionVector,
  type IVersionVector as strims_dao_v1_IVersionVector,
} from "../../dao/v1/dao.pb";

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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IUpdateProfileResponse = {
  profile?: IProfile;
}

export class UpdateProfileResponse {
  profile: Profile | undefined;

  constructor(v?: IUpdateProfileResponse) {
    this.profile = v?.profile && new Profile(v.profile);
  }

  static encode(m: UpdateProfileResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.profile) Profile.encode(m.profile, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateProfileResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UpdateProfileResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.profile = Profile.decode(r, r.uint32());
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

export type IGetProfileRequest = object;

export class GetProfileRequest {

  constructor(v?: IGetProfileRequest) {
    void v; // an empty message has no fields to set
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
  profile?: IProfile;
}

export class GetProfileResponse {
  profile: Profile | undefined;

  constructor(v?: IGetProfileResponse) {
    this.profile = v?.profile && new Profile(v.profile);
  }

  static encode(m: GetProfileResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.profile) Profile.encode(m.profile, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetProfileResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new GetProfileResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 2:
        m.profile = Profile.decode(r, r.uint32());
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

export type IStorageKey = {
  kdfType?: KDFType;
  kdfOptions?: StorageKey_IKdfOptions
}

export class StorageKey {
  kdfType: KDFType;
  kdfOptions: StorageKey_TKdfOptions;

  constructor(v?: IStorageKey) {
    this.kdfType = v?.kdfType || 0;
    this.kdfOptions = v?.kdfOptions ? new StorageKey_KdfOptions(v.kdfOptions) : new StorageKey_KdfOptions();
  }

  static encode(m: StorageKey, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.kdfType) w.uint32(8).uint32(m.kdfType);
    switch (m.kdfOptions.case) {
      case StorageKey_KdfOptionsCase.PBKDF2_OPTIONS:
      StorageKey_PBKDF2Options.encode(m.kdfOptions.pbkdf2Options, w.uint32(18).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): StorageKey {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new StorageKey();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.kdfType = r.uint32();
        break;
        case 2:
        m.kdfOptions = new StorageKey_KdfOptions({ pbkdf2Options: StorageKey_PBKDF2Options.decode(r, r.uint32()) });
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

export enum StorageKey_KdfOptionsCase {
  NOT_SET = 0,
  PBKDF2_OPTIONS = 2,
}

export type StorageKey_IKdfOptions =
{ case?: StorageKey_KdfOptionsCase.NOT_SET }
|{ case?: StorageKey_KdfOptionsCase.PBKDF2_OPTIONS, pbkdf2Options: StorageKey_IPBKDF2Options }
;

export type StorageKey_TKdfOptions = Readonly<
{ case: StorageKey_KdfOptionsCase.NOT_SET }
|{ case: StorageKey_KdfOptionsCase.PBKDF2_OPTIONS, pbkdf2Options: StorageKey_PBKDF2Options }
>;

class StorageKey_KdfOptionsImpl {
  declare pbkdf2Options?: StorageKey_PBKDF2Options;
  case: StorageKey_KdfOptionsCase = StorageKey_KdfOptionsCase.NOT_SET;

  constructor(v?: StorageKey_IKdfOptions) {
    if (v && "pbkdf2Options" in v) {
      this.case = StorageKey_KdfOptionsCase.PBKDF2_OPTIONS;
      this.pbkdf2Options = new StorageKey_PBKDF2Options(v.pbkdf2Options);
    }
  }
}

export const StorageKey_KdfOptions = StorageKey_KdfOptionsImpl as {
  new (): Readonly<{ case: StorageKey_KdfOptionsCase.NOT_SET }>;
  new <T extends StorageKey_IKdfOptions>(v: T): Readonly<
  T extends { pbkdf2Options: StorageKey_IPBKDF2Options } ? { case: StorageKey_KdfOptionsCase.PBKDF2_OPTIONS, pbkdf2Options: StorageKey_PBKDF2Options } :
  never
  >;
};

export type StorageKey_IPBKDF2Options = {
  iterations?: number;
  keySize?: number;
  salt?: Uint8Array;
}

export class StorageKey_PBKDF2Options {
  iterations: number;
  keySize: number;
  salt: Uint8Array;

  constructor(v?: StorageKey_IPBKDF2Options) {
    this.iterations = v?.iterations || 0;
    this.keySize = v?.keySize || 0;
    this.salt = v?.salt || new Uint8Array();
  }

  static encode(m: StorageKey_PBKDF2Options, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.iterations) w.uint32(8).uint32(m.iterations);
    if (m.keySize) w.uint32(16).uint32(m.keySize);
    if (m.salt.length) w.uint32(26).bytes(m.salt);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): StorageKey_PBKDF2Options {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new StorageKey_PBKDF2Options();
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IProfileID = {
  nextId?: bigint;
  lastId?: bigint;
  nextRange?: IProfileID;
}

export class ProfileID {
  nextId: bigint;
  lastId: bigint;
  nextRange: ProfileID | undefined;

  constructor(v?: IProfileID) {
    this.nextId = v?.nextId || BigInt(0);
    this.lastId = v?.lastId || BigInt(0);
    this.nextRange = v?.nextRange && new ProfileID(v.nextRange);
  }

  static encode(m: ProfileID, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.nextId) w.uint32(8).uint64(m.nextId);
    if (m.lastId) w.uint32(16).uint64(m.lastId);
    if (m.nextRange) ProfileID.encode(m.nextRange, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ProfileID {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
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
        m.nextRange = ProfileID.decode(r, r.uint32());
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new DeleteDeviceRequest();
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

export type IDeleteDeviceResponse = object;

export class DeleteDeviceResponse {

  constructor(v?: IDeleteDeviceResponse) {
    void v; // an empty message has no fields to set
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new GetDeviceRequest();
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

export type IGetDeviceResponse = {
  device?: IDevice;
}

export class GetDeviceResponse {
  device: Device | undefined;

  constructor(v?: IGetDeviceResponse) {
    this.device = v?.device && new Device(v.device);
  }

  static encode(m: GetDeviceResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.device) Device.encode(m.device, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetDeviceResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new GetDeviceResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.device = Device.decode(r, r.uint32());
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

export type IListDevicesRequest = object;

export class ListDevicesRequest {

  constructor(v?: IListDevicesRequest) {
    void v; // an empty message has no fields to set
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
  devices?: IDevice[];
  currentDevice?: IDevice;
}

export class ListDevicesResponse {
  devices: Device[];
  currentDevice: Device | undefined;

  constructor(v?: IListDevicesResponse) {
    this.devices = v?.devices ? v.devices.map(v => new Device(v)) : [];
    this.currentDevice = v?.currentDevice && new Device(v.currentDevice);
  }

  static encode(m: ListDevicesResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.devices) Device.encode(v, w.uint32(10).fork()).ldelim();
    if (m.currentDevice) Device.encode(m.currentDevice, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListDevicesResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ListDevicesResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.devices.push(Device.decode(r, r.uint32()));
        break;
        case 2:
        m.currentDevice = Device.decode(r, r.uint32());
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

export enum KDFType {
  KDF_TYPE_UNDEFINED = 0,
  KDF_TYPE_PBKDF2_SHA256 = 1,
}


import Reader from "../../../../lib/pb/reader";
import Writer from "../../../../lib/pb/writer";

import {
  Certificate as strims_type_Certificate,
  ICertificate as strims_type_ICertificate,
} from "../../type/certificate";
import {
  Key as strims_type_Key,
  IKey as strims_type_IKey,
} from "../../type/key";
import {
  DirectoryListingSnippet as strims_network_v1_DirectoryListingSnippet,
  IDirectoryListingSnippet as strims_network_v1_IDirectoryListingSnippet,
} from "../../network/v1/directory";

export interface IVideoChannel {
  id?: bigint;
  key?: strims_type_IKey | undefined;
  token?: Uint8Array;
  directoryListingSnippet?: strims_network_v1_IDirectoryListingSnippet | undefined;
  owner?: VideoChannel.IOwner
}

export class VideoChannel {
  id: bigint = BigInt(0);
  key: strims_type_Key | undefined;
  token: Uint8Array = new Uint8Array();
  directoryListingSnippet: strims_network_v1_DirectoryListingSnippet | undefined;
  owner: VideoChannel.TOwner;

  constructor(v?: IVideoChannel) {
    this.id = v?.id || BigInt(0);
    this.key = v?.key && new strims_type_Key(v.key);
    this.token = v?.token || new Uint8Array();
    this.directoryListingSnippet = v?.directoryListingSnippet && new strims_network_v1_DirectoryListingSnippet(v.directoryListingSnippet);
    this.owner = new VideoChannel.Owner(v?.owner);
  }

  static encode(m: VideoChannel, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.key) strims_type_Key.encode(m.key, w.uint32(18).fork()).ldelim();
    if (m.token) w.uint32(26).bytes(m.token);
    if (m.directoryListingSnippet) strims_network_v1_DirectoryListingSnippet.encode(m.directoryListingSnippet, w.uint32(34).fork()).ldelim();
    switch (m.owner.case) {
      case VideoChannel.OwnerCase.LOCAL:
      VideoChannel.Local.encode(m.owner.local, w.uint32(8010).fork()).ldelim();
      break;
      case VideoChannel.OwnerCase.LOCAL_SHARE:
      VideoChannel.LocalShare.encode(m.owner.localShare, w.uint32(8018).fork()).ldelim();
      break;
      case VideoChannel.OwnerCase.REMOTE_SHARE:
      VideoChannel.RemoteShare.encode(m.owner.remoteShare, w.uint32(8026).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoChannel {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new VideoChannel();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 1001:
        m.owner = new VideoChannel.Owner({ local: VideoChannel.Local.decode(r, r.uint32()) });
        break;
        case 1002:
        m.owner = new VideoChannel.Owner({ localShare: VideoChannel.LocalShare.decode(r, r.uint32()) });
        break;
        case 1003:
        m.owner = new VideoChannel.Owner({ remoteShare: VideoChannel.RemoteShare.decode(r, r.uint32()) });
        break;
        case 2:
        m.key = strims_type_Key.decode(r, r.uint32());
        break;
        case 3:
        m.token = r.bytes();
        break;
        case 4:
        m.directoryListingSnippet = strims_network_v1_DirectoryListingSnippet.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace VideoChannel {
  export enum OwnerCase {
    NOT_SET = 0,
    LOCAL = 1001,
    LOCAL_SHARE = 1002,
    REMOTE_SHARE = 1003,
  }

  export type IOwner =
  { case?: OwnerCase.NOT_SET }
  |{ case?: OwnerCase.LOCAL, local: VideoChannel.ILocal }
  |{ case?: OwnerCase.LOCAL_SHARE, localShare: VideoChannel.ILocalShare }
  |{ case?: OwnerCase.REMOTE_SHARE, remoteShare: VideoChannel.IRemoteShare }
  ;

  export type TOwner = Readonly<
  { case: OwnerCase.NOT_SET }
  |{ case: OwnerCase.LOCAL, local: VideoChannel.Local }
  |{ case: OwnerCase.LOCAL_SHARE, localShare: VideoChannel.LocalShare }
  |{ case: OwnerCase.REMOTE_SHARE, remoteShare: VideoChannel.RemoteShare }
  >;

  class OwnerImpl {
    local: VideoChannel.Local;
    localShare: VideoChannel.LocalShare;
    remoteShare: VideoChannel.RemoteShare;
    case: OwnerCase = OwnerCase.NOT_SET;

    constructor(v?: IOwner) {
      if (v && "local" in v) {
        this.case = OwnerCase.LOCAL;
        this.local = new VideoChannel.Local(v.local);
      } else
      if (v && "localShare" in v) {
        this.case = OwnerCase.LOCAL_SHARE;
        this.localShare = new VideoChannel.LocalShare(v.localShare);
      } else
      if (v && "remoteShare" in v) {
        this.case = OwnerCase.REMOTE_SHARE;
        this.remoteShare = new VideoChannel.RemoteShare(v.remoteShare);
      }
    }
  }

  export const Owner = OwnerImpl as {
    new (): Readonly<{ case: OwnerCase.NOT_SET }>;
    new <T extends IOwner>(v: T): Readonly<
    T extends { local: VideoChannel.ILocal } ? { case: OwnerCase.LOCAL, local: VideoChannel.Local } :
    T extends { localShare: VideoChannel.ILocalShare } ? { case: OwnerCase.LOCAL_SHARE, localShare: VideoChannel.LocalShare } :
    T extends { remoteShare: VideoChannel.IRemoteShare } ? { case: OwnerCase.REMOTE_SHARE, remoteShare: VideoChannel.RemoteShare } :
    never
    >;
  };

  export interface ILocal {
    authKey?: Uint8Array;
    networkKey?: Uint8Array;
  }

  export class Local {
    authKey: Uint8Array = new Uint8Array();
    networkKey: Uint8Array = new Uint8Array();

    constructor(v?: ILocal) {
      this.authKey = v?.authKey || new Uint8Array();
      this.networkKey = v?.networkKey || new Uint8Array();
    }

    static encode(m: Local, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.authKey) w.uint32(10).bytes(m.authKey);
      if (m.networkKey) w.uint32(18).bytes(m.networkKey);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Local {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Local();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.authKey = r.bytes();
          break;
          case 2:
          m.networkKey = r.bytes();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface ILocalShare {
    certificate?: strims_type_ICertificate | undefined;
  }

  export class LocalShare {
    certificate: strims_type_Certificate | undefined;

    constructor(v?: ILocalShare) {
      this.certificate = v?.certificate && new strims_type_Certificate(v.certificate);
    }

    static encode(m: LocalShare, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.certificate) strims_type_Certificate.encode(m.certificate, w.uint32(10).fork()).ldelim();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): LocalShare {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new LocalShare();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.certificate = strims_type_Certificate.decode(r, r.uint32());
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface IRemoteShare {
    id?: bigint;
    networkKey?: Uint8Array;
    serviceKey?: Uint8Array;
    serviceSalt?: Uint8Array;
    serverAddr?: string;
  }

  export class RemoteShare {
    id: bigint = BigInt(0);
    networkKey: Uint8Array = new Uint8Array();
    serviceKey: Uint8Array = new Uint8Array();
    serviceSalt: Uint8Array = new Uint8Array();
    serverAddr: string = "";

    constructor(v?: IRemoteShare) {
      this.id = v?.id || BigInt(0);
      this.networkKey = v?.networkKey || new Uint8Array();
      this.serviceKey = v?.serviceKey || new Uint8Array();
      this.serviceSalt = v?.serviceSalt || new Uint8Array();
      this.serverAddr = v?.serverAddr || "";
    }

    static encode(m: RemoteShare, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.id) w.uint32(8).uint64(m.id);
      if (m.networkKey) w.uint32(18).bytes(m.networkKey);
      if (m.serviceKey) w.uint32(26).bytes(m.serviceKey);
      if (m.serviceSalt) w.uint32(34).bytes(m.serviceSalt);
      if (m.serverAddr) w.uint32(42).string(m.serverAddr);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): RemoteShare {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new RemoteShare();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.id = r.uint64();
          break;
          case 2:
          m.networkKey = r.bytes();
          break;
          case 3:
          m.serviceKey = r.bytes();
          break;
          case 4:
          m.serviceSalt = r.bytes();
          break;
          case 5:
          m.serverAddr = r.string();
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

export interface IVideoChannelListRequest {
}

export class VideoChannelListRequest {

  constructor(v?: IVideoChannelListRequest) {
    // noop
  }

  static encode(m: VideoChannelListRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoChannelListRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new VideoChannelListRequest();
  }
}

export interface IVideoChannelListResponse {
  channels?: IVideoChannel[];
}

export class VideoChannelListResponse {
  channels: VideoChannel[] = [];

  constructor(v?: IVideoChannelListResponse) {
    if (v?.channels) this.channels = v.channels.map(v => new VideoChannel(v));
  }

  static encode(m: VideoChannelListResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.channels) VideoChannel.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoChannelListResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new VideoChannelListResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.channels.push(VideoChannel.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IVideoChannelCreateRequest {
  directoryListingSnippet?: strims_network_v1_IDirectoryListingSnippet | undefined;
  networkKey?: Uint8Array;
}

export class VideoChannelCreateRequest {
  directoryListingSnippet: strims_network_v1_DirectoryListingSnippet | undefined;
  networkKey: Uint8Array = new Uint8Array();

  constructor(v?: IVideoChannelCreateRequest) {
    this.directoryListingSnippet = v?.directoryListingSnippet && new strims_network_v1_DirectoryListingSnippet(v.directoryListingSnippet);
    this.networkKey = v?.networkKey || new Uint8Array();
  }

  static encode(m: VideoChannelCreateRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.directoryListingSnippet) strims_network_v1_DirectoryListingSnippet.encode(m.directoryListingSnippet, w.uint32(10).fork()).ldelim();
    if (m.networkKey) w.uint32(18).bytes(m.networkKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoChannelCreateRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new VideoChannelCreateRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.directoryListingSnippet = strims_network_v1_DirectoryListingSnippet.decode(r, r.uint32());
        break;
        case 2:
        m.networkKey = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IVideoChannelCreateResponse {
  channel?: IVideoChannel | undefined;
}

export class VideoChannelCreateResponse {
  channel: VideoChannel | undefined;

  constructor(v?: IVideoChannelCreateResponse) {
    this.channel = v?.channel && new VideoChannel(v.channel);
  }

  static encode(m: VideoChannelCreateResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.channel) VideoChannel.encode(m.channel, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoChannelCreateResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new VideoChannelCreateResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.channel = VideoChannel.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IVideoChannelUpdateRequest {
  id?: bigint;
  directoryListingSnippet?: strims_network_v1_IDirectoryListingSnippet | undefined;
  networkKey?: Uint8Array;
}

export class VideoChannelUpdateRequest {
  id: bigint = BigInt(0);
  directoryListingSnippet: strims_network_v1_DirectoryListingSnippet | undefined;
  networkKey: Uint8Array = new Uint8Array();

  constructor(v?: IVideoChannelUpdateRequest) {
    this.id = v?.id || BigInt(0);
    this.directoryListingSnippet = v?.directoryListingSnippet && new strims_network_v1_DirectoryListingSnippet(v.directoryListingSnippet);
    this.networkKey = v?.networkKey || new Uint8Array();
  }

  static encode(m: VideoChannelUpdateRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.directoryListingSnippet) strims_network_v1_DirectoryListingSnippet.encode(m.directoryListingSnippet, w.uint32(18).fork()).ldelim();
    if (m.networkKey) w.uint32(26).bytes(m.networkKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoChannelUpdateRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new VideoChannelUpdateRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.directoryListingSnippet = strims_network_v1_DirectoryListingSnippet.decode(r, r.uint32());
        break;
        case 3:
        m.networkKey = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IVideoChannelUpdateResponse {
  channel?: IVideoChannel | undefined;
}

export class VideoChannelUpdateResponse {
  channel: VideoChannel | undefined;

  constructor(v?: IVideoChannelUpdateResponse) {
    this.channel = v?.channel && new VideoChannel(v.channel);
  }

  static encode(m: VideoChannelUpdateResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.channel) VideoChannel.encode(m.channel, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoChannelUpdateResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new VideoChannelUpdateResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.channel = VideoChannel.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export interface IVideoChannelDeleteRequest {
  id?: bigint;
}

export class VideoChannelDeleteRequest {
  id: bigint = BigInt(0);

  constructor(v?: IVideoChannelDeleteRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: VideoChannelDeleteRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoChannelDeleteRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new VideoChannelDeleteRequest();
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

export interface IVideoChannelDeleteResponse {
}

export class VideoChannelDeleteResponse {

  constructor(v?: IVideoChannelDeleteResponse) {
    // noop
  }

  static encode(m: VideoChannelDeleteResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoChannelDeleteResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new VideoChannelDeleteResponse();
  }
}


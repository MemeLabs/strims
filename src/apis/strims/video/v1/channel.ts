import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  strims_type_Certificate,
  strims_type_ICertificate,
} from "../../type/certificate";
import {
  strims_type_Key,
  strims_type_IKey,
} from "../../type/key";
import {
  strims_network_v1_directory_ListingSnippet,
  strims_network_v1_directory_IListingSnippet,
} from "../../network/v1/directory/directory";

export type IVideoChannel = {
  id?: bigint;
  key?: strims_type_IKey;
  token?: Uint8Array;
  directoryListingSnippet?: strims_network_v1_directory_IListingSnippet;
  owner?: VideoChannel.IOwner
}

export class VideoChannel {
  id: bigint;
  key: strims_type_Key | undefined;
  token: Uint8Array;
  directoryListingSnippet: strims_network_v1_directory_ListingSnippet | undefined;
  owner: VideoChannel.TOwner;

  constructor(v?: IVideoChannel) {
    this.id = v?.id || BigInt(0);
    this.key = v?.key && new strims_type_Key(v.key);
    this.token = v?.token || new Uint8Array();
    this.directoryListingSnippet = v?.directoryListingSnippet && new strims_network_v1_directory_ListingSnippet(v.directoryListingSnippet);
    this.owner = new VideoChannel.Owner(v?.owner);
  }

  static encode(m: VideoChannel, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.key) strims_type_Key.encode(m.key, w.uint32(18).fork()).ldelim();
    if (m.token.length) w.uint32(26).bytes(m.token);
    if (m.directoryListingSnippet) strims_network_v1_directory_ListingSnippet.encode(m.directoryListingSnippet, w.uint32(34).fork()).ldelim();
    switch (m.owner.case) {
      case VideoChannel.OwnerCase.LOCAL:
      strims_video_v1_VideoChannel_Local.encode(m.owner.local, w.uint32(8010).fork()).ldelim();
      break;
      case VideoChannel.OwnerCase.LOCAL_SHARE:
      strims_video_v1_VideoChannel_LocalShare.encode(m.owner.localShare, w.uint32(8018).fork()).ldelim();
      break;
      case VideoChannel.OwnerCase.REMOTE_SHARE:
      strims_video_v1_VideoChannel_RemoteShare.encode(m.owner.remoteShare, w.uint32(8026).fork()).ldelim();
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
        m.owner = new VideoChannel.Owner({ local: strims_video_v1_VideoChannel_Local.decode(r, r.uint32()) });
        break;
        case 1002:
        m.owner = new VideoChannel.Owner({ localShare: strims_video_v1_VideoChannel_LocalShare.decode(r, r.uint32()) });
        break;
        case 1003:
        m.owner = new VideoChannel.Owner({ remoteShare: strims_video_v1_VideoChannel_RemoteShare.decode(r, r.uint32()) });
        break;
        case 2:
        m.key = strims_type_Key.decode(r, r.uint32());
        break;
        case 3:
        m.token = r.bytes();
        break;
        case 4:
        m.directoryListingSnippet = strims_network_v1_directory_ListingSnippet.decode(r, r.uint32());
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
  |{ case?: OwnerCase.LOCAL, local: strims_video_v1_VideoChannel_ILocal }
  |{ case?: OwnerCase.LOCAL_SHARE, localShare: strims_video_v1_VideoChannel_ILocalShare }
  |{ case?: OwnerCase.REMOTE_SHARE, remoteShare: strims_video_v1_VideoChannel_IRemoteShare }
  ;

  export type TOwner = Readonly<
  { case: OwnerCase.NOT_SET }
  |{ case: OwnerCase.LOCAL, local: strims_video_v1_VideoChannel_Local }
  |{ case: OwnerCase.LOCAL_SHARE, localShare: strims_video_v1_VideoChannel_LocalShare }
  |{ case: OwnerCase.REMOTE_SHARE, remoteShare: strims_video_v1_VideoChannel_RemoteShare }
  >;

  class OwnerImpl {
    local: strims_video_v1_VideoChannel_Local;
    localShare: strims_video_v1_VideoChannel_LocalShare;
    remoteShare: strims_video_v1_VideoChannel_RemoteShare;
    case: OwnerCase = OwnerCase.NOT_SET;

    constructor(v?: IOwner) {
      if (v && "local" in v) {
        this.case = OwnerCase.LOCAL;
        this.local = new strims_video_v1_VideoChannel_Local(v.local);
      } else
      if (v && "localShare" in v) {
        this.case = OwnerCase.LOCAL_SHARE;
        this.localShare = new strims_video_v1_VideoChannel_LocalShare(v.localShare);
      } else
      if (v && "remoteShare" in v) {
        this.case = OwnerCase.REMOTE_SHARE;
        this.remoteShare = new strims_video_v1_VideoChannel_RemoteShare(v.remoteShare);
      }
    }
  }

  export const Owner = OwnerImpl as {
    new (): Readonly<{ case: OwnerCase.NOT_SET }>;
    new <T extends IOwner>(v: T): Readonly<
    T extends { local: strims_video_v1_VideoChannel_ILocal } ? { case: OwnerCase.LOCAL, local: strims_video_v1_VideoChannel_Local } :
    T extends { localShare: strims_video_v1_VideoChannel_ILocalShare } ? { case: OwnerCase.LOCAL_SHARE, localShare: strims_video_v1_VideoChannel_LocalShare } :
    T extends { remoteShare: strims_video_v1_VideoChannel_IRemoteShare } ? { case: OwnerCase.REMOTE_SHARE, remoteShare: strims_video_v1_VideoChannel_RemoteShare } :
    never
    >;
  };

  export type ILocal = {
    authKey?: Uint8Array;
    networkKey?: Uint8Array;
  }

  export class Local {
    authKey: Uint8Array;
    networkKey: Uint8Array;

    constructor(v?: ILocal) {
      this.authKey = v?.authKey || new Uint8Array();
      this.networkKey = v?.networkKey || new Uint8Array();
    }

    static encode(m: Local, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.authKey.length) w.uint32(10).bytes(m.authKey);
      if (m.networkKey.length) w.uint32(18).bytes(m.networkKey);
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

  export type ILocalShare = {
    certificate?: strims_type_ICertificate;
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

  export type IRemoteShare = {
    id?: bigint;
    networkKey?: Uint8Array;
    serviceKey?: Uint8Array;
    serviceSalt?: Uint8Array;
    serverAddr?: string;
  }

  export class RemoteShare {
    id: bigint;
    networkKey: Uint8Array;
    serviceKey: Uint8Array;
    serviceSalt: Uint8Array;
    serverAddr: string;

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
      if (m.networkKey.length) w.uint32(18).bytes(m.networkKey);
      if (m.serviceKey.length) w.uint32(26).bytes(m.serviceKey);
      if (m.serviceSalt.length) w.uint32(34).bytes(m.serviceSalt);
      if (m.serverAddr.length) w.uint32(42).string(m.serverAddr);
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

export type IVideoChannelListRequest = Record<string, any>;

export class VideoChannelListRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IVideoChannelListRequest) {
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

export type IVideoChannelListResponse = {
  channels?: strims_video_v1_IVideoChannel[];
}

export class VideoChannelListResponse {
  channels: strims_video_v1_VideoChannel[];

  constructor(v?: IVideoChannelListResponse) {
    this.channels = v?.channels ? v.channels.map(v => new strims_video_v1_VideoChannel(v)) : [];
  }

  static encode(m: VideoChannelListResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.channels) strims_video_v1_VideoChannel.encode(v, w.uint32(10).fork()).ldelim();
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
        m.channels.push(strims_video_v1_VideoChannel.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IVideoChannelGetRequest = {
  id?: bigint;
}

export class VideoChannelGetRequest {
  id: bigint;

  constructor(v?: IVideoChannelGetRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: VideoChannelGetRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoChannelGetRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new VideoChannelGetRequest();
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

export type IVideoChannelGetResponse = {
  channel?: strims_video_v1_IVideoChannel;
}

export class VideoChannelGetResponse {
  channel: strims_video_v1_VideoChannel | undefined;

  constructor(v?: IVideoChannelGetResponse) {
    this.channel = v?.channel && new strims_video_v1_VideoChannel(v.channel);
  }

  static encode(m: VideoChannelGetResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.channel) strims_video_v1_VideoChannel.encode(m.channel, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoChannelGetResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new VideoChannelGetResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.channel = strims_video_v1_VideoChannel.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IVideoChannelCreateRequest = {
  directoryListingSnippet?: strims_network_v1_directory_IListingSnippet;
  networkKey?: Uint8Array;
}

export class VideoChannelCreateRequest {
  directoryListingSnippet: strims_network_v1_directory_ListingSnippet | undefined;
  networkKey: Uint8Array;

  constructor(v?: IVideoChannelCreateRequest) {
    this.directoryListingSnippet = v?.directoryListingSnippet && new strims_network_v1_directory_ListingSnippet(v.directoryListingSnippet);
    this.networkKey = v?.networkKey || new Uint8Array();
  }

  static encode(m: VideoChannelCreateRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.directoryListingSnippet) strims_network_v1_directory_ListingSnippet.encode(m.directoryListingSnippet, w.uint32(10).fork()).ldelim();
    if (m.networkKey.length) w.uint32(18).bytes(m.networkKey);
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
        m.directoryListingSnippet = strims_network_v1_directory_ListingSnippet.decode(r, r.uint32());
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

export type IVideoChannelCreateResponse = {
  channel?: strims_video_v1_IVideoChannel;
}

export class VideoChannelCreateResponse {
  channel: strims_video_v1_VideoChannel | undefined;

  constructor(v?: IVideoChannelCreateResponse) {
    this.channel = v?.channel && new strims_video_v1_VideoChannel(v.channel);
  }

  static encode(m: VideoChannelCreateResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.channel) strims_video_v1_VideoChannel.encode(m.channel, w.uint32(10).fork()).ldelim();
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
        m.channel = strims_video_v1_VideoChannel.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IVideoChannelUpdateRequest = {
  id?: bigint;
  directoryListingSnippet?: strims_network_v1_directory_IListingSnippet;
  networkKey?: Uint8Array;
}

export class VideoChannelUpdateRequest {
  id: bigint;
  directoryListingSnippet: strims_network_v1_directory_ListingSnippet | undefined;
  networkKey: Uint8Array;

  constructor(v?: IVideoChannelUpdateRequest) {
    this.id = v?.id || BigInt(0);
    this.directoryListingSnippet = v?.directoryListingSnippet && new strims_network_v1_directory_ListingSnippet(v.directoryListingSnippet);
    this.networkKey = v?.networkKey || new Uint8Array();
  }

  static encode(m: VideoChannelUpdateRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.directoryListingSnippet) strims_network_v1_directory_ListingSnippet.encode(m.directoryListingSnippet, w.uint32(18).fork()).ldelim();
    if (m.networkKey.length) w.uint32(26).bytes(m.networkKey);
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
        m.directoryListingSnippet = strims_network_v1_directory_ListingSnippet.decode(r, r.uint32());
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

export type IVideoChannelUpdateResponse = {
  channel?: strims_video_v1_IVideoChannel;
}

export class VideoChannelUpdateResponse {
  channel: strims_video_v1_VideoChannel | undefined;

  constructor(v?: IVideoChannelUpdateResponse) {
    this.channel = v?.channel && new strims_video_v1_VideoChannel(v.channel);
  }

  static encode(m: VideoChannelUpdateResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.channel) strims_video_v1_VideoChannel.encode(m.channel, w.uint32(10).fork()).ldelim();
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
        m.channel = strims_video_v1_VideoChannel.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IVideoChannelDeleteRequest = {
  id?: bigint;
}

export class VideoChannelDeleteRequest {
  id: bigint;

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

export type IVideoChannelDeleteResponse = Record<string, any>;

export class VideoChannelDeleteResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IVideoChannelDeleteResponse) {
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

/* @internal */
export const strims_video_v1_VideoChannel = VideoChannel;
/* @internal */
export type strims_video_v1_VideoChannel = VideoChannel;
/* @internal */
export type strims_video_v1_IVideoChannel = IVideoChannel;
/* @internal */
export const strims_video_v1_VideoChannelListRequest = VideoChannelListRequest;
/* @internal */
export type strims_video_v1_VideoChannelListRequest = VideoChannelListRequest;
/* @internal */
export type strims_video_v1_IVideoChannelListRequest = IVideoChannelListRequest;
/* @internal */
export const strims_video_v1_VideoChannelListResponse = VideoChannelListResponse;
/* @internal */
export type strims_video_v1_VideoChannelListResponse = VideoChannelListResponse;
/* @internal */
export type strims_video_v1_IVideoChannelListResponse = IVideoChannelListResponse;
/* @internal */
export const strims_video_v1_VideoChannelGetRequest = VideoChannelGetRequest;
/* @internal */
export type strims_video_v1_VideoChannelGetRequest = VideoChannelGetRequest;
/* @internal */
export type strims_video_v1_IVideoChannelGetRequest = IVideoChannelGetRequest;
/* @internal */
export const strims_video_v1_VideoChannelGetResponse = VideoChannelGetResponse;
/* @internal */
export type strims_video_v1_VideoChannelGetResponse = VideoChannelGetResponse;
/* @internal */
export type strims_video_v1_IVideoChannelGetResponse = IVideoChannelGetResponse;
/* @internal */
export const strims_video_v1_VideoChannelCreateRequest = VideoChannelCreateRequest;
/* @internal */
export type strims_video_v1_VideoChannelCreateRequest = VideoChannelCreateRequest;
/* @internal */
export type strims_video_v1_IVideoChannelCreateRequest = IVideoChannelCreateRequest;
/* @internal */
export const strims_video_v1_VideoChannelCreateResponse = VideoChannelCreateResponse;
/* @internal */
export type strims_video_v1_VideoChannelCreateResponse = VideoChannelCreateResponse;
/* @internal */
export type strims_video_v1_IVideoChannelCreateResponse = IVideoChannelCreateResponse;
/* @internal */
export const strims_video_v1_VideoChannelUpdateRequest = VideoChannelUpdateRequest;
/* @internal */
export type strims_video_v1_VideoChannelUpdateRequest = VideoChannelUpdateRequest;
/* @internal */
export type strims_video_v1_IVideoChannelUpdateRequest = IVideoChannelUpdateRequest;
/* @internal */
export const strims_video_v1_VideoChannelUpdateResponse = VideoChannelUpdateResponse;
/* @internal */
export type strims_video_v1_VideoChannelUpdateResponse = VideoChannelUpdateResponse;
/* @internal */
export type strims_video_v1_IVideoChannelUpdateResponse = IVideoChannelUpdateResponse;
/* @internal */
export const strims_video_v1_VideoChannelDeleteRequest = VideoChannelDeleteRequest;
/* @internal */
export type strims_video_v1_VideoChannelDeleteRequest = VideoChannelDeleteRequest;
/* @internal */
export type strims_video_v1_IVideoChannelDeleteRequest = IVideoChannelDeleteRequest;
/* @internal */
export const strims_video_v1_VideoChannelDeleteResponse = VideoChannelDeleteResponse;
/* @internal */
export type strims_video_v1_VideoChannelDeleteResponse = VideoChannelDeleteResponse;
/* @internal */
export type strims_video_v1_IVideoChannelDeleteResponse = IVideoChannelDeleteResponse;
/* @internal */
export const strims_video_v1_VideoChannel_Local = VideoChannel.Local;
/* @internal */
export type strims_video_v1_VideoChannel_Local = VideoChannel.Local;
/* @internal */
export type strims_video_v1_VideoChannel_ILocal = VideoChannel.ILocal;
/* @internal */
export const strims_video_v1_VideoChannel_LocalShare = VideoChannel.LocalShare;
/* @internal */
export type strims_video_v1_VideoChannel_LocalShare = VideoChannel.LocalShare;
/* @internal */
export type strims_video_v1_VideoChannel_ILocalShare = VideoChannel.ILocalShare;
/* @internal */
export const strims_video_v1_VideoChannel_RemoteShare = VideoChannel.RemoteShare;
/* @internal */
export type strims_video_v1_VideoChannel_RemoteShare = VideoChannel.RemoteShare;
/* @internal */
export type strims_video_v1_VideoChannel_IRemoteShare = VideoChannel.IRemoteShare;

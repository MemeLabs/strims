// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  Certificate as strims_type_Certificate,
  type ICertificate as strims_type_ICertificate,
} from "../../type/certificate.pb";
import {
  Key as strims_type_Key,
  type IKey as strims_type_IKey,
} from "../../type/key.pb";
import {
  ListingSnippet as strims_network_v1_directory_ListingSnippet,
  type IListingSnippet as strims_network_v1_directory_IListingSnippet,
} from "../../network/v1/directory/directory.pb";

export type IVideoChannel = {
  id?: bigint;
  key?: strims_type_IKey;
  token?: Uint8Array;
  directoryListingSnippet?: strims_network_v1_directory_IListingSnippet;
  owner?: VideoChannel_IOwner
}

export class VideoChannel {
  id: bigint;
  key: strims_type_Key | undefined;
  token: Uint8Array;
  directoryListingSnippet: strims_network_v1_directory_ListingSnippet | undefined;
  owner: VideoChannel_TOwner;

  constructor(v?: IVideoChannel) {
    this.id = v?.id || BigInt(0);
    this.key = v?.key && new strims_type_Key(v.key);
    this.token = v?.token || new Uint8Array();
    this.directoryListingSnippet = v?.directoryListingSnippet && new strims_network_v1_directory_ListingSnippet(v.directoryListingSnippet);
    this.owner = v?.owner ? new VideoChannel_Owner(v.owner) : new VideoChannel_Owner();
  }

  static encode(m: VideoChannel, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.key) strims_type_Key.encode(m.key, w.uint32(18).fork()).ldelim();
    if (m.token.length) w.uint32(26).bytes(m.token);
    if (m.directoryListingSnippet) strims_network_v1_directory_ListingSnippet.encode(m.directoryListingSnippet, w.uint32(34).fork()).ldelim();
    switch (m.owner.case) {
      case VideoChannel_OwnerCase.LOCAL:
      VideoChannel_Local.encode(m.owner.local, w.uint32(8010).fork()).ldelim();
      break;
      case VideoChannel_OwnerCase.LOCAL_SHARE:
      VideoChannel_LocalShare.encode(m.owner.localShare, w.uint32(8018).fork()).ldelim();
      break;
      case VideoChannel_OwnerCase.REMOTE_SHARE:
      VideoChannel_RemoteShare.encode(m.owner.remoteShare, w.uint32(8026).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoChannel {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new VideoChannel();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 1001:
        m.owner = new VideoChannel_Owner({ local: VideoChannel_Local.decode(r, r.uint32()) });
        break;
        case 1002:
        m.owner = new VideoChannel_Owner({ localShare: VideoChannel_LocalShare.decode(r, r.uint32()) });
        break;
        case 1003:
        m.owner = new VideoChannel_Owner({ remoteShare: VideoChannel_RemoteShare.decode(r, r.uint32()) });
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export enum VideoChannel_OwnerCase {
  NOT_SET = 0,
  LOCAL = 1001,
  LOCAL_SHARE = 1002,
  REMOTE_SHARE = 1003,
}

export type VideoChannel_IOwner =
{ case?: VideoChannel_OwnerCase.NOT_SET }
|{ case?: VideoChannel_OwnerCase.LOCAL, local: VideoChannel_ILocal }
|{ case?: VideoChannel_OwnerCase.LOCAL_SHARE, localShare: VideoChannel_ILocalShare }
|{ case?: VideoChannel_OwnerCase.REMOTE_SHARE, remoteShare: VideoChannel_IRemoteShare }
;

export type VideoChannel_TOwner = Readonly<
{ case: VideoChannel_OwnerCase.NOT_SET }
|{ case: VideoChannel_OwnerCase.LOCAL, local: VideoChannel_Local }
|{ case: VideoChannel_OwnerCase.LOCAL_SHARE, localShare: VideoChannel_LocalShare }
|{ case: VideoChannel_OwnerCase.REMOTE_SHARE, remoteShare: VideoChannel_RemoteShare }
>;

class VideoChannel_OwnerImpl {
  declare local?: VideoChannel_Local;
  declare localShare?: VideoChannel_LocalShare;
  declare remoteShare?: VideoChannel_RemoteShare;
  case: VideoChannel_OwnerCase = VideoChannel_OwnerCase.NOT_SET;

  constructor(v?: VideoChannel_IOwner) {
    if (v && "local" in v) {
      this.case = VideoChannel_OwnerCase.LOCAL;
      this.local = new VideoChannel_Local(v.local);
    } else
    if (v && "localShare" in v) {
      this.case = VideoChannel_OwnerCase.LOCAL_SHARE;
      this.localShare = new VideoChannel_LocalShare(v.localShare);
    } else
    if (v && "remoteShare" in v) {
      this.case = VideoChannel_OwnerCase.REMOTE_SHARE;
      this.remoteShare = new VideoChannel_RemoteShare(v.remoteShare);
    }
  }
}

export const VideoChannel_Owner = VideoChannel_OwnerImpl as {
  new (): Readonly<{ case: VideoChannel_OwnerCase.NOT_SET }>;
  new <T extends VideoChannel_IOwner>(v: T): Readonly<
  T extends { local: VideoChannel_ILocal } ? { case: VideoChannel_OwnerCase.LOCAL, local: VideoChannel_Local } :
  T extends { localShare: VideoChannel_ILocalShare } ? { case: VideoChannel_OwnerCase.LOCAL_SHARE, localShare: VideoChannel_LocalShare } :
  T extends { remoteShare: VideoChannel_IRemoteShare } ? { case: VideoChannel_OwnerCase.REMOTE_SHARE, remoteShare: VideoChannel_RemoteShare } :
  never
  >;
};

export type VideoChannel_ILocal = {
  authKey?: Uint8Array;
  networkKey?: Uint8Array;
}

export class VideoChannel_Local {
  authKey: Uint8Array;
  networkKey: Uint8Array;

  constructor(v?: VideoChannel_ILocal) {
    this.authKey = v?.authKey || new Uint8Array();
    this.networkKey = v?.networkKey || new Uint8Array();
  }

  static encode(m: VideoChannel_Local, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.authKey.length) w.uint32(10).bytes(m.authKey);
    if (m.networkKey.length) w.uint32(18).bytes(m.networkKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoChannel_Local {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new VideoChannel_Local();
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type VideoChannel_ILocalShare = {
  certificate?: strims_type_ICertificate;
}

export class VideoChannel_LocalShare {
  certificate: strims_type_Certificate | undefined;

  constructor(v?: VideoChannel_ILocalShare) {
    this.certificate = v?.certificate && new strims_type_Certificate(v.certificate);
  }

  static encode(m: VideoChannel_LocalShare, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.certificate) strims_type_Certificate.encode(m.certificate, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoChannel_LocalShare {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new VideoChannel_LocalShare();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.certificate = strims_type_Certificate.decode(r, r.uint32());
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

export type VideoChannel_IRemoteShare = {
  id?: bigint;
  networkKey?: Uint8Array;
  serviceKey?: Uint8Array;
  serviceSalt?: Uint8Array;
  serverAddr?: string;
}

export class VideoChannel_RemoteShare {
  id: bigint;
  networkKey: Uint8Array;
  serviceKey: Uint8Array;
  serviceSalt: Uint8Array;
  serverAddr: string;

  constructor(v?: VideoChannel_IRemoteShare) {
    this.id = v?.id || BigInt(0);
    this.networkKey = v?.networkKey || new Uint8Array();
    this.serviceKey = v?.serviceKey || new Uint8Array();
    this.serviceSalt = v?.serviceSalt || new Uint8Array();
    this.serverAddr = v?.serverAddr || "";
  }

  static encode(m: VideoChannel_RemoteShare, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.networkKey.length) w.uint32(18).bytes(m.networkKey);
    if (m.serviceKey.length) w.uint32(26).bytes(m.serviceKey);
    if (m.serviceSalt.length) w.uint32(34).bytes(m.serviceSalt);
    if (m.serverAddr.length) w.uint32(42).string(m.serverAddr);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoChannel_RemoteShare {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new VideoChannel_RemoteShare();
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IVideoChannelListRequest = object;

export class VideoChannelListRequest {

  constructor(v?: IVideoChannelListRequest) {
    void v; // an empty message has no fields to set
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
  channels?: IVideoChannel[];
}

export class VideoChannelListResponse {
  channels: VideoChannel[];

  constructor(v?: IVideoChannelListResponse) {
    this.channels = v?.channels ? v.channels.map(v => new VideoChannel(v)) : [];
  }

  static encode(m: VideoChannelListResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.channels) VideoChannel.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoChannelListResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new VideoChannelListResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.channels.push(VideoChannel.decode(r, r.uint32()));
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new VideoChannelGetRequest();
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

export type IVideoChannelGetResponse = {
  channel?: IVideoChannel;
}

export class VideoChannelGetResponse {
  channel: VideoChannel | undefined;

  constructor(v?: IVideoChannelGetResponse) {
    this.channel = v?.channel && new VideoChannel(v.channel);
  }

  static encode(m: VideoChannelGetResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.channel) VideoChannel.encode(m.channel, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): VideoChannelGetResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new VideoChannelGetResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.channel = VideoChannel.decode(r, r.uint32());
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IVideoChannelCreateResponse = {
  channel?: IVideoChannel;
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new VideoChannelCreateResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.channel = VideoChannel.decode(r, r.uint32());
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IVideoChannelUpdateResponse = {
  channel?: IVideoChannel;
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new VideoChannelUpdateResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.channel = VideoChannel.decode(r, r.uint32());
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new VideoChannelDeleteRequest();
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

export type IVideoChannelDeleteResponse = object;

export class VideoChannelDeleteResponse {

  constructor(v?: IVideoChannelDeleteResponse) {
    void v; // an empty message has no fields to set
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


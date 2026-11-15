// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  Image as strims_type_Image,
  type IImage as strims_type_IImage,
} from "../../../type/image.pb";
import {
  BoolValue as google_protobuf_BoolValue,
  type IBoolValue as google_protobuf_IBoolValue,
  StringValue as google_protobuf_StringValue,
  type IStringValue as google_protobuf_IStringValue,
} from "../../../../google/protobuf/wrappers.pb";

export type IServerConfig = {
  integrations?: ServerConfig_IIntegrations;
  publishQuota?: number;
  joinQuota?: number;
  broadcastInterval?: number;
  refreshInterval?: number;
  sessionTimeout?: number;
  minPingInterval?: number;
  maxPingInterval?: number;
  embedLoadInterval?: number;
  loadMediaEmbedTimeout?: number;
}

export class ServerConfig {
  integrations: ServerConfig_Integrations | undefined;
  publishQuota: number;
  joinQuota: number;
  broadcastInterval: number;
  refreshInterval: number;
  sessionTimeout: number;
  minPingInterval: number;
  maxPingInterval: number;
  embedLoadInterval: number;
  loadMediaEmbedTimeout: number;

  constructor(v?: IServerConfig) {
    this.integrations = v?.integrations && new ServerConfig_Integrations(v.integrations);
    this.publishQuota = v?.publishQuota || 0;
    this.joinQuota = v?.joinQuota || 0;
    this.broadcastInterval = v?.broadcastInterval || 0;
    this.refreshInterval = v?.refreshInterval || 0;
    this.sessionTimeout = v?.sessionTimeout || 0;
    this.minPingInterval = v?.minPingInterval || 0;
    this.maxPingInterval = v?.maxPingInterval || 0;
    this.embedLoadInterval = v?.embedLoadInterval || 0;
    this.loadMediaEmbedTimeout = v?.loadMediaEmbedTimeout || 0;
  }

  static encode(m: ServerConfig, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.integrations) ServerConfig_Integrations.encode(m.integrations, w.uint32(10).fork()).ldelim();
    if (m.publishQuota) w.uint32(16).uint32(m.publishQuota);
    if (m.joinQuota) w.uint32(24).uint32(m.joinQuota);
    if (m.broadcastInterval) w.uint32(32).uint32(m.broadcastInterval);
    if (m.refreshInterval) w.uint32(40).uint32(m.refreshInterval);
    if (m.sessionTimeout) w.uint32(48).uint32(m.sessionTimeout);
    if (m.minPingInterval) w.uint32(56).uint32(m.minPingInterval);
    if (m.maxPingInterval) w.uint32(64).uint32(m.maxPingInterval);
    if (m.embedLoadInterval) w.uint32(72).uint32(m.embedLoadInterval);
    if (m.loadMediaEmbedTimeout) w.uint32(80).uint32(m.loadMediaEmbedTimeout);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ServerConfig {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ServerConfig();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.integrations = ServerConfig_Integrations.decode(r, r.uint32());
        break;
        case 2:
        m.publishQuota = r.uint32();
        break;
        case 3:
        m.joinQuota = r.uint32();
        break;
        case 4:
        m.broadcastInterval = r.uint32();
        break;
        case 5:
        m.refreshInterval = r.uint32();
        break;
        case 6:
        m.sessionTimeout = r.uint32();
        break;
        case 7:
        m.minPingInterval = r.uint32();
        break;
        case 8:
        m.maxPingInterval = r.uint32();
        break;
        case 9:
        m.embedLoadInterval = r.uint32();
        break;
        case 10:
        m.loadMediaEmbedTimeout = r.uint32();
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

export type ServerConfig_IIntegrations = {
  angelthump?: ServerConfig_Integrations_IAngelThump;
  twitch?: ServerConfig_Integrations_ITwitch;
  youtube?: ServerConfig_Integrations_IYouTube;
  swarm?: ServerConfig_Integrations_ISwarm;
}

export class ServerConfig_Integrations {
  angelthump: ServerConfig_Integrations_AngelThump | undefined;
  twitch: ServerConfig_Integrations_Twitch | undefined;
  youtube: ServerConfig_Integrations_YouTube | undefined;
  swarm: ServerConfig_Integrations_Swarm | undefined;

  constructor(v?: ServerConfig_IIntegrations) {
    this.angelthump = v?.angelthump && new ServerConfig_Integrations_AngelThump(v.angelthump);
    this.twitch = v?.twitch && new ServerConfig_Integrations_Twitch(v.twitch);
    this.youtube = v?.youtube && new ServerConfig_Integrations_YouTube(v.youtube);
    this.swarm = v?.swarm && new ServerConfig_Integrations_Swarm(v.swarm);
  }

  static encode(m: ServerConfig_Integrations, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.angelthump) ServerConfig_Integrations_AngelThump.encode(m.angelthump, w.uint32(10).fork()).ldelim();
    if (m.twitch) ServerConfig_Integrations_Twitch.encode(m.twitch, w.uint32(18).fork()).ldelim();
    if (m.youtube) ServerConfig_Integrations_YouTube.encode(m.youtube, w.uint32(26).fork()).ldelim();
    if (m.swarm) ServerConfig_Integrations_Swarm.encode(m.swarm, w.uint32(34).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ServerConfig_Integrations {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ServerConfig_Integrations();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.angelthump = ServerConfig_Integrations_AngelThump.decode(r, r.uint32());
        break;
        case 2:
        m.twitch = ServerConfig_Integrations_Twitch.decode(r, r.uint32());
        break;
        case 3:
        m.youtube = ServerConfig_Integrations_YouTube.decode(r, r.uint32());
        break;
        case 4:
        m.swarm = ServerConfig_Integrations_Swarm.decode(r, r.uint32());
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

export type ServerConfig_Integrations_IAngelThump = {
  enable?: boolean;
}

export class ServerConfig_Integrations_AngelThump {
  enable: boolean;

  constructor(v?: ServerConfig_Integrations_IAngelThump) {
    this.enable = v?.enable || false;
  }

  static encode(m: ServerConfig_Integrations_AngelThump, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.enable) w.uint32(8).bool(m.enable);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ServerConfig_Integrations_AngelThump {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ServerConfig_Integrations_AngelThump();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.enable = r.bool();
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

export type ServerConfig_Integrations_ITwitch = {
  enable?: boolean;
  clientId?: string;
  clientSecret?: string;
}

export class ServerConfig_Integrations_Twitch {
  enable: boolean;
  clientId: string;
  clientSecret: string;

  constructor(v?: ServerConfig_Integrations_ITwitch) {
    this.enable = v?.enable || false;
    this.clientId = v?.clientId || "";
    this.clientSecret = v?.clientSecret || "";
  }

  static encode(m: ServerConfig_Integrations_Twitch, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.enable) w.uint32(8).bool(m.enable);
    if (m.clientId.length) w.uint32(18).string(m.clientId);
    if (m.clientSecret.length) w.uint32(26).string(m.clientSecret);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ServerConfig_Integrations_Twitch {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ServerConfig_Integrations_Twitch();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.enable = r.bool();
        break;
        case 2:
        m.clientId = r.string();
        break;
        case 3:
        m.clientSecret = r.string();
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

export type ServerConfig_Integrations_IYouTube = {
  enable?: boolean;
  publicApiKey?: string;
}

export class ServerConfig_Integrations_YouTube {
  enable: boolean;
  publicApiKey: string;

  constructor(v?: ServerConfig_Integrations_IYouTube) {
    this.enable = v?.enable || false;
    this.publicApiKey = v?.publicApiKey || "";
  }

  static encode(m: ServerConfig_Integrations_YouTube, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.enable) w.uint32(8).bool(m.enable);
    if (m.publicApiKey.length) w.uint32(18).string(m.publicApiKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ServerConfig_Integrations_YouTube {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ServerConfig_Integrations_YouTube();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.enable = r.bool();
        break;
        case 2:
        m.publicApiKey = r.string();
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

export type ServerConfig_Integrations_ISwarm = {
  enable?: boolean;
}

export class ServerConfig_Integrations_Swarm {
  enable: boolean;

  constructor(v?: ServerConfig_Integrations_ISwarm) {
    this.enable = v?.enable || false;
  }

  static encode(m: ServerConfig_Integrations_Swarm, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.enable) w.uint32(8).bool(m.enable);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ServerConfig_Integrations_Swarm {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ServerConfig_Integrations_Swarm();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.enable = r.bool();
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

export type IClientConfig = {
  integrations?: ClientConfig_IIntegrations;
  publishQuota?: number;
  joinQuota?: number;
  minPingInterval?: number;
  maxPingInterval?: number;
}

export class ClientConfig {
  integrations: ClientConfig_Integrations | undefined;
  publishQuota: number;
  joinQuota: number;
  minPingInterval: number;
  maxPingInterval: number;

  constructor(v?: IClientConfig) {
    this.integrations = v?.integrations && new ClientConfig_Integrations(v.integrations);
    this.publishQuota = v?.publishQuota || 0;
    this.joinQuota = v?.joinQuota || 0;
    this.minPingInterval = v?.minPingInterval || 0;
    this.maxPingInterval = v?.maxPingInterval || 0;
  }

  static encode(m: ClientConfig, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.integrations) ClientConfig_Integrations.encode(m.integrations, w.uint32(10).fork()).ldelim();
    if (m.publishQuota) w.uint32(16).uint32(m.publishQuota);
    if (m.joinQuota) w.uint32(24).uint32(m.joinQuota);
    if (m.minPingInterval) w.uint32(32).uint32(m.minPingInterval);
    if (m.maxPingInterval) w.uint32(40).uint32(m.maxPingInterval);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ClientConfig {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ClientConfig();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.integrations = ClientConfig_Integrations.decode(r, r.uint32());
        break;
        case 2:
        m.publishQuota = r.uint32();
        break;
        case 3:
        m.joinQuota = r.uint32();
        break;
        case 4:
        m.minPingInterval = r.uint32();
        break;
        case 5:
        m.maxPingInterval = r.uint32();
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

export type ClientConfig_IIntegrations = {
  angelthump?: boolean;
  twitch?: boolean;
  youtube?: boolean;
  swarm?: boolean;
}

export class ClientConfig_Integrations {
  angelthump: boolean;
  twitch: boolean;
  youtube: boolean;
  swarm: boolean;

  constructor(v?: ClientConfig_IIntegrations) {
    this.angelthump = v?.angelthump || false;
    this.twitch = v?.twitch || false;
    this.youtube = v?.youtube || false;
    this.swarm = v?.swarm || false;
  }

  static encode(m: ClientConfig_Integrations, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.angelthump) w.uint32(8).bool(m.angelthump);
    if (m.twitch) w.uint32(16).bool(m.twitch);
    if (m.youtube) w.uint32(24).bool(m.youtube);
    if (m.swarm) w.uint32(32).bool(m.swarm);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ClientConfig_Integrations {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ClientConfig_Integrations();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.angelthump = r.bool();
        break;
        case 2:
        m.twitch = r.bool();
        break;
        case 3:
        m.youtube = r.bool();
        break;
        case 4:
        m.swarm = r.bool();
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

export type IGetEventsRequest = {
  networkKey?: Uint8Array;
}

export class GetEventsRequest {
  networkKey: Uint8Array;

  constructor(v?: IGetEventsRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
  }

  static encode(m: GetEventsRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetEventsRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new GetEventsRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
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

export type ITestPublishRequest = {
  networkKey?: Uint8Array;
}

export class TestPublishRequest {
  networkKey: Uint8Array;

  constructor(v?: ITestPublishRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
  }

  static encode(m: TestPublishRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): TestPublishRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new TestPublishRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
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

export type ITestPublishResponse = object;

export class TestPublishResponse {

  constructor(v?: ITestPublishResponse) {
    void v; // an empty message has no fields to set
  }

  static encode(m: TestPublishResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): TestPublishResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new TestPublishResponse();
  }
}

export type IListing = {
  content?: Listing_IContent
}

export class Listing {
  content: Listing_TContent;

  constructor(v?: IListing) {
    this.content = v?.content ? new Listing_Content(v.content) : new Listing_Content();
  }

  static encode(m: Listing, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.content.case) {
      case Listing_ContentCase.MEDIA:
      Listing_Media.encode(m.content.media, w.uint32(8010).fork()).ldelim();
      break;
      case Listing_ContentCase.SERVICE:
      Listing_Service.encode(m.content.service, w.uint32(8018).fork()).ldelim();
      break;
      case Listing_ContentCase.EMBED:
      Listing_Embed.encode(m.content.embed, w.uint32(8026).fork()).ldelim();
      break;
      case Listing_ContentCase.CHAT:
      Listing_Chat.encode(m.content.chat, w.uint32(8034).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Listing {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Listing();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1001:
        m.content = new Listing_Content({ media: Listing_Media.decode(r, r.uint32()) });
        break;
        case 1002:
        m.content = new Listing_Content({ service: Listing_Service.decode(r, r.uint32()) });
        break;
        case 1003:
        m.content = new Listing_Content({ embed: Listing_Embed.decode(r, r.uint32()) });
        break;
        case 1004:
        m.content = new Listing_Content({ chat: Listing_Chat.decode(r, r.uint32()) });
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

export enum Listing_ContentCase {
  NOT_SET = 0,
  MEDIA = 1001,
  SERVICE = 1002,
  EMBED = 1003,
  CHAT = 1004,
}

export type Listing_IContent =
{ case?: Listing_ContentCase.NOT_SET }
|{ case?: Listing_ContentCase.MEDIA, media: Listing_IMedia }
|{ case?: Listing_ContentCase.SERVICE, service: Listing_IService }
|{ case?: Listing_ContentCase.EMBED, embed: Listing_IEmbed }
|{ case?: Listing_ContentCase.CHAT, chat: Listing_IChat }
;

export type Listing_TContent = Readonly<
{ case: Listing_ContentCase.NOT_SET }
|{ case: Listing_ContentCase.MEDIA, media: Listing_Media }
|{ case: Listing_ContentCase.SERVICE, service: Listing_Service }
|{ case: Listing_ContentCase.EMBED, embed: Listing_Embed }
|{ case: Listing_ContentCase.CHAT, chat: Listing_Chat }
>;

class Listing_ContentImpl {
  declare media?: Listing_Media;
  declare service?: Listing_Service;
  declare embed?: Listing_Embed;
  declare chat?: Listing_Chat;
  case: Listing_ContentCase = Listing_ContentCase.NOT_SET;

  constructor(v?: Listing_IContent) {
    if (v && "media" in v) {
      this.case = Listing_ContentCase.MEDIA;
      this.media = new Listing_Media(v.media);
    } else
    if (v && "service" in v) {
      this.case = Listing_ContentCase.SERVICE;
      this.service = new Listing_Service(v.service);
    } else
    if (v && "embed" in v) {
      this.case = Listing_ContentCase.EMBED;
      this.embed = new Listing_Embed(v.embed);
    } else
    if (v && "chat" in v) {
      this.case = Listing_ContentCase.CHAT;
      this.chat = new Listing_Chat(v.chat);
    }
  }
}

export const Listing_Content = Listing_ContentImpl as {
  new (): Readonly<{ case: Listing_ContentCase.NOT_SET }>;
  new <T extends Listing_IContent>(v: T): Readonly<
  T extends { media: Listing_IMedia } ? { case: Listing_ContentCase.MEDIA, media: Listing_Media } :
  T extends { service: Listing_IService } ? { case: Listing_ContentCase.SERVICE, service: Listing_Service } :
  T extends { embed: Listing_IEmbed } ? { case: Listing_ContentCase.EMBED, embed: Listing_Embed } :
  T extends { chat: Listing_IChat } ? { case: Listing_ContentCase.CHAT, chat: Listing_Chat } :
  never
  >;
};

export type Listing_IMedia = {
  mimeType?: string;
  swarmUri?: string;
}

export class Listing_Media {
  mimeType: string;
  swarmUri: string;

  constructor(v?: Listing_IMedia) {
    this.mimeType = v?.mimeType || "";
    this.swarmUri = v?.swarmUri || "";
  }

  static encode(m: Listing_Media, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.mimeType.length) w.uint32(10).string(m.mimeType);
    if (m.swarmUri.length) w.uint32(18).string(m.swarmUri);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Listing_Media {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Listing_Media();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.mimeType = r.string();
        break;
        case 2:
        m.swarmUri = r.string();
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

export type Listing_IService = {
  type?: string;
  swarmUri?: string;
}

export class Listing_Service {
  type: string;
  swarmUri: string;

  constructor(v?: Listing_IService) {
    this.type = v?.type || "";
    this.swarmUri = v?.swarmUri || "";
  }

  static encode(m: Listing_Service, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.type.length) w.uint32(10).string(m.type);
    if (m.swarmUri.length) w.uint32(18).string(m.swarmUri);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Listing_Service {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Listing_Service();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.type = r.string();
        break;
        case 2:
        m.swarmUri = r.string();
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

export type Listing_IEmbed = {
  service?: Listing_Embed_Service;
  id?: string;
  queryParams?: Map<string, string> | { [key: string]: string };
}

export class Listing_Embed {
  service: Listing_Embed_Service;
  id: string;
  queryParams: Map<string, string>;

  constructor(v?: Listing_IEmbed) {
    this.service = v?.service || 0;
    this.id = v?.id || "";
    if (v?.queryParams) this.queryParams = new Map(v.queryParams instanceof Map ? v.queryParams : Object.entries(v.queryParams).map(([k, v]) => [String(k), v]));
    else this.queryParams = new Map<string, string>();
  }

  static encode(m: Listing_Embed, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.service) w.uint32(8).uint32(m.service);
    if (m.id.length) w.uint32(18).string(m.id);
    for (const [k, v] of m.queryParams) w.uint32(26).fork().uint32(10).string(k).uint32(18).string(v).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Listing_Embed {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Listing_Embed();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.service = r.uint32();
        break;
        case 2:
        m.id = r.string();
        break;
        case 3:
        {
          const entryParentEnd = r.enter(r.uint32());
          const fend = r.end;
          let key: string = "";
          let value: string = "";
          while (r.pos < fend) {
            const ftag = r.uint32();
            switch (ftag >> 3) {
              case 1:
              key = r.string();
              break;
              case 2:
              value = r.string();
              break;
              default:
              r.skipType(ftag & 7, ftag >>> 3);
              break;
            }
          }
          r.leave(entryParentEnd);
          m.queryParams.set(key, value);
        }
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

export enum Listing_Embed_Service {
  DIRECTORY_LISTING_EMBED_SERVICE_UNDEFINED = 0,
  DIRECTORY_LISTING_EMBED_SERVICE_ANGELTHUMP = 1,
  DIRECTORY_LISTING_EMBED_SERVICE_TWITCH_STREAM = 2,
  DIRECTORY_LISTING_EMBED_SERVICE_TWITCH_VOD = 3,
  DIRECTORY_LISTING_EMBED_SERVICE_YOUTUBE = 4,
}

export type Listing_IChat = {
  key?: Uint8Array;
  name?: string;
}

export class Listing_Chat {
  key: Uint8Array;
  name: string;

  constructor(v?: Listing_IChat) {
    this.key = v?.key || new Uint8Array();
    this.name = v?.name || "";
  }

  static encode(m: Listing_Chat, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.key.length) w.uint32(10).bytes(m.key);
    if (m.name.length) w.uint32(18).string(m.name);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Listing_Chat {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Listing_Chat();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.key = r.bytes();
        break;
        case 2:
        m.name = r.string();
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

export type IListingSnippetImage = {
  sourceOneof?: ListingSnippetImage_ISourceOneof
}

export class ListingSnippetImage {
  sourceOneof: ListingSnippetImage_TSourceOneof;

  constructor(v?: IListingSnippetImage) {
    this.sourceOneof = v?.sourceOneof ? new ListingSnippetImage_SourceOneof(v.sourceOneof) : new ListingSnippetImage_SourceOneof();
  }

  static encode(m: ListingSnippetImage, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.sourceOneof.case) {
      case ListingSnippetImage_SourceOneofCase.URL:
      w.uint32(8010).string(m.sourceOneof.url);
      break;
      case ListingSnippetImage_SourceOneofCase.IMAGE:
      strims_type_Image.encode(m.sourceOneof.image, w.uint32(8018).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListingSnippetImage {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ListingSnippetImage();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1001:
        m.sourceOneof = new ListingSnippetImage_SourceOneof({ url: r.string() });
        break;
        case 1002:
        m.sourceOneof = new ListingSnippetImage_SourceOneof({ image: strims_type_Image.decode(r, r.uint32()) });
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

export enum ListingSnippetImage_SourceOneofCase {
  NOT_SET = 0,
  URL = 1001,
  IMAGE = 1002,
}

export type ListingSnippetImage_ISourceOneof =
{ case?: ListingSnippetImage_SourceOneofCase.NOT_SET }
|{ case?: ListingSnippetImage_SourceOneofCase.URL, url: string }
|{ case?: ListingSnippetImage_SourceOneofCase.IMAGE, image: strims_type_IImage }
;

export type ListingSnippetImage_TSourceOneof = Readonly<
{ case: ListingSnippetImage_SourceOneofCase.NOT_SET }
|{ case: ListingSnippetImage_SourceOneofCase.URL, url: string }
|{ case: ListingSnippetImage_SourceOneofCase.IMAGE, image: strims_type_Image }
>;

class ListingSnippetImage_SourceOneofImpl {
  declare url?: string;
  declare image?: strims_type_Image;
  case: ListingSnippetImage_SourceOneofCase = ListingSnippetImage_SourceOneofCase.NOT_SET;

  constructor(v?: ListingSnippetImage_ISourceOneof) {
    if (v && "url" in v) {
      this.case = ListingSnippetImage_SourceOneofCase.URL;
      this.url = v.url;
    } else
    if (v && "image" in v) {
      this.case = ListingSnippetImage_SourceOneofCase.IMAGE;
      this.image = new strims_type_Image(v.image);
    }
  }
}

export const ListingSnippetImage_SourceOneof = ListingSnippetImage_SourceOneofImpl as {
  new (): Readonly<{ case: ListingSnippetImage_SourceOneofCase.NOT_SET }>;
  new <T extends ListingSnippetImage_ISourceOneof>(v: T): Readonly<
  T extends { url: string } ? { case: ListingSnippetImage_SourceOneofCase.URL, url: string } :
  T extends { image: strims_type_IImage } ? { case: ListingSnippetImage_SourceOneofCase.IMAGE, image: strims_type_Image } :
  never
  >;
};

export type IListingSnippet = {
  title?: string;
  description?: string;
  tags?: string[];
  category?: string;
  channelName?: string;
  userCount?: bigint;
  live?: boolean;
  isMature?: boolean;
  thumbnail?: IListingSnippetImage;
  channelLogo?: IListingSnippetImage;
  videoHeight?: number;
  videoWidth?: number;
  themeColor?: number;
  startTime?: bigint;
  /**
   * A media listing's renditions: every swarm its stream is published in,
   * the listing's own among them, each another encoding of the stream on
   * its timeline; empty for a stream of one. The snippet is signed by the
   * listing's swarm key, which so binds the renditions to the stream.
   */
  renditions?: IListingRendition[];
}

export class ListingSnippet {
  title: string;
  description: string;
  tags: string[];
  category: string;
  channelName: string;
  userCount: bigint;
  live: boolean;
  isMature: boolean;
  thumbnail: ListingSnippetImage | undefined;
  channelLogo: ListingSnippetImage | undefined;
  videoHeight: number;
  videoWidth: number;
  themeColor: number;
  startTime: bigint;
  /**
   * A media listing's renditions: every swarm its stream is published in,
   * the listing's own among them, each another encoding of the stream on
   * its timeline; empty for a stream of one. The snippet is signed by the
   * listing's swarm key, which so binds the renditions to the stream.
   */
  renditions: ListingRendition[];

  constructor(v?: IListingSnippet) {
    this.title = v?.title || "";
    this.description = v?.description || "";
    this.tags = v?.tags ? v.tags : [];
    this.category = v?.category || "";
    this.channelName = v?.channelName || "";
    this.userCount = v?.userCount || BigInt(0);
    this.live = v?.live || false;
    this.isMature = v?.isMature || false;
    this.thumbnail = v?.thumbnail && new ListingSnippetImage(v.thumbnail);
    this.channelLogo = v?.channelLogo && new ListingSnippetImage(v.channelLogo);
    this.videoHeight = v?.videoHeight || 0;
    this.videoWidth = v?.videoWidth || 0;
    this.themeColor = v?.themeColor || 0;
    this.startTime = v?.startTime || BigInt(0);
    this.renditions = v?.renditions ? v.renditions.map(v => new ListingRendition(v)) : [];
  }

  static encode(m: ListingSnippet, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.title.length) w.uint32(10).string(m.title);
    if (m.description.length) w.uint32(18).string(m.description);
    for (const v of m.tags) w.uint32(26).string(v);
    if (m.category.length) w.uint32(34).string(m.category);
    if (m.channelName.length) w.uint32(42).string(m.channelName);
    if (m.userCount) w.uint32(48).uint64(m.userCount);
    if (m.live) w.uint32(56).bool(m.live);
    if (m.isMature) w.uint32(64).bool(m.isMature);
    if (m.thumbnail) ListingSnippetImage.encode(m.thumbnail, w.uint32(74).fork()).ldelim();
    if (m.channelLogo) ListingSnippetImage.encode(m.channelLogo, w.uint32(82).fork()).ldelim();
    if (m.videoHeight) w.uint32(88).uint32(m.videoHeight);
    if (m.videoWidth) w.uint32(96).uint32(m.videoWidth);
    if (m.themeColor) w.uint32(109).fixed32(m.themeColor);
    if (m.startTime) w.uint32(112).int64(m.startTime);
    for (const v of m.renditions) ListingRendition.encode(v, w.uint32(122).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListingSnippet {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ListingSnippet();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.title = r.string();
        break;
        case 2:
        m.description = r.string();
        break;
        case 3:
        m.tags.push(r.string())
        break;
        case 4:
        m.category = r.string();
        break;
        case 5:
        m.channelName = r.string();
        break;
        case 6:
        m.userCount = r.uint64();
        break;
        case 7:
        m.live = r.bool();
        break;
        case 8:
        m.isMature = r.bool();
        break;
        case 9:
        m.thumbnail = ListingSnippetImage.decode(r, r.uint32());
        break;
        case 10:
        m.channelLogo = ListingSnippetImage.decode(r, r.uint32());
        break;
        case 11:
        m.videoHeight = r.uint32();
        break;
        case 12:
        m.videoWidth = r.uint32();
        break;
        case 13:
        m.themeColor = r.fixed32();
        break;
        case 14:
        m.startTime = r.int64();
        break;
        case 15:
        m.renditions.push(ListingRendition.decode(r, r.uint32()));
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

/** One encoding of a media listing's stream, in a swarm of its own. */
export type IListingRendition = {
  /** The MIME type of its swarm's records, codecs included. */
  mimeType?: string;
  swarmUri?: string;
  /** Its nominal bits a second, audio included; 0 when unknown. */
  bitrate?: bigint;
  /** Its picture's size in pixels; 0 for audio alone. */
  width?: number;
  height?: number;
  /** Its frames a second; 0 when unknown. */
  frameRate?: number;
}

/** One encoding of a media listing's stream, in a swarm of its own. */
export class ListingRendition {
  /** The MIME type of its swarm's records, codecs included. */
  mimeType: string;
  swarmUri: string;
  /** Its nominal bits a second, audio included; 0 when unknown. */
  bitrate: bigint;
  /** Its picture's size in pixels; 0 for audio alone. */
  width: number;
  height: number;
  /** Its frames a second; 0 when unknown. */
  frameRate: number;

  constructor(v?: IListingRendition) {
    this.mimeType = v?.mimeType || "";
    this.swarmUri = v?.swarmUri || "";
    this.bitrate = v?.bitrate || BigInt(0);
    this.width = v?.width || 0;
    this.height = v?.height || 0;
    this.frameRate = v?.frameRate || 0;
  }

  static encode(m: ListingRendition, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.mimeType.length) w.uint32(10).string(m.mimeType);
    if (m.swarmUri.length) w.uint32(18).string(m.swarmUri);
    if (m.bitrate) w.uint32(24).uint64(m.bitrate);
    if (m.width) w.uint32(32).uint32(m.width);
    if (m.height) w.uint32(40).uint32(m.height);
    if (m.frameRate) w.uint32(49).double(m.frameRate);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListingRendition {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ListingRendition();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.mimeType = r.string();
        break;
        case 2:
        m.swarmUri = r.string();
        break;
        case 3:
        m.bitrate = r.uint64();
        break;
        case 4:
        m.width = r.uint32();
        break;
        case 5:
        m.height = r.uint32();
        break;
        case 6:
        m.frameRate = r.double();
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

/**
 * What a listing snippet states: its content without images, which travel
 * apart, named by digest. An empty digest names no image.
 */
export type IListingSnippetBody = {
  /** The content, with both images unset. */
  snippet?: IListingSnippet;
  /** The SHA-256 of the image's encoded ListingSnippetImage. */
  thumbnailDigest?: Uint8Array;
  channelLogoDigest?: Uint8Array;
  /** Unix nanoseconds when the publisher signed it. */
  timestamp?: bigint;
}

/**
 * What a listing snippet states: its content without images, which travel
 * apart, named by digest. An empty digest names no image.
 */
export class ListingSnippetBody {
  /** The content, with both images unset. */
  snippet: ListingSnippet | undefined;
  /** The SHA-256 of the image's encoded ListingSnippetImage. */
  thumbnailDigest: Uint8Array;
  channelLogoDigest: Uint8Array;
  /** Unix nanoseconds when the publisher signed it. */
  timestamp: bigint;

  constructor(v?: IListingSnippetBody) {
    this.snippet = v?.snippet && new ListingSnippet(v.snippet);
    this.thumbnailDigest = v?.thumbnailDigest || new Uint8Array();
    this.channelLogoDigest = v?.channelLogoDigest || new Uint8Array();
    this.timestamp = v?.timestamp || BigInt(0);
  }

  static encode(m: ListingSnippetBody, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.snippet) ListingSnippet.encode(m.snippet, w.uint32(10).fork()).ldelim();
    if (m.thumbnailDigest.length) w.uint32(18).bytes(m.thumbnailDigest);
    if (m.channelLogoDigest.length) w.uint32(26).bytes(m.channelLogoDigest);
    if (m.timestamp) w.uint32(32).int64(m.timestamp);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListingSnippetBody {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ListingSnippetBody();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.snippet = ListingSnippet.decode(r, r.uint32());
        break;
        case 2:
        m.thumbnailDigest = r.bytes();
        break;
        case 3:
        m.channelLogoDigest = r.bytes();
        break;
        case 4:
        m.timestamp = r.int64();
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

/**
 * A ListingSnippetBody's encoding, signed in the directory-listing-snippet
 * domain by the listing's media swarm key: the key whose public half is
 * the swarm id.
 */
export type ISignedListingSnippet = {
  body?: Uint8Array;
  signature?: Uint8Array;
}

/**
 * A ListingSnippetBody's encoding, signed in the directory-listing-snippet
 * domain by the listing's media swarm key: the key whose public half is
 * the swarm id.
 */
export class SignedListingSnippet {
  body: Uint8Array;
  signature: Uint8Array;

  constructor(v?: ISignedListingSnippet) {
    this.body = v?.body || new Uint8Array();
    this.signature = v?.signature || new Uint8Array();
  }

  static encode(m: SignedListingSnippet, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.body.length) w.uint32(10).bytes(m.body);
    if (m.signature.length) w.uint32(18).bytes(m.signature);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SignedListingSnippet {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new SignedListingSnippet();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.body = r.bytes();
        break;
        case 2:
        m.signature = r.bytes();
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

export type IEvent = {
  body?: Event_IBody
}

export class Event {
  body: Event_TBody;

  constructor(v?: IEvent) {
    this.body = v?.body ? new Event_Body(v.body) : new Event_Body();
  }

  static encode(m: Event, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case Event_BodyCase.LISTING_CHANGE:
      Event_ListingChange.encode(m.body.listingChange, w.uint32(8010).fork()).ldelim();
      break;
      case Event_BodyCase.UNPUBLISH:
      Event_Unpublish.encode(m.body.unpublish, w.uint32(8018).fork()).ldelim();
      break;
      case Event_BodyCase.USER_COUNT_CHANGE:
      Event_UserCountChange.encode(m.body.userCountChange, w.uint32(8026).fork()).ldelim();
      break;
      case Event_BodyCase.USER_PRESENCE_CHANGE:
      Event_UserPresenceChange.encode(m.body.userPresenceChange, w.uint32(8034).fork()).ldelim();
      break;
      case Event_BodyCase.PING:
      Event_Ping.encode(m.body.ping, w.uint32(8042).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Event {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Event();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1001:
        m.body = new Event_Body({ listingChange: Event_ListingChange.decode(r, r.uint32()) });
        break;
        case 1002:
        m.body = new Event_Body({ unpublish: Event_Unpublish.decode(r, r.uint32()) });
        break;
        case 1003:
        m.body = new Event_Body({ userCountChange: Event_UserCountChange.decode(r, r.uint32()) });
        break;
        case 1004:
        m.body = new Event_Body({ userPresenceChange: Event_UserPresenceChange.decode(r, r.uint32()) });
        break;
        case 1005:
        m.body = new Event_Body({ ping: Event_Ping.decode(r, r.uint32()) });
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

export enum Event_BodyCase {
  NOT_SET = 0,
  LISTING_CHANGE = 1001,
  UNPUBLISH = 1002,
  USER_COUNT_CHANGE = 1003,
  USER_PRESENCE_CHANGE = 1004,
  PING = 1005,
}

export type Event_IBody =
{ case?: Event_BodyCase.NOT_SET }
|{ case?: Event_BodyCase.LISTING_CHANGE, listingChange: Event_IListingChange }
|{ case?: Event_BodyCase.UNPUBLISH, unpublish: Event_IUnpublish }
|{ case?: Event_BodyCase.USER_COUNT_CHANGE, userCountChange: Event_IUserCountChange }
|{ case?: Event_BodyCase.USER_PRESENCE_CHANGE, userPresenceChange: Event_IUserPresenceChange }
|{ case?: Event_BodyCase.PING, ping: Event_IPing }
;

export type Event_TBody = Readonly<
{ case: Event_BodyCase.NOT_SET }
|{ case: Event_BodyCase.LISTING_CHANGE, listingChange: Event_ListingChange }
|{ case: Event_BodyCase.UNPUBLISH, unpublish: Event_Unpublish }
|{ case: Event_BodyCase.USER_COUNT_CHANGE, userCountChange: Event_UserCountChange }
|{ case: Event_BodyCase.USER_PRESENCE_CHANGE, userPresenceChange: Event_UserPresenceChange }
|{ case: Event_BodyCase.PING, ping: Event_Ping }
>;

class Event_BodyImpl {
  declare listingChange?: Event_ListingChange;
  declare unpublish?: Event_Unpublish;
  declare userCountChange?: Event_UserCountChange;
  declare userPresenceChange?: Event_UserPresenceChange;
  declare ping?: Event_Ping;
  case: Event_BodyCase = Event_BodyCase.NOT_SET;

  constructor(v?: Event_IBody) {
    if (v && "listingChange" in v) {
      this.case = Event_BodyCase.LISTING_CHANGE;
      this.listingChange = new Event_ListingChange(v.listingChange);
    } else
    if (v && "unpublish" in v) {
      this.case = Event_BodyCase.UNPUBLISH;
      this.unpublish = new Event_Unpublish(v.unpublish);
    } else
    if (v && "userCountChange" in v) {
      this.case = Event_BodyCase.USER_COUNT_CHANGE;
      this.userCountChange = new Event_UserCountChange(v.userCountChange);
    } else
    if (v && "userPresenceChange" in v) {
      this.case = Event_BodyCase.USER_PRESENCE_CHANGE;
      this.userPresenceChange = new Event_UserPresenceChange(v.userPresenceChange);
    } else
    if (v && "ping" in v) {
      this.case = Event_BodyCase.PING;
      this.ping = new Event_Ping(v.ping);
    }
  }
}

export const Event_Body = Event_BodyImpl as {
  new (): Readonly<{ case: Event_BodyCase.NOT_SET }>;
  new <T extends Event_IBody>(v: T): Readonly<
  T extends { listingChange: Event_IListingChange } ? { case: Event_BodyCase.LISTING_CHANGE, listingChange: Event_ListingChange } :
  T extends { unpublish: Event_IUnpublish } ? { case: Event_BodyCase.UNPUBLISH, unpublish: Event_Unpublish } :
  T extends { userCountChange: Event_IUserCountChange } ? { case: Event_BodyCase.USER_COUNT_CHANGE, userCountChange: Event_UserCountChange } :
  T extends { userPresenceChange: Event_IUserPresenceChange } ? { case: Event_BodyCase.USER_PRESENCE_CHANGE, userPresenceChange: Event_UserPresenceChange } :
  T extends { ping: Event_IPing } ? { case: Event_BodyCase.PING, ping: Event_Ping } :
  never
  >;
};

export type Event_IListingChange = {
  id?: bigint;
  listing?: IListing;
  /**
   * A media listing's snippet, as its publisher signed it. The snippets
   * the server makes itself, an embed's or the empty one of a listing
   * that has none, carry no signature.
   */
  snippet?: ISignedListingSnippet;
  moderation?: IListingModeration;
  /**
   * The encoded ListingSnippetImages the snippet's digests name, in any
   * order.
   */
  snippetImages?: Uint8Array[];
}

export class Event_ListingChange {
  id: bigint;
  listing: Listing | undefined;
  /**
   * A media listing's snippet, as its publisher signed it. The snippets
   * the server makes itself, an embed's or the empty one of a listing
   * that has none, carry no signature.
   */
  snippet: SignedListingSnippet | undefined;
  moderation: ListingModeration | undefined;
  /**
   * The encoded ListingSnippetImages the snippet's digests name, in any
   * order.
   */
  snippetImages: Uint8Array[];

  constructor(v?: Event_IListingChange) {
    this.id = v?.id || BigInt(0);
    this.listing = v?.listing && new Listing(v.listing);
    this.snippet = v?.snippet && new SignedListingSnippet(v.snippet);
    this.moderation = v?.moderation && new ListingModeration(v.moderation);
    this.snippetImages = v?.snippetImages ? v.snippetImages : [];
  }

  static encode(m: Event_ListingChange, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.listing) Listing.encode(m.listing, w.uint32(18).fork()).ldelim();
    if (m.snippet) SignedListingSnippet.encode(m.snippet, w.uint32(26).fork()).ldelim();
    if (m.moderation) ListingModeration.encode(m.moderation, w.uint32(34).fork()).ldelim();
    for (const v of m.snippetImages) w.uint32(42).bytes(v);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Event_ListingChange {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Event_ListingChange();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.listing = Listing.decode(r, r.uint32());
        break;
        case 3:
        m.snippet = SignedListingSnippet.decode(r, r.uint32());
        break;
        case 4:
        m.moderation = ListingModeration.decode(r, r.uint32());
        break;
        case 5:
        m.snippetImages.push(r.bytes())
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

export type Event_IUnpublish = {
  id?: bigint;
}

export class Event_Unpublish {
  id: bigint;

  constructor(v?: Event_IUnpublish) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: Event_Unpublish, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Event_Unpublish {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Event_Unpublish();
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

export type Event_IUserCountChange = {
  id?: bigint;
  userCount?: number;
  recentUserCount?: number;
}

export class Event_UserCountChange {
  id: bigint;
  userCount: number;
  recentUserCount: number;

  constructor(v?: Event_IUserCountChange) {
    this.id = v?.id || BigInt(0);
    this.userCount = v?.userCount || 0;
    this.recentUserCount = v?.recentUserCount || 0;
  }

  static encode(m: Event_UserCountChange, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.userCount) w.uint32(16).uint32(m.userCount);
    if (m.recentUserCount) w.uint32(24).uint32(m.recentUserCount);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Event_UserCountChange {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Event_UserCountChange();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.userCount = r.uint32();
        break;
        case 3:
        m.recentUserCount = r.uint32();
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

export type Event_IUserPresenceChange = {
  id?: bigint;
  alias?: string;
  peerKey?: Uint8Array;
  online?: boolean;
  listingIds?: bigint[];
}

export class Event_UserPresenceChange {
  id: bigint;
  alias: string;
  peerKey: Uint8Array;
  online: boolean;
  listingIds: bigint[];

  constructor(v?: Event_IUserPresenceChange) {
    this.id = v?.id || BigInt(0);
    this.alias = v?.alias || "";
    this.peerKey = v?.peerKey || new Uint8Array();
    this.online = v?.online || false;
    this.listingIds = v?.listingIds ? v.listingIds : [];
  }

  static encode(m: Event_UserPresenceChange, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.alias.length) w.uint32(18).string(m.alias);
    if (m.peerKey.length) w.uint32(26).bytes(m.peerKey);
    if (m.online) w.uint32(32).bool(m.online);
    m.listingIds.reduce((w, v) => w.uint64(v), w.uint32(42).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Event_UserPresenceChange {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Event_UserPresenceChange();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.alias = r.string();
        break;
        case 3:
        m.peerKey = r.bytes();
        break;
        case 4:
        m.online = r.bool();
        break;
        case 5:
        for (const flen = r.uint32(), fend = r.pos + flen; r.pos < fend;) m.listingIds.push(r.uint64());
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

export type Event_IPing = {
  time?: bigint;
}

export class Event_Ping {
  time: bigint;

  constructor(v?: Event_IPing) {
    this.time = v?.time || BigInt(0);
  }

  static encode(m: Event_Ping, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.time) w.uint32(8).int64(m.time);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Event_Ping {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Event_Ping();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.time = r.int64();
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

export type IListingModeration = {
  isMature?: google_protobuf_IBoolValue;
  isBanned?: google_protobuf_IBoolValue;
  category?: google_protobuf_IStringValue;
}

export class ListingModeration {
  isMature: google_protobuf_BoolValue | undefined;
  isBanned: google_protobuf_BoolValue | undefined;
  category: google_protobuf_StringValue | undefined;

  constructor(v?: IListingModeration) {
    this.isMature = v?.isMature && new google_protobuf_BoolValue(v.isMature);
    this.isBanned = v?.isBanned && new google_protobuf_BoolValue(v.isBanned);
    this.category = v?.category && new google_protobuf_StringValue(v.category);
  }

  static encode(m: ListingModeration, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.isMature) google_protobuf_BoolValue.encode(m.isMature, w.uint32(18).fork()).ldelim();
    if (m.isBanned) google_protobuf_BoolValue.encode(m.isBanned, w.uint32(26).fork()).ldelim();
    if (m.category) google_protobuf_StringValue.encode(m.category, w.uint32(34).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListingModeration {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ListingModeration();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 2:
        m.isMature = google_protobuf_BoolValue.decode(r, r.uint32());
        break;
        case 3:
        m.isBanned = google_protobuf_BoolValue.decode(r, r.uint32());
        break;
        case 4:
        m.category = google_protobuf_StringValue.decode(r, r.uint32());
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

export type IListingQuery = {
  query?: ListingQuery_IQuery
}

export class ListingQuery {
  query: ListingQuery_TQuery;

  constructor(v?: IListingQuery) {
    this.query = v?.query ? new ListingQuery_Query(v.query) : new ListingQuery_Query();
  }

  static encode(m: ListingQuery, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.query.case) {
      case ListingQuery_QueryCase.ID:
      w.uint32(8008).uint64(m.query.id);
      break;
      case ListingQuery_QueryCase.LISTING:
      Listing.encode(m.query.listing, w.uint32(8018).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListingQuery {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ListingQuery();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1001:
        m.query = new ListingQuery_Query({ id: r.uint64() });
        break;
        case 1002:
        m.query = new ListingQuery_Query({ listing: Listing.decode(r, r.uint32()) });
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

export enum ListingQuery_QueryCase {
  NOT_SET = 0,
  ID = 1001,
  LISTING = 1002,
}

export type ListingQuery_IQuery =
{ case?: ListingQuery_QueryCase.NOT_SET }
|{ case?: ListingQuery_QueryCase.ID, id: bigint }
|{ case?: ListingQuery_QueryCase.LISTING, listing: IListing }
;

export type ListingQuery_TQuery = Readonly<
{ case: ListingQuery_QueryCase.NOT_SET }
|{ case: ListingQuery_QueryCase.ID, id: bigint }
|{ case: ListingQuery_QueryCase.LISTING, listing: Listing }
>;

class ListingQuery_QueryImpl {
  declare id?: bigint;
  declare listing?: Listing;
  case: ListingQuery_QueryCase = ListingQuery_QueryCase.NOT_SET;

  constructor(v?: ListingQuery_IQuery) {
    if (v && "id" in v) {
      this.case = ListingQuery_QueryCase.ID;
      this.id = v.id;
    } else
    if (v && "listing" in v) {
      this.case = ListingQuery_QueryCase.LISTING;
      this.listing = new Listing(v.listing);
    }
  }
}

export const ListingQuery_Query = ListingQuery_QueryImpl as {
  new (): Readonly<{ case: ListingQuery_QueryCase.NOT_SET }>;
  new <T extends ListingQuery_IQuery>(v: T): Readonly<
  T extends { id: bigint } ? { case: ListingQuery_QueryCase.ID, id: bigint } :
  T extends { listing: IListing } ? { case: ListingQuery_QueryCase.LISTING, listing: Listing } :
  never
  >;
};

export type IListingRecord = {
  id?: bigint;
  networkId?: bigint;
  listing?: IListing;
  moderation?: IListingModeration;
  notes?: string;
}

export class ListingRecord {
  id: bigint;
  networkId: bigint;
  listing: Listing | undefined;
  moderation: ListingModeration | undefined;
  notes: string;

  constructor(v?: IListingRecord) {
    this.id = v?.id || BigInt(0);
    this.networkId = v?.networkId || BigInt(0);
    this.listing = v?.listing && new Listing(v.listing);
    this.moderation = v?.moderation && new ListingModeration(v.moderation);
    this.notes = v?.notes || "";
  }

  static encode(m: ListingRecord, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.networkId) w.uint32(16).uint64(m.networkId);
    if (m.listing) Listing.encode(m.listing, w.uint32(26).fork()).ldelim();
    if (m.moderation) ListingModeration.encode(m.moderation, w.uint32(34).fork()).ldelim();
    if (m.notes.length) w.uint32(42).string(m.notes);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListingRecord {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ListingRecord();
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
        m.listing = Listing.decode(r, r.uint32());
        break;
        case 4:
        m.moderation = ListingModeration.decode(r, r.uint32());
        break;
        case 5:
        m.notes = r.string();
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

export type IUserModeration = {
  disableJoin?: google_protobuf_IBoolValue;
  disablePublish?: google_protobuf_IBoolValue;
  isModerator?: google_protobuf_IBoolValue;
  isAdmin?: google_protobuf_IBoolValue;
}

export class UserModeration {
  disableJoin: google_protobuf_BoolValue | undefined;
  disablePublish: google_protobuf_BoolValue | undefined;
  isModerator: google_protobuf_BoolValue | undefined;
  isAdmin: google_protobuf_BoolValue | undefined;

  constructor(v?: IUserModeration) {
    this.disableJoin = v?.disableJoin && new google_protobuf_BoolValue(v.disableJoin);
    this.disablePublish = v?.disablePublish && new google_protobuf_BoolValue(v.disablePublish);
    this.isModerator = v?.isModerator && new google_protobuf_BoolValue(v.isModerator);
    this.isAdmin = v?.isAdmin && new google_protobuf_BoolValue(v.isAdmin);
  }

  static encode(m: UserModeration, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.disableJoin) google_protobuf_BoolValue.encode(m.disableJoin, w.uint32(10).fork()).ldelim();
    if (m.disablePublish) google_protobuf_BoolValue.encode(m.disablePublish, w.uint32(18).fork()).ldelim();
    if (m.isModerator) google_protobuf_BoolValue.encode(m.isModerator, w.uint32(26).fork()).ldelim();
    if (m.isAdmin) google_protobuf_BoolValue.encode(m.isAdmin, w.uint32(34).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UserModeration {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UserModeration();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.disableJoin = google_protobuf_BoolValue.decode(r, r.uint32());
        break;
        case 2:
        m.disablePublish = google_protobuf_BoolValue.decode(r, r.uint32());
        break;
        case 3:
        m.isModerator = google_protobuf_BoolValue.decode(r, r.uint32());
        break;
        case 4:
        m.isAdmin = google_protobuf_BoolValue.decode(r, r.uint32());
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

export type IUserRecord = {
  id?: bigint;
  networkId?: bigint;
  peerKey?: Uint8Array;
  moderation?: IUserModeration;
}

export class UserRecord {
  id: bigint;
  networkId: bigint;
  peerKey: Uint8Array;
  moderation: UserModeration | undefined;

  constructor(v?: IUserRecord) {
    this.id = v?.id || BigInt(0);
    this.networkId = v?.networkId || BigInt(0);
    this.peerKey = v?.peerKey || new Uint8Array();
    this.moderation = v?.moderation && new UserModeration(v.moderation);
  }

  static encode(m: UserRecord, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.networkId) w.uint32(16).uint64(m.networkId);
    if (m.peerKey.length) w.uint32(26).bytes(m.peerKey);
    if (m.moderation) UserModeration.encode(m.moderation, w.uint32(34).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UserRecord {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UserRecord();
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
        m.peerKey = r.bytes();
        break;
        case 4:
        m.moderation = UserModeration.decode(r, r.uint32());
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

export type IEventBroadcast = {
  events?: IEvent[];
}

export class EventBroadcast {
  events: Event[];

  constructor(v?: IEventBroadcast) {
    this.events = v?.events ? v.events.map(v => new Event(v)) : [];
  }

  static encode(m: EventBroadcast, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.events) Event.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): EventBroadcast {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new EventBroadcast();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.events.push(Event.decode(r, r.uint32()));
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

export type IAssetBundle = {
  icon?: strims_type_IImage;
  directory?: IClientConfig;
}

export class AssetBundle {
  icon: strims_type_Image | undefined;
  directory: ClientConfig | undefined;

  constructor(v?: IAssetBundle) {
    this.icon = v?.icon && new strims_type_Image(v.icon);
    this.directory = v?.directory && new ClientConfig(v.directory);
  }

  static encode(m: AssetBundle, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.icon) strims_type_Image.encode(m.icon, w.uint32(10).fork()).ldelim();
    if (m.directory) ClientConfig.encode(m.directory, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): AssetBundle {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new AssetBundle();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.icon = strims_type_Image.decode(r, r.uint32());
        break;
        case 2:
        m.directory = ClientConfig.decode(r, r.uint32());
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

export type IPublishRequest = {
  listing?: IListing;
}

export class PublishRequest {
  listing: Listing | undefined;

  constructor(v?: IPublishRequest) {
    this.listing = v?.listing && new Listing(v.listing);
  }

  static encode(m: PublishRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.listing) Listing.encode(m.listing, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PublishRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new PublishRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.listing = Listing.decode(r, r.uint32());
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

export type IPublishResponse = {
  id?: bigint;
}

export class PublishResponse {
  id: bigint;

  constructor(v?: IPublishResponse) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: PublishResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PublishResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new PublishResponse();
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

export type IUnpublishRequest = {
  id?: bigint;
}

export class UnpublishRequest {
  id: bigint;

  constructor(v?: IUnpublishRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: UnpublishRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UnpublishRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UnpublishRequest();
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

export type IUnpublishResponse = object;

export class UnpublishResponse {

  constructor(v?: IUnpublishResponse) {
    void v; // an empty message has no fields to set
  }

  static encode(m: UnpublishResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UnpublishResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new UnpublishResponse();
  }
}

export type IJoinRequest = {
  query?: IListingQuery;
}

export class JoinRequest {
  query: ListingQuery | undefined;

  constructor(v?: IJoinRequest) {
    this.query = v?.query && new ListingQuery(v.query);
  }

  static encode(m: JoinRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.query) ListingQuery.encode(m.query, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): JoinRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new JoinRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.query = ListingQuery.decode(r, r.uint32());
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

export type IJoinResponse = {
  id?: bigint;
}

export class JoinResponse {
  id: bigint;

  constructor(v?: IJoinResponse) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: JoinResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): JoinResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new JoinResponse();
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

export type IPartRequest = {
  id?: bigint;
}

export class PartRequest {
  id: bigint;

  constructor(v?: IPartRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: PartRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PartRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new PartRequest();
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

export type IPartResponse = object;

export class PartResponse {

  constructor(v?: IPartResponse) {
    void v; // an empty message has no fields to set
  }

  static encode(m: PartResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PartResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new PartResponse();
  }
}

export type IPingRequest = object;

export class PingRequest {

  constructor(v?: IPingRequest) {
    void v; // an empty message has no fields to set
  }

  static encode(m: PingRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PingRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new PingRequest();
  }
}

export type IPingResponse = object;

export class PingResponse {

  constructor(v?: IPingResponse) {
    void v; // an empty message has no fields to set
  }

  static encode(m: PingResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PingResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new PingResponse();
  }
}

export type IModerateListingRequest = {
  id?: bigint;
  moderation?: IListingModeration;
}

export class ModerateListingRequest {
  id: bigint;
  moderation: ListingModeration | undefined;

  constructor(v?: IModerateListingRequest) {
    this.id = v?.id || BigInt(0);
    this.moderation = v?.moderation && new ListingModeration(v.moderation);
  }

  static encode(m: ModerateListingRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.moderation) ListingModeration.encode(m.moderation, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ModerateListingRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ModerateListingRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.moderation = ListingModeration.decode(r, r.uint32());
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

export type IModerateListingResponse = object;

export class ModerateListingResponse {

  constructor(v?: IModerateListingResponse) {
    void v; // an empty message has no fields to set
  }

  static encode(m: ModerateListingResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ModerateListingResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new ModerateListingResponse();
  }
}

export type IModerateUserRequest = {
  peerKey?: Uint8Array;
  moderation?: IUserModeration;
}

export class ModerateUserRequest {
  peerKey: Uint8Array;
  moderation: UserModeration | undefined;

  constructor(v?: IModerateUserRequest) {
    this.peerKey = v?.peerKey || new Uint8Array();
    this.moderation = v?.moderation && new UserModeration(v.moderation);
  }

  static encode(m: ModerateUserRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.peerKey.length) w.uint32(10).bytes(m.peerKey);
    if (m.moderation) UserModeration.encode(m.moderation, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ModerateUserRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ModerateUserRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.peerKey = r.bytes();
        break;
        case 2:
        m.moderation = UserModeration.decode(r, r.uint32());
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

export type IModerateUserResponse = object;

export class ModerateUserResponse {

  constructor(v?: IModerateUserResponse) {
    void v; // an empty message has no fields to set
  }

  static encode(m: ModerateUserResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ModerateUserResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new ModerateUserResponse();
  }
}

export type INetwork = {
  id?: bigint;
  name?: string;
  key?: Uint8Array;
}

export class Network {
  id: bigint;
  name: string;
  key: Uint8Array;

  constructor(v?: INetwork) {
    this.id = v?.id || BigInt(0);
    this.name = v?.name || "";
    this.key = v?.key || new Uint8Array();
  }

  static encode(m: Network, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.name.length) w.uint32(18).string(m.name);
    if (m.key.length) w.uint32(26).bytes(m.key);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Network {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
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
        m.key = r.bytes();
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

export type INetworkListingsItem = {
  id?: bigint;
  listing?: IListing;
  snippet?: IListingSnippet;
  moderation?: IListingModeration;
  userCount?: number;
  recentUserCount?: number;
}

export class NetworkListingsItem {
  id: bigint;
  listing: Listing | undefined;
  snippet: ListingSnippet | undefined;
  moderation: ListingModeration | undefined;
  userCount: number;
  recentUserCount: number;

  constructor(v?: INetworkListingsItem) {
    this.id = v?.id || BigInt(0);
    this.listing = v?.listing && new Listing(v.listing);
    this.snippet = v?.snippet && new ListingSnippet(v.snippet);
    this.moderation = v?.moderation && new ListingModeration(v.moderation);
    this.userCount = v?.userCount || 0;
    this.recentUserCount = v?.recentUserCount || 0;
  }

  static encode(m: NetworkListingsItem, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.listing) Listing.encode(m.listing, w.uint32(18).fork()).ldelim();
    if (m.snippet) ListingSnippet.encode(m.snippet, w.uint32(26).fork()).ldelim();
    if (m.moderation) ListingModeration.encode(m.moderation, w.uint32(34).fork()).ldelim();
    if (m.userCount) w.uint32(40).uint32(m.userCount);
    if (m.recentUserCount) w.uint32(48).uint32(m.recentUserCount);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): NetworkListingsItem {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new NetworkListingsItem();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.listing = Listing.decode(r, r.uint32());
        break;
        case 3:
        m.snippet = ListingSnippet.decode(r, r.uint32());
        break;
        case 4:
        m.moderation = ListingModeration.decode(r, r.uint32());
        break;
        case 5:
        m.userCount = r.uint32();
        break;
        case 6:
        m.recentUserCount = r.uint32();
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

export type INetworkListings = {
  network?: INetwork;
  listings?: INetworkListingsItem[];
}

export class NetworkListings {
  network: Network | undefined;
  listings: NetworkListingsItem[];

  constructor(v?: INetworkListings) {
    this.network = v?.network && new Network(v.network);
    this.listings = v?.listings ? v.listings.map(v => new NetworkListingsItem(v)) : [];
  }

  static encode(m: NetworkListings, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.network) Network.encode(m.network, w.uint32(10).fork()).ldelim();
    for (const v of m.listings) NetworkListingsItem.encode(v, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): NetworkListings {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new NetworkListings();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.network = Network.decode(r, r.uint32());
        break;
        case 2:
        m.listings.push(NetworkListingsItem.decode(r, r.uint32()));
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

export type IFrontendPublishRequest = {
  networkKey?: Uint8Array;
  listing?: IListing;
}

export class FrontendPublishRequest {
  networkKey: Uint8Array;
  listing: Listing | undefined;

  constructor(v?: IFrontendPublishRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.listing = v?.listing && new Listing(v.listing);
  }

  static encode(m: FrontendPublishRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    if (m.listing) Listing.encode(m.listing, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendPublishRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new FrontendPublishRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkKey = r.bytes();
        break;
        case 2:
        m.listing = Listing.decode(r, r.uint32());
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

export type IFrontendPublishResponse = {
  id?: bigint;
}

export class FrontendPublishResponse {
  id: bigint;

  constructor(v?: IFrontendPublishResponse) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: FrontendPublishResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendPublishResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new FrontendPublishResponse();
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

export type IFrontendUnpublishRequest = {
  networkKey?: Uint8Array;
  id?: bigint;
}

export class FrontendUnpublishRequest {
  networkKey: Uint8Array;
  id: bigint;

  constructor(v?: IFrontendUnpublishRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.id = v?.id || BigInt(0);
  }

  static encode(m: FrontendUnpublishRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    if (m.id) w.uint32(16).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendUnpublishRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new FrontendUnpublishRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkKey = r.bytes();
        break;
        case 2:
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

export type IFrontendUnpublishResponse = object;

export class FrontendUnpublishResponse {

  constructor(v?: IFrontendUnpublishResponse) {
    void v; // an empty message has no fields to set
  }

  static encode(m: FrontendUnpublishResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendUnpublishResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new FrontendUnpublishResponse();
  }
}

export type IFrontendJoinRequest = {
  networkKey?: Uint8Array;
  query?: IListingQuery;
}

export class FrontendJoinRequest {
  networkKey: Uint8Array;
  query: ListingQuery | undefined;

  constructor(v?: IFrontendJoinRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.query = v?.query && new ListingQuery(v.query);
  }

  static encode(m: FrontendJoinRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    if (m.query) ListingQuery.encode(m.query, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendJoinRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new FrontendJoinRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkKey = r.bytes();
        break;
        case 2:
        m.query = ListingQuery.decode(r, r.uint32());
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

export type IFrontendJoinResponse = {
  id?: bigint;
}

export class FrontendJoinResponse {
  id: bigint;

  constructor(v?: IFrontendJoinResponse) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: FrontendJoinResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendJoinResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new FrontendJoinResponse();
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

export type IFrontendPartRequest = {
  networkKey?: Uint8Array;
  id?: bigint;
}

export class FrontendPartRequest {
  networkKey: Uint8Array;
  id: bigint;

  constructor(v?: IFrontendPartRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.id = v?.id || BigInt(0);
  }

  static encode(m: FrontendPartRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    if (m.id) w.uint32(16).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendPartRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new FrontendPartRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkKey = r.bytes();
        break;
        case 2:
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

export type IFrontendPartResponse = object;

export class FrontendPartResponse {

  constructor(v?: IFrontendPartResponse) {
    void v; // an empty message has no fields to set
  }

  static encode(m: FrontendPartResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendPartResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new FrontendPartResponse();
  }
}

export type IFrontendTestRequest = {
  networkKey?: Uint8Array;
}

export class FrontendTestRequest {
  networkKey: Uint8Array;

  constructor(v?: IFrontendTestRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
  }

  static encode(m: FrontendTestRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendTestRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new FrontendTestRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
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

export type IFrontendTestResponse = object;

export class FrontendTestResponse {

  constructor(v?: IFrontendTestResponse) {
    void v; // an empty message has no fields to set
  }

  static encode(m: FrontendTestResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendTestResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new FrontendTestResponse();
  }
}

export type IFrontendModerateListingRequest = {
  networkKey?: Uint8Array;
  id?: bigint;
  moderation?: IListingModeration;
}

export class FrontendModerateListingRequest {
  networkKey: Uint8Array;
  id: bigint;
  moderation: ListingModeration | undefined;

  constructor(v?: IFrontendModerateListingRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.id = v?.id || BigInt(0);
    this.moderation = v?.moderation && new ListingModeration(v.moderation);
  }

  static encode(m: FrontendModerateListingRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    if (m.id) w.uint32(16).uint64(m.id);
    if (m.moderation) ListingModeration.encode(m.moderation, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendModerateListingRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new FrontendModerateListingRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkKey = r.bytes();
        break;
        case 2:
        m.id = r.uint64();
        break;
        case 3:
        m.moderation = ListingModeration.decode(r, r.uint32());
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

export type IFrontendModerateListingResponse = object;

export class FrontendModerateListingResponse {

  constructor(v?: IFrontendModerateListingResponse) {
    void v; // an empty message has no fields to set
  }

  static encode(m: FrontendModerateListingResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendModerateListingResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new FrontendModerateListingResponse();
  }
}

export type IFrontendModerateUserRequest = {
  networkKey?: Uint8Array;
  alias?: string;
  moderation?: IUserModeration;
}

export class FrontendModerateUserRequest {
  networkKey: Uint8Array;
  alias: string;
  moderation: UserModeration | undefined;

  constructor(v?: IFrontendModerateUserRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.alias = v?.alias || "";
    this.moderation = v?.moderation && new UserModeration(v.moderation);
  }

  static encode(m: FrontendModerateUserRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    if (m.alias.length) w.uint32(18).string(m.alias);
    if (m.moderation) UserModeration.encode(m.moderation, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendModerateUserRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new FrontendModerateUserRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkKey = r.bytes();
        break;
        case 2:
        m.alias = r.string();
        break;
        case 3:
        m.moderation = UserModeration.decode(r, r.uint32());
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

export type IFrontendModerateUserResponse = object;

export class FrontendModerateUserResponse {

  constructor(v?: IFrontendModerateUserResponse) {
    void v; // an empty message has no fields to set
  }

  static encode(m: FrontendModerateUserResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendModerateUserResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new FrontendModerateUserResponse();
  }
}

export type IFrontendGetUsersRequest = object;

export class FrontendGetUsersRequest {

  constructor(v?: IFrontendGetUsersRequest) {
    void v; // an empty message has no fields to set
  }

  static encode(m: FrontendGetUsersRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendGetUsersRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new FrontendGetUsersRequest();
  }
}

export type IFrontendGetUsersResponse = {
  users?: FrontendGetUsersResponse_IUser[];
  networks?: Map<bigint, Network>;
}

export class FrontendGetUsersResponse {
  users: FrontendGetUsersResponse_User[];
  networks: Map<bigint, Network>;

  constructor(v?: IFrontendGetUsersResponse) {
    this.users = v?.users ? v.users.map(v => new FrontendGetUsersResponse_User(v)) : [];
    if (v?.networks) this.networks = new Map(Array.from(v.networks).map(([k, v]) => [k, new Network(v)]));
    else this.networks = new Map<bigint, Network>();
  }

  static encode(m: FrontendGetUsersResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.users) FrontendGetUsersResponse_User.encode(v, w.uint32(10).fork()).ldelim();
    for (const [k, v] of m.networks) Network.encode(v, w.uint32(18).fork().uint32(8).uint64(k).uint32(18).fork()).ldelim().ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendGetUsersResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new FrontendGetUsersResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.users.push(FrontendGetUsersResponse_User.decode(r, r.uint32()));
        break;
        case 2:
        {
          const entryParentEnd = r.enter(r.uint32());
          const fend = r.end;
          let key: bigint = BigInt(0);
          let value: Network | undefined;
          while (r.pos < fend) {
            const ftag = r.uint32();
            switch (ftag >> 3) {
              case 1:
              key = r.uint64();
              break;
              case 2:
              value = Network.decode(r, r.uint32());
              break;
              default:
              r.skipType(ftag & 7, ftag >>> 3);
              break;
            }
          }
          r.leave(entryParentEnd);
          m.networks.set(key, value ?? new Network());
        }
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

export type FrontendGetUsersResponse_IAlias = {
  alias?: string;
  networkIds?: bigint[];
}

export class FrontendGetUsersResponse_Alias {
  alias: string;
  networkIds: bigint[];

  constructor(v?: FrontendGetUsersResponse_IAlias) {
    this.alias = v?.alias || "";
    this.networkIds = v?.networkIds ? v.networkIds : [];
  }

  static encode(m: FrontendGetUsersResponse_Alias, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.alias.length) w.uint32(10).string(m.alias);
    m.networkIds.reduce((w, v) => w.uint64(v), w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendGetUsersResponse_Alias {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new FrontendGetUsersResponse_Alias();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.alias = r.string();
        break;
        case 2:
        for (const flen = r.uint32(), fend = r.pos + flen; r.pos < fend;) m.networkIds.push(r.uint64());
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

export type FrontendGetUsersResponse_IUser = {
  aliases?: FrontendGetUsersResponse_IAlias[];
  peerKey?: Uint8Array;
}

export class FrontendGetUsersResponse_User {
  aliases: FrontendGetUsersResponse_Alias[];
  peerKey: Uint8Array;

  constructor(v?: FrontendGetUsersResponse_IUser) {
    this.aliases = v?.aliases ? v.aliases.map(v => new FrontendGetUsersResponse_Alias(v)) : [];
    this.peerKey = v?.peerKey || new Uint8Array();
  }

  static encode(m: FrontendGetUsersResponse_User, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.aliases) FrontendGetUsersResponse_Alias.encode(v, w.uint32(10).fork()).ldelim();
    if (m.peerKey.length) w.uint32(18).bytes(m.peerKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendGetUsersResponse_User {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new FrontendGetUsersResponse_User();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.aliases.push(FrontendGetUsersResponse_Alias.decode(r, r.uint32()));
        break;
        case 2:
        m.peerKey = r.bytes();
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

export type IFrontendGetListingRequest = {
  query?: IListingQuery;
  networkKey?: Uint8Array;
}

export class FrontendGetListingRequest {
  query: ListingQuery | undefined;
  networkKey: Uint8Array;

  constructor(v?: IFrontendGetListingRequest) {
    this.query = v?.query && new ListingQuery(v.query);
    this.networkKey = v?.networkKey || new Uint8Array();
  }

  static encode(m: FrontendGetListingRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.query) ListingQuery.encode(m.query, w.uint32(10).fork()).ldelim();
    if (m.networkKey.length) w.uint32(18).bytes(m.networkKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendGetListingRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new FrontendGetListingRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.query = ListingQuery.decode(r, r.uint32());
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

export type IFrontendGetListingResponse = {
  id?: bigint;
  listing?: IListing;
  snippet?: IListingSnippet;
  moderation?: IListingModeration;
  userCount?: number;
  recentUserCount?: number;
}

export class FrontendGetListingResponse {
  id: bigint;
  listing: Listing | undefined;
  snippet: ListingSnippet | undefined;
  moderation: ListingModeration | undefined;
  userCount: number;
  recentUserCount: number;

  constructor(v?: IFrontendGetListingResponse) {
    this.id = v?.id || BigInt(0);
    this.listing = v?.listing && new Listing(v.listing);
    this.snippet = v?.snippet && new ListingSnippet(v.snippet);
    this.moderation = v?.moderation && new ListingModeration(v.moderation);
    this.userCount = v?.userCount || 0;
    this.recentUserCount = v?.recentUserCount || 0;
  }

  static encode(m: FrontendGetListingResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.listing) Listing.encode(m.listing, w.uint32(18).fork()).ldelim();
    if (m.snippet) ListingSnippet.encode(m.snippet, w.uint32(26).fork()).ldelim();
    if (m.moderation) ListingModeration.encode(m.moderation, w.uint32(34).fork()).ldelim();
    if (m.userCount) w.uint32(40).uint32(m.userCount);
    if (m.recentUserCount) w.uint32(48).uint32(m.recentUserCount);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendGetListingResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new FrontendGetListingResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.listing = Listing.decode(r, r.uint32());
        break;
        case 3:
        m.snippet = ListingSnippet.decode(r, r.uint32());
        break;
        case 4:
        m.moderation = ListingModeration.decode(r, r.uint32());
        break;
        case 5:
        m.userCount = r.uint32();
        break;
        case 6:
        m.recentUserCount = r.uint32();
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

export type IFrontendGetListingsRequest = {
  contentTypes?: ListingContentType[];
  networkKeys?: Uint8Array[];
}

export class FrontendGetListingsRequest {
  contentTypes: ListingContentType[];
  networkKeys: Uint8Array[];

  constructor(v?: IFrontendGetListingsRequest) {
    this.contentTypes = v?.contentTypes ? v.contentTypes : [];
    this.networkKeys = v?.networkKeys ? v.networkKeys : [];
  }

  static encode(m: FrontendGetListingsRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    m.contentTypes.reduce((w, v) => w.uint32(v), w.uint32(10).fork()).ldelim();
    for (const v of m.networkKeys) w.uint32(18).bytes(v);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendGetListingsRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new FrontendGetListingsRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        for (const flen = r.uint32(), fend = r.pos + flen; r.pos < fend;) m.contentTypes.push(r.uint32());
        break;
        case 2:
        m.networkKeys.push(r.bytes())
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

export type IFrontendGetListingsResponse = {
  listings?: INetworkListings[];
}

export class FrontendGetListingsResponse {
  listings: NetworkListings[];

  constructor(v?: IFrontendGetListingsResponse) {
    this.listings = v?.listings ? v.listings.map(v => new NetworkListings(v)) : [];
  }

  static encode(m: FrontendGetListingsResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.listings) NetworkListings.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendGetListingsResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new FrontendGetListingsResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.listings.push(NetworkListings.decode(r, r.uint32()));
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

export type IFrontendWatchListingsRequest = {
  contentTypes?: ListingContentType[];
  networkKeys?: Uint8Array[];
  listingId?: bigint;
}

export class FrontendWatchListingsRequest {
  contentTypes: ListingContentType[];
  networkKeys: Uint8Array[];
  listingId: bigint;

  constructor(v?: IFrontendWatchListingsRequest) {
    this.contentTypes = v?.contentTypes ? v.contentTypes : [];
    this.networkKeys = v?.networkKeys ? v.networkKeys : [];
    this.listingId = v?.listingId || BigInt(0);
  }

  static encode(m: FrontendWatchListingsRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    m.contentTypes.reduce((w, v) => w.uint32(v), w.uint32(10).fork()).ldelim();
    for (const v of m.networkKeys) w.uint32(18).bytes(v);
    if (m.listingId) w.uint32(24).uint64(m.listingId);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendWatchListingsRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new FrontendWatchListingsRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        for (const flen = r.uint32(), fend = r.pos + flen; r.pos < fend;) m.contentTypes.push(r.uint32());
        break;
        case 2:
        m.networkKeys.push(r.bytes())
        break;
        case 3:
        m.listingId = r.uint64();
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

export type IFrontendWatchListingsResponse = {
  events?: FrontendWatchListingsResponse_IEvent[];
}

export class FrontendWatchListingsResponse {
  events: FrontendWatchListingsResponse_Event[];

  constructor(v?: IFrontendWatchListingsResponse) {
    this.events = v?.events ? v.events.map(v => new FrontendWatchListingsResponse_Event(v)) : [];
  }

  static encode(m: FrontendWatchListingsResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.events) FrontendWatchListingsResponse_Event.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendWatchListingsResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new FrontendWatchListingsResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.events.push(FrontendWatchListingsResponse_Event.decode(r, r.uint32()));
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

export type FrontendWatchListingsResponse_IChange = {
  listings?: INetworkListings;
}

export class FrontendWatchListingsResponse_Change {
  listings: NetworkListings | undefined;

  constructor(v?: FrontendWatchListingsResponse_IChange) {
    this.listings = v?.listings && new NetworkListings(v.listings);
  }

  static encode(m: FrontendWatchListingsResponse_Change, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.listings) NetworkListings.encode(m.listings, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendWatchListingsResponse_Change {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new FrontendWatchListingsResponse_Change();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.listings = NetworkListings.decode(r, r.uint32());
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

export type FrontendWatchListingsResponse_IUnpublish = {
  networkId?: bigint;
  listingId?: bigint;
}

export class FrontendWatchListingsResponse_Unpublish {
  networkId: bigint;
  listingId: bigint;

  constructor(v?: FrontendWatchListingsResponse_IUnpublish) {
    this.networkId = v?.networkId || BigInt(0);
    this.listingId = v?.listingId || BigInt(0);
  }

  static encode(m: FrontendWatchListingsResponse_Unpublish, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkId) w.uint32(8).uint64(m.networkId);
    if (m.listingId) w.uint32(16).uint64(m.listingId);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendWatchListingsResponse_Unpublish {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new FrontendWatchListingsResponse_Unpublish();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkId = r.uint64();
        break;
        case 2:
        m.listingId = r.uint64();
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

export type FrontendWatchListingsResponse_IUserCountChange = {
  networkId?: bigint;
  listingId?: bigint;
  userCount?: number;
  recentUserCount?: number;
}

export class FrontendWatchListingsResponse_UserCountChange {
  networkId: bigint;
  listingId: bigint;
  userCount: number;
  recentUserCount: number;

  constructor(v?: FrontendWatchListingsResponse_IUserCountChange) {
    this.networkId = v?.networkId || BigInt(0);
    this.listingId = v?.listingId || BigInt(0);
    this.userCount = v?.userCount || 0;
    this.recentUserCount = v?.recentUserCount || 0;
  }

  static encode(m: FrontendWatchListingsResponse_UserCountChange, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkId) w.uint32(8).uint64(m.networkId);
    if (m.listingId) w.uint32(16).uint64(m.listingId);
    if (m.userCount) w.uint32(24).uint32(m.userCount);
    if (m.recentUserCount) w.uint32(32).uint32(m.recentUserCount);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendWatchListingsResponse_UserCountChange {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new FrontendWatchListingsResponse_UserCountChange();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkId = r.uint64();
        break;
        case 2:
        m.listingId = r.uint64();
        break;
        case 3:
        m.userCount = r.uint32();
        break;
        case 4:
        m.recentUserCount = r.uint32();
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

export type FrontendWatchListingsResponse_IEvent = {
  event?: FrontendWatchListingsResponse_Event_IEvent
}

export class FrontendWatchListingsResponse_Event {
  event: FrontendWatchListingsResponse_Event_TEvent;

  constructor(v?: FrontendWatchListingsResponse_IEvent) {
    this.event = v?.event ? new FrontendWatchListingsResponse_Event_Event(v.event) : new FrontendWatchListingsResponse_Event_Event();
  }

  static encode(m: FrontendWatchListingsResponse_Event, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.event.case) {
      case FrontendWatchListingsResponse_Event_EventCase.CHANGE:
      FrontendWatchListingsResponse_Change.encode(m.event.change, w.uint32(8010).fork()).ldelim();
      break;
      case FrontendWatchListingsResponse_Event_EventCase.UNPUBLISH:
      FrontendWatchListingsResponse_Unpublish.encode(m.event.unpublish, w.uint32(8018).fork()).ldelim();
      break;
      case FrontendWatchListingsResponse_Event_EventCase.USER_COUNT_CHANGE:
      FrontendWatchListingsResponse_UserCountChange.encode(m.event.userCountChange, w.uint32(8026).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendWatchListingsResponse_Event {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new FrontendWatchListingsResponse_Event();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1001:
        m.event = new FrontendWatchListingsResponse_Event_Event({ change: FrontendWatchListingsResponse_Change.decode(r, r.uint32()) });
        break;
        case 1002:
        m.event = new FrontendWatchListingsResponse_Event_Event({ unpublish: FrontendWatchListingsResponse_Unpublish.decode(r, r.uint32()) });
        break;
        case 1003:
        m.event = new FrontendWatchListingsResponse_Event_Event({ userCountChange: FrontendWatchListingsResponse_UserCountChange.decode(r, r.uint32()) });
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

export enum FrontendWatchListingsResponse_Event_EventCase {
  NOT_SET = 0,
  CHANGE = 1001,
  UNPUBLISH = 1002,
  USER_COUNT_CHANGE = 1003,
}

export type FrontendWatchListingsResponse_Event_IEvent =
{ case?: FrontendWatchListingsResponse_Event_EventCase.NOT_SET }
|{ case?: FrontendWatchListingsResponse_Event_EventCase.CHANGE, change: FrontendWatchListingsResponse_IChange }
|{ case?: FrontendWatchListingsResponse_Event_EventCase.UNPUBLISH, unpublish: FrontendWatchListingsResponse_IUnpublish }
|{ case?: FrontendWatchListingsResponse_Event_EventCase.USER_COUNT_CHANGE, userCountChange: FrontendWatchListingsResponse_IUserCountChange }
;

export type FrontendWatchListingsResponse_Event_TEvent = Readonly<
{ case: FrontendWatchListingsResponse_Event_EventCase.NOT_SET }
|{ case: FrontendWatchListingsResponse_Event_EventCase.CHANGE, change: FrontendWatchListingsResponse_Change }
|{ case: FrontendWatchListingsResponse_Event_EventCase.UNPUBLISH, unpublish: FrontendWatchListingsResponse_Unpublish }
|{ case: FrontendWatchListingsResponse_Event_EventCase.USER_COUNT_CHANGE, userCountChange: FrontendWatchListingsResponse_UserCountChange }
>;

class FrontendWatchListingsResponse_Event_EventImpl {
  declare change?: FrontendWatchListingsResponse_Change;
  declare unpublish?: FrontendWatchListingsResponse_Unpublish;
  declare userCountChange?: FrontendWatchListingsResponse_UserCountChange;
  case: FrontendWatchListingsResponse_Event_EventCase = FrontendWatchListingsResponse_Event_EventCase.NOT_SET;

  constructor(v?: FrontendWatchListingsResponse_Event_IEvent) {
    if (v && "change" in v) {
      this.case = FrontendWatchListingsResponse_Event_EventCase.CHANGE;
      this.change = new FrontendWatchListingsResponse_Change(v.change);
    } else
    if (v && "unpublish" in v) {
      this.case = FrontendWatchListingsResponse_Event_EventCase.UNPUBLISH;
      this.unpublish = new FrontendWatchListingsResponse_Unpublish(v.unpublish);
    } else
    if (v && "userCountChange" in v) {
      this.case = FrontendWatchListingsResponse_Event_EventCase.USER_COUNT_CHANGE;
      this.userCountChange = new FrontendWatchListingsResponse_UserCountChange(v.userCountChange);
    }
  }
}

export const FrontendWatchListingsResponse_Event_Event = FrontendWatchListingsResponse_Event_EventImpl as {
  new (): Readonly<{ case: FrontendWatchListingsResponse_Event_EventCase.NOT_SET }>;
  new <T extends FrontendWatchListingsResponse_Event_IEvent>(v: T): Readonly<
  T extends { change: FrontendWatchListingsResponse_IChange } ? { case: FrontendWatchListingsResponse_Event_EventCase.CHANGE, change: FrontendWatchListingsResponse_Change } :
  T extends { unpublish: FrontendWatchListingsResponse_IUnpublish } ? { case: FrontendWatchListingsResponse_Event_EventCase.UNPUBLISH, unpublish: FrontendWatchListingsResponse_Unpublish } :
  T extends { userCountChange: FrontendWatchListingsResponse_IUserCountChange } ? { case: FrontendWatchListingsResponse_Event_EventCase.USER_COUNT_CHANGE, userCountChange: FrontendWatchListingsResponse_UserCountChange } :
  never
  >;
};

export type IFrontendWatchListingUsersRequest = {
  networkKey?: Uint8Array;
  query?: IListingQuery;
}

export class FrontendWatchListingUsersRequest {
  networkKey: Uint8Array;
  query: ListingQuery | undefined;

  constructor(v?: IFrontendWatchListingUsersRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.query = v?.query && new ListingQuery(v.query);
  }

  static encode(m: FrontendWatchListingUsersRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    if (m.query) ListingQuery.encode(m.query, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendWatchListingUsersRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new FrontendWatchListingUsersRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkKey = r.bytes();
        break;
        case 2:
        m.query = ListingQuery.decode(r, r.uint32());
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

export type IFrontendWatchListingUsersResponse = {
  type?: FrontendWatchListingUsersResponse_UserEventType;
  users?: FrontendWatchListingUsersResponse_IUser[];
}

export class FrontendWatchListingUsersResponse {
  type: FrontendWatchListingUsersResponse_UserEventType;
  users: FrontendWatchListingUsersResponse_User[];

  constructor(v?: IFrontendWatchListingUsersResponse) {
    this.type = v?.type || 0;
    this.users = v?.users ? v.users.map(v => new FrontendWatchListingUsersResponse_User(v)) : [];
  }

  static encode(m: FrontendWatchListingUsersResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.type) w.uint32(8).uint32(m.type);
    for (const v of m.users) FrontendWatchListingUsersResponse_User.encode(v, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendWatchListingUsersResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new FrontendWatchListingUsersResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.type = r.uint32();
        break;
        case 2:
        m.users.push(FrontendWatchListingUsersResponse_User.decode(r, r.uint32()));
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

export type FrontendWatchListingUsersResponse_IUser = {
  id?: bigint;
  alias?: string;
  peerKey?: Uint8Array;
}

export class FrontendWatchListingUsersResponse_User {
  id: bigint;
  alias: string;
  peerKey: Uint8Array;

  constructor(v?: FrontendWatchListingUsersResponse_IUser) {
    this.id = v?.id || BigInt(0);
    this.alias = v?.alias || "";
    this.peerKey = v?.peerKey || new Uint8Array();
  }

  static encode(m: FrontendWatchListingUsersResponse_User, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.alias.length) w.uint32(18).string(m.alias);
    if (m.peerKey.length) w.uint32(26).bytes(m.peerKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendWatchListingUsersResponse_User {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new FrontendWatchListingUsersResponse_User();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.alias = r.string();
        break;
        case 3:
        m.peerKey = r.bytes();
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

export enum FrontendWatchListingUsersResponse_UserEventType {
  USER_EVENT_TYPE_JOIN = 0,
  USER_EVENT_TYPE_PART = 1,
  USER_EVENT_TYPE_RENAME = 2,
}

export type IFrontendWatchAssetBundlesRequest = object;

export class FrontendWatchAssetBundlesRequest {

  constructor(v?: IFrontendWatchAssetBundlesRequest) {
    void v; // an empty message has no fields to set
  }

  static encode(m: FrontendWatchAssetBundlesRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendWatchAssetBundlesRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new FrontendWatchAssetBundlesRequest();
  }
}

export type IFrontendWatchAssetBundlesResponse = {
  networkId?: bigint;
  networkKey?: Uint8Array;
  assetBundle?: IAssetBundle;
}

export class FrontendWatchAssetBundlesResponse {
  networkId: bigint;
  networkKey: Uint8Array;
  assetBundle: AssetBundle | undefined;

  constructor(v?: IFrontendWatchAssetBundlesResponse) {
    this.networkId = v?.networkId || BigInt(0);
    this.networkKey = v?.networkKey || new Uint8Array();
    this.assetBundle = v?.assetBundle && new AssetBundle(v.assetBundle);
  }

  static encode(m: FrontendWatchAssetBundlesResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkId) w.uint32(8).uint64(m.networkId);
    if (m.networkKey.length) w.uint32(18).bytes(m.networkKey);
    if (m.assetBundle) AssetBundle.encode(m.assetBundle, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendWatchAssetBundlesResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new FrontendWatchAssetBundlesResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkId = r.uint64();
        break;
        case 2:
        m.networkKey = r.bytes();
        break;
        case 3:
        m.assetBundle = AssetBundle.decode(r, r.uint32());
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

export type ISnippetSubscribeRequest = {
  swarmId?: Uint8Array;
}

export class SnippetSubscribeRequest {
  swarmId: Uint8Array;

  constructor(v?: ISnippetSubscribeRequest) {
    this.swarmId = v?.swarmId || new Uint8Array();
  }

  static encode(m: SnippetSubscribeRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.swarmId.length) w.uint32(10).bytes(m.swarmId);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SnippetSubscribeRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new SnippetSubscribeRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.swarmId = r.bytes();
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

/**
 * A snippet stream's response: a signed snippet, an image, or both. Each
 * image a snippet names that the subscriber lacks comes before the
 * snippet, the last one in the snippet's response.
 */
export type ISnippetSubscribeResponse = {
  snippet?: ISignedListingSnippet;
  /** An encoded ListingSnippetImage. */
  image?: Uint8Array;
}

/**
 * A snippet stream's response: a signed snippet, an image, or both. Each
 * image a snippet names that the subscriber lacks comes before the
 * snippet, the last one in the snippet's response.
 */
export class SnippetSubscribeResponse {
  snippet: SignedListingSnippet | undefined;
  /** An encoded ListingSnippetImage. */
  image: Uint8Array;

  constructor(v?: ISnippetSubscribeResponse) {
    this.snippet = v?.snippet && new SignedListingSnippet(v.snippet);
    this.image = v?.image || new Uint8Array();
  }

  static encode(m: SnippetSubscribeResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.snippet) SignedListingSnippet.encode(m.snippet, w.uint32(10).fork()).ldelim();
    if (m.image.length) w.uint32(18).bytes(m.image);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SnippetSubscribeResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new SnippetSubscribeResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.snippet = SignedListingSnippet.decode(r, r.uint32());
        break;
        case 2:
        m.image = r.bytes();
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

export enum ListingContentType {
  LISTING_CONTENT_TYPE_UNDEFINED = 0,
  LISTING_CONTENT_TYPE_MEDIA = 1,
  LISTING_CONTENT_TYPE_SERVICE = 2,
  LISTING_CONTENT_TYPE_EMBED = 3,
  LISTING_CONTENT_TYPE_CHAT = 4,
}


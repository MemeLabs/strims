import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  strims_type_Image,
  strims_type_IImage,
} from "../../../type/image";
import {
  google_protobuf_BoolValue,
  google_protobuf_IBoolValue,
  google_protobuf_BytesValue,
  google_protobuf_IBytesValue,
  google_protobuf_Int64Value,
  google_protobuf_IInt64Value,
  google_protobuf_StringValue,
  google_protobuf_IStringValue,
  google_protobuf_UInt32Value,
  google_protobuf_IUInt32Value,
  google_protobuf_UInt64Value,
  google_protobuf_IUInt64Value,
} from "../../../../google/protobuf/wrappers";

export type IServerConfig = {
  integrations?: strims_network_v1_directory_ServerConfig_IIntegrations;
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
  integrations: strims_network_v1_directory_ServerConfig_Integrations | undefined;
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
    this.integrations = v?.integrations && new strims_network_v1_directory_ServerConfig_Integrations(v.integrations);
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
    if (m.integrations) strims_network_v1_directory_ServerConfig_Integrations.encode(m.integrations, w.uint32(10).fork()).ldelim();
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ServerConfig();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.integrations = strims_network_v1_directory_ServerConfig_Integrations.decode(r, r.uint32());
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
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace ServerConfig {
  export type IIntegrations = {
    angelthump?: strims_network_v1_directory_ServerConfig_Integrations_IAngelThump;
    twitch?: strims_network_v1_directory_ServerConfig_Integrations_ITwitch;
    youtube?: strims_network_v1_directory_ServerConfig_Integrations_IYouTube;
    swarm?: strims_network_v1_directory_ServerConfig_Integrations_ISwarm;
  }

  export class Integrations {
    angelthump: strims_network_v1_directory_ServerConfig_Integrations_AngelThump | undefined;
    twitch: strims_network_v1_directory_ServerConfig_Integrations_Twitch | undefined;
    youtube: strims_network_v1_directory_ServerConfig_Integrations_YouTube | undefined;
    swarm: strims_network_v1_directory_ServerConfig_Integrations_Swarm | undefined;

    constructor(v?: IIntegrations) {
      this.angelthump = v?.angelthump && new strims_network_v1_directory_ServerConfig_Integrations_AngelThump(v.angelthump);
      this.twitch = v?.twitch && new strims_network_v1_directory_ServerConfig_Integrations_Twitch(v.twitch);
      this.youtube = v?.youtube && new strims_network_v1_directory_ServerConfig_Integrations_YouTube(v.youtube);
      this.swarm = v?.swarm && new strims_network_v1_directory_ServerConfig_Integrations_Swarm(v.swarm);
    }

    static encode(m: Integrations, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.angelthump) strims_network_v1_directory_ServerConfig_Integrations_AngelThump.encode(m.angelthump, w.uint32(10).fork()).ldelim();
      if (m.twitch) strims_network_v1_directory_ServerConfig_Integrations_Twitch.encode(m.twitch, w.uint32(18).fork()).ldelim();
      if (m.youtube) strims_network_v1_directory_ServerConfig_Integrations_YouTube.encode(m.youtube, w.uint32(26).fork()).ldelim();
      if (m.swarm) strims_network_v1_directory_ServerConfig_Integrations_Swarm.encode(m.swarm, w.uint32(34).fork()).ldelim();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Integrations {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Integrations();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.angelthump = strims_network_v1_directory_ServerConfig_Integrations_AngelThump.decode(r, r.uint32());
          break;
          case 2:
          m.twitch = strims_network_v1_directory_ServerConfig_Integrations_Twitch.decode(r, r.uint32());
          break;
          case 3:
          m.youtube = strims_network_v1_directory_ServerConfig_Integrations_YouTube.decode(r, r.uint32());
          break;
          case 4:
          m.swarm = strims_network_v1_directory_ServerConfig_Integrations_Swarm.decode(r, r.uint32());
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export namespace Integrations {
    export type IAngelThump = {
      enable?: boolean;
    }

    export class AngelThump {
      enable: boolean;

      constructor(v?: IAngelThump) {
        this.enable = v?.enable || false;
      }

      static encode(m: AngelThump, w?: Writer): Writer {
        if (!w) w = new Writer();
        if (m.enable) w.uint32(8).bool(m.enable);
        return w;
      }

      static decode(r: Reader | Uint8Array, length?: number): AngelThump {
        r = r instanceof Reader ? r : new Reader(r);
        const end = length === undefined ? r.len : r.pos + length;
        const m = new AngelThump();
        while (r.pos < end) {
          const tag = r.uint32();
          switch (tag >> 3) {
            case 1:
            m.enable = r.bool();
            break;
            default:
            r.skipType(tag & 7);
            break;
          }
        }
        return m;
      }
    }

    export type ITwitch = {
      enable?: boolean;
      clientId?: string;
      clientSecret?: string;
    }

    export class Twitch {
      enable: boolean;
      clientId: string;
      clientSecret: string;

      constructor(v?: ITwitch) {
        this.enable = v?.enable || false;
        this.clientId = v?.clientId || "";
        this.clientSecret = v?.clientSecret || "";
      }

      static encode(m: Twitch, w?: Writer): Writer {
        if (!w) w = new Writer();
        if (m.enable) w.uint32(8).bool(m.enable);
        if (m.clientId.length) w.uint32(18).string(m.clientId);
        if (m.clientSecret.length) w.uint32(26).string(m.clientSecret);
        return w;
      }

      static decode(r: Reader | Uint8Array, length?: number): Twitch {
        r = r instanceof Reader ? r : new Reader(r);
        const end = length === undefined ? r.len : r.pos + length;
        const m = new Twitch();
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
            r.skipType(tag & 7);
            break;
          }
        }
        return m;
      }
    }

    export type IYouTube = {
      enable?: boolean;
      publicApiKey?: string;
    }

    export class YouTube {
      enable: boolean;
      publicApiKey: string;

      constructor(v?: IYouTube) {
        this.enable = v?.enable || false;
        this.publicApiKey = v?.publicApiKey || "";
      }

      static encode(m: YouTube, w?: Writer): Writer {
        if (!w) w = new Writer();
        if (m.enable) w.uint32(8).bool(m.enable);
        if (m.publicApiKey.length) w.uint32(18).string(m.publicApiKey);
        return w;
      }

      static decode(r: Reader | Uint8Array, length?: number): YouTube {
        r = r instanceof Reader ? r : new Reader(r);
        const end = length === undefined ? r.len : r.pos + length;
        const m = new YouTube();
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
            r.skipType(tag & 7);
            break;
          }
        }
        return m;
      }
    }

    export type ISwarm = {
      enable?: boolean;
    }

    export class Swarm {
      enable: boolean;

      constructor(v?: ISwarm) {
        this.enable = v?.enable || false;
      }

      static encode(m: Swarm, w?: Writer): Writer {
        if (!w) w = new Writer();
        if (m.enable) w.uint32(8).bool(m.enable);
        return w;
      }

      static decode(r: Reader | Uint8Array, length?: number): Swarm {
        r = r instanceof Reader ? r : new Reader(r);
        const end = length === undefined ? r.len : r.pos + length;
        const m = new Swarm();
        while (r.pos < end) {
          const tag = r.uint32();
          switch (tag >> 3) {
            case 1:
            m.enable = r.bool();
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

export type IClientConfig = {
  integrations?: strims_network_v1_directory_ClientConfig_IIntegrations;
  publishQuota?: number;
  joinQuota?: number;
  minPingInterval?: number;
  maxPingInterval?: number;
}

export class ClientConfig {
  integrations: strims_network_v1_directory_ClientConfig_Integrations | undefined;
  publishQuota: number;
  joinQuota: number;
  minPingInterval: number;
  maxPingInterval: number;

  constructor(v?: IClientConfig) {
    this.integrations = v?.integrations && new strims_network_v1_directory_ClientConfig_Integrations(v.integrations);
    this.publishQuota = v?.publishQuota || 0;
    this.joinQuota = v?.joinQuota || 0;
    this.minPingInterval = v?.minPingInterval || 0;
    this.maxPingInterval = v?.maxPingInterval || 0;
  }

  static encode(m: ClientConfig, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.integrations) strims_network_v1_directory_ClientConfig_Integrations.encode(m.integrations, w.uint32(10).fork()).ldelim();
    if (m.publishQuota) w.uint32(16).uint32(m.publishQuota);
    if (m.joinQuota) w.uint32(24).uint32(m.joinQuota);
    if (m.minPingInterval) w.uint32(32).uint32(m.minPingInterval);
    if (m.maxPingInterval) w.uint32(40).uint32(m.maxPingInterval);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ClientConfig {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ClientConfig();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.integrations = strims_network_v1_directory_ClientConfig_Integrations.decode(r, r.uint32());
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
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace ClientConfig {
  export type IIntegrations = {
    angelthump?: boolean;
    twitch?: boolean;
    youtube?: boolean;
    swarm?: boolean;
  }

  export class Integrations {
    angelthump: boolean;
    twitch: boolean;
    youtube: boolean;
    swarm: boolean;

    constructor(v?: IIntegrations) {
      this.angelthump = v?.angelthump || false;
      this.twitch = v?.twitch || false;
      this.youtube = v?.youtube || false;
      this.swarm = v?.swarm || false;
    }

    static encode(m: Integrations, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.angelthump) w.uint32(8).bool(m.angelthump);
      if (m.twitch) w.uint32(16).bool(m.twitch);
      if (m.youtube) w.uint32(24).bool(m.youtube);
      if (m.swarm) w.uint32(32).bool(m.swarm);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Integrations {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Integrations();
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
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetEventsRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new TestPublishRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
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

export type ITestPublishResponse = Record<string, any>;

export class TestPublishResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: ITestPublishResponse) {
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
  content?: Listing.IContent
}

export class Listing {
  content: Listing.TContent;

  constructor(v?: IListing) {
    this.content = new Listing.Content(v?.content);
  }

  static encode(m: Listing, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.content.case) {
      case Listing.ContentCase.MEDIA:
      strims_network_v1_directory_Listing_Media.encode(m.content.media, w.uint32(8010).fork()).ldelim();
      break;
      case Listing.ContentCase.SERVICE:
      strims_network_v1_directory_Listing_Service.encode(m.content.service, w.uint32(8018).fork()).ldelim();
      break;
      case Listing.ContentCase.EMBED:
      strims_network_v1_directory_Listing_Embed.encode(m.content.embed, w.uint32(8026).fork()).ldelim();
      break;
      case Listing.ContentCase.CHAT:
      strims_network_v1_directory_Listing_Chat.encode(m.content.chat, w.uint32(8034).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Listing {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Listing();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1001:
        m.content = new Listing.Content({ media: strims_network_v1_directory_Listing_Media.decode(r, r.uint32()) });
        break;
        case 1002:
        m.content = new Listing.Content({ service: strims_network_v1_directory_Listing_Service.decode(r, r.uint32()) });
        break;
        case 1003:
        m.content = new Listing.Content({ embed: strims_network_v1_directory_Listing_Embed.decode(r, r.uint32()) });
        break;
        case 1004:
        m.content = new Listing.Content({ chat: strims_network_v1_directory_Listing_Chat.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace Listing {
  export enum ContentCase {
    NOT_SET = 0,
    MEDIA = 1001,
    SERVICE = 1002,
    EMBED = 1003,
    CHAT = 1004,
  }

  export type IContent =
  { case?: ContentCase.NOT_SET }
  |{ case?: ContentCase.MEDIA, media: strims_network_v1_directory_Listing_IMedia }
  |{ case?: ContentCase.SERVICE, service: strims_network_v1_directory_Listing_IService }
  |{ case?: ContentCase.EMBED, embed: strims_network_v1_directory_Listing_IEmbed }
  |{ case?: ContentCase.CHAT, chat: strims_network_v1_directory_Listing_IChat }
  ;

  export type TContent = Readonly<
  { case: ContentCase.NOT_SET }
  |{ case: ContentCase.MEDIA, media: strims_network_v1_directory_Listing_Media }
  |{ case: ContentCase.SERVICE, service: strims_network_v1_directory_Listing_Service }
  |{ case: ContentCase.EMBED, embed: strims_network_v1_directory_Listing_Embed }
  |{ case: ContentCase.CHAT, chat: strims_network_v1_directory_Listing_Chat }
  >;

  class ContentImpl {
    media: strims_network_v1_directory_Listing_Media;
    service: strims_network_v1_directory_Listing_Service;
    embed: strims_network_v1_directory_Listing_Embed;
    chat: strims_network_v1_directory_Listing_Chat;
    case: ContentCase = ContentCase.NOT_SET;

    constructor(v?: IContent) {
      if (v && "media" in v) {
        this.case = ContentCase.MEDIA;
        this.media = new strims_network_v1_directory_Listing_Media(v.media);
      } else
      if (v && "service" in v) {
        this.case = ContentCase.SERVICE;
        this.service = new strims_network_v1_directory_Listing_Service(v.service);
      } else
      if (v && "embed" in v) {
        this.case = ContentCase.EMBED;
        this.embed = new strims_network_v1_directory_Listing_Embed(v.embed);
      } else
      if (v && "chat" in v) {
        this.case = ContentCase.CHAT;
        this.chat = new strims_network_v1_directory_Listing_Chat(v.chat);
      }
    }
  }

  export const Content = ContentImpl as {
    new (): Readonly<{ case: ContentCase.NOT_SET }>;
    new <T extends IContent>(v: T): Readonly<
    T extends { media: strims_network_v1_directory_Listing_IMedia } ? { case: ContentCase.MEDIA, media: strims_network_v1_directory_Listing_Media } :
    T extends { service: strims_network_v1_directory_Listing_IService } ? { case: ContentCase.SERVICE, service: strims_network_v1_directory_Listing_Service } :
    T extends { embed: strims_network_v1_directory_Listing_IEmbed } ? { case: ContentCase.EMBED, embed: strims_network_v1_directory_Listing_Embed } :
    T extends { chat: strims_network_v1_directory_Listing_IChat } ? { case: ContentCase.CHAT, chat: strims_network_v1_directory_Listing_Chat } :
    never
    >;
  };

  export type IMedia = {
    mimeType?: string;
    swarmUri?: string;
  }

  export class Media {
    mimeType: string;
    swarmUri: string;

    constructor(v?: IMedia) {
      this.mimeType = v?.mimeType || "";
      this.swarmUri = v?.swarmUri || "";
    }

    static encode(m: Media, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.mimeType.length) w.uint32(10).string(m.mimeType);
      if (m.swarmUri.length) w.uint32(18).string(m.swarmUri);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Media {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Media();
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
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type IService = {
    type?: string;
    swarmUri?: string;
  }

  export class Service {
    type: string;
    swarmUri: string;

    constructor(v?: IService) {
      this.type = v?.type || "";
      this.swarmUri = v?.swarmUri || "";
    }

    static encode(m: Service, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.type.length) w.uint32(10).string(m.type);
      if (m.swarmUri.length) w.uint32(18).string(m.swarmUri);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Service {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Service();
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
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type IEmbed = {
    service?: strims_network_v1_directory_Listing_Embed_Service;
    id?: string;
    queryParams?: Map<string, string> | { [key: string]: string };
  }

  export class Embed {
    service: strims_network_v1_directory_Listing_Embed_Service;
    id: string;
    queryParams: Map<string, string>;

    constructor(v?: IEmbed) {
      this.service = v?.service || 0;
      this.id = v?.id || "";
      if (v?.queryParams) this.queryParams = new Map(v.queryParams instanceof Map ? v.queryParams : Object.entries(v.queryParams).map(([k, v]) => [String(k), v]));
      else this.queryParams = new Map<string, string>();
    }

    static encode(m: Embed, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.service) w.uint32(8).uint32(m.service);
      if (m.id.length) w.uint32(18).string(m.id);
      for (const [k, v] of m.queryParams) w.uint32(26).fork().uint32(10).string(k).uint32(18).string(v).ldelim();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Embed {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Embed();
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
            const flen = r.uint32();
            const fend = r.pos + flen;
            let key: string;
            let value: string;
            while (r.pos < fend) {
              const ftag = r.uint32();
              switch (ftag >> 3) {
                case 1:
                key = r.string()
                break;
                case 2:
                value = r.string();
                break;
              }
            }
            m.queryParams.set(key, value)
          }
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export namespace Embed {
    export enum Service {
      DIRECTORY_LISTING_EMBED_SERVICE_UNDEFINED = 0,
      DIRECTORY_LISTING_EMBED_SERVICE_ANGELTHUMP = 1,
      DIRECTORY_LISTING_EMBED_SERVICE_TWITCH_STREAM = 2,
      DIRECTORY_LISTING_EMBED_SERVICE_TWITCH_VOD = 3,
      DIRECTORY_LISTING_EMBED_SERVICE_YOUTUBE = 4,
    }
  }

  export type IChat = {
    key?: Uint8Array;
    name?: string;
  }

  export class Chat {
    key: Uint8Array;
    name: string;

    constructor(v?: IChat) {
      this.key = v?.key || new Uint8Array();
      this.name = v?.name || "";
    }

    static encode(m: Chat, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.key.length) w.uint32(10).bytes(m.key);
      if (m.name.length) w.uint32(18).string(m.name);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Chat {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Chat();
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
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

}

export type IListingSnippetImage = {
  sourceOneof?: ListingSnippetImage.ISourceOneof
}

export class ListingSnippetImage {
  sourceOneof: ListingSnippetImage.TSourceOneof;

  constructor(v?: IListingSnippetImage) {
    this.sourceOneof = new ListingSnippetImage.SourceOneof(v?.sourceOneof);
  }

  static encode(m: ListingSnippetImage, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.sourceOneof.case) {
      case ListingSnippetImage.SourceOneofCase.URL:
      w.uint32(8010).string(m.sourceOneof.url);
      break;
      case ListingSnippetImage.SourceOneofCase.IMAGE:
      strims_type_Image.encode(m.sourceOneof.image, w.uint32(8018).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListingSnippetImage {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ListingSnippetImage();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1001:
        m.sourceOneof = new ListingSnippetImage.SourceOneof({ url: r.string() });
        break;
        case 1002:
        m.sourceOneof = new ListingSnippetImage.SourceOneof({ image: strims_type_Image.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace ListingSnippetImage {
  export enum SourceOneofCase {
    NOT_SET = 0,
    URL = 1001,
    IMAGE = 1002,
  }

  export type ISourceOneof =
  { case?: SourceOneofCase.NOT_SET }
  |{ case?: SourceOneofCase.URL, url: string }
  |{ case?: SourceOneofCase.IMAGE, image: strims_type_IImage }
  ;

  export type TSourceOneof = Readonly<
  { case: SourceOneofCase.NOT_SET }
  |{ case: SourceOneofCase.URL, url: string }
  |{ case: SourceOneofCase.IMAGE, image: strims_type_Image }
  >;

  class SourceOneofImpl {
    url: string;
    image: strims_type_Image;
    case: SourceOneofCase = SourceOneofCase.NOT_SET;

    constructor(v?: ISourceOneof) {
      if (v && "url" in v) {
        this.case = SourceOneofCase.URL;
        this.url = v.url;
      } else
      if (v && "image" in v) {
        this.case = SourceOneofCase.IMAGE;
        this.image = new strims_type_Image(v.image);
      }
    }
  }

  export const SourceOneof = SourceOneofImpl as {
    new (): Readonly<{ case: SourceOneofCase.NOT_SET }>;
    new <T extends ISourceOneof>(v: T): Readonly<
    T extends { url: string } ? { case: SourceOneofCase.URL, url: string } :
    T extends { image: strims_type_IImage } ? { case: SourceOneofCase.IMAGE, image: strims_type_Image } :
    never
    >;
  };

}

export type IListingSnippet = {
  title?: string;
  description?: string;
  tags?: string[];
  category?: string;
  channelName?: string;
  userCount?: bigint;
  live?: boolean;
  isMature?: boolean;
  thumbnail?: strims_network_v1_directory_IListingSnippetImage;
  channelLogo?: strims_network_v1_directory_IListingSnippetImage;
  videoHeight?: number;
  videoWidth?: number;
  themeColor?: number;
  startTime?: bigint;
  key?: Uint8Array;
  signature?: Uint8Array;
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
  thumbnail: strims_network_v1_directory_ListingSnippetImage | undefined;
  channelLogo: strims_network_v1_directory_ListingSnippetImage | undefined;
  videoHeight: number;
  videoWidth: number;
  themeColor: number;
  startTime: bigint;
  key: Uint8Array;
  signature: Uint8Array;

  constructor(v?: IListingSnippet) {
    this.title = v?.title || "";
    this.description = v?.description || "";
    this.tags = v?.tags ? v.tags : [];
    this.category = v?.category || "";
    this.channelName = v?.channelName || "";
    this.userCount = v?.userCount || BigInt(0);
    this.live = v?.live || false;
    this.isMature = v?.isMature || false;
    this.thumbnail = v?.thumbnail && new strims_network_v1_directory_ListingSnippetImage(v.thumbnail);
    this.channelLogo = v?.channelLogo && new strims_network_v1_directory_ListingSnippetImage(v.channelLogo);
    this.videoHeight = v?.videoHeight || 0;
    this.videoWidth = v?.videoWidth || 0;
    this.themeColor = v?.themeColor || 0;
    this.startTime = v?.startTime || BigInt(0);
    this.key = v?.key || new Uint8Array();
    this.signature = v?.signature || new Uint8Array();
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
    if (m.thumbnail) strims_network_v1_directory_ListingSnippetImage.encode(m.thumbnail, w.uint32(74).fork()).ldelim();
    if (m.channelLogo) strims_network_v1_directory_ListingSnippetImage.encode(m.channelLogo, w.uint32(82).fork()).ldelim();
    if (m.videoHeight) w.uint32(88).uint32(m.videoHeight);
    if (m.videoWidth) w.uint32(96).uint32(m.videoWidth);
    if (m.themeColor) w.uint32(109).fixed32(m.themeColor);
    if (m.startTime) w.uint32(112).int64(m.startTime);
    if (m.key.length) w.uint32(80010).bytes(m.key);
    if (m.signature.length) w.uint32(80018).bytes(m.signature);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListingSnippet {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
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
        m.thumbnail = strims_network_v1_directory_ListingSnippetImage.decode(r, r.uint32());
        break;
        case 10:
        m.channelLogo = strims_network_v1_directory_ListingSnippetImage.decode(r, r.uint32());
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
        case 10001:
        m.key = r.bytes();
        break;
        case 10002:
        m.signature = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IListingSnippetDelta = {
  title?: google_protobuf_IStringValue;
  description?: google_protobuf_IStringValue;
  category?: google_protobuf_IStringValue;
  channelName?: google_protobuf_IStringValue;
  userCount?: google_protobuf_IUInt64Value;
  live?: google_protobuf_IBoolValue;
  isMature?: google_protobuf_IBoolValue;
  key?: google_protobuf_IBytesValue;
  signature?: google_protobuf_IBytesValue;
  videoHeight?: google_protobuf_IUInt32Value;
  videoWidth?: google_protobuf_IUInt32Value;
  themeColor?: google_protobuf_IUInt32Value;
  startTime?: google_protobuf_IInt64Value;
  tagsOneof?: ListingSnippetDelta.ITagsOneof
  thumbnailOneof?: ListingSnippetDelta.IThumbnailOneof
  channelLogoOneof?: ListingSnippetDelta.IChannelLogoOneof
}

export class ListingSnippetDelta {
  title: google_protobuf_StringValue | undefined;
  description: google_protobuf_StringValue | undefined;
  category: google_protobuf_StringValue | undefined;
  channelName: google_protobuf_StringValue | undefined;
  userCount: google_protobuf_UInt64Value | undefined;
  live: google_protobuf_BoolValue | undefined;
  isMature: google_protobuf_BoolValue | undefined;
  key: google_protobuf_BytesValue | undefined;
  signature: google_protobuf_BytesValue | undefined;
  videoHeight: google_protobuf_UInt32Value | undefined;
  videoWidth: google_protobuf_UInt32Value | undefined;
  themeColor: google_protobuf_UInt32Value | undefined;
  startTime: google_protobuf_Int64Value | undefined;
  tagsOneof: ListingSnippetDelta.TTagsOneof;
  thumbnailOneof: ListingSnippetDelta.TThumbnailOneof;
  channelLogoOneof: ListingSnippetDelta.TChannelLogoOneof;

  constructor(v?: IListingSnippetDelta) {
    this.title = v?.title && new google_protobuf_StringValue(v.title);
    this.description = v?.description && new google_protobuf_StringValue(v.description);
    this.category = v?.category && new google_protobuf_StringValue(v.category);
    this.channelName = v?.channelName && new google_protobuf_StringValue(v.channelName);
    this.userCount = v?.userCount && new google_protobuf_UInt64Value(v.userCount);
    this.live = v?.live && new google_protobuf_BoolValue(v.live);
    this.isMature = v?.isMature && new google_protobuf_BoolValue(v.isMature);
    this.key = v?.key && new google_protobuf_BytesValue(v.key);
    this.signature = v?.signature && new google_protobuf_BytesValue(v.signature);
    this.videoHeight = v?.videoHeight && new google_protobuf_UInt32Value(v.videoHeight);
    this.videoWidth = v?.videoWidth && new google_protobuf_UInt32Value(v.videoWidth);
    this.themeColor = v?.themeColor && new google_protobuf_UInt32Value(v.themeColor);
    this.startTime = v?.startTime && new google_protobuf_Int64Value(v.startTime);
    this.tagsOneof = new ListingSnippetDelta.TagsOneof(v?.tagsOneof);
    this.thumbnailOneof = new ListingSnippetDelta.ThumbnailOneof(v?.thumbnailOneof);
    this.channelLogoOneof = new ListingSnippetDelta.ChannelLogoOneof(v?.channelLogoOneof);
  }

  static encode(m: ListingSnippetDelta, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.title) google_protobuf_StringValue.encode(m.title, w.uint32(10).fork()).ldelim();
    if (m.description) google_protobuf_StringValue.encode(m.description, w.uint32(18).fork()).ldelim();
    if (m.category) google_protobuf_StringValue.encode(m.category, w.uint32(26).fork()).ldelim();
    if (m.channelName) google_protobuf_StringValue.encode(m.channelName, w.uint32(34).fork()).ldelim();
    if (m.userCount) google_protobuf_UInt64Value.encode(m.userCount, w.uint32(42).fork()).ldelim();
    if (m.live) google_protobuf_BoolValue.encode(m.live, w.uint32(50).fork()).ldelim();
    if (m.isMature) google_protobuf_BoolValue.encode(m.isMature, w.uint32(58).fork()).ldelim();
    if (m.key) google_protobuf_BytesValue.encode(m.key, w.uint32(66).fork()).ldelim();
    if (m.signature) google_protobuf_BytesValue.encode(m.signature, w.uint32(74).fork()).ldelim();
    if (m.videoHeight) google_protobuf_UInt32Value.encode(m.videoHeight, w.uint32(82).fork()).ldelim();
    if (m.videoWidth) google_protobuf_UInt32Value.encode(m.videoWidth, w.uint32(90).fork()).ldelim();
    if (m.themeColor) google_protobuf_UInt32Value.encode(m.themeColor, w.uint32(98).fork()).ldelim();
    if (m.startTime) google_protobuf_Int64Value.encode(m.startTime, w.uint32(106).fork()).ldelim();
    switch (m.tagsOneof.case) {
      case ListingSnippetDelta.TagsOneofCase.TAGS:
      strims_network_v1_directory_ListingSnippetDelta_Tags.encode(m.tagsOneof.tags, w.uint32(8010).fork()).ldelim();
      break;
    }
    switch (m.thumbnailOneof.case) {
      case ListingSnippetDelta.ThumbnailOneofCase.THUMBNAIL:
      strims_network_v1_directory_ListingSnippetImage.encode(m.thumbnailOneof.thumbnail, w.uint32(16010).fork()).ldelim();
      break;
    }
    switch (m.channelLogoOneof.case) {
      case ListingSnippetDelta.ChannelLogoOneofCase.CHANNEL_LOGO:
      strims_network_v1_directory_ListingSnippetImage.encode(m.channelLogoOneof.channelLogo, w.uint32(24010).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListingSnippetDelta {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ListingSnippetDelta();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.title = google_protobuf_StringValue.decode(r, r.uint32());
        break;
        case 2:
        m.description = google_protobuf_StringValue.decode(r, r.uint32());
        break;
        case 3:
        m.category = google_protobuf_StringValue.decode(r, r.uint32());
        break;
        case 4:
        m.channelName = google_protobuf_StringValue.decode(r, r.uint32());
        break;
        case 5:
        m.userCount = google_protobuf_UInt64Value.decode(r, r.uint32());
        break;
        case 6:
        m.live = google_protobuf_BoolValue.decode(r, r.uint32());
        break;
        case 7:
        m.isMature = google_protobuf_BoolValue.decode(r, r.uint32());
        break;
        case 8:
        m.key = google_protobuf_BytesValue.decode(r, r.uint32());
        break;
        case 9:
        m.signature = google_protobuf_BytesValue.decode(r, r.uint32());
        break;
        case 10:
        m.videoHeight = google_protobuf_UInt32Value.decode(r, r.uint32());
        break;
        case 11:
        m.videoWidth = google_protobuf_UInt32Value.decode(r, r.uint32());
        break;
        case 12:
        m.themeColor = google_protobuf_UInt32Value.decode(r, r.uint32());
        break;
        case 13:
        m.startTime = google_protobuf_Int64Value.decode(r, r.uint32());
        break;
        case 1001:
        m.tagsOneof = new ListingSnippetDelta.TagsOneof({ tags: strims_network_v1_directory_ListingSnippetDelta_Tags.decode(r, r.uint32()) });
        break;
        case 2001:
        m.thumbnailOneof = new ListingSnippetDelta.ThumbnailOneof({ thumbnail: strims_network_v1_directory_ListingSnippetImage.decode(r, r.uint32()) });
        break;
        case 3001:
        m.channelLogoOneof = new ListingSnippetDelta.ChannelLogoOneof({ channelLogo: strims_network_v1_directory_ListingSnippetImage.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace ListingSnippetDelta {
  export enum TagsOneofCase {
    NOT_SET = 0,
    TAGS = 1001,
  }

  export type ITagsOneof =
  { case?: TagsOneofCase.NOT_SET }
  |{ case?: TagsOneofCase.TAGS, tags: strims_network_v1_directory_ListingSnippetDelta_ITags }
  ;

  export type TTagsOneof = Readonly<
  { case: TagsOneofCase.NOT_SET }
  |{ case: TagsOneofCase.TAGS, tags: strims_network_v1_directory_ListingSnippetDelta_Tags }
  >;

  class TagsOneofImpl {
    tags: strims_network_v1_directory_ListingSnippetDelta_Tags;
    case: TagsOneofCase = TagsOneofCase.NOT_SET;

    constructor(v?: ITagsOneof) {
      if (v && "tags" in v) {
        this.case = TagsOneofCase.TAGS;
        this.tags = new strims_network_v1_directory_ListingSnippetDelta_Tags(v.tags);
      }
    }
  }

  export const TagsOneof = TagsOneofImpl as {
    new (): Readonly<{ case: TagsOneofCase.NOT_SET }>;
    new <T extends ITagsOneof>(v: T): Readonly<
    T extends { tags: strims_network_v1_directory_ListingSnippetDelta_ITags } ? { case: TagsOneofCase.TAGS, tags: strims_network_v1_directory_ListingSnippetDelta_Tags } :
    never
    >;
  };

  export enum ThumbnailOneofCase {
    NOT_SET = 0,
    THUMBNAIL = 2001,
  }

  export type IThumbnailOneof =
  { case?: ThumbnailOneofCase.NOT_SET }
  |{ case?: ThumbnailOneofCase.THUMBNAIL, thumbnail: strims_network_v1_directory_IListingSnippetImage }
  ;

  export type TThumbnailOneof = Readonly<
  { case: ThumbnailOneofCase.NOT_SET }
  |{ case: ThumbnailOneofCase.THUMBNAIL, thumbnail: strims_network_v1_directory_ListingSnippetImage }
  >;

  class ThumbnailOneofImpl {
    thumbnail: strims_network_v1_directory_ListingSnippetImage;
    case: ThumbnailOneofCase = ThumbnailOneofCase.NOT_SET;

    constructor(v?: IThumbnailOneof) {
      if (v && "thumbnail" in v) {
        this.case = ThumbnailOneofCase.THUMBNAIL;
        this.thumbnail = new strims_network_v1_directory_ListingSnippetImage(v.thumbnail);
      }
    }
  }

  export const ThumbnailOneof = ThumbnailOneofImpl as {
    new (): Readonly<{ case: ThumbnailOneofCase.NOT_SET }>;
    new <T extends IThumbnailOneof>(v: T): Readonly<
    T extends { thumbnail: strims_network_v1_directory_IListingSnippetImage } ? { case: ThumbnailOneofCase.THUMBNAIL, thumbnail: strims_network_v1_directory_ListingSnippetImage } :
    never
    >;
  };

  export enum ChannelLogoOneofCase {
    NOT_SET = 0,
    CHANNEL_LOGO = 3001,
  }

  export type IChannelLogoOneof =
  { case?: ChannelLogoOneofCase.NOT_SET }
  |{ case?: ChannelLogoOneofCase.CHANNEL_LOGO, channelLogo: strims_network_v1_directory_IListingSnippetImage }
  ;

  export type TChannelLogoOneof = Readonly<
  { case: ChannelLogoOneofCase.NOT_SET }
  |{ case: ChannelLogoOneofCase.CHANNEL_LOGO, channelLogo: strims_network_v1_directory_ListingSnippetImage }
  >;

  class ChannelLogoOneofImpl {
    channelLogo: strims_network_v1_directory_ListingSnippetImage;
    case: ChannelLogoOneofCase = ChannelLogoOneofCase.NOT_SET;

    constructor(v?: IChannelLogoOneof) {
      if (v && "channelLogo" in v) {
        this.case = ChannelLogoOneofCase.CHANNEL_LOGO;
        this.channelLogo = new strims_network_v1_directory_ListingSnippetImage(v.channelLogo);
      }
    }
  }

  export const ChannelLogoOneof = ChannelLogoOneofImpl as {
    new (): Readonly<{ case: ChannelLogoOneofCase.NOT_SET }>;
    new <T extends IChannelLogoOneof>(v: T): Readonly<
    T extends { channelLogo: strims_network_v1_directory_IListingSnippetImage } ? { case: ChannelLogoOneofCase.CHANNEL_LOGO, channelLogo: strims_network_v1_directory_ListingSnippetImage } :
    never
    >;
  };

  export type ITags = {
    tags?: string[];
  }

  export class Tags {
    tags: string[];

    constructor(v?: ITags) {
      this.tags = v?.tags ? v.tags : [];
    }

    static encode(m: Tags, w?: Writer): Writer {
      if (!w) w = new Writer();
      for (const v of m.tags) w.uint32(8010).string(v);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Tags {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Tags();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1001:
          m.tags.push(r.string())
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

export type IEvent = {
  body?: Event.IBody
}

export class Event {
  body: Event.TBody;

  constructor(v?: IEvent) {
    this.body = new Event.Body(v?.body);
  }

  static encode(m: Event, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case Event.BodyCase.LISTING_CHANGE:
      strims_network_v1_directory_Event_ListingChange.encode(m.body.listingChange, w.uint32(8010).fork()).ldelim();
      break;
      case Event.BodyCase.UNPUBLISH:
      strims_network_v1_directory_Event_Unpublish.encode(m.body.unpublish, w.uint32(8018).fork()).ldelim();
      break;
      case Event.BodyCase.USER_COUNT_CHANGE:
      strims_network_v1_directory_Event_UserCountChange.encode(m.body.userCountChange, w.uint32(8026).fork()).ldelim();
      break;
      case Event.BodyCase.USER_PRESENCE_CHANGE:
      strims_network_v1_directory_Event_UserPresenceChange.encode(m.body.userPresenceChange, w.uint32(8034).fork()).ldelim();
      break;
      case Event.BodyCase.PING:
      strims_network_v1_directory_Event_Ping.encode(m.body.ping, w.uint32(8042).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Event {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Event();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1001:
        m.body = new Event.Body({ listingChange: strims_network_v1_directory_Event_ListingChange.decode(r, r.uint32()) });
        break;
        case 1002:
        m.body = new Event.Body({ unpublish: strims_network_v1_directory_Event_Unpublish.decode(r, r.uint32()) });
        break;
        case 1003:
        m.body = new Event.Body({ userCountChange: strims_network_v1_directory_Event_UserCountChange.decode(r, r.uint32()) });
        break;
        case 1004:
        m.body = new Event.Body({ userPresenceChange: strims_network_v1_directory_Event_UserPresenceChange.decode(r, r.uint32()) });
        break;
        case 1005:
        m.body = new Event.Body({ ping: strims_network_v1_directory_Event_Ping.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace Event {
  export enum BodyCase {
    NOT_SET = 0,
    LISTING_CHANGE = 1001,
    UNPUBLISH = 1002,
    USER_COUNT_CHANGE = 1003,
    USER_PRESENCE_CHANGE = 1004,
    PING = 1005,
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.LISTING_CHANGE, listingChange: strims_network_v1_directory_Event_IListingChange }
  |{ case?: BodyCase.UNPUBLISH, unpublish: strims_network_v1_directory_Event_IUnpublish }
  |{ case?: BodyCase.USER_COUNT_CHANGE, userCountChange: strims_network_v1_directory_Event_IUserCountChange }
  |{ case?: BodyCase.USER_PRESENCE_CHANGE, userPresenceChange: strims_network_v1_directory_Event_IUserPresenceChange }
  |{ case?: BodyCase.PING, ping: strims_network_v1_directory_Event_IPing }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.LISTING_CHANGE, listingChange: strims_network_v1_directory_Event_ListingChange }
  |{ case: BodyCase.UNPUBLISH, unpublish: strims_network_v1_directory_Event_Unpublish }
  |{ case: BodyCase.USER_COUNT_CHANGE, userCountChange: strims_network_v1_directory_Event_UserCountChange }
  |{ case: BodyCase.USER_PRESENCE_CHANGE, userPresenceChange: strims_network_v1_directory_Event_UserPresenceChange }
  |{ case: BodyCase.PING, ping: strims_network_v1_directory_Event_Ping }
  >;

  class BodyImpl {
    listingChange: strims_network_v1_directory_Event_ListingChange;
    unpublish: strims_network_v1_directory_Event_Unpublish;
    userCountChange: strims_network_v1_directory_Event_UserCountChange;
    userPresenceChange: strims_network_v1_directory_Event_UserPresenceChange;
    ping: strims_network_v1_directory_Event_Ping;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "listingChange" in v) {
        this.case = BodyCase.LISTING_CHANGE;
        this.listingChange = new strims_network_v1_directory_Event_ListingChange(v.listingChange);
      } else
      if (v && "unpublish" in v) {
        this.case = BodyCase.UNPUBLISH;
        this.unpublish = new strims_network_v1_directory_Event_Unpublish(v.unpublish);
      } else
      if (v && "userCountChange" in v) {
        this.case = BodyCase.USER_COUNT_CHANGE;
        this.userCountChange = new strims_network_v1_directory_Event_UserCountChange(v.userCountChange);
      } else
      if (v && "userPresenceChange" in v) {
        this.case = BodyCase.USER_PRESENCE_CHANGE;
        this.userPresenceChange = new strims_network_v1_directory_Event_UserPresenceChange(v.userPresenceChange);
      } else
      if (v && "ping" in v) {
        this.case = BodyCase.PING;
        this.ping = new strims_network_v1_directory_Event_Ping(v.ping);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { listingChange: strims_network_v1_directory_Event_IListingChange } ? { case: BodyCase.LISTING_CHANGE, listingChange: strims_network_v1_directory_Event_ListingChange } :
    T extends { unpublish: strims_network_v1_directory_Event_IUnpublish } ? { case: BodyCase.UNPUBLISH, unpublish: strims_network_v1_directory_Event_Unpublish } :
    T extends { userCountChange: strims_network_v1_directory_Event_IUserCountChange } ? { case: BodyCase.USER_COUNT_CHANGE, userCountChange: strims_network_v1_directory_Event_UserCountChange } :
    T extends { userPresenceChange: strims_network_v1_directory_Event_IUserPresenceChange } ? { case: BodyCase.USER_PRESENCE_CHANGE, userPresenceChange: strims_network_v1_directory_Event_UserPresenceChange } :
    T extends { ping: strims_network_v1_directory_Event_IPing } ? { case: BodyCase.PING, ping: strims_network_v1_directory_Event_Ping } :
    never
    >;
  };

  export type IListingChange = {
    id?: bigint;
    listing?: strims_network_v1_directory_IListing;
    snippet?: strims_network_v1_directory_IListingSnippet;
    moderation?: strims_network_v1_directory_IListingModeration;
  }

  export class ListingChange {
    id: bigint;
    listing: strims_network_v1_directory_Listing | undefined;
    snippet: strims_network_v1_directory_ListingSnippet | undefined;
    moderation: strims_network_v1_directory_ListingModeration | undefined;

    constructor(v?: IListingChange) {
      this.id = v?.id || BigInt(0);
      this.listing = v?.listing && new strims_network_v1_directory_Listing(v.listing);
      this.snippet = v?.snippet && new strims_network_v1_directory_ListingSnippet(v.snippet);
      this.moderation = v?.moderation && new strims_network_v1_directory_ListingModeration(v.moderation);
    }

    static encode(m: ListingChange, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.id) w.uint32(8).uint64(m.id);
      if (m.listing) strims_network_v1_directory_Listing.encode(m.listing, w.uint32(18).fork()).ldelim();
      if (m.snippet) strims_network_v1_directory_ListingSnippet.encode(m.snippet, w.uint32(26).fork()).ldelim();
      if (m.moderation) strims_network_v1_directory_ListingModeration.encode(m.moderation, w.uint32(34).fork()).ldelim();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): ListingChange {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new ListingChange();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.id = r.uint64();
          break;
          case 2:
          m.listing = strims_network_v1_directory_Listing.decode(r, r.uint32());
          break;
          case 3:
          m.snippet = strims_network_v1_directory_ListingSnippet.decode(r, r.uint32());
          break;
          case 4:
          m.moderation = strims_network_v1_directory_ListingModeration.decode(r, r.uint32());
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type IUnpublish = {
    id?: bigint;
  }

  export class Unpublish {
    id: bigint;

    constructor(v?: IUnpublish) {
      this.id = v?.id || BigInt(0);
    }

    static encode(m: Unpublish, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.id) w.uint32(8).uint64(m.id);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Unpublish {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Unpublish();
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

  export type IUserCountChange = {
    id?: bigint;
    userCount?: number;
    recentUserCount?: number;
  }

  export class UserCountChange {
    id: bigint;
    userCount: number;
    recentUserCount: number;

    constructor(v?: IUserCountChange) {
      this.id = v?.id || BigInt(0);
      this.userCount = v?.userCount || 0;
      this.recentUserCount = v?.recentUserCount || 0;
    }

    static encode(m: UserCountChange, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.id) w.uint32(8).uint64(m.id);
      if (m.userCount) w.uint32(16).uint32(m.userCount);
      if (m.recentUserCount) w.uint32(24).uint32(m.recentUserCount);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): UserCountChange {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new UserCountChange();
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
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type IUserPresenceChange = {
    id?: bigint;
    alias?: string;
    peerKey?: Uint8Array;
    online?: boolean;
    listingIds?: bigint[];
  }

  export class UserPresenceChange {
    id: bigint;
    alias: string;
    peerKey: Uint8Array;
    online: boolean;
    listingIds: bigint[];

    constructor(v?: IUserPresenceChange) {
      this.id = v?.id || BigInt(0);
      this.alias = v?.alias || "";
      this.peerKey = v?.peerKey || new Uint8Array();
      this.online = v?.online || false;
      this.listingIds = v?.listingIds ? v.listingIds : [];
    }

    static encode(m: UserPresenceChange, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.id) w.uint32(8).uint64(m.id);
      if (m.alias.length) w.uint32(18).string(m.alias);
      if (m.peerKey.length) w.uint32(26).bytes(m.peerKey);
      if (m.online) w.uint32(32).bool(m.online);
      m.listingIds.reduce((w, v) => w.uint64(v), w.uint32(42).fork()).ldelim();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): UserPresenceChange {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new UserPresenceChange();
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
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type IPing = {
    time?: bigint;
  }

  export class Ping {
    time: bigint;

    constructor(v?: IPing) {
      this.time = v?.time || BigInt(0);
    }

    static encode(m: Ping, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.time) w.uint32(8).int64(m.time);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Ping {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Ping();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.time = r.int64();
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
    const end = length === undefined ? r.len : r.pos + length;
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
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IListingQuery = {
  query?: ListingQuery.IQuery
}

export class ListingQuery {
  query: ListingQuery.TQuery;

  constructor(v?: IListingQuery) {
    this.query = new ListingQuery.Query(v?.query);
  }

  static encode(m: ListingQuery, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.query.case) {
      case ListingQuery.QueryCase.ID:
      w.uint32(8008).uint64(m.query.id);
      break;
      case ListingQuery.QueryCase.LISTING:
      strims_network_v1_directory_Listing.encode(m.query.listing, w.uint32(8018).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListingQuery {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ListingQuery();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1001:
        m.query = new ListingQuery.Query({ id: r.uint64() });
        break;
        case 1002:
        m.query = new ListingQuery.Query({ listing: strims_network_v1_directory_Listing.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace ListingQuery {
  export enum QueryCase {
    NOT_SET = 0,
    ID = 1001,
    LISTING = 1002,
  }

  export type IQuery =
  { case?: QueryCase.NOT_SET }
  |{ case?: QueryCase.ID, id: bigint }
  |{ case?: QueryCase.LISTING, listing: strims_network_v1_directory_IListing }
  ;

  export type TQuery = Readonly<
  { case: QueryCase.NOT_SET }
  |{ case: QueryCase.ID, id: bigint }
  |{ case: QueryCase.LISTING, listing: strims_network_v1_directory_Listing }
  >;

  class QueryImpl {
    id: bigint;
    listing: strims_network_v1_directory_Listing;
    case: QueryCase = QueryCase.NOT_SET;

    constructor(v?: IQuery) {
      if (v && "id" in v) {
        this.case = QueryCase.ID;
        this.id = v.id;
      } else
      if (v && "listing" in v) {
        this.case = QueryCase.LISTING;
        this.listing = new strims_network_v1_directory_Listing(v.listing);
      }
    }
  }

  export const Query = QueryImpl as {
    new (): Readonly<{ case: QueryCase.NOT_SET }>;
    new <T extends IQuery>(v: T): Readonly<
    T extends { id: bigint } ? { case: QueryCase.ID, id: bigint } :
    T extends { listing: strims_network_v1_directory_IListing } ? { case: QueryCase.LISTING, listing: strims_network_v1_directory_Listing } :
    never
    >;
  };

}

export type IListingRecord = {
  id?: bigint;
  networkId?: bigint;
  listing?: strims_network_v1_directory_IListing;
  moderation?: strims_network_v1_directory_IListingModeration;
  notes?: string;
}

export class ListingRecord {
  id: bigint;
  networkId: bigint;
  listing: strims_network_v1_directory_Listing | undefined;
  moderation: strims_network_v1_directory_ListingModeration | undefined;
  notes: string;

  constructor(v?: IListingRecord) {
    this.id = v?.id || BigInt(0);
    this.networkId = v?.networkId || BigInt(0);
    this.listing = v?.listing && new strims_network_v1_directory_Listing(v.listing);
    this.moderation = v?.moderation && new strims_network_v1_directory_ListingModeration(v.moderation);
    this.notes = v?.notes || "";
  }

  static encode(m: ListingRecord, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.networkId) w.uint32(16).uint64(m.networkId);
    if (m.listing) strims_network_v1_directory_Listing.encode(m.listing, w.uint32(26).fork()).ldelim();
    if (m.moderation) strims_network_v1_directory_ListingModeration.encode(m.moderation, w.uint32(34).fork()).ldelim();
    if (m.notes.length) w.uint32(42).string(m.notes);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListingRecord {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
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
        m.listing = strims_network_v1_directory_Listing.decode(r, r.uint32());
        break;
        case 4:
        m.moderation = strims_network_v1_directory_ListingModeration.decode(r, r.uint32());
        break;
        case 5:
        m.notes = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
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
    const end = length === undefined ? r.len : r.pos + length;
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
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IUserRecord = {
  id?: bigint;
  networkId?: bigint;
  peerKey?: Uint8Array;
  moderation?: strims_network_v1_directory_IUserModeration;
}

export class UserRecord {
  id: bigint;
  networkId: bigint;
  peerKey: Uint8Array;
  moderation: strims_network_v1_directory_UserModeration | undefined;

  constructor(v?: IUserRecord) {
    this.id = v?.id || BigInt(0);
    this.networkId = v?.networkId || BigInt(0);
    this.peerKey = v?.peerKey || new Uint8Array();
    this.moderation = v?.moderation && new strims_network_v1_directory_UserModeration(v.moderation);
  }

  static encode(m: UserRecord, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.networkId) w.uint32(16).uint64(m.networkId);
    if (m.peerKey.length) w.uint32(26).bytes(m.peerKey);
    if (m.moderation) strims_network_v1_directory_UserModeration.encode(m.moderation, w.uint32(34).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UserRecord {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
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
        m.moderation = strims_network_v1_directory_UserModeration.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IEventBroadcast = {
  events?: strims_network_v1_directory_IEvent[];
}

export class EventBroadcast {
  events: strims_network_v1_directory_Event[];

  constructor(v?: IEventBroadcast) {
    this.events = v?.events ? v.events.map(v => new strims_network_v1_directory_Event(v)) : [];
  }

  static encode(m: EventBroadcast, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.events) strims_network_v1_directory_Event.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): EventBroadcast {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new EventBroadcast();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.events.push(strims_network_v1_directory_Event.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IAssetBundle = {
  icon?: strims_type_IImage;
  directory?: strims_network_v1_directory_IClientConfig;
}

export class AssetBundle {
  icon: strims_type_Image | undefined;
  directory: strims_network_v1_directory_ClientConfig | undefined;

  constructor(v?: IAssetBundle) {
    this.icon = v?.icon && new strims_type_Image(v.icon);
    this.directory = v?.directory && new strims_network_v1_directory_ClientConfig(v.directory);
  }

  static encode(m: AssetBundle, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.icon) strims_type_Image.encode(m.icon, w.uint32(10).fork()).ldelim();
    if (m.directory) strims_network_v1_directory_ClientConfig.encode(m.directory, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): AssetBundle {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new AssetBundle();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.icon = strims_type_Image.decode(r, r.uint32());
        break;
        case 2:
        m.directory = strims_network_v1_directory_ClientConfig.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IPublishRequest = {
  listing?: strims_network_v1_directory_IListing;
}

export class PublishRequest {
  listing: strims_network_v1_directory_Listing | undefined;

  constructor(v?: IPublishRequest) {
    this.listing = v?.listing && new strims_network_v1_directory_Listing(v.listing);
  }

  static encode(m: PublishRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.listing) strims_network_v1_directory_Listing.encode(m.listing, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PublishRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new PublishRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.listing = strims_network_v1_directory_Listing.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new PublishResponse();
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UnpublishRequest();
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

export type IUnpublishResponse = Record<string, any>;

export class UnpublishResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IUnpublishResponse) {
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
  query?: strims_network_v1_directory_IListingQuery;
}

export class JoinRequest {
  query: strims_network_v1_directory_ListingQuery | undefined;

  constructor(v?: IJoinRequest) {
    this.query = v?.query && new strims_network_v1_directory_ListingQuery(v.query);
  }

  static encode(m: JoinRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.query) strims_network_v1_directory_ListingQuery.encode(m.query, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): JoinRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new JoinRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.query = strims_network_v1_directory_ListingQuery.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new JoinResponse();
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new PartRequest();
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

export type IPartResponse = Record<string, any>;

export class PartResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IPartResponse) {
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

export type IPingRequest = Record<string, any>;

export class PingRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IPingRequest) {
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

export type IPingResponse = Record<string, any>;

export class PingResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IPingResponse) {
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
  moderation?: strims_network_v1_directory_IListingModeration;
}

export class ModerateListingRequest {
  id: bigint;
  moderation: strims_network_v1_directory_ListingModeration | undefined;

  constructor(v?: IModerateListingRequest) {
    this.id = v?.id || BigInt(0);
    this.moderation = v?.moderation && new strims_network_v1_directory_ListingModeration(v.moderation);
  }

  static encode(m: ModerateListingRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.moderation) strims_network_v1_directory_ListingModeration.encode(m.moderation, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ModerateListingRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ModerateListingRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.moderation = strims_network_v1_directory_ListingModeration.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IModerateListingResponse = Record<string, any>;

export class ModerateListingResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IModerateListingResponse) {
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
  moderation?: strims_network_v1_directory_IUserModeration;
}

export class ModerateUserRequest {
  peerKey: Uint8Array;
  moderation: strims_network_v1_directory_UserModeration | undefined;

  constructor(v?: IModerateUserRequest) {
    this.peerKey = v?.peerKey || new Uint8Array();
    this.moderation = v?.moderation && new strims_network_v1_directory_UserModeration(v.moderation);
  }

  static encode(m: ModerateUserRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.peerKey.length) w.uint32(10).bytes(m.peerKey);
    if (m.moderation) strims_network_v1_directory_UserModeration.encode(m.moderation, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ModerateUserRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ModerateUserRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.peerKey = r.bytes();
        break;
        case 2:
        m.moderation = strims_network_v1_directory_UserModeration.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IModerateUserResponse = Record<string, any>;

export class ModerateUserResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IModerateUserResponse) {
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
        m.key = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type INetworkListingsItem = {
  id?: bigint;
  listing?: strims_network_v1_directory_IListing;
  snippet?: strims_network_v1_directory_IListingSnippet;
  moderation?: strims_network_v1_directory_IListingModeration;
  userCount?: number;
  recentUserCount?: number;
}

export class NetworkListingsItem {
  id: bigint;
  listing: strims_network_v1_directory_Listing | undefined;
  snippet: strims_network_v1_directory_ListingSnippet | undefined;
  moderation: strims_network_v1_directory_ListingModeration | undefined;
  userCount: number;
  recentUserCount: number;

  constructor(v?: INetworkListingsItem) {
    this.id = v?.id || BigInt(0);
    this.listing = v?.listing && new strims_network_v1_directory_Listing(v.listing);
    this.snippet = v?.snippet && new strims_network_v1_directory_ListingSnippet(v.snippet);
    this.moderation = v?.moderation && new strims_network_v1_directory_ListingModeration(v.moderation);
    this.userCount = v?.userCount || 0;
    this.recentUserCount = v?.recentUserCount || 0;
  }

  static encode(m: NetworkListingsItem, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.listing) strims_network_v1_directory_Listing.encode(m.listing, w.uint32(18).fork()).ldelim();
    if (m.snippet) strims_network_v1_directory_ListingSnippet.encode(m.snippet, w.uint32(26).fork()).ldelim();
    if (m.moderation) strims_network_v1_directory_ListingModeration.encode(m.moderation, w.uint32(34).fork()).ldelim();
    if (m.userCount) w.uint32(40).uint32(m.userCount);
    if (m.recentUserCount) w.uint32(48).uint32(m.recentUserCount);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): NetworkListingsItem {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new NetworkListingsItem();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.listing = strims_network_v1_directory_Listing.decode(r, r.uint32());
        break;
        case 3:
        m.snippet = strims_network_v1_directory_ListingSnippet.decode(r, r.uint32());
        break;
        case 4:
        m.moderation = strims_network_v1_directory_ListingModeration.decode(r, r.uint32());
        break;
        case 5:
        m.userCount = r.uint32();
        break;
        case 6:
        m.recentUserCount = r.uint32();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type INetworkListings = {
  network?: strims_network_v1_directory_INetwork;
  listings?: strims_network_v1_directory_INetworkListingsItem[];
}

export class NetworkListings {
  network: strims_network_v1_directory_Network | undefined;
  listings: strims_network_v1_directory_NetworkListingsItem[];

  constructor(v?: INetworkListings) {
    this.network = v?.network && new strims_network_v1_directory_Network(v.network);
    this.listings = v?.listings ? v.listings.map(v => new strims_network_v1_directory_NetworkListingsItem(v)) : [];
  }

  static encode(m: NetworkListings, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.network) strims_network_v1_directory_Network.encode(m.network, w.uint32(10).fork()).ldelim();
    for (const v of m.listings) strims_network_v1_directory_NetworkListingsItem.encode(v, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): NetworkListings {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new NetworkListings();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.network = strims_network_v1_directory_Network.decode(r, r.uint32());
        break;
        case 2:
        m.listings.push(strims_network_v1_directory_NetworkListingsItem.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IFrontendPublishRequest = {
  networkKey?: Uint8Array;
  listing?: strims_network_v1_directory_IListing;
}

export class FrontendPublishRequest {
  networkKey: Uint8Array;
  listing: strims_network_v1_directory_Listing | undefined;

  constructor(v?: IFrontendPublishRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.listing = v?.listing && new strims_network_v1_directory_Listing(v.listing);
  }

  static encode(m: FrontendPublishRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    if (m.listing) strims_network_v1_directory_Listing.encode(m.listing, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendPublishRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new FrontendPublishRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkKey = r.bytes();
        break;
        case 2:
        m.listing = strims_network_v1_directory_Listing.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new FrontendPublishResponse();
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
    const end = length === undefined ? r.len : r.pos + length;
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
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IFrontendUnpublishResponse = Record<string, any>;

export class FrontendUnpublishResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IFrontendUnpublishResponse) {
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
  query?: strims_network_v1_directory_IListingQuery;
}

export class FrontendJoinRequest {
  networkKey: Uint8Array;
  query: strims_network_v1_directory_ListingQuery | undefined;

  constructor(v?: IFrontendJoinRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.query = v?.query && new strims_network_v1_directory_ListingQuery(v.query);
  }

  static encode(m: FrontendJoinRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    if (m.query) strims_network_v1_directory_ListingQuery.encode(m.query, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendJoinRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new FrontendJoinRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkKey = r.bytes();
        break;
        case 2:
        m.query = strims_network_v1_directory_ListingQuery.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new FrontendJoinResponse();
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
    const end = length === undefined ? r.len : r.pos + length;
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
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IFrontendPartResponse = Record<string, any>;

export class FrontendPartResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IFrontendPartResponse) {
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new FrontendTestRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
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

export type IFrontendTestResponse = Record<string, any>;

export class FrontendTestResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IFrontendTestResponse) {
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
  moderation?: strims_network_v1_directory_IListingModeration;
}

export class FrontendModerateListingRequest {
  networkKey: Uint8Array;
  id: bigint;
  moderation: strims_network_v1_directory_ListingModeration | undefined;

  constructor(v?: IFrontendModerateListingRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.id = v?.id || BigInt(0);
    this.moderation = v?.moderation && new strims_network_v1_directory_ListingModeration(v.moderation);
  }

  static encode(m: FrontendModerateListingRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    if (m.id) w.uint32(16).uint64(m.id);
    if (m.moderation) strims_network_v1_directory_ListingModeration.encode(m.moderation, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendModerateListingRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
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
        m.moderation = strims_network_v1_directory_ListingModeration.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IFrontendModerateListingResponse = Record<string, any>;

export class FrontendModerateListingResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IFrontendModerateListingResponse) {
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
  moderation?: strims_network_v1_directory_IUserModeration;
}

export class FrontendModerateUserRequest {
  networkKey: Uint8Array;
  alias: string;
  moderation: strims_network_v1_directory_UserModeration | undefined;

  constructor(v?: IFrontendModerateUserRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.alias = v?.alias || "";
    this.moderation = v?.moderation && new strims_network_v1_directory_UserModeration(v.moderation);
  }

  static encode(m: FrontendModerateUserRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    if (m.alias.length) w.uint32(18).string(m.alias);
    if (m.moderation) strims_network_v1_directory_UserModeration.encode(m.moderation, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendModerateUserRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
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
        m.moderation = strims_network_v1_directory_UserModeration.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IFrontendModerateUserResponse = Record<string, any>;

export class FrontendModerateUserResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IFrontendModerateUserResponse) {
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

export type IFrontendGetUsersRequest = Record<string, any>;

export class FrontendGetUsersRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IFrontendGetUsersRequest) {
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
  users?: strims_network_v1_directory_FrontendGetUsersResponse_IUser[];
  networks?: Map<bigint, strims_network_v1_directory_Network>;
}

export class FrontendGetUsersResponse {
  users: strims_network_v1_directory_FrontendGetUsersResponse_User[];
  networks: Map<bigint, strims_network_v1_directory_Network>;

  constructor(v?: IFrontendGetUsersResponse) {
    this.users = v?.users ? v.users.map(v => new strims_network_v1_directory_FrontendGetUsersResponse_User(v)) : [];
    if (v?.networks) this.networks = new Map(Array.from(v.networks).map(([k, v]) => [k, new strims_network_v1_directory_Network(v)]));
    else this.networks = new Map<bigint, strims_network_v1_directory_Network>();
  }

  static encode(m: FrontendGetUsersResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.users) strims_network_v1_directory_FrontendGetUsersResponse_User.encode(v, w.uint32(10).fork()).ldelim();
    for (const [k, v] of m.networks) strims_network_v1_directory_Network.encode(v, w.uint32(18).fork().uint32(8).uint64(k).uint32(18).fork()).ldelim().ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendGetUsersResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new FrontendGetUsersResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.users.push(strims_network_v1_directory_FrontendGetUsersResponse_User.decode(r, r.uint32()));
        break;
        case 2:
        {
          const flen = r.uint32();
          const fend = r.pos + flen;
          let key: bigint;
          let value: strims_network_v1_directory_Network;
          while (r.pos < fend) {
            const ftag = r.uint32();
            switch (ftag >> 3) {
              case 1:
              key = r.uint64()
              break;
              case 2:
              value = strims_network_v1_directory_Network.decode(r, r.uint32());
              break;
            }
          }
          m.networks.set(key, value)
        }
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace FrontendGetUsersResponse {
  export type IAlias = {
    alias?: string;
    networkIds?: bigint[];
  }

  export class Alias {
    alias: string;
    networkIds: bigint[];

    constructor(v?: IAlias) {
      this.alias = v?.alias || "";
      this.networkIds = v?.networkIds ? v.networkIds : [];
    }

    static encode(m: Alias, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.alias.length) w.uint32(10).string(m.alias);
      m.networkIds.reduce((w, v) => w.uint64(v), w.uint32(18).fork()).ldelim();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Alias {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Alias();
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
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type IUser = {
    aliases?: strims_network_v1_directory_FrontendGetUsersResponse_IAlias[];
    peerKey?: Uint8Array;
  }

  export class User {
    aliases: strims_network_v1_directory_FrontendGetUsersResponse_Alias[];
    peerKey: Uint8Array;

    constructor(v?: IUser) {
      this.aliases = v?.aliases ? v.aliases.map(v => new strims_network_v1_directory_FrontendGetUsersResponse_Alias(v)) : [];
      this.peerKey = v?.peerKey || new Uint8Array();
    }

    static encode(m: User, w?: Writer): Writer {
      if (!w) w = new Writer();
      for (const v of m.aliases) strims_network_v1_directory_FrontendGetUsersResponse_Alias.encode(v, w.uint32(10).fork()).ldelim();
      if (m.peerKey.length) w.uint32(18).bytes(m.peerKey);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): User {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new User();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.aliases.push(strims_network_v1_directory_FrontendGetUsersResponse_Alias.decode(r, r.uint32()));
          break;
          case 2:
          m.peerKey = r.bytes();
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

export type IFrontendGetListingRequest = {
  query?: strims_network_v1_directory_IListingQuery;
  networkKey?: Uint8Array;
}

export class FrontendGetListingRequest {
  query: strims_network_v1_directory_ListingQuery | undefined;
  networkKey: Uint8Array;

  constructor(v?: IFrontendGetListingRequest) {
    this.query = v?.query && new strims_network_v1_directory_ListingQuery(v.query);
    this.networkKey = v?.networkKey || new Uint8Array();
  }

  static encode(m: FrontendGetListingRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.query) strims_network_v1_directory_ListingQuery.encode(m.query, w.uint32(10).fork()).ldelim();
    if (m.networkKey.length) w.uint32(18).bytes(m.networkKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendGetListingRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new FrontendGetListingRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.query = strims_network_v1_directory_ListingQuery.decode(r, r.uint32());
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

export type IFrontendGetListingResponse = {
  id?: bigint;
  listing?: strims_network_v1_directory_IListing;
  snippet?: strims_network_v1_directory_IListingSnippet;
  moderation?: strims_network_v1_directory_IListingModeration;
  userCount?: number;
  recentUserCount?: number;
}

export class FrontendGetListingResponse {
  id: bigint;
  listing: strims_network_v1_directory_Listing | undefined;
  snippet: strims_network_v1_directory_ListingSnippet | undefined;
  moderation: strims_network_v1_directory_ListingModeration | undefined;
  userCount: number;
  recentUserCount: number;

  constructor(v?: IFrontendGetListingResponse) {
    this.id = v?.id || BigInt(0);
    this.listing = v?.listing && new strims_network_v1_directory_Listing(v.listing);
    this.snippet = v?.snippet && new strims_network_v1_directory_ListingSnippet(v.snippet);
    this.moderation = v?.moderation && new strims_network_v1_directory_ListingModeration(v.moderation);
    this.userCount = v?.userCount || 0;
    this.recentUserCount = v?.recentUserCount || 0;
  }

  static encode(m: FrontendGetListingResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.listing) strims_network_v1_directory_Listing.encode(m.listing, w.uint32(18).fork()).ldelim();
    if (m.snippet) strims_network_v1_directory_ListingSnippet.encode(m.snippet, w.uint32(26).fork()).ldelim();
    if (m.moderation) strims_network_v1_directory_ListingModeration.encode(m.moderation, w.uint32(34).fork()).ldelim();
    if (m.userCount) w.uint32(40).uint32(m.userCount);
    if (m.recentUserCount) w.uint32(48).uint32(m.recentUserCount);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendGetListingResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new FrontendGetListingResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.listing = strims_network_v1_directory_Listing.decode(r, r.uint32());
        break;
        case 3:
        m.snippet = strims_network_v1_directory_ListingSnippet.decode(r, r.uint32());
        break;
        case 4:
        m.moderation = strims_network_v1_directory_ListingModeration.decode(r, r.uint32());
        break;
        case 5:
        m.userCount = r.uint32();
        break;
        case 6:
        m.recentUserCount = r.uint32();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IFrontendGetListingsRequest = {
  contentTypes?: strims_network_v1_directory_ListingContentType[];
  networkKeys?: Uint8Array[];
}

export class FrontendGetListingsRequest {
  contentTypes: strims_network_v1_directory_ListingContentType[];
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
    const end = length === undefined ? r.len : r.pos + length;
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
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IFrontendGetListingsResponse = {
  listings?: strims_network_v1_directory_INetworkListings[];
}

export class FrontendGetListingsResponse {
  listings: strims_network_v1_directory_NetworkListings[];

  constructor(v?: IFrontendGetListingsResponse) {
    this.listings = v?.listings ? v.listings.map(v => new strims_network_v1_directory_NetworkListings(v)) : [];
  }

  static encode(m: FrontendGetListingsResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.listings) strims_network_v1_directory_NetworkListings.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendGetListingsResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new FrontendGetListingsResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.listings.push(strims_network_v1_directory_NetworkListings.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IFrontendWatchListingsRequest = {
  contentTypes?: strims_network_v1_directory_ListingContentType[];
  networkKeys?: Uint8Array[];
  listingId?: bigint;
}

export class FrontendWatchListingsRequest {
  contentTypes: strims_network_v1_directory_ListingContentType[];
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
    const end = length === undefined ? r.len : r.pos + length;
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
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IFrontendWatchListingsResponse = {
  events?: strims_network_v1_directory_FrontendWatchListingsResponse_IEvent[];
}

export class FrontendWatchListingsResponse {
  events: strims_network_v1_directory_FrontendWatchListingsResponse_Event[];

  constructor(v?: IFrontendWatchListingsResponse) {
    this.events = v?.events ? v.events.map(v => new strims_network_v1_directory_FrontendWatchListingsResponse_Event(v)) : [];
  }

  static encode(m: FrontendWatchListingsResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.events) strims_network_v1_directory_FrontendWatchListingsResponse_Event.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendWatchListingsResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new FrontendWatchListingsResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.events.push(strims_network_v1_directory_FrontendWatchListingsResponse_Event.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace FrontendWatchListingsResponse {
  export type IChange = {
    listings?: strims_network_v1_directory_INetworkListings;
  }

  export class Change {
    listings: strims_network_v1_directory_NetworkListings | undefined;

    constructor(v?: IChange) {
      this.listings = v?.listings && new strims_network_v1_directory_NetworkListings(v.listings);
    }

    static encode(m: Change, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.listings) strims_network_v1_directory_NetworkListings.encode(m.listings, w.uint32(10).fork()).ldelim();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Change {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Change();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.listings = strims_network_v1_directory_NetworkListings.decode(r, r.uint32());
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type IUnpublish = {
    networkId?: bigint;
    listingId?: bigint;
  }

  export class Unpublish {
    networkId: bigint;
    listingId: bigint;

    constructor(v?: IUnpublish) {
      this.networkId = v?.networkId || BigInt(0);
      this.listingId = v?.listingId || BigInt(0);
    }

    static encode(m: Unpublish, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.networkId) w.uint32(8).uint64(m.networkId);
      if (m.listingId) w.uint32(16).uint64(m.listingId);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Unpublish {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Unpublish();
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
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type IUserCountChange = {
    networkId?: bigint;
    listingId?: bigint;
    userCount?: number;
    recentUserCount?: number;
  }

  export class UserCountChange {
    networkId: bigint;
    listingId: bigint;
    userCount: number;
    recentUserCount: number;

    constructor(v?: IUserCountChange) {
      this.networkId = v?.networkId || BigInt(0);
      this.listingId = v?.listingId || BigInt(0);
      this.userCount = v?.userCount || 0;
      this.recentUserCount = v?.recentUserCount || 0;
    }

    static encode(m: UserCountChange, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.networkId) w.uint32(8).uint64(m.networkId);
      if (m.listingId) w.uint32(16).uint64(m.listingId);
      if (m.userCount) w.uint32(24).uint32(m.userCount);
      if (m.recentUserCount) w.uint32(32).uint32(m.recentUserCount);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): UserCountChange {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new UserCountChange();
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
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type IEvent = {
    event?: Event.IEvent
  }

  export class Event {
    event: Event.TEvent;

    constructor(v?: IEvent) {
      this.event = new Event.Event(v?.event);
    }

    static encode(m: Event, w?: Writer): Writer {
      if (!w) w = new Writer();
      switch (m.event.case) {
        case Event.EventCase.CHANGE:
        strims_network_v1_directory_FrontendWatchListingsResponse_Change.encode(m.event.change, w.uint32(8010).fork()).ldelim();
        break;
        case Event.EventCase.UNPUBLISH:
        strims_network_v1_directory_FrontendWatchListingsResponse_Unpublish.encode(m.event.unpublish, w.uint32(8018).fork()).ldelim();
        break;
        case Event.EventCase.USER_COUNT_CHANGE:
        strims_network_v1_directory_FrontendWatchListingsResponse_UserCountChange.encode(m.event.userCountChange, w.uint32(8026).fork()).ldelim();
        break;
      }
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Event {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Event();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1001:
          m.event = new Event.Event({ change: strims_network_v1_directory_FrontendWatchListingsResponse_Change.decode(r, r.uint32()) });
          break;
          case 1002:
          m.event = new Event.Event({ unpublish: strims_network_v1_directory_FrontendWatchListingsResponse_Unpublish.decode(r, r.uint32()) });
          break;
          case 1003:
          m.event = new Event.Event({ userCountChange: strims_network_v1_directory_FrontendWatchListingsResponse_UserCountChange.decode(r, r.uint32()) });
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export namespace Event {
    export enum EventCase {
      NOT_SET = 0,
      CHANGE = 1001,
      UNPUBLISH = 1002,
      USER_COUNT_CHANGE = 1003,
    }

    export type IEvent =
    { case?: EventCase.NOT_SET }
    |{ case?: EventCase.CHANGE, change: strims_network_v1_directory_FrontendWatchListingsResponse_IChange }
    |{ case?: EventCase.UNPUBLISH, unpublish: strims_network_v1_directory_FrontendWatchListingsResponse_IUnpublish }
    |{ case?: EventCase.USER_COUNT_CHANGE, userCountChange: strims_network_v1_directory_FrontendWatchListingsResponse_IUserCountChange }
    ;

    export type TEvent = Readonly<
    { case: EventCase.NOT_SET }
    |{ case: EventCase.CHANGE, change: strims_network_v1_directory_FrontendWatchListingsResponse_Change }
    |{ case: EventCase.UNPUBLISH, unpublish: strims_network_v1_directory_FrontendWatchListingsResponse_Unpublish }
    |{ case: EventCase.USER_COUNT_CHANGE, userCountChange: strims_network_v1_directory_FrontendWatchListingsResponse_UserCountChange }
    >;

    class EventImpl {
      change: strims_network_v1_directory_FrontendWatchListingsResponse_Change;
      unpublish: strims_network_v1_directory_FrontendWatchListingsResponse_Unpublish;
      userCountChange: strims_network_v1_directory_FrontendWatchListingsResponse_UserCountChange;
      case: EventCase = EventCase.NOT_SET;

      constructor(v?: IEvent) {
        if (v && "change" in v) {
          this.case = EventCase.CHANGE;
          this.change = new strims_network_v1_directory_FrontendWatchListingsResponse_Change(v.change);
        } else
        if (v && "unpublish" in v) {
          this.case = EventCase.UNPUBLISH;
          this.unpublish = new strims_network_v1_directory_FrontendWatchListingsResponse_Unpublish(v.unpublish);
        } else
        if (v && "userCountChange" in v) {
          this.case = EventCase.USER_COUNT_CHANGE;
          this.userCountChange = new strims_network_v1_directory_FrontendWatchListingsResponse_UserCountChange(v.userCountChange);
        }
      }
    }

    export const Event = EventImpl as {
      new (): Readonly<{ case: EventCase.NOT_SET }>;
      new <T extends IEvent>(v: T): Readonly<
      T extends { change: strims_network_v1_directory_FrontendWatchListingsResponse_IChange } ? { case: EventCase.CHANGE, change: strims_network_v1_directory_FrontendWatchListingsResponse_Change } :
      T extends { unpublish: strims_network_v1_directory_FrontendWatchListingsResponse_IUnpublish } ? { case: EventCase.UNPUBLISH, unpublish: strims_network_v1_directory_FrontendWatchListingsResponse_Unpublish } :
      T extends { userCountChange: strims_network_v1_directory_FrontendWatchListingsResponse_IUserCountChange } ? { case: EventCase.USER_COUNT_CHANGE, userCountChange: strims_network_v1_directory_FrontendWatchListingsResponse_UserCountChange } :
      never
      >;
    };

  }

}

export type IFrontendWatchListingUsersRequest = {
  networkKey?: Uint8Array;
  query?: strims_network_v1_directory_IListingQuery;
}

export class FrontendWatchListingUsersRequest {
  networkKey: Uint8Array;
  query: strims_network_v1_directory_ListingQuery | undefined;

  constructor(v?: IFrontendWatchListingUsersRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.query = v?.query && new strims_network_v1_directory_ListingQuery(v.query);
  }

  static encode(m: FrontendWatchListingUsersRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    if (m.query) strims_network_v1_directory_ListingQuery.encode(m.query, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendWatchListingUsersRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new FrontendWatchListingUsersRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkKey = r.bytes();
        break;
        case 2:
        m.query = strims_network_v1_directory_ListingQuery.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IFrontendWatchListingUsersResponse = {
  type?: strims_network_v1_directory_FrontendWatchListingUsersResponse_UserEventType;
  users?: strims_network_v1_directory_FrontendWatchListingUsersResponse_IUser[];
}

export class FrontendWatchListingUsersResponse {
  type: strims_network_v1_directory_FrontendWatchListingUsersResponse_UserEventType;
  users: strims_network_v1_directory_FrontendWatchListingUsersResponse_User[];

  constructor(v?: IFrontendWatchListingUsersResponse) {
    this.type = v?.type || 0;
    this.users = v?.users ? v.users.map(v => new strims_network_v1_directory_FrontendWatchListingUsersResponse_User(v)) : [];
  }

  static encode(m: FrontendWatchListingUsersResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.type) w.uint32(8).uint32(m.type);
    for (const v of m.users) strims_network_v1_directory_FrontendWatchListingUsersResponse_User.encode(v, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendWatchListingUsersResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new FrontendWatchListingUsersResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.type = r.uint32();
        break;
        case 2:
        m.users.push(strims_network_v1_directory_FrontendWatchListingUsersResponse_User.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace FrontendWatchListingUsersResponse {
  export type IUser = {
    id?: bigint;
    alias?: string;
    peerKey?: Uint8Array;
  }

  export class User {
    id: bigint;
    alias: string;
    peerKey: Uint8Array;

    constructor(v?: IUser) {
      this.id = v?.id || BigInt(0);
      this.alias = v?.alias || "";
      this.peerKey = v?.peerKey || new Uint8Array();
    }

    static encode(m: User, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.id) w.uint32(8).uint64(m.id);
      if (m.alias.length) w.uint32(18).string(m.alias);
      if (m.peerKey.length) w.uint32(26).bytes(m.peerKey);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): User {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new User();
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
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export enum UserEventType {
    USER_EVENT_TYPE_JOIN = 0,
    USER_EVENT_TYPE_PART = 1,
    USER_EVENT_TYPE_RENAME = 2,
  }
}

export type IFrontendWatchAssetBundlesRequest = Record<string, any>;

export class FrontendWatchAssetBundlesRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IFrontendWatchAssetBundlesRequest) {
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
  assetBundle?: strims_network_v1_directory_IAssetBundle;
}

export class FrontendWatchAssetBundlesResponse {
  networkId: bigint;
  networkKey: Uint8Array;
  assetBundle: strims_network_v1_directory_AssetBundle | undefined;

  constructor(v?: IFrontendWatchAssetBundlesResponse) {
    this.networkId = v?.networkId || BigInt(0);
    this.networkKey = v?.networkKey || new Uint8Array();
    this.assetBundle = v?.assetBundle && new strims_network_v1_directory_AssetBundle(v.assetBundle);
  }

  static encode(m: FrontendWatchAssetBundlesResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkId) w.uint32(8).uint64(m.networkId);
    if (m.networkKey.length) w.uint32(18).bytes(m.networkKey);
    if (m.assetBundle) strims_network_v1_directory_AssetBundle.encode(m.assetBundle, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendWatchAssetBundlesResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
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
        m.assetBundle = strims_network_v1_directory_AssetBundle.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new SnippetSubscribeRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.swarmId = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ISnippetSubscribeResponse = {
  snippetDelta?: strims_network_v1_directory_IListingSnippetDelta;
}

export class SnippetSubscribeResponse {
  snippetDelta: strims_network_v1_directory_ListingSnippetDelta | undefined;

  constructor(v?: ISnippetSubscribeResponse) {
    this.snippetDelta = v?.snippetDelta && new strims_network_v1_directory_ListingSnippetDelta(v.snippetDelta);
  }

  static encode(m: SnippetSubscribeResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.snippetDelta) strims_network_v1_directory_ListingSnippetDelta.encode(m.snippetDelta, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SnippetSubscribeResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new SnippetSubscribeResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.snippetDelta = strims_network_v1_directory_ListingSnippetDelta.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
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
/* @internal */
export const strims_network_v1_directory_ServerConfig = ServerConfig;
/* @internal */
export type strims_network_v1_directory_ServerConfig = ServerConfig;
/* @internal */
export type strims_network_v1_directory_IServerConfig = IServerConfig;
/* @internal */
export const strims_network_v1_directory_ClientConfig = ClientConfig;
/* @internal */
export type strims_network_v1_directory_ClientConfig = ClientConfig;
/* @internal */
export type strims_network_v1_directory_IClientConfig = IClientConfig;
/* @internal */
export const strims_network_v1_directory_GetEventsRequest = GetEventsRequest;
/* @internal */
export type strims_network_v1_directory_GetEventsRequest = GetEventsRequest;
/* @internal */
export type strims_network_v1_directory_IGetEventsRequest = IGetEventsRequest;
/* @internal */
export const strims_network_v1_directory_TestPublishRequest = TestPublishRequest;
/* @internal */
export type strims_network_v1_directory_TestPublishRequest = TestPublishRequest;
/* @internal */
export type strims_network_v1_directory_ITestPublishRequest = ITestPublishRequest;
/* @internal */
export const strims_network_v1_directory_TestPublishResponse = TestPublishResponse;
/* @internal */
export type strims_network_v1_directory_TestPublishResponse = TestPublishResponse;
/* @internal */
export type strims_network_v1_directory_ITestPublishResponse = ITestPublishResponse;
/* @internal */
export const strims_network_v1_directory_Listing = Listing;
/* @internal */
export type strims_network_v1_directory_Listing = Listing;
/* @internal */
export type strims_network_v1_directory_IListing = IListing;
/* @internal */
export const strims_network_v1_directory_ListingSnippetImage = ListingSnippetImage;
/* @internal */
export type strims_network_v1_directory_ListingSnippetImage = ListingSnippetImage;
/* @internal */
export type strims_network_v1_directory_IListingSnippetImage = IListingSnippetImage;
/* @internal */
export const strims_network_v1_directory_ListingSnippet = ListingSnippet;
/* @internal */
export type strims_network_v1_directory_ListingSnippet = ListingSnippet;
/* @internal */
export type strims_network_v1_directory_IListingSnippet = IListingSnippet;
/* @internal */
export const strims_network_v1_directory_ListingSnippetDelta = ListingSnippetDelta;
/* @internal */
export type strims_network_v1_directory_ListingSnippetDelta = ListingSnippetDelta;
/* @internal */
export type strims_network_v1_directory_IListingSnippetDelta = IListingSnippetDelta;
/* @internal */
export const strims_network_v1_directory_Event = Event;
/* @internal */
export type strims_network_v1_directory_Event = Event;
/* @internal */
export type strims_network_v1_directory_IEvent = IEvent;
/* @internal */
export const strims_network_v1_directory_ListingModeration = ListingModeration;
/* @internal */
export type strims_network_v1_directory_ListingModeration = ListingModeration;
/* @internal */
export type strims_network_v1_directory_IListingModeration = IListingModeration;
/* @internal */
export const strims_network_v1_directory_ListingQuery = ListingQuery;
/* @internal */
export type strims_network_v1_directory_ListingQuery = ListingQuery;
/* @internal */
export type strims_network_v1_directory_IListingQuery = IListingQuery;
/* @internal */
export const strims_network_v1_directory_ListingRecord = ListingRecord;
/* @internal */
export type strims_network_v1_directory_ListingRecord = ListingRecord;
/* @internal */
export type strims_network_v1_directory_IListingRecord = IListingRecord;
/* @internal */
export const strims_network_v1_directory_UserModeration = UserModeration;
/* @internal */
export type strims_network_v1_directory_UserModeration = UserModeration;
/* @internal */
export type strims_network_v1_directory_IUserModeration = IUserModeration;
/* @internal */
export const strims_network_v1_directory_UserRecord = UserRecord;
/* @internal */
export type strims_network_v1_directory_UserRecord = UserRecord;
/* @internal */
export type strims_network_v1_directory_IUserRecord = IUserRecord;
/* @internal */
export const strims_network_v1_directory_EventBroadcast = EventBroadcast;
/* @internal */
export type strims_network_v1_directory_EventBroadcast = EventBroadcast;
/* @internal */
export type strims_network_v1_directory_IEventBroadcast = IEventBroadcast;
/* @internal */
export const strims_network_v1_directory_AssetBundle = AssetBundle;
/* @internal */
export type strims_network_v1_directory_AssetBundle = AssetBundle;
/* @internal */
export type strims_network_v1_directory_IAssetBundle = IAssetBundle;
/* @internal */
export const strims_network_v1_directory_PublishRequest = PublishRequest;
/* @internal */
export type strims_network_v1_directory_PublishRequest = PublishRequest;
/* @internal */
export type strims_network_v1_directory_IPublishRequest = IPublishRequest;
/* @internal */
export const strims_network_v1_directory_PublishResponse = PublishResponse;
/* @internal */
export type strims_network_v1_directory_PublishResponse = PublishResponse;
/* @internal */
export type strims_network_v1_directory_IPublishResponse = IPublishResponse;
/* @internal */
export const strims_network_v1_directory_UnpublishRequest = UnpublishRequest;
/* @internal */
export type strims_network_v1_directory_UnpublishRequest = UnpublishRequest;
/* @internal */
export type strims_network_v1_directory_IUnpublishRequest = IUnpublishRequest;
/* @internal */
export const strims_network_v1_directory_UnpublishResponse = UnpublishResponse;
/* @internal */
export type strims_network_v1_directory_UnpublishResponse = UnpublishResponse;
/* @internal */
export type strims_network_v1_directory_IUnpublishResponse = IUnpublishResponse;
/* @internal */
export const strims_network_v1_directory_JoinRequest = JoinRequest;
/* @internal */
export type strims_network_v1_directory_JoinRequest = JoinRequest;
/* @internal */
export type strims_network_v1_directory_IJoinRequest = IJoinRequest;
/* @internal */
export const strims_network_v1_directory_JoinResponse = JoinResponse;
/* @internal */
export type strims_network_v1_directory_JoinResponse = JoinResponse;
/* @internal */
export type strims_network_v1_directory_IJoinResponse = IJoinResponse;
/* @internal */
export const strims_network_v1_directory_PartRequest = PartRequest;
/* @internal */
export type strims_network_v1_directory_PartRequest = PartRequest;
/* @internal */
export type strims_network_v1_directory_IPartRequest = IPartRequest;
/* @internal */
export const strims_network_v1_directory_PartResponse = PartResponse;
/* @internal */
export type strims_network_v1_directory_PartResponse = PartResponse;
/* @internal */
export type strims_network_v1_directory_IPartResponse = IPartResponse;
/* @internal */
export const strims_network_v1_directory_PingRequest = PingRequest;
/* @internal */
export type strims_network_v1_directory_PingRequest = PingRequest;
/* @internal */
export type strims_network_v1_directory_IPingRequest = IPingRequest;
/* @internal */
export const strims_network_v1_directory_PingResponse = PingResponse;
/* @internal */
export type strims_network_v1_directory_PingResponse = PingResponse;
/* @internal */
export type strims_network_v1_directory_IPingResponse = IPingResponse;
/* @internal */
export const strims_network_v1_directory_ModerateListingRequest = ModerateListingRequest;
/* @internal */
export type strims_network_v1_directory_ModerateListingRequest = ModerateListingRequest;
/* @internal */
export type strims_network_v1_directory_IModerateListingRequest = IModerateListingRequest;
/* @internal */
export const strims_network_v1_directory_ModerateListingResponse = ModerateListingResponse;
/* @internal */
export type strims_network_v1_directory_ModerateListingResponse = ModerateListingResponse;
/* @internal */
export type strims_network_v1_directory_IModerateListingResponse = IModerateListingResponse;
/* @internal */
export const strims_network_v1_directory_ModerateUserRequest = ModerateUserRequest;
/* @internal */
export type strims_network_v1_directory_ModerateUserRequest = ModerateUserRequest;
/* @internal */
export type strims_network_v1_directory_IModerateUserRequest = IModerateUserRequest;
/* @internal */
export const strims_network_v1_directory_ModerateUserResponse = ModerateUserResponse;
/* @internal */
export type strims_network_v1_directory_ModerateUserResponse = ModerateUserResponse;
/* @internal */
export type strims_network_v1_directory_IModerateUserResponse = IModerateUserResponse;
/* @internal */
export const strims_network_v1_directory_Network = Network;
/* @internal */
export type strims_network_v1_directory_Network = Network;
/* @internal */
export type strims_network_v1_directory_INetwork = INetwork;
/* @internal */
export const strims_network_v1_directory_NetworkListingsItem = NetworkListingsItem;
/* @internal */
export type strims_network_v1_directory_NetworkListingsItem = NetworkListingsItem;
/* @internal */
export type strims_network_v1_directory_INetworkListingsItem = INetworkListingsItem;
/* @internal */
export const strims_network_v1_directory_NetworkListings = NetworkListings;
/* @internal */
export type strims_network_v1_directory_NetworkListings = NetworkListings;
/* @internal */
export type strims_network_v1_directory_INetworkListings = INetworkListings;
/* @internal */
export const strims_network_v1_directory_FrontendPublishRequest = FrontendPublishRequest;
/* @internal */
export type strims_network_v1_directory_FrontendPublishRequest = FrontendPublishRequest;
/* @internal */
export type strims_network_v1_directory_IFrontendPublishRequest = IFrontendPublishRequest;
/* @internal */
export const strims_network_v1_directory_FrontendPublishResponse = FrontendPublishResponse;
/* @internal */
export type strims_network_v1_directory_FrontendPublishResponse = FrontendPublishResponse;
/* @internal */
export type strims_network_v1_directory_IFrontendPublishResponse = IFrontendPublishResponse;
/* @internal */
export const strims_network_v1_directory_FrontendUnpublishRequest = FrontendUnpublishRequest;
/* @internal */
export type strims_network_v1_directory_FrontendUnpublishRequest = FrontendUnpublishRequest;
/* @internal */
export type strims_network_v1_directory_IFrontendUnpublishRequest = IFrontendUnpublishRequest;
/* @internal */
export const strims_network_v1_directory_FrontendUnpublishResponse = FrontendUnpublishResponse;
/* @internal */
export type strims_network_v1_directory_FrontendUnpublishResponse = FrontendUnpublishResponse;
/* @internal */
export type strims_network_v1_directory_IFrontendUnpublishResponse = IFrontendUnpublishResponse;
/* @internal */
export const strims_network_v1_directory_FrontendJoinRequest = FrontendJoinRequest;
/* @internal */
export type strims_network_v1_directory_FrontendJoinRequest = FrontendJoinRequest;
/* @internal */
export type strims_network_v1_directory_IFrontendJoinRequest = IFrontendJoinRequest;
/* @internal */
export const strims_network_v1_directory_FrontendJoinResponse = FrontendJoinResponse;
/* @internal */
export type strims_network_v1_directory_FrontendJoinResponse = FrontendJoinResponse;
/* @internal */
export type strims_network_v1_directory_IFrontendJoinResponse = IFrontendJoinResponse;
/* @internal */
export const strims_network_v1_directory_FrontendPartRequest = FrontendPartRequest;
/* @internal */
export type strims_network_v1_directory_FrontendPartRequest = FrontendPartRequest;
/* @internal */
export type strims_network_v1_directory_IFrontendPartRequest = IFrontendPartRequest;
/* @internal */
export const strims_network_v1_directory_FrontendPartResponse = FrontendPartResponse;
/* @internal */
export type strims_network_v1_directory_FrontendPartResponse = FrontendPartResponse;
/* @internal */
export type strims_network_v1_directory_IFrontendPartResponse = IFrontendPartResponse;
/* @internal */
export const strims_network_v1_directory_FrontendTestRequest = FrontendTestRequest;
/* @internal */
export type strims_network_v1_directory_FrontendTestRequest = FrontendTestRequest;
/* @internal */
export type strims_network_v1_directory_IFrontendTestRequest = IFrontendTestRequest;
/* @internal */
export const strims_network_v1_directory_FrontendTestResponse = FrontendTestResponse;
/* @internal */
export type strims_network_v1_directory_FrontendTestResponse = FrontendTestResponse;
/* @internal */
export type strims_network_v1_directory_IFrontendTestResponse = IFrontendTestResponse;
/* @internal */
export const strims_network_v1_directory_FrontendModerateListingRequest = FrontendModerateListingRequest;
/* @internal */
export type strims_network_v1_directory_FrontendModerateListingRequest = FrontendModerateListingRequest;
/* @internal */
export type strims_network_v1_directory_IFrontendModerateListingRequest = IFrontendModerateListingRequest;
/* @internal */
export const strims_network_v1_directory_FrontendModerateListingResponse = FrontendModerateListingResponse;
/* @internal */
export type strims_network_v1_directory_FrontendModerateListingResponse = FrontendModerateListingResponse;
/* @internal */
export type strims_network_v1_directory_IFrontendModerateListingResponse = IFrontendModerateListingResponse;
/* @internal */
export const strims_network_v1_directory_FrontendModerateUserRequest = FrontendModerateUserRequest;
/* @internal */
export type strims_network_v1_directory_FrontendModerateUserRequest = FrontendModerateUserRequest;
/* @internal */
export type strims_network_v1_directory_IFrontendModerateUserRequest = IFrontendModerateUserRequest;
/* @internal */
export const strims_network_v1_directory_FrontendModerateUserResponse = FrontendModerateUserResponse;
/* @internal */
export type strims_network_v1_directory_FrontendModerateUserResponse = FrontendModerateUserResponse;
/* @internal */
export type strims_network_v1_directory_IFrontendModerateUserResponse = IFrontendModerateUserResponse;
/* @internal */
export const strims_network_v1_directory_FrontendGetUsersRequest = FrontendGetUsersRequest;
/* @internal */
export type strims_network_v1_directory_FrontendGetUsersRequest = FrontendGetUsersRequest;
/* @internal */
export type strims_network_v1_directory_IFrontendGetUsersRequest = IFrontendGetUsersRequest;
/* @internal */
export const strims_network_v1_directory_FrontendGetUsersResponse = FrontendGetUsersResponse;
/* @internal */
export type strims_network_v1_directory_FrontendGetUsersResponse = FrontendGetUsersResponse;
/* @internal */
export type strims_network_v1_directory_IFrontendGetUsersResponse = IFrontendGetUsersResponse;
/* @internal */
export const strims_network_v1_directory_FrontendGetListingRequest = FrontendGetListingRequest;
/* @internal */
export type strims_network_v1_directory_FrontendGetListingRequest = FrontendGetListingRequest;
/* @internal */
export type strims_network_v1_directory_IFrontendGetListingRequest = IFrontendGetListingRequest;
/* @internal */
export const strims_network_v1_directory_FrontendGetListingResponse = FrontendGetListingResponse;
/* @internal */
export type strims_network_v1_directory_FrontendGetListingResponse = FrontendGetListingResponse;
/* @internal */
export type strims_network_v1_directory_IFrontendGetListingResponse = IFrontendGetListingResponse;
/* @internal */
export const strims_network_v1_directory_FrontendGetListingsRequest = FrontendGetListingsRequest;
/* @internal */
export type strims_network_v1_directory_FrontendGetListingsRequest = FrontendGetListingsRequest;
/* @internal */
export type strims_network_v1_directory_IFrontendGetListingsRequest = IFrontendGetListingsRequest;
/* @internal */
export const strims_network_v1_directory_FrontendGetListingsResponse = FrontendGetListingsResponse;
/* @internal */
export type strims_network_v1_directory_FrontendGetListingsResponse = FrontendGetListingsResponse;
/* @internal */
export type strims_network_v1_directory_IFrontendGetListingsResponse = IFrontendGetListingsResponse;
/* @internal */
export const strims_network_v1_directory_FrontendWatchListingsRequest = FrontendWatchListingsRequest;
/* @internal */
export type strims_network_v1_directory_FrontendWatchListingsRequest = FrontendWatchListingsRequest;
/* @internal */
export type strims_network_v1_directory_IFrontendWatchListingsRequest = IFrontendWatchListingsRequest;
/* @internal */
export const strims_network_v1_directory_FrontendWatchListingsResponse = FrontendWatchListingsResponse;
/* @internal */
export type strims_network_v1_directory_FrontendWatchListingsResponse = FrontendWatchListingsResponse;
/* @internal */
export type strims_network_v1_directory_IFrontendWatchListingsResponse = IFrontendWatchListingsResponse;
/* @internal */
export const strims_network_v1_directory_FrontendWatchListingUsersRequest = FrontendWatchListingUsersRequest;
/* @internal */
export type strims_network_v1_directory_FrontendWatchListingUsersRequest = FrontendWatchListingUsersRequest;
/* @internal */
export type strims_network_v1_directory_IFrontendWatchListingUsersRequest = IFrontendWatchListingUsersRequest;
/* @internal */
export const strims_network_v1_directory_FrontendWatchListingUsersResponse = FrontendWatchListingUsersResponse;
/* @internal */
export type strims_network_v1_directory_FrontendWatchListingUsersResponse = FrontendWatchListingUsersResponse;
/* @internal */
export type strims_network_v1_directory_IFrontendWatchListingUsersResponse = IFrontendWatchListingUsersResponse;
/* @internal */
export const strims_network_v1_directory_FrontendWatchAssetBundlesRequest = FrontendWatchAssetBundlesRequest;
/* @internal */
export type strims_network_v1_directory_FrontendWatchAssetBundlesRequest = FrontendWatchAssetBundlesRequest;
/* @internal */
export type strims_network_v1_directory_IFrontendWatchAssetBundlesRequest = IFrontendWatchAssetBundlesRequest;
/* @internal */
export const strims_network_v1_directory_FrontendWatchAssetBundlesResponse = FrontendWatchAssetBundlesResponse;
/* @internal */
export type strims_network_v1_directory_FrontendWatchAssetBundlesResponse = FrontendWatchAssetBundlesResponse;
/* @internal */
export type strims_network_v1_directory_IFrontendWatchAssetBundlesResponse = IFrontendWatchAssetBundlesResponse;
/* @internal */
export const strims_network_v1_directory_SnippetSubscribeRequest = SnippetSubscribeRequest;
/* @internal */
export type strims_network_v1_directory_SnippetSubscribeRequest = SnippetSubscribeRequest;
/* @internal */
export type strims_network_v1_directory_ISnippetSubscribeRequest = ISnippetSubscribeRequest;
/* @internal */
export const strims_network_v1_directory_SnippetSubscribeResponse = SnippetSubscribeResponse;
/* @internal */
export type strims_network_v1_directory_SnippetSubscribeResponse = SnippetSubscribeResponse;
/* @internal */
export type strims_network_v1_directory_ISnippetSubscribeResponse = ISnippetSubscribeResponse;
/* @internal */
export const strims_network_v1_directory_ServerConfig_Integrations = ServerConfig.Integrations;
/* @internal */
export type strims_network_v1_directory_ServerConfig_Integrations = ServerConfig.Integrations;
/* @internal */
export type strims_network_v1_directory_ServerConfig_IIntegrations = ServerConfig.IIntegrations;
/* @internal */
export const strims_network_v1_directory_ServerConfig_Integrations_AngelThump = ServerConfig.Integrations.AngelThump;
/* @internal */
export type strims_network_v1_directory_ServerConfig_Integrations_AngelThump = ServerConfig.Integrations.AngelThump;
/* @internal */
export type strims_network_v1_directory_ServerConfig_Integrations_IAngelThump = ServerConfig.Integrations.IAngelThump;
/* @internal */
export const strims_network_v1_directory_ServerConfig_Integrations_Twitch = ServerConfig.Integrations.Twitch;
/* @internal */
export type strims_network_v1_directory_ServerConfig_Integrations_Twitch = ServerConfig.Integrations.Twitch;
/* @internal */
export type strims_network_v1_directory_ServerConfig_Integrations_ITwitch = ServerConfig.Integrations.ITwitch;
/* @internal */
export const strims_network_v1_directory_ServerConfig_Integrations_YouTube = ServerConfig.Integrations.YouTube;
/* @internal */
export type strims_network_v1_directory_ServerConfig_Integrations_YouTube = ServerConfig.Integrations.YouTube;
/* @internal */
export type strims_network_v1_directory_ServerConfig_Integrations_IYouTube = ServerConfig.Integrations.IYouTube;
/* @internal */
export const strims_network_v1_directory_ServerConfig_Integrations_Swarm = ServerConfig.Integrations.Swarm;
/* @internal */
export type strims_network_v1_directory_ServerConfig_Integrations_Swarm = ServerConfig.Integrations.Swarm;
/* @internal */
export type strims_network_v1_directory_ServerConfig_Integrations_ISwarm = ServerConfig.Integrations.ISwarm;
/* @internal */
export const strims_network_v1_directory_ClientConfig_Integrations = ClientConfig.Integrations;
/* @internal */
export type strims_network_v1_directory_ClientConfig_Integrations = ClientConfig.Integrations;
/* @internal */
export type strims_network_v1_directory_ClientConfig_IIntegrations = ClientConfig.IIntegrations;
/* @internal */
export const strims_network_v1_directory_Listing_Media = Listing.Media;
/* @internal */
export type strims_network_v1_directory_Listing_Media = Listing.Media;
/* @internal */
export type strims_network_v1_directory_Listing_IMedia = Listing.IMedia;
/* @internal */
export const strims_network_v1_directory_Listing_Service = Listing.Service;
/* @internal */
export type strims_network_v1_directory_Listing_Service = Listing.Service;
/* @internal */
export type strims_network_v1_directory_Listing_IService = Listing.IService;
/* @internal */
export const strims_network_v1_directory_Listing_Embed = Listing.Embed;
/* @internal */
export type strims_network_v1_directory_Listing_Embed = Listing.Embed;
/* @internal */
export type strims_network_v1_directory_Listing_IEmbed = Listing.IEmbed;
/* @internal */
export const strims_network_v1_directory_Listing_Chat = Listing.Chat;
/* @internal */
export type strims_network_v1_directory_Listing_Chat = Listing.Chat;
/* @internal */
export type strims_network_v1_directory_Listing_IChat = Listing.IChat;
/* @internal */
export const strims_network_v1_directory_ListingSnippetDelta_Tags = ListingSnippetDelta.Tags;
/* @internal */
export type strims_network_v1_directory_ListingSnippetDelta_Tags = ListingSnippetDelta.Tags;
/* @internal */
export type strims_network_v1_directory_ListingSnippetDelta_ITags = ListingSnippetDelta.ITags;
/* @internal */
export const strims_network_v1_directory_Event_ListingChange = Event.ListingChange;
/* @internal */
export type strims_network_v1_directory_Event_ListingChange = Event.ListingChange;
/* @internal */
export type strims_network_v1_directory_Event_IListingChange = Event.IListingChange;
/* @internal */
export const strims_network_v1_directory_Event_Unpublish = Event.Unpublish;
/* @internal */
export type strims_network_v1_directory_Event_Unpublish = Event.Unpublish;
/* @internal */
export type strims_network_v1_directory_Event_IUnpublish = Event.IUnpublish;
/* @internal */
export const strims_network_v1_directory_Event_UserCountChange = Event.UserCountChange;
/* @internal */
export type strims_network_v1_directory_Event_UserCountChange = Event.UserCountChange;
/* @internal */
export type strims_network_v1_directory_Event_IUserCountChange = Event.IUserCountChange;
/* @internal */
export const strims_network_v1_directory_Event_UserPresenceChange = Event.UserPresenceChange;
/* @internal */
export type strims_network_v1_directory_Event_UserPresenceChange = Event.UserPresenceChange;
/* @internal */
export type strims_network_v1_directory_Event_IUserPresenceChange = Event.IUserPresenceChange;
/* @internal */
export const strims_network_v1_directory_Event_Ping = Event.Ping;
/* @internal */
export type strims_network_v1_directory_Event_Ping = Event.Ping;
/* @internal */
export type strims_network_v1_directory_Event_IPing = Event.IPing;
/* @internal */
export const strims_network_v1_directory_FrontendGetUsersResponse_Alias = FrontendGetUsersResponse.Alias;
/* @internal */
export type strims_network_v1_directory_FrontendGetUsersResponse_Alias = FrontendGetUsersResponse.Alias;
/* @internal */
export type strims_network_v1_directory_FrontendGetUsersResponse_IAlias = FrontendGetUsersResponse.IAlias;
/* @internal */
export const strims_network_v1_directory_FrontendGetUsersResponse_User = FrontendGetUsersResponse.User;
/* @internal */
export type strims_network_v1_directory_FrontendGetUsersResponse_User = FrontendGetUsersResponse.User;
/* @internal */
export type strims_network_v1_directory_FrontendGetUsersResponse_IUser = FrontendGetUsersResponse.IUser;
/* @internal */
export const strims_network_v1_directory_FrontendWatchListingsResponse_Change = FrontendWatchListingsResponse.Change;
/* @internal */
export type strims_network_v1_directory_FrontendWatchListingsResponse_Change = FrontendWatchListingsResponse.Change;
/* @internal */
export type strims_network_v1_directory_FrontendWatchListingsResponse_IChange = FrontendWatchListingsResponse.IChange;
/* @internal */
export const strims_network_v1_directory_FrontendWatchListingsResponse_Unpublish = FrontendWatchListingsResponse.Unpublish;
/* @internal */
export type strims_network_v1_directory_FrontendWatchListingsResponse_Unpublish = FrontendWatchListingsResponse.Unpublish;
/* @internal */
export type strims_network_v1_directory_FrontendWatchListingsResponse_IUnpublish = FrontendWatchListingsResponse.IUnpublish;
/* @internal */
export const strims_network_v1_directory_FrontendWatchListingsResponse_UserCountChange = FrontendWatchListingsResponse.UserCountChange;
/* @internal */
export type strims_network_v1_directory_FrontendWatchListingsResponse_UserCountChange = FrontendWatchListingsResponse.UserCountChange;
/* @internal */
export type strims_network_v1_directory_FrontendWatchListingsResponse_IUserCountChange = FrontendWatchListingsResponse.IUserCountChange;
/* @internal */
export const strims_network_v1_directory_FrontendWatchListingsResponse_Event = FrontendWatchListingsResponse.Event;
/* @internal */
export type strims_network_v1_directory_FrontendWatchListingsResponse_Event = FrontendWatchListingsResponse.Event;
/* @internal */
export type strims_network_v1_directory_FrontendWatchListingsResponse_IEvent = FrontendWatchListingsResponse.IEvent;
/* @internal */
export const strims_network_v1_directory_FrontendWatchListingUsersResponse_User = FrontendWatchListingUsersResponse.User;
/* @internal */
export type strims_network_v1_directory_FrontendWatchListingUsersResponse_User = FrontendWatchListingUsersResponse.User;
/* @internal */
export type strims_network_v1_directory_FrontendWatchListingUsersResponse_IUser = FrontendWatchListingUsersResponse.IUser;
/* @internal */
export const strims_network_v1_directory_ListingContentType = ListingContentType;
/* @internal */
export type strims_network_v1_directory_ListingContentType = ListingContentType;
/* @internal */
export const strims_network_v1_directory_Listing_Embed_Service = Listing.Embed.Service;
/* @internal */
export type strims_network_v1_directory_Listing_Embed_Service = Listing.Embed.Service;
/* @internal */
export const strims_network_v1_directory_FrontendWatchListingUsersResponse_UserEventType = FrontendWatchListingUsersResponse.UserEventType;
/* @internal */
export type strims_network_v1_directory_FrontendWatchListingUsersResponse_UserEventType = FrontendWatchListingUsersResponse.UserEventType;

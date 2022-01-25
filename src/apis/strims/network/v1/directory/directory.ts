import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  Image as strims_type_Image,
  IImage as strims_type_IImage,
} from "../../../type/image";
import {
  BoolValue as google_protobuf_BoolValue,
  IBoolValue as google_protobuf_IBoolValue,
  BytesValue as google_protobuf_BytesValue,
  IBytesValue as google_protobuf_IBytesValue,
  StringValue as google_protobuf_StringValue,
  IStringValue as google_protobuf_IStringValue,
  UInt32Value as google_protobuf_UInt32Value,
  IUInt32Value as google_protobuf_IUInt32Value,
  UInt64Value as google_protobuf_UInt64Value,
  IUInt64Value as google_protobuf_IUInt64Value,
} from "../../../../google/protobuf/wrappers";

export type IServerConfig = {
  integrations?: ServerConfig.IIntegrations;
}

export class ServerConfig {
  integrations: ServerConfig.Integrations | undefined;

  constructor(v?: IServerConfig) {
    this.integrations = v?.integrations && new ServerConfig.Integrations(v.integrations);
  }

  static encode(m: ServerConfig, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.integrations) ServerConfig.Integrations.encode(m.integrations, w.uint32(10).fork()).ldelim();
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
        m.integrations = ServerConfig.Integrations.decode(r, r.uint32());
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
    angelthump?: ServerConfig.Integrations.IAngelThump;
    twitch?: ServerConfig.Integrations.ITwitch;
    youtube?: ServerConfig.Integrations.IYouTube;
    swarm?: ServerConfig.Integrations.ISwarm;
  }

  export class Integrations {
    angelthump: ServerConfig.Integrations.AngelThump | undefined;
    twitch: ServerConfig.Integrations.Twitch | undefined;
    youtube: ServerConfig.Integrations.YouTube | undefined;
    swarm: ServerConfig.Integrations.Swarm | undefined;

    constructor(v?: IIntegrations) {
      this.angelthump = v?.angelthump && new ServerConfig.Integrations.AngelThump(v.angelthump);
      this.twitch = v?.twitch && new ServerConfig.Integrations.Twitch(v.twitch);
      this.youtube = v?.youtube && new ServerConfig.Integrations.YouTube(v.youtube);
      this.swarm = v?.swarm && new ServerConfig.Integrations.Swarm(v.swarm);
    }

    static encode(m: Integrations, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.angelthump) ServerConfig.Integrations.AngelThump.encode(m.angelthump, w.uint32(10).fork()).ldelim();
      if (m.twitch) ServerConfig.Integrations.Twitch.encode(m.twitch, w.uint32(18).fork()).ldelim();
      if (m.youtube) ServerConfig.Integrations.YouTube.encode(m.youtube, w.uint32(26).fork()).ldelim();
      if (m.swarm) ServerConfig.Integrations.Swarm.encode(m.swarm, w.uint32(34).fork()).ldelim();
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
          m.angelthump = ServerConfig.Integrations.AngelThump.decode(r, r.uint32());
          break;
          case 2:
          m.twitch = ServerConfig.Integrations.Twitch.decode(r, r.uint32());
          break;
          case 3:
          m.youtube = ServerConfig.Integrations.YouTube.decode(r, r.uint32());
          break;
          case 4:
          m.swarm = ServerConfig.Integrations.Swarm.decode(r, r.uint32());
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
        if (m.clientId) w.uint32(18).string(m.clientId);
        if (m.clientSecret) w.uint32(26).string(m.clientSecret);
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
        if (m.publicApiKey) w.uint32(18).string(m.publicApiKey);
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
    if (m.networkKey) w.uint32(10).bytes(m.networkKey);
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
    if (m.networkKey) w.uint32(10).bytes(m.networkKey);
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

export type ITestPublishResponse = {
}

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
      Listing.Media.encode(m.content.media, w.uint32(8010).fork()).ldelim();
      break;
      case Listing.ContentCase.SERVICE:
      Listing.Service.encode(m.content.service, w.uint32(8018).fork()).ldelim();
      break;
      case Listing.ContentCase.EMBED:
      Listing.Embed.encode(m.content.embed, w.uint32(8026).fork()).ldelim();
      break;
      case Listing.ContentCase.CHAT:
      Listing.Chat.encode(m.content.chat, w.uint32(8034).fork()).ldelim();
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
        m.content = new Listing.Content({ media: Listing.Media.decode(r, r.uint32()) });
        break;
        case 1002:
        m.content = new Listing.Content({ service: Listing.Service.decode(r, r.uint32()) });
        break;
        case 1003:
        m.content = new Listing.Content({ embed: Listing.Embed.decode(r, r.uint32()) });
        break;
        case 1004:
        m.content = new Listing.Content({ chat: Listing.Chat.decode(r, r.uint32()) });
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
  |{ case?: ContentCase.MEDIA, media: Listing.IMedia }
  |{ case?: ContentCase.SERVICE, service: Listing.IService }
  |{ case?: ContentCase.EMBED, embed: Listing.IEmbed }
  |{ case?: ContentCase.CHAT, chat: Listing.IChat }
  ;

  export type TContent = Readonly<
  { case: ContentCase.NOT_SET }
  |{ case: ContentCase.MEDIA, media: Listing.Media }
  |{ case: ContentCase.SERVICE, service: Listing.Service }
  |{ case: ContentCase.EMBED, embed: Listing.Embed }
  |{ case: ContentCase.CHAT, chat: Listing.Chat }
  >;

  class ContentImpl {
    media: Listing.Media;
    service: Listing.Service;
    embed: Listing.Embed;
    chat: Listing.Chat;
    case: ContentCase = ContentCase.NOT_SET;

    constructor(v?: IContent) {
      if (v && "media" in v) {
        this.case = ContentCase.MEDIA;
        this.media = new Listing.Media(v.media);
      } else
      if (v && "service" in v) {
        this.case = ContentCase.SERVICE;
        this.service = new Listing.Service(v.service);
      } else
      if (v && "embed" in v) {
        this.case = ContentCase.EMBED;
        this.embed = new Listing.Embed(v.embed);
      } else
      if (v && "chat" in v) {
        this.case = ContentCase.CHAT;
        this.chat = new Listing.Chat(v.chat);
      }
    }
  }

  export const Content = ContentImpl as {
    new (): Readonly<{ case: ContentCase.NOT_SET }>;
    new <T extends IContent>(v: T): Readonly<
    T extends { media: Listing.IMedia } ? { case: ContentCase.MEDIA, media: Listing.Media } :
    T extends { service: Listing.IService } ? { case: ContentCase.SERVICE, service: Listing.Service } :
    T extends { embed: Listing.IEmbed } ? { case: ContentCase.EMBED, embed: Listing.Embed } :
    T extends { chat: Listing.IChat } ? { case: ContentCase.CHAT, chat: Listing.Chat } :
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
      if (m.mimeType) w.uint32(10).string(m.mimeType);
      if (m.swarmUri) w.uint32(18).string(m.swarmUri);
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
  }

  export class Service {
    type: string;

    constructor(v?: IService) {
      this.type = v?.type || "";
    }

    static encode(m: Service, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.type) w.uint32(10).string(m.type);
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
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type IEmbed = {
    service?: Listing.Embed.Service;
    id?: string;
    queryParams?: Map<string, string> | { [key: string]: string };
  }

  export class Embed {
    service: Listing.Embed.Service;
    id: string;
    queryParams: Map<string, string>;

    constructor(v?: IEmbed) {
      this.service = v?.service || 0;
      this.id = v?.id || "";
      if (v?.queryParams) this.queryParams = new Map(v.queryParams instanceof Map ? v.queryParams : Object.entries(v.queryParams));
      else this.queryParams = new Map<string, string>();
    }

    static encode(m: Embed, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.service) w.uint32(8).uint32(m.service);
      if (m.id) w.uint32(18).string(m.id);
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
      if (m.key) w.uint32(10).bytes(m.key);
      if (m.name) w.uint32(18).string(m.name);
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
  viewerCount?: bigint;
  live?: boolean;
  isMature?: boolean;
  thumbnail?: IListingSnippetImage;
  channelLogo?: IListingSnippetImage;
  videoHeight?: number;
  videoWidth?: number;
  key?: Uint8Array;
  signature?: Uint8Array;
}

export class ListingSnippet {
  title: string;
  description: string;
  tags: string[];
  category: string;
  channelName: string;
  viewerCount: bigint;
  live: boolean;
  isMature: boolean;
  thumbnail: ListingSnippetImage | undefined;
  channelLogo: ListingSnippetImage | undefined;
  videoHeight: number;
  videoWidth: number;
  key: Uint8Array;
  signature: Uint8Array;

  constructor(v?: IListingSnippet) {
    this.title = v?.title || "";
    this.description = v?.description || "";
    this.tags = v?.tags ? v.tags : [];
    this.category = v?.category || "";
    this.channelName = v?.channelName || "";
    this.viewerCount = v?.viewerCount || BigInt(0);
    this.live = v?.live || false;
    this.isMature = v?.isMature || false;
    this.thumbnail = v?.thumbnail && new ListingSnippetImage(v.thumbnail);
    this.channelLogo = v?.channelLogo && new ListingSnippetImage(v.channelLogo);
    this.videoHeight = v?.videoHeight || 0;
    this.videoWidth = v?.videoWidth || 0;
    this.key = v?.key || new Uint8Array();
    this.signature = v?.signature || new Uint8Array();
  }

  static encode(m: ListingSnippet, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.title) w.uint32(10).string(m.title);
    if (m.description) w.uint32(18).string(m.description);
    for (const v of m.tags) w.uint32(26).string(v);
    if (m.category) w.uint32(34).string(m.category);
    if (m.channelName) w.uint32(42).string(m.channelName);
    if (m.viewerCount) w.uint32(48).uint64(m.viewerCount);
    if (m.live) w.uint32(56).bool(m.live);
    if (m.isMature) w.uint32(64).bool(m.isMature);
    if (m.thumbnail) ListingSnippetImage.encode(m.thumbnail, w.uint32(74).fork()).ldelim();
    if (m.channelLogo) ListingSnippetImage.encode(m.channelLogo, w.uint32(82).fork()).ldelim();
    if (m.videoHeight) w.uint32(88).uint32(m.videoHeight);
    if (m.videoWidth) w.uint32(96).uint32(m.videoWidth);
    if (m.key) w.uint32(80010).bytes(m.key);
    if (m.signature) w.uint32(80018).bytes(m.signature);
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
        m.viewerCount = r.uint64();
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
  viewerCount?: google_protobuf_IUInt64Value;
  live?: google_protobuf_IBoolValue;
  isMature?: google_protobuf_IBoolValue;
  key?: google_protobuf_IBytesValue;
  signature?: google_protobuf_IBytesValue;
  videoHeight?: google_protobuf_IUInt32Value;
  videoWidth?: google_protobuf_IUInt32Value;
  tagsOneof?: ListingSnippetDelta.ITagsOneof
  thumbnailOneof?: ListingSnippetDelta.IThumbnailOneof
  channelLogoOneof?: ListingSnippetDelta.IChannelLogoOneof
}

export class ListingSnippetDelta {
  title: google_protobuf_StringValue | undefined;
  description: google_protobuf_StringValue | undefined;
  category: google_protobuf_StringValue | undefined;
  channelName: google_protobuf_StringValue | undefined;
  viewerCount: google_protobuf_UInt64Value | undefined;
  live: google_protobuf_BoolValue | undefined;
  isMature: google_protobuf_BoolValue | undefined;
  key: google_protobuf_BytesValue | undefined;
  signature: google_protobuf_BytesValue | undefined;
  videoHeight: google_protobuf_UInt32Value | undefined;
  videoWidth: google_protobuf_UInt32Value | undefined;
  tagsOneof: ListingSnippetDelta.TTagsOneof;
  thumbnailOneof: ListingSnippetDelta.TThumbnailOneof;
  channelLogoOneof: ListingSnippetDelta.TChannelLogoOneof;

  constructor(v?: IListingSnippetDelta) {
    this.title = v?.title && new google_protobuf_StringValue(v.title);
    this.description = v?.description && new google_protobuf_StringValue(v.description);
    this.category = v?.category && new google_protobuf_StringValue(v.category);
    this.channelName = v?.channelName && new google_protobuf_StringValue(v.channelName);
    this.viewerCount = v?.viewerCount && new google_protobuf_UInt64Value(v.viewerCount);
    this.live = v?.live && new google_protobuf_BoolValue(v.live);
    this.isMature = v?.isMature && new google_protobuf_BoolValue(v.isMature);
    this.key = v?.key && new google_protobuf_BytesValue(v.key);
    this.signature = v?.signature && new google_protobuf_BytesValue(v.signature);
    this.videoHeight = v?.videoHeight && new google_protobuf_UInt32Value(v.videoHeight);
    this.videoWidth = v?.videoWidth && new google_protobuf_UInt32Value(v.videoWidth);
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
    if (m.viewerCount) google_protobuf_UInt64Value.encode(m.viewerCount, w.uint32(42).fork()).ldelim();
    if (m.live) google_protobuf_BoolValue.encode(m.live, w.uint32(50).fork()).ldelim();
    if (m.isMature) google_protobuf_BoolValue.encode(m.isMature, w.uint32(58).fork()).ldelim();
    if (m.key) google_protobuf_BytesValue.encode(m.key, w.uint32(66).fork()).ldelim();
    if (m.signature) google_protobuf_BytesValue.encode(m.signature, w.uint32(74).fork()).ldelim();
    if (m.videoHeight) google_protobuf_UInt32Value.encode(m.videoHeight, w.uint32(82).fork()).ldelim();
    if (m.videoWidth) google_protobuf_UInt32Value.encode(m.videoWidth, w.uint32(90).fork()).ldelim();
    switch (m.tagsOneof.case) {
      case ListingSnippetDelta.TagsOneofCase.TAGS:
      ListingSnippetDelta.Tags.encode(m.tagsOneof.tags, w.uint32(8010).fork()).ldelim();
      break;
    }
    switch (m.thumbnailOneof.case) {
      case ListingSnippetDelta.ThumbnailOneofCase.THUMBNAIL:
      ListingSnippetImage.encode(m.thumbnailOneof.thumbnail, w.uint32(16010).fork()).ldelim();
      break;
    }
    switch (m.channelLogoOneof.case) {
      case ListingSnippetDelta.ChannelLogoOneofCase.CHANNEL_LOGO:
      ListingSnippetImage.encode(m.channelLogoOneof.channelLogo, w.uint32(24010).fork()).ldelim();
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
        m.viewerCount = google_protobuf_UInt64Value.decode(r, r.uint32());
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
        case 1001:
        m.tagsOneof = new ListingSnippetDelta.TagsOneof({ tags: ListingSnippetDelta.Tags.decode(r, r.uint32()) });
        break;
        case 2001:
        m.thumbnailOneof = new ListingSnippetDelta.ThumbnailOneof({ thumbnail: ListingSnippetImage.decode(r, r.uint32()) });
        break;
        case 3001:
        m.channelLogoOneof = new ListingSnippetDelta.ChannelLogoOneof({ channelLogo: ListingSnippetImage.decode(r, r.uint32()) });
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
  |{ case?: TagsOneofCase.TAGS, tags: ListingSnippetDelta.ITags }
  ;

  export type TTagsOneof = Readonly<
  { case: TagsOneofCase.NOT_SET }
  |{ case: TagsOneofCase.TAGS, tags: ListingSnippetDelta.Tags }
  >;

  class TagsOneofImpl {
    tags: ListingSnippetDelta.Tags;
    case: TagsOneofCase = TagsOneofCase.NOT_SET;

    constructor(v?: ITagsOneof) {
      if (v && "tags" in v) {
        this.case = TagsOneofCase.TAGS;
        this.tags = new ListingSnippetDelta.Tags(v.tags);
      }
    }
  }

  export const TagsOneof = TagsOneofImpl as {
    new (): Readonly<{ case: TagsOneofCase.NOT_SET }>;
    new <T extends ITagsOneof>(v: T): Readonly<
    T extends { tags: ListingSnippetDelta.ITags } ? { case: TagsOneofCase.TAGS, tags: ListingSnippetDelta.Tags } :
    never
    >;
  };

  export enum ThumbnailOneofCase {
    NOT_SET = 0,
    THUMBNAIL = 2001,
  }

  export type IThumbnailOneof =
  { case?: ThumbnailOneofCase.NOT_SET }
  |{ case?: ThumbnailOneofCase.THUMBNAIL, thumbnail: IListingSnippetImage }
  ;

  export type TThumbnailOneof = Readonly<
  { case: ThumbnailOneofCase.NOT_SET }
  |{ case: ThumbnailOneofCase.THUMBNAIL, thumbnail: ListingSnippetImage }
  >;

  class ThumbnailOneofImpl {
    thumbnail: ListingSnippetImage;
    case: ThumbnailOneofCase = ThumbnailOneofCase.NOT_SET;

    constructor(v?: IThumbnailOneof) {
      if (v && "thumbnail" in v) {
        this.case = ThumbnailOneofCase.THUMBNAIL;
        this.thumbnail = new ListingSnippetImage(v.thumbnail);
      }
    }
  }

  export const ThumbnailOneof = ThumbnailOneofImpl as {
    new (): Readonly<{ case: ThumbnailOneofCase.NOT_SET }>;
    new <T extends IThumbnailOneof>(v: T): Readonly<
    T extends { thumbnail: IListingSnippetImage } ? { case: ThumbnailOneofCase.THUMBNAIL, thumbnail: ListingSnippetImage } :
    never
    >;
  };

  export enum ChannelLogoOneofCase {
    NOT_SET = 0,
    CHANNEL_LOGO = 3001,
  }

  export type IChannelLogoOneof =
  { case?: ChannelLogoOneofCase.NOT_SET }
  |{ case?: ChannelLogoOneofCase.CHANNEL_LOGO, channelLogo: IListingSnippetImage }
  ;

  export type TChannelLogoOneof = Readonly<
  { case: ChannelLogoOneofCase.NOT_SET }
  |{ case: ChannelLogoOneofCase.CHANNEL_LOGO, channelLogo: ListingSnippetImage }
  >;

  class ChannelLogoOneofImpl {
    channelLogo: ListingSnippetImage;
    case: ChannelLogoOneofCase = ChannelLogoOneofCase.NOT_SET;

    constructor(v?: IChannelLogoOneof) {
      if (v && "channelLogo" in v) {
        this.case = ChannelLogoOneofCase.CHANNEL_LOGO;
        this.channelLogo = new ListingSnippetImage(v.channelLogo);
      }
    }
  }

  export const ChannelLogoOneof = ChannelLogoOneofImpl as {
    new (): Readonly<{ case: ChannelLogoOneofCase.NOT_SET }>;
    new <T extends IChannelLogoOneof>(v: T): Readonly<
    T extends { channelLogo: IListingSnippetImage } ? { case: ChannelLogoOneofCase.CHANNEL_LOGO, channelLogo: ListingSnippetImage } :
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
      Event.ListingChange.encode(m.body.listingChange, w.uint32(8010).fork()).ldelim();
      break;
      case Event.BodyCase.UNPUBLISH:
      Event.Unpublish.encode(m.body.unpublish, w.uint32(8018).fork()).ldelim();
      break;
      case Event.BodyCase.VIEWER_COUNT_CHANGE:
      Event.ViewerCountChange.encode(m.body.viewerCountChange, w.uint32(8026).fork()).ldelim();
      break;
      case Event.BodyCase.VIEWER_STATE_CHANGE:
      Event.ViewerStateChange.encode(m.body.viewerStateChange, w.uint32(8034).fork()).ldelim();
      break;
      case Event.BodyCase.PING:
      Event.Ping.encode(m.body.ping, w.uint32(8042).fork()).ldelim();
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
        m.body = new Event.Body({ listingChange: Event.ListingChange.decode(r, r.uint32()) });
        break;
        case 1002:
        m.body = new Event.Body({ unpublish: Event.Unpublish.decode(r, r.uint32()) });
        break;
        case 1003:
        m.body = new Event.Body({ viewerCountChange: Event.ViewerCountChange.decode(r, r.uint32()) });
        break;
        case 1004:
        m.body = new Event.Body({ viewerStateChange: Event.ViewerStateChange.decode(r, r.uint32()) });
        break;
        case 1005:
        m.body = new Event.Body({ ping: Event.Ping.decode(r, r.uint32()) });
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
    VIEWER_COUNT_CHANGE = 1003,
    VIEWER_STATE_CHANGE = 1004,
    PING = 1005,
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.LISTING_CHANGE, listingChange: Event.IListingChange }
  |{ case?: BodyCase.UNPUBLISH, unpublish: Event.IUnpublish }
  |{ case?: BodyCase.VIEWER_COUNT_CHANGE, viewerCountChange: Event.IViewerCountChange }
  |{ case?: BodyCase.VIEWER_STATE_CHANGE, viewerStateChange: Event.IViewerStateChange }
  |{ case?: BodyCase.PING, ping: Event.IPing }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.LISTING_CHANGE, listingChange: Event.ListingChange }
  |{ case: BodyCase.UNPUBLISH, unpublish: Event.Unpublish }
  |{ case: BodyCase.VIEWER_COUNT_CHANGE, viewerCountChange: Event.ViewerCountChange }
  |{ case: BodyCase.VIEWER_STATE_CHANGE, viewerStateChange: Event.ViewerStateChange }
  |{ case: BodyCase.PING, ping: Event.Ping }
  >;

  class BodyImpl {
    listingChange: Event.ListingChange;
    unpublish: Event.Unpublish;
    viewerCountChange: Event.ViewerCountChange;
    viewerStateChange: Event.ViewerStateChange;
    ping: Event.Ping;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "listingChange" in v) {
        this.case = BodyCase.LISTING_CHANGE;
        this.listingChange = new Event.ListingChange(v.listingChange);
      } else
      if (v && "unpublish" in v) {
        this.case = BodyCase.UNPUBLISH;
        this.unpublish = new Event.Unpublish(v.unpublish);
      } else
      if (v && "viewerCountChange" in v) {
        this.case = BodyCase.VIEWER_COUNT_CHANGE;
        this.viewerCountChange = new Event.ViewerCountChange(v.viewerCountChange);
      } else
      if (v && "viewerStateChange" in v) {
        this.case = BodyCase.VIEWER_STATE_CHANGE;
        this.viewerStateChange = new Event.ViewerStateChange(v.viewerStateChange);
      } else
      if (v && "ping" in v) {
        this.case = BodyCase.PING;
        this.ping = new Event.Ping(v.ping);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { listingChange: Event.IListingChange } ? { case: BodyCase.LISTING_CHANGE, listingChange: Event.ListingChange } :
    T extends { unpublish: Event.IUnpublish } ? { case: BodyCase.UNPUBLISH, unpublish: Event.Unpublish } :
    T extends { viewerCountChange: Event.IViewerCountChange } ? { case: BodyCase.VIEWER_COUNT_CHANGE, viewerCountChange: Event.ViewerCountChange } :
    T extends { viewerStateChange: Event.IViewerStateChange } ? { case: BodyCase.VIEWER_STATE_CHANGE, viewerStateChange: Event.ViewerStateChange } :
    T extends { ping: Event.IPing } ? { case: BodyCase.PING, ping: Event.Ping } :
    never
    >;
  };

  export type IListingChange = {
    id?: bigint;
    listing?: IListing;
    snippet?: IListingSnippet;
    moderation?: IListingModeration;
  }

  export class ListingChange {
    id: bigint;
    listing: Listing | undefined;
    snippet: ListingSnippet | undefined;
    moderation: ListingModeration | undefined;

    constructor(v?: IListingChange) {
      this.id = v?.id || BigInt(0);
      this.listing = v?.listing && new Listing(v.listing);
      this.snippet = v?.snippet && new ListingSnippet(v.snippet);
      this.moderation = v?.moderation && new ListingModeration(v.moderation);
    }

    static encode(m: ListingChange, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.id) w.uint32(8).uint64(m.id);
      if (m.listing) Listing.encode(m.listing, w.uint32(18).fork()).ldelim();
      if (m.snippet) ListingSnippet.encode(m.snippet, w.uint32(26).fork()).ldelim();
      if (m.moderation) ListingModeration.encode(m.moderation, w.uint32(34).fork()).ldelim();
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
          m.listing = Listing.decode(r, r.uint32());
          break;
          case 3:
          m.snippet = ListingSnippet.decode(r, r.uint32());
          break;
          case 4:
          m.moderation = ListingModeration.decode(r, r.uint32());
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

  export type IViewerCountChange = {
    id?: bigint;
    count?: number;
  }

  export class ViewerCountChange {
    id: bigint;
    count: number;

    constructor(v?: IViewerCountChange) {
      this.id = v?.id || BigInt(0);
      this.count = v?.count || 0;
    }

    static encode(m: ViewerCountChange, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.id) w.uint32(8).uint64(m.id);
      if (m.count) w.uint32(16).uint32(m.count);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): ViewerCountChange {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new ViewerCountChange();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.id = r.uint64();
          break;
          case 2:
          m.count = r.uint32();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type IViewerStateChange = {
    subject?: string;
    online?: boolean;
    viewingIds?: bigint[];
  }

  export class ViewerStateChange {
    subject: string;
    online: boolean;
    viewingIds: bigint[];

    constructor(v?: IViewerStateChange) {
      this.subject = v?.subject || "";
      this.online = v?.online || false;
      this.viewingIds = v?.viewingIds ? v.viewingIds : [];
    }

    static encode(m: ViewerStateChange, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.subject) w.uint32(10).string(m.subject);
      if (m.online) w.uint32(16).bool(m.online);
      m.viewingIds.reduce((w, v) => w.uint64(v), w.uint32(26).fork()).ldelim();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): ViewerStateChange {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new ViewerStateChange();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.subject = r.string();
          break;
          case 2:
          m.online = r.bool();
          break;
          case 3:
          for (const flen = r.uint32(), fend = r.pos + flen; r.pos < fend;) m.viewingIds.push(r.uint64());
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
  isMature?: boolean;
  isBanned?: boolean;
}

export class ListingModeration {
  isMature: boolean;
  isBanned: boolean;

  constructor(v?: IListingModeration) {
    this.isMature = v?.isMature || false;
    this.isBanned = v?.isBanned || false;
  }

  static encode(m: ListingModeration, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.isMature) w.uint32(32).bool(m.isMature);
    if (m.isBanned) w.uint32(40).bool(m.isBanned);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListingModeration {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ListingModeration();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 4:
        m.isMature = r.bool();
        break;
        case 5:
        m.isBanned = r.bool();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

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
    if (m.notes) w.uint32(42).string(m.notes);
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
        m.listing = Listing.decode(r, r.uint32());
        break;
        case 4:
        m.moderation = ListingModeration.decode(r, r.uint32());
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new EventBroadcast();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.events.push(Event.decode(r, r.uint32()));
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new PublishRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.listing = Listing.decode(r, r.uint32());
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

export type IUnpublishResponse = {
}

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
  id?: bigint;
}

export class JoinRequest {
  id: bigint;

  constructor(v?: IJoinRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: JoinRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
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

export type IJoinResponse = {
}

export class JoinResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IJoinResponse) {
  }

  static encode(m: JoinResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): JoinResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new JoinResponse();
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

export type IPartResponse = {
}

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

export type IPingRequest = {
}

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

export type IPingResponse = {
}

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

export type IFrontendOpenRequest = {
}

export class FrontendOpenRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IFrontendOpenRequest) {
  }

  static encode(m: FrontendOpenRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendOpenRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new FrontendOpenRequest();
  }
}

export type IFrontendOpenResponse = {
  networkId?: bigint;
  networkKey?: Uint8Array;
  body?: FrontendOpenResponse.IBody
}

export class FrontendOpenResponse {
  networkId: bigint;
  networkKey: Uint8Array;
  body: FrontendOpenResponse.TBody;

  constructor(v?: IFrontendOpenResponse) {
    this.networkId = v?.networkId || BigInt(0);
    this.networkKey = v?.networkKey || new Uint8Array();
    this.body = new FrontendOpenResponse.Body(v?.body);
  }

  static encode(m: FrontendOpenResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkId) w.uint32(8).uint64(m.networkId);
    if (m.networkKey) w.uint32(18).bytes(m.networkKey);
    switch (m.body.case) {
      case FrontendOpenResponse.BodyCase.CLOSE:
      FrontendOpenResponse.Close.encode(m.body.close, w.uint32(8010).fork()).ldelim();
      break;
      case FrontendOpenResponse.BodyCase.BROADCAST:
      EventBroadcast.encode(m.body.broadcast, w.uint32(8018).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendOpenResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new FrontendOpenResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkId = r.uint64();
        break;
        case 2:
        m.networkKey = r.bytes();
        break;
        case 1001:
        m.body = new FrontendOpenResponse.Body({ close: FrontendOpenResponse.Close.decode(r, r.uint32()) });
        break;
        case 1002:
        m.body = new FrontendOpenResponse.Body({ broadcast: EventBroadcast.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace FrontendOpenResponse {
  export enum BodyCase {
    NOT_SET = 0,
    CLOSE = 1001,
    BROADCAST = 1002,
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.CLOSE, close: FrontendOpenResponse.IClose }
  |{ case?: BodyCase.BROADCAST, broadcast: IEventBroadcast }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.CLOSE, close: FrontendOpenResponse.Close }
  |{ case: BodyCase.BROADCAST, broadcast: EventBroadcast }
  >;

  class BodyImpl {
    close: FrontendOpenResponse.Close;
    broadcast: EventBroadcast;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "close" in v) {
        this.case = BodyCase.CLOSE;
        this.close = new FrontendOpenResponse.Close(v.close);
      } else
      if (v && "broadcast" in v) {
        this.case = BodyCase.BROADCAST;
        this.broadcast = new EventBroadcast(v.broadcast);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { close: FrontendOpenResponse.IClose } ? { case: BodyCase.CLOSE, close: FrontendOpenResponse.Close } :
    T extends { broadcast: IEventBroadcast } ? { case: BodyCase.BROADCAST, broadcast: EventBroadcast } :
    never
    >;
  };

  export type IClose = {
  }

  export class Close {

    // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
    constructor(v?: IClose) {
    }

    static encode(m: Close, w?: Writer): Writer {
      if (!w) w = new Writer();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Close {
      if (r instanceof Reader && length) r.skip(length);
      return new Close();
    }
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
    if (m.networkKey) w.uint32(10).bytes(m.networkKey);
    if (m.listing) Listing.encode(m.listing, w.uint32(18).fork()).ldelim();
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
        m.listing = Listing.decode(r, r.uint32());
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
    if (m.networkKey) w.uint32(10).bytes(m.networkKey);
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

export type IFrontendUnpublishResponse = {
}

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
  id?: bigint;
}

export class FrontendJoinRequest {
  networkKey: Uint8Array;
  id: bigint;

  constructor(v?: IFrontendJoinRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.id = v?.id || BigInt(0);
  }

  static encode(m: FrontendJoinRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey) w.uint32(10).bytes(m.networkKey);
    if (m.id) w.uint32(16).uint64(m.id);
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
    if (m.networkKey) w.uint32(10).bytes(m.networkKey);
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

export type IFrontendPartResponse = {
}

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
    if (m.networkKey) w.uint32(10).bytes(m.networkKey);
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

export type IFrontendTestResponse = {
}

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

export type IFrontendGetListingRecordRequest = {
  id?: bigint;
}

export class FrontendGetListingRecordRequest {
  id: bigint;

  constructor(v?: IFrontendGetListingRecordRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: FrontendGetListingRecordRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendGetListingRecordRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new FrontendGetListingRecordRequest();
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

export type IFrontendGetListingRecordResponse = {
  record?: IListingRecord;
}

export class FrontendGetListingRecordResponse {
  record: ListingRecord | undefined;

  constructor(v?: IFrontendGetListingRecordResponse) {
    this.record = v?.record && new ListingRecord(v.record);
  }

  static encode(m: FrontendGetListingRecordResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.record) ListingRecord.encode(m.record, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendGetListingRecordResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new FrontendGetListingRecordResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.record = ListingRecord.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IFrontendListListingRecordsRequest = {
}

export class FrontendListListingRecordsRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IFrontendListListingRecordsRequest) {
  }

  static encode(m: FrontendListListingRecordsRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendListListingRecordsRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new FrontendListListingRecordsRequest();
  }
}

export type IFrontendListListingRecordsResponse = {
  records?: IListingRecord[];
}

export class FrontendListListingRecordsResponse {
  records: ListingRecord[];

  constructor(v?: IFrontendListListingRecordsResponse) {
    this.records = v?.records ? v.records.map(v => new ListingRecord(v)) : [];
  }

  static encode(m: FrontendListListingRecordsResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.records) ListingRecord.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendListListingRecordsResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new FrontendListListingRecordsResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.records.push(ListingRecord.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IFrontendUpdateListingRecordRequest = {
  id?: bigint;
  notes?: string;
  moderation?: IListingModeration;
}

export class FrontendUpdateListingRecordRequest {
  id: bigint;
  notes: string;
  moderation: ListingModeration | undefined;

  constructor(v?: IFrontendUpdateListingRecordRequest) {
    this.id = v?.id || BigInt(0);
    this.notes = v?.notes || "";
    this.moderation = v?.moderation && new ListingModeration(v.moderation);
  }

  static encode(m: FrontendUpdateListingRecordRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.notes) w.uint32(26).string(m.notes);
    if (m.moderation) ListingModeration.encode(m.moderation, w.uint32(34).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendUpdateListingRecordRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new FrontendUpdateListingRecordRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 3:
        m.notes = r.string();
        break;
        case 4:
        m.moderation = ListingModeration.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IFrontendUpdateListingRecordResponse = {
  record?: IListingRecord;
}

export class FrontendUpdateListingRecordResponse {
  record: ListingRecord | undefined;

  constructor(v?: IFrontendUpdateListingRecordResponse) {
    this.record = v?.record && new ListingRecord(v.record);
  }

  static encode(m: FrontendUpdateListingRecordResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.record) ListingRecord.encode(m.record, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): FrontendUpdateListingRecordResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new FrontendUpdateListingRecordResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.record = ListingRecord.decode(r, r.uint32());
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
    if (m.swarmId) w.uint32(10).bytes(m.swarmId);
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
  snippetDelta?: IListingSnippetDelta;
}

export class SnippetSubscribeResponse {
  snippetDelta: ListingSnippetDelta | undefined;

  constructor(v?: ISnippetSubscribeResponse) {
    this.snippetDelta = v?.snippetDelta && new ListingSnippetDelta(v.snippetDelta);
  }

  static encode(m: SnippetSubscribeResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.snippetDelta) ListingSnippetDelta.encode(m.snippetDelta, w.uint32(10).fork()).ldelim();
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
        m.snippetDelta = ListingSnippetDelta.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}


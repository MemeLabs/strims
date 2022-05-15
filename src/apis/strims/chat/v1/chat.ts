import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  Key as strims_type_Key,
  IKey as strims_type_IKey,
} from "../../type/key";

export type IServerEvent = {
  body?: ServerEvent.IBody
}

export class ServerEvent {
  body: ServerEvent.TBody;

  constructor(v?: IServerEvent) {
    this.body = new ServerEvent.Body(v?.body);
  }

  static encode(m: ServerEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case ServerEvent.BodyCase.MESSAGE:
      Message.encode(m.body.message, w.uint32(8010).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ServerEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ServerEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1001:
        m.body = new ServerEvent.Body({ message: Message.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace ServerEvent {
  export enum BodyCase {
    NOT_SET = 0,
    MESSAGE = 1001,
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.MESSAGE, message: IMessage }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.MESSAGE, message: Message }
  >;

  class BodyImpl {
    message: Message;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "message" in v) {
        this.case = BodyCase.MESSAGE;
        this.message = new Message(v.message);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { message: IMessage } ? { case: BodyCase.MESSAGE, message: Message } :
    never
    >;
  };

}

export type IRoom = {
  name?: string;
  css?: string;
}

export class Room {
  name: string;
  css: string;

  constructor(v?: IRoom) {
    this.name = v?.name || "";
    this.css = v?.css || "";
  }

  static encode(m: Room, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.name.length) w.uint32(10).string(m.name);
    if (m.css.length) w.uint32(18).string(m.css);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Room {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Room();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.name = r.string();
        break;
        case 2:
        m.css = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IServer = {
  id?: bigint;
  networkKey?: Uint8Array;
  key?: strims_type_IKey;
  room?: IRoom;
  adminPeerKeys?: Uint8Array[];
}

export class Server {
  id: bigint;
  networkKey: Uint8Array;
  key: strims_type_Key | undefined;
  room: Room | undefined;
  adminPeerKeys: Uint8Array[];

  constructor(v?: IServer) {
    this.id = v?.id || BigInt(0);
    this.networkKey = v?.networkKey || new Uint8Array();
    this.key = v?.key && new strims_type_Key(v.key);
    this.room = v?.room && new Room(v.room);
    this.adminPeerKeys = v?.adminPeerKeys ? v.adminPeerKeys : [];
  }

  static encode(m: Server, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.networkKey.length) w.uint32(18).bytes(m.networkKey);
    if (m.key) strims_type_Key.encode(m.key, w.uint32(26).fork()).ldelim();
    if (m.room) Room.encode(m.room, w.uint32(34).fork()).ldelim();
    for (const v of m.adminPeerKeys) w.uint32(42).bytes(v);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Server {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Server();
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
        m.key = strims_type_Key.decode(r, r.uint32());
        break;
        case 4:
        m.room = Room.decode(r, r.uint32());
        break;
        case 5:
        m.adminPeerKeys.push(r.bytes())
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IEmoteImage = {
  data?: Uint8Array;
  fileType?: EmoteFileType;
  height?: number;
  width?: number;
  scale?: EmoteScale;
}

export class EmoteImage {
  data: Uint8Array;
  fileType: EmoteFileType;
  height: number;
  width: number;
  scale: EmoteScale;

  constructor(v?: IEmoteImage) {
    this.data = v?.data || new Uint8Array();
    this.fileType = v?.fileType || 0;
    this.height = v?.height || 0;
    this.width = v?.width || 0;
    this.scale = v?.scale || 0;
  }

  static encode(m: EmoteImage, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.data.length) w.uint32(26).bytes(m.data);
    if (m.fileType) w.uint32(32).uint32(m.fileType);
    if (m.height) w.uint32(40).uint32(m.height);
    if (m.width) w.uint32(48).uint32(m.width);
    if (m.scale) w.uint32(56).uint32(m.scale);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): EmoteImage {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new EmoteImage();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 3:
        m.data = r.bytes();
        break;
        case 4:
        m.fileType = r.uint32();
        break;
        case 5:
        m.height = r.uint32();
        break;
        case 6:
        m.width = r.uint32();
        break;
        case 7:
        m.scale = r.uint32();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IEmoteEffect = {
  effect?: EmoteEffect.IEffect
}

export class EmoteEffect {
  effect: EmoteEffect.TEffect;

  constructor(v?: IEmoteEffect) {
    this.effect = new EmoteEffect.Effect(v?.effect);
  }

  static encode(m: EmoteEffect, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.effect.case) {
      case EmoteEffect.EffectCase.CUSTOM_CSS:
      EmoteEffect.CustomCSS.encode(m.effect.customCss, w.uint32(8010).fork()).ldelim();
      break;
      case EmoteEffect.EffectCase.SPRITE_ANIMATION:
      EmoteEffect.SpriteAnimation.encode(m.effect.spriteAnimation, w.uint32(8018).fork()).ldelim();
      break;
      case EmoteEffect.EffectCase.DEFAULT_MODIFIERS:
      EmoteEffect.DefaultModifiers.encode(m.effect.defaultModifiers, w.uint32(8026).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): EmoteEffect {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new EmoteEffect();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1001:
        m.effect = new EmoteEffect.Effect({ customCss: EmoteEffect.CustomCSS.decode(r, r.uint32()) });
        break;
        case 1002:
        m.effect = new EmoteEffect.Effect({ spriteAnimation: EmoteEffect.SpriteAnimation.decode(r, r.uint32()) });
        break;
        case 1003:
        m.effect = new EmoteEffect.Effect({ defaultModifiers: EmoteEffect.DefaultModifiers.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace EmoteEffect {
  export enum EffectCase {
    NOT_SET = 0,
    CUSTOM_CSS = 1001,
    SPRITE_ANIMATION = 1002,
    DEFAULT_MODIFIERS = 1003,
  }

  export type IEffect =
  { case?: EffectCase.NOT_SET }
  |{ case?: EffectCase.CUSTOM_CSS, customCss: EmoteEffect.ICustomCSS }
  |{ case?: EffectCase.SPRITE_ANIMATION, spriteAnimation: EmoteEffect.ISpriteAnimation }
  |{ case?: EffectCase.DEFAULT_MODIFIERS, defaultModifiers: EmoteEffect.IDefaultModifiers }
  ;

  export type TEffect = Readonly<
  { case: EffectCase.NOT_SET }
  |{ case: EffectCase.CUSTOM_CSS, customCss: EmoteEffect.CustomCSS }
  |{ case: EffectCase.SPRITE_ANIMATION, spriteAnimation: EmoteEffect.SpriteAnimation }
  |{ case: EffectCase.DEFAULT_MODIFIERS, defaultModifiers: EmoteEffect.DefaultModifiers }
  >;

  class EffectImpl {
    customCss: EmoteEffect.CustomCSS;
    spriteAnimation: EmoteEffect.SpriteAnimation;
    defaultModifiers: EmoteEffect.DefaultModifiers;
    case: EffectCase = EffectCase.NOT_SET;

    constructor(v?: IEffect) {
      if (v && "customCss" in v) {
        this.case = EffectCase.CUSTOM_CSS;
        this.customCss = new EmoteEffect.CustomCSS(v.customCss);
      } else
      if (v && "spriteAnimation" in v) {
        this.case = EffectCase.SPRITE_ANIMATION;
        this.spriteAnimation = new EmoteEffect.SpriteAnimation(v.spriteAnimation);
      } else
      if (v && "defaultModifiers" in v) {
        this.case = EffectCase.DEFAULT_MODIFIERS;
        this.defaultModifiers = new EmoteEffect.DefaultModifiers(v.defaultModifiers);
      }
    }
  }

  export const Effect = EffectImpl as {
    new (): Readonly<{ case: EffectCase.NOT_SET }>;
    new <T extends IEffect>(v: T): Readonly<
    T extends { customCss: EmoteEffect.ICustomCSS } ? { case: EffectCase.CUSTOM_CSS, customCss: EmoteEffect.CustomCSS } :
    T extends { spriteAnimation: EmoteEffect.ISpriteAnimation } ? { case: EffectCase.SPRITE_ANIMATION, spriteAnimation: EmoteEffect.SpriteAnimation } :
    T extends { defaultModifiers: EmoteEffect.IDefaultModifiers } ? { case: EffectCase.DEFAULT_MODIFIERS, defaultModifiers: EmoteEffect.DefaultModifiers } :
    never
    >;
  };

  export type ICustomCSS = {
    css?: string;
  }

  export class CustomCSS {
    css: string;

    constructor(v?: ICustomCSS) {
      this.css = v?.css || "";
    }

    static encode(m: CustomCSS, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.css.length) w.uint32(10).string(m.css);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): CustomCSS {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new CustomCSS();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.css = r.string();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type ISpriteAnimation = {
    frameCount?: number;
    durationMs?: number;
    iterationCount?: number;
    endOnFrame?: number;
    loopForever?: boolean;
    alternateDirection?: boolean;
  }

  export class SpriteAnimation {
    frameCount: number;
    durationMs: number;
    iterationCount: number;
    endOnFrame: number;
    loopForever: boolean;
    alternateDirection: boolean;

    constructor(v?: ISpriteAnimation) {
      this.frameCount = v?.frameCount || 0;
      this.durationMs = v?.durationMs || 0;
      this.iterationCount = v?.iterationCount || 0;
      this.endOnFrame = v?.endOnFrame || 0;
      this.loopForever = v?.loopForever || false;
      this.alternateDirection = v?.alternateDirection || false;
    }

    static encode(m: SpriteAnimation, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.frameCount) w.uint32(8).uint32(m.frameCount);
      if (m.durationMs) w.uint32(16).uint32(m.durationMs);
      if (m.iterationCount) w.uint32(24).uint32(m.iterationCount);
      if (m.endOnFrame) w.uint32(32).uint32(m.endOnFrame);
      if (m.loopForever) w.uint32(40).bool(m.loopForever);
      if (m.alternateDirection) w.uint32(48).bool(m.alternateDirection);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): SpriteAnimation {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new SpriteAnimation();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.frameCount = r.uint32();
          break;
          case 2:
          m.durationMs = r.uint32();
          break;
          case 3:
          m.iterationCount = r.uint32();
          break;
          case 4:
          m.endOnFrame = r.uint32();
          break;
          case 5:
          m.loopForever = r.bool();
          break;
          case 6:
          m.alternateDirection = r.bool();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type IDefaultModifiers = {
    modifiers?: string[];
  }

  export class DefaultModifiers {
    modifiers: string[];

    constructor(v?: IDefaultModifiers) {
      this.modifiers = v?.modifiers ? v.modifiers : [];
    }

    static encode(m: DefaultModifiers, w?: Writer): Writer {
      if (!w) w = new Writer();
      for (const v of m.modifiers) w.uint32(10).string(v);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): DefaultModifiers {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new DefaultModifiers();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.modifiers.push(r.string())
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

export type IEmoteContributor = {
  name?: string;
  link?: string;
}

export class EmoteContributor {
  name: string;
  link: string;

  constructor(v?: IEmoteContributor) {
    this.name = v?.name || "";
    this.link = v?.link || "";
  }

  static encode(m: EmoteContributor, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.name.length) w.uint32(10).string(m.name);
    if (m.link.length) w.uint32(18).string(m.link);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): EmoteContributor {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new EmoteContributor();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.name = r.string();
        break;
        case 2:
        m.link = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IEmote = {
  id?: bigint;
  serverId?: bigint;
  name?: string;
  images?: IEmoteImage[];
  effects?: IEmoteEffect[];
  contributor?: IEmoteContributor;
}

export class Emote {
  id: bigint;
  serverId: bigint;
  name: string;
  images: EmoteImage[];
  effects: EmoteEffect[];
  contributor: EmoteContributor | undefined;

  constructor(v?: IEmote) {
    this.id = v?.id || BigInt(0);
    this.serverId = v?.serverId || BigInt(0);
    this.name = v?.name || "";
    this.images = v?.images ? v.images.map(v => new EmoteImage(v)) : [];
    this.effects = v?.effects ? v.effects.map(v => new EmoteEffect(v)) : [];
    this.contributor = v?.contributor && new EmoteContributor(v.contributor);
  }

  static encode(m: Emote, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.serverId) w.uint32(16).uint64(m.serverId);
    if (m.name.length) w.uint32(26).string(m.name);
    for (const v of m.images) EmoteImage.encode(v, w.uint32(34).fork()).ldelim();
    for (const v of m.effects) EmoteEffect.encode(v, w.uint32(42).fork()).ldelim();
    if (m.contributor) EmoteContributor.encode(m.contributor, w.uint32(50).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Emote {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Emote();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.serverId = r.uint64();
        break;
        case 3:
        m.name = r.string();
        break;
        case 4:
        m.images.push(EmoteImage.decode(r, r.uint32()));
        break;
        case 5:
        m.effects.push(EmoteEffect.decode(r, r.uint32()));
        break;
        case 6:
        m.contributor = EmoteContributor.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IModifier = {
  id?: bigint;
  serverId?: bigint;
  name?: string;
  priority?: number;
  internal?: boolean;
  extraWrapCount?: number;
}

export class Modifier {
  id: bigint;
  serverId: bigint;
  name: string;
  priority: number;
  internal: boolean;
  extraWrapCount: number;

  constructor(v?: IModifier) {
    this.id = v?.id || BigInt(0);
    this.serverId = v?.serverId || BigInt(0);
    this.name = v?.name || "";
    this.priority = v?.priority || 0;
    this.internal = v?.internal || false;
    this.extraWrapCount = v?.extraWrapCount || 0;
  }

  static encode(m: Modifier, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.serverId) w.uint32(16).uint64(m.serverId);
    if (m.name.length) w.uint32(26).string(m.name);
    if (m.priority) w.uint32(32).uint32(m.priority);
    if (m.internal) w.uint32(40).bool(m.internal);
    if (m.extraWrapCount) w.uint32(48).uint32(m.extraWrapCount);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Modifier {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Modifier();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.serverId = r.uint64();
        break;
        case 3:
        m.name = r.string();
        break;
        case 4:
        m.priority = r.uint32();
        break;
        case 5:
        m.internal = r.bool();
        break;
        case 6:
        m.extraWrapCount = r.uint32();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ITag = {
  id?: bigint;
  serverId?: bigint;
  name?: string;
  color?: string;
  sensitive?: boolean;
}

export class Tag {
  id: bigint;
  serverId: bigint;
  name: string;
  color: string;
  sensitive: boolean;

  constructor(v?: ITag) {
    this.id = v?.id || BigInt(0);
    this.serverId = v?.serverId || BigInt(0);
    this.name = v?.name || "";
    this.color = v?.color || "";
    this.sensitive = v?.sensitive || false;
  }

  static encode(m: Tag, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.serverId) w.uint32(16).uint64(m.serverId);
    if (m.name.length) w.uint32(26).string(m.name);
    if (m.color.length) w.uint32(34).string(m.color);
    if (m.sensitive) w.uint32(40).bool(m.sensitive);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Tag {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Tag();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.serverId = r.uint64();
        break;
        case 3:
        m.name = r.string();
        break;
        case 4:
        m.color = r.string();
        break;
        case 5:
        m.sensitive = r.bool();
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
  isDelta?: boolean;
  removedIds?: bigint[];
  room?: IRoom;
  emotes?: IEmote[];
  modifiers?: IModifier[];
  tags?: ITag[];
}

export class AssetBundle {
  isDelta: boolean;
  removedIds: bigint[];
  room: Room | undefined;
  emotes: Emote[];
  modifiers: Modifier[];
  tags: Tag[];

  constructor(v?: IAssetBundle) {
    this.isDelta = v?.isDelta || false;
    this.removedIds = v?.removedIds ? v.removedIds : [];
    this.room = v?.room && new Room(v.room);
    this.emotes = v?.emotes ? v.emotes.map(v => new Emote(v)) : [];
    this.modifiers = v?.modifiers ? v.modifiers.map(v => new Modifier(v)) : [];
    this.tags = v?.tags ? v.tags.map(v => new Tag(v)) : [];
  }

  static encode(m: AssetBundle, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.isDelta) w.uint32(8).bool(m.isDelta);
    m.removedIds.reduce((w, v) => w.uint64(v), w.uint32(18).fork()).ldelim();
    if (m.room) Room.encode(m.room, w.uint32(26).fork()).ldelim();
    for (const v of m.emotes) Emote.encode(v, w.uint32(34).fork()).ldelim();
    for (const v of m.modifiers) Modifier.encode(v, w.uint32(42).fork()).ldelim();
    for (const v of m.tags) Tag.encode(v, w.uint32(50).fork()).ldelim();
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
        m.isDelta = r.bool();
        break;
        case 2:
        for (const flen = r.uint32(), fend = r.pos + flen; r.pos < fend;) m.removedIds.push(r.uint64());
        break;
        case 3:
        m.room = Room.decode(r, r.uint32());
        break;
        case 4:
        m.emotes.push(Emote.decode(r, r.uint32()));
        break;
        case 5:
        m.modifiers.push(Modifier.decode(r, r.uint32()));
        break;
        case 6:
        m.tags.push(Tag.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IMessage = {
  serverTime?: bigint;
  peerKey?: Uint8Array;
  nick?: string;
  body?: string;
  entities?: Message.IEntities;
}

export class Message {
  serverTime: bigint;
  peerKey: Uint8Array;
  nick: string;
  body: string;
  entities: Message.Entities | undefined;

  constructor(v?: IMessage) {
    this.serverTime = v?.serverTime || BigInt(0);
    this.peerKey = v?.peerKey || new Uint8Array();
    this.nick = v?.nick || "";
    this.body = v?.body || "";
    this.entities = v?.entities && new Message.Entities(v.entities);
  }

  static encode(m: Message, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverTime) w.uint32(8).int64(m.serverTime);
    if (m.peerKey.length) w.uint32(18).bytes(m.peerKey);
    if (m.nick.length) w.uint32(26).string(m.nick);
    if (m.body.length) w.uint32(34).string(m.body);
    if (m.entities) Message.Entities.encode(m.entities, w.uint32(42).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Message {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Message();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverTime = r.int64();
        break;
        case 2:
        m.peerKey = r.bytes();
        break;
        case 3:
        m.nick = r.string();
        break;
        case 4:
        m.body = r.string();
        break;
        case 5:
        m.entities = Message.Entities.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace Message {
  export type IEntities = {
    links?: Message.Entities.ILink[];
    emotes?: Message.Entities.IEmote[];
    nicks?: Message.Entities.INick[];
    tags?: Message.Entities.ITag[];
    codeBlocks?: Message.Entities.ICodeBlock[];
    spoilers?: Message.Entities.ISpoiler[];
    greenText?: Message.Entities.IGenericEntity;
    selfMessage?: Message.Entities.IGenericEntity;
  }

  export class Entities {
    links: Message.Entities.Link[];
    emotes: Message.Entities.Emote[];
    nicks: Message.Entities.Nick[];
    tags: Message.Entities.Tag[];
    codeBlocks: Message.Entities.CodeBlock[];
    spoilers: Message.Entities.Spoiler[];
    greenText: Message.Entities.GenericEntity | undefined;
    selfMessage: Message.Entities.GenericEntity | undefined;

    constructor(v?: IEntities) {
      this.links = v?.links ? v.links.map(v => new Message.Entities.Link(v)) : [];
      this.emotes = v?.emotes ? v.emotes.map(v => new Message.Entities.Emote(v)) : [];
      this.nicks = v?.nicks ? v.nicks.map(v => new Message.Entities.Nick(v)) : [];
      this.tags = v?.tags ? v.tags.map(v => new Message.Entities.Tag(v)) : [];
      this.codeBlocks = v?.codeBlocks ? v.codeBlocks.map(v => new Message.Entities.CodeBlock(v)) : [];
      this.spoilers = v?.spoilers ? v.spoilers.map(v => new Message.Entities.Spoiler(v)) : [];
      this.greenText = v?.greenText && new Message.Entities.GenericEntity(v.greenText);
      this.selfMessage = v?.selfMessage && new Message.Entities.GenericEntity(v.selfMessage);
    }

    static encode(m: Entities, w?: Writer): Writer {
      if (!w) w = new Writer();
      for (const v of m.links) Message.Entities.Link.encode(v, w.uint32(10).fork()).ldelim();
      for (const v of m.emotes) Message.Entities.Emote.encode(v, w.uint32(18).fork()).ldelim();
      for (const v of m.nicks) Message.Entities.Nick.encode(v, w.uint32(26).fork()).ldelim();
      for (const v of m.tags) Message.Entities.Tag.encode(v, w.uint32(34).fork()).ldelim();
      for (const v of m.codeBlocks) Message.Entities.CodeBlock.encode(v, w.uint32(42).fork()).ldelim();
      for (const v of m.spoilers) Message.Entities.Spoiler.encode(v, w.uint32(50).fork()).ldelim();
      if (m.greenText) Message.Entities.GenericEntity.encode(m.greenText, w.uint32(58).fork()).ldelim();
      if (m.selfMessage) Message.Entities.GenericEntity.encode(m.selfMessage, w.uint32(66).fork()).ldelim();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Entities {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Entities();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.links.push(Message.Entities.Link.decode(r, r.uint32()));
          break;
          case 2:
          m.emotes.push(Message.Entities.Emote.decode(r, r.uint32()));
          break;
          case 3:
          m.nicks.push(Message.Entities.Nick.decode(r, r.uint32()));
          break;
          case 4:
          m.tags.push(Message.Entities.Tag.decode(r, r.uint32()));
          break;
          case 5:
          m.codeBlocks.push(Message.Entities.CodeBlock.decode(r, r.uint32()));
          break;
          case 6:
          m.spoilers.push(Message.Entities.Spoiler.decode(r, r.uint32()));
          break;
          case 7:
          m.greenText = Message.Entities.GenericEntity.decode(r, r.uint32());
          break;
          case 8:
          m.selfMessage = Message.Entities.GenericEntity.decode(r, r.uint32());
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export namespace Entities {
    export type IBounds = {
      start?: number;
      end?: number;
    }

    export class Bounds {
      start: number;
      end: number;

      constructor(v?: IBounds) {
        this.start = v?.start || 0;
        this.end = v?.end || 0;
      }

      static encode(m: Bounds, w?: Writer): Writer {
        if (!w) w = new Writer();
        if (m.start) w.uint32(8).uint32(m.start);
        if (m.end) w.uint32(16).uint32(m.end);
        return w;
      }

      static decode(r: Reader | Uint8Array, length?: number): Bounds {
        r = r instanceof Reader ? r : new Reader(r);
        const end = length === undefined ? r.len : r.pos + length;
        const m = new Bounds();
        while (r.pos < end) {
          const tag = r.uint32();
          switch (tag >> 3) {
            case 1:
            m.start = r.uint32();
            break;
            case 2:
            m.end = r.uint32();
            break;
            default:
            r.skipType(tag & 7);
            break;
          }
        }
        return m;
      }
    }

    export type ILink = {
      bounds?: Message.Entities.IBounds;
      url?: string;
    }

    export class Link {
      bounds: Message.Entities.Bounds | undefined;
      url: string;

      constructor(v?: ILink) {
        this.bounds = v?.bounds && new Message.Entities.Bounds(v.bounds);
        this.url = v?.url || "";
      }

      static encode(m: Link, w?: Writer): Writer {
        if (!w) w = new Writer();
        if (m.bounds) Message.Entities.Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
        if (m.url.length) w.uint32(18).string(m.url);
        return w;
      }

      static decode(r: Reader | Uint8Array, length?: number): Link {
        r = r instanceof Reader ? r : new Reader(r);
        const end = length === undefined ? r.len : r.pos + length;
        const m = new Link();
        while (r.pos < end) {
          const tag = r.uint32();
          switch (tag >> 3) {
            case 1:
            m.bounds = Message.Entities.Bounds.decode(r, r.uint32());
            break;
            case 2:
            m.url = r.string();
            break;
            default:
            r.skipType(tag & 7);
            break;
          }
        }
        return m;
      }
    }

    export type IEmote = {
      bounds?: Message.Entities.IBounds;
      name?: string;
      modifiers?: string[];
      combo?: number;
    }

    export class Emote {
      bounds: Message.Entities.Bounds | undefined;
      name: string;
      modifiers: string[];
      combo: number;

      constructor(v?: IEmote) {
        this.bounds = v?.bounds && new Message.Entities.Bounds(v.bounds);
        this.name = v?.name || "";
        this.modifiers = v?.modifiers ? v.modifiers : [];
        this.combo = v?.combo || 0;
      }

      static encode(m: Emote, w?: Writer): Writer {
        if (!w) w = new Writer();
        if (m.bounds) Message.Entities.Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
        if (m.name.length) w.uint32(18).string(m.name);
        for (const v of m.modifiers) w.uint32(26).string(v);
        if (m.combo) w.uint32(32).uint32(m.combo);
        return w;
      }

      static decode(r: Reader | Uint8Array, length?: number): Emote {
        r = r instanceof Reader ? r : new Reader(r);
        const end = length === undefined ? r.len : r.pos + length;
        const m = new Emote();
        while (r.pos < end) {
          const tag = r.uint32();
          switch (tag >> 3) {
            case 1:
            m.bounds = Message.Entities.Bounds.decode(r, r.uint32());
            break;
            case 2:
            m.name = r.string();
            break;
            case 3:
            m.modifiers.push(r.string())
            break;
            case 4:
            m.combo = r.uint32();
            break;
            default:
            r.skipType(tag & 7);
            break;
          }
        }
        return m;
      }
    }

    export type INick = {
      bounds?: Message.Entities.IBounds;
      nick?: string;
      peerKey?: Uint8Array;
    }

    export class Nick {
      bounds: Message.Entities.Bounds | undefined;
      nick: string;
      peerKey: Uint8Array;

      constructor(v?: INick) {
        this.bounds = v?.bounds && new Message.Entities.Bounds(v.bounds);
        this.nick = v?.nick || "";
        this.peerKey = v?.peerKey || new Uint8Array();
      }

      static encode(m: Nick, w?: Writer): Writer {
        if (!w) w = new Writer();
        if (m.bounds) Message.Entities.Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
        if (m.nick.length) w.uint32(18).string(m.nick);
        if (m.peerKey.length) w.uint32(26).bytes(m.peerKey);
        return w;
      }

      static decode(r: Reader | Uint8Array, length?: number): Nick {
        r = r instanceof Reader ? r : new Reader(r);
        const end = length === undefined ? r.len : r.pos + length;
        const m = new Nick();
        while (r.pos < end) {
          const tag = r.uint32();
          switch (tag >> 3) {
            case 1:
            m.bounds = Message.Entities.Bounds.decode(r, r.uint32());
            break;
            case 2:
            m.nick = r.string();
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

    export type ITag = {
      bounds?: Message.Entities.IBounds;
      name?: string;
    }

    export class Tag {
      bounds: Message.Entities.Bounds | undefined;
      name: string;

      constructor(v?: ITag) {
        this.bounds = v?.bounds && new Message.Entities.Bounds(v.bounds);
        this.name = v?.name || "";
      }

      static encode(m: Tag, w?: Writer): Writer {
        if (!w) w = new Writer();
        if (m.bounds) Message.Entities.Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
        if (m.name.length) w.uint32(18).string(m.name);
        return w;
      }

      static decode(r: Reader | Uint8Array, length?: number): Tag {
        r = r instanceof Reader ? r : new Reader(r);
        const end = length === undefined ? r.len : r.pos + length;
        const m = new Tag();
        while (r.pos < end) {
          const tag = r.uint32();
          switch (tag >> 3) {
            case 1:
            m.bounds = Message.Entities.Bounds.decode(r, r.uint32());
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

    export type ICodeBlock = {
      bounds?: Message.Entities.IBounds;
    }

    export class CodeBlock {
      bounds: Message.Entities.Bounds | undefined;

      constructor(v?: ICodeBlock) {
        this.bounds = v?.bounds && new Message.Entities.Bounds(v.bounds);
      }

      static encode(m: CodeBlock, w?: Writer): Writer {
        if (!w) w = new Writer();
        if (m.bounds) Message.Entities.Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
        return w;
      }

      static decode(r: Reader | Uint8Array, length?: number): CodeBlock {
        r = r instanceof Reader ? r : new Reader(r);
        const end = length === undefined ? r.len : r.pos + length;
        const m = new CodeBlock();
        while (r.pos < end) {
          const tag = r.uint32();
          switch (tag >> 3) {
            case 1:
            m.bounds = Message.Entities.Bounds.decode(r, r.uint32());
            break;
            default:
            r.skipType(tag & 7);
            break;
          }
        }
        return m;
      }
    }

    export type ISpoiler = {
      bounds?: Message.Entities.IBounds;
    }

    export class Spoiler {
      bounds: Message.Entities.Bounds | undefined;

      constructor(v?: ISpoiler) {
        this.bounds = v?.bounds && new Message.Entities.Bounds(v.bounds);
      }

      static encode(m: Spoiler, w?: Writer): Writer {
        if (!w) w = new Writer();
        if (m.bounds) Message.Entities.Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
        return w;
      }

      static decode(r: Reader | Uint8Array, length?: number): Spoiler {
        r = r instanceof Reader ? r : new Reader(r);
        const end = length === undefined ? r.len : r.pos + length;
        const m = new Spoiler();
        while (r.pos < end) {
          const tag = r.uint32();
          switch (tag >> 3) {
            case 1:
            m.bounds = Message.Entities.Bounds.decode(r, r.uint32());
            break;
            default:
            r.skipType(tag & 7);
            break;
          }
        }
        return m;
      }
    }

    export type IGenericEntity = {
      bounds?: Message.Entities.IBounds;
    }

    export class GenericEntity {
      bounds: Message.Entities.Bounds | undefined;

      constructor(v?: IGenericEntity) {
        this.bounds = v?.bounds && new Message.Entities.Bounds(v.bounds);
      }

      static encode(m: GenericEntity, w?: Writer): Writer {
        if (!w) w = new Writer();
        if (m.bounds) Message.Entities.Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
        return w;
      }

      static decode(r: Reader | Uint8Array, length?: number): GenericEntity {
        r = r instanceof Reader ? r : new Reader(r);
        const end = length === undefined ? r.len : r.pos + length;
        const m = new GenericEntity();
        while (r.pos < end) {
          const tag = r.uint32();
          switch (tag >> 3) {
            case 1:
            m.bounds = Message.Entities.Bounds.decode(r, r.uint32());
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

export type IProfile = {
  id?: bigint;
  serverId?: bigint;
  peerKey?: Uint8Array;
  alias?: string;
  mutes?: Profile.IMute[];
  muteDeadline?: bigint;
}

export class Profile {
  id: bigint;
  serverId: bigint;
  peerKey: Uint8Array;
  alias: string;
  mutes: Profile.Mute[];
  muteDeadline: bigint;

  constructor(v?: IProfile) {
    this.id = v?.id || BigInt(0);
    this.serverId = v?.serverId || BigInt(0);
    this.peerKey = v?.peerKey || new Uint8Array();
    this.alias = v?.alias || "";
    this.mutes = v?.mutes ? v.mutes.map(v => new Profile.Mute(v)) : [];
    this.muteDeadline = v?.muteDeadline || BigInt(0);
  }

  static encode(m: Profile, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.serverId) w.uint32(16).uint64(m.serverId);
    if (m.peerKey.length) w.uint32(26).bytes(m.peerKey);
    if (m.alias.length) w.uint32(34).string(m.alias);
    for (const v of m.mutes) Profile.Mute.encode(v, w.uint32(42).fork()).ldelim();
    if (m.muteDeadline) w.uint32(48).int64(m.muteDeadline);
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
        m.serverId = r.uint64();
        break;
        case 3:
        m.peerKey = r.bytes();
        break;
        case 4:
        m.alias = r.string();
        break;
        case 5:
        m.mutes.push(Profile.Mute.decode(r, r.uint32()));
        break;
        case 6:
        m.muteDeadline = r.int64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace Profile {
  export type IMute = {
    createdAt?: bigint;
    durationSecs?: number;
    message?: string;
    moderatorPeerKey?: Uint8Array;
  }

  export class Mute {
    createdAt: bigint;
    durationSecs: number;
    message: string;
    moderatorPeerKey: Uint8Array;

    constructor(v?: IMute) {
      this.createdAt = v?.createdAt || BigInt(0);
      this.durationSecs = v?.durationSecs || 0;
      this.message = v?.message || "";
      this.moderatorPeerKey = v?.moderatorPeerKey || new Uint8Array();
    }

    static encode(m: Mute, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.createdAt) w.uint32(8).int64(m.createdAt);
      if (m.durationSecs) w.uint32(16).uint32(m.durationSecs);
      if (m.message.length) w.uint32(26).string(m.message);
      if (m.moderatorPeerKey.length) w.uint32(34).bytes(m.moderatorPeerKey);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Mute {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Mute();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.createdAt = r.int64();
          break;
          case 2:
          m.durationSecs = r.uint32();
          break;
          case 3:
          m.message = r.string();
          break;
          case 4:
          m.moderatorPeerKey = r.bytes();
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

export type IUIConfig = {
  showTime?: boolean;
  showFlairIcons?: boolean;
  timestampFormat?: string;
  maxLines?: number;
  notificationWhisper?: boolean;
  soundNotificationWhisper?: boolean;
  notificationHighlight?: boolean;
  soundNotificationHighlight?: boolean;
  notificationSoundFile?: UIConfig.ISoundFile;
  highlight?: boolean;
  customHighlight?: string;
  highlights?: UIConfig.IHighlight[];
  tags?: UIConfig.ITag[];
  showRemoved?: UIConfig.ShowRemoved;
  showWhispersInChat?: boolean;
  ignores?: UIConfig.IIgnore[];
  focusMentioned?: boolean;
  notificationTimeout?: boolean;
  ignoreMentions?: boolean;
  autocompleteHelper?: boolean;
  autocompleteEmotePreview?: boolean;
  taggedVisibility?: boolean;
  hideNsfw?: boolean;
  animateForever?: boolean;
  formatterGreen?: boolean;
  formatterEmote?: boolean;
  formatterCombo?: boolean;
  emoteModifiers?: boolean;
  disableSpoilers?: boolean;
  viewerStateIndicator?: UIConfig.ViewerStateIndicator;
  hiddenEmotes?: string[];
  shortenLinks?: boolean;
  compactEmoteSpacing?: boolean;
  normalizeAliasCase?: boolean;
}

export class UIConfig {
  showTime: boolean;
  showFlairIcons: boolean;
  timestampFormat: string;
  maxLines: number;
  notificationWhisper: boolean;
  soundNotificationWhisper: boolean;
  notificationHighlight: boolean;
  soundNotificationHighlight: boolean;
  notificationSoundFile: UIConfig.SoundFile | undefined;
  highlight: boolean;
  customHighlight: string;
  highlights: UIConfig.Highlight[];
  tags: UIConfig.Tag[];
  showRemoved: UIConfig.ShowRemoved;
  showWhispersInChat: boolean;
  ignores: UIConfig.Ignore[];
  focusMentioned: boolean;
  notificationTimeout: boolean;
  ignoreMentions: boolean;
  autocompleteHelper: boolean;
  autocompleteEmotePreview: boolean;
  taggedVisibility: boolean;
  hideNsfw: boolean;
  animateForever: boolean;
  formatterGreen: boolean;
  formatterEmote: boolean;
  formatterCombo: boolean;
  emoteModifiers: boolean;
  disableSpoilers: boolean;
  viewerStateIndicator: UIConfig.ViewerStateIndicator;
  hiddenEmotes: string[];
  shortenLinks: boolean;
  compactEmoteSpacing: boolean;
  normalizeAliasCase: boolean;

  constructor(v?: IUIConfig) {
    this.showTime = v?.showTime || false;
    this.showFlairIcons = v?.showFlairIcons || false;
    this.timestampFormat = v?.timestampFormat || "";
    this.maxLines = v?.maxLines || 0;
    this.notificationWhisper = v?.notificationWhisper || false;
    this.soundNotificationWhisper = v?.soundNotificationWhisper || false;
    this.notificationHighlight = v?.notificationHighlight || false;
    this.soundNotificationHighlight = v?.soundNotificationHighlight || false;
    this.notificationSoundFile = v?.notificationSoundFile && new UIConfig.SoundFile(v.notificationSoundFile);
    this.highlight = v?.highlight || false;
    this.customHighlight = v?.customHighlight || "";
    this.highlights = v?.highlights ? v.highlights.map(v => new UIConfig.Highlight(v)) : [];
    this.tags = v?.tags ? v.tags.map(v => new UIConfig.Tag(v)) : [];
    this.showRemoved = v?.showRemoved || 0;
    this.showWhispersInChat = v?.showWhispersInChat || false;
    this.ignores = v?.ignores ? v.ignores.map(v => new UIConfig.Ignore(v)) : [];
    this.focusMentioned = v?.focusMentioned || false;
    this.notificationTimeout = v?.notificationTimeout || false;
    this.ignoreMentions = v?.ignoreMentions || false;
    this.autocompleteHelper = v?.autocompleteHelper || false;
    this.autocompleteEmotePreview = v?.autocompleteEmotePreview || false;
    this.taggedVisibility = v?.taggedVisibility || false;
    this.hideNsfw = v?.hideNsfw || false;
    this.animateForever = v?.animateForever || false;
    this.formatterGreen = v?.formatterGreen || false;
    this.formatterEmote = v?.formatterEmote || false;
    this.formatterCombo = v?.formatterCombo || false;
    this.emoteModifiers = v?.emoteModifiers || false;
    this.disableSpoilers = v?.disableSpoilers || false;
    this.viewerStateIndicator = v?.viewerStateIndicator || 0;
    this.hiddenEmotes = v?.hiddenEmotes ? v.hiddenEmotes : [];
    this.shortenLinks = v?.shortenLinks || false;
    this.compactEmoteSpacing = v?.compactEmoteSpacing || false;
    this.normalizeAliasCase = v?.normalizeAliasCase || false;
  }

  static encode(m: UIConfig, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.showTime) w.uint32(8).bool(m.showTime);
    if (m.showFlairIcons) w.uint32(16).bool(m.showFlairIcons);
    if (m.timestampFormat.length) w.uint32(26).string(m.timestampFormat);
    if (m.maxLines) w.uint32(32).uint32(m.maxLines);
    if (m.notificationWhisper) w.uint32(40).bool(m.notificationWhisper);
    if (m.soundNotificationWhisper) w.uint32(48).bool(m.soundNotificationWhisper);
    if (m.notificationHighlight) w.uint32(56).bool(m.notificationHighlight);
    if (m.soundNotificationHighlight) w.uint32(64).bool(m.soundNotificationHighlight);
    if (m.notificationSoundFile) UIConfig.SoundFile.encode(m.notificationSoundFile, w.uint32(74).fork()).ldelim();
    if (m.highlight) w.uint32(80).bool(m.highlight);
    if (m.customHighlight.length) w.uint32(90).string(m.customHighlight);
    for (const v of m.highlights) UIConfig.Highlight.encode(v, w.uint32(98).fork()).ldelim();
    for (const v of m.tags) UIConfig.Tag.encode(v, w.uint32(106).fork()).ldelim();
    if (m.showRemoved) w.uint32(112).uint32(m.showRemoved);
    if (m.showWhispersInChat) w.uint32(120).bool(m.showWhispersInChat);
    for (const v of m.ignores) UIConfig.Ignore.encode(v, w.uint32(130).fork()).ldelim();
    if (m.focusMentioned) w.uint32(136).bool(m.focusMentioned);
    if (m.notificationTimeout) w.uint32(144).bool(m.notificationTimeout);
    if (m.ignoreMentions) w.uint32(152).bool(m.ignoreMentions);
    if (m.autocompleteHelper) w.uint32(160).bool(m.autocompleteHelper);
    if (m.autocompleteEmotePreview) w.uint32(168).bool(m.autocompleteEmotePreview);
    if (m.taggedVisibility) w.uint32(176).bool(m.taggedVisibility);
    if (m.hideNsfw) w.uint32(184).bool(m.hideNsfw);
    if (m.animateForever) w.uint32(192).bool(m.animateForever);
    if (m.formatterGreen) w.uint32(200).bool(m.formatterGreen);
    if (m.formatterEmote) w.uint32(208).bool(m.formatterEmote);
    if (m.formatterCombo) w.uint32(216).bool(m.formatterCombo);
    if (m.emoteModifiers) w.uint32(224).bool(m.emoteModifiers);
    if (m.disableSpoilers) w.uint32(232).bool(m.disableSpoilers);
    if (m.viewerStateIndicator) w.uint32(240).uint32(m.viewerStateIndicator);
    for (const v of m.hiddenEmotes) w.uint32(250).string(v);
    if (m.shortenLinks) w.uint32(256).bool(m.shortenLinks);
    if (m.compactEmoteSpacing) w.uint32(264).bool(m.compactEmoteSpacing);
    if (m.normalizeAliasCase) w.uint32(272).bool(m.normalizeAliasCase);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UIConfig {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UIConfig();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.showTime = r.bool();
        break;
        case 2:
        m.showFlairIcons = r.bool();
        break;
        case 3:
        m.timestampFormat = r.string();
        break;
        case 4:
        m.maxLines = r.uint32();
        break;
        case 5:
        m.notificationWhisper = r.bool();
        break;
        case 6:
        m.soundNotificationWhisper = r.bool();
        break;
        case 7:
        m.notificationHighlight = r.bool();
        break;
        case 8:
        m.soundNotificationHighlight = r.bool();
        break;
        case 9:
        m.notificationSoundFile = UIConfig.SoundFile.decode(r, r.uint32());
        break;
        case 10:
        m.highlight = r.bool();
        break;
        case 11:
        m.customHighlight = r.string();
        break;
        case 12:
        m.highlights.push(UIConfig.Highlight.decode(r, r.uint32()));
        break;
        case 13:
        m.tags.push(UIConfig.Tag.decode(r, r.uint32()));
        break;
        case 14:
        m.showRemoved = r.uint32();
        break;
        case 15:
        m.showWhispersInChat = r.bool();
        break;
        case 16:
        m.ignores.push(UIConfig.Ignore.decode(r, r.uint32()));
        break;
        case 17:
        m.focusMentioned = r.bool();
        break;
        case 18:
        m.notificationTimeout = r.bool();
        break;
        case 19:
        m.ignoreMentions = r.bool();
        break;
        case 20:
        m.autocompleteHelper = r.bool();
        break;
        case 21:
        m.autocompleteEmotePreview = r.bool();
        break;
        case 22:
        m.taggedVisibility = r.bool();
        break;
        case 23:
        m.hideNsfw = r.bool();
        break;
        case 24:
        m.animateForever = r.bool();
        break;
        case 25:
        m.formatterGreen = r.bool();
        break;
        case 26:
        m.formatterEmote = r.bool();
        break;
        case 27:
        m.formatterCombo = r.bool();
        break;
        case 28:
        m.emoteModifiers = r.bool();
        break;
        case 29:
        m.disableSpoilers = r.bool();
        break;
        case 30:
        m.viewerStateIndicator = r.uint32();
        break;
        case 31:
        m.hiddenEmotes.push(r.string())
        break;
        case 32:
        m.shortenLinks = r.bool();
        break;
        case 33:
        m.compactEmoteSpacing = r.bool();
        break;
        case 34:
        m.normalizeAliasCase = r.bool();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace UIConfig {
  export type ISoundFile = {
    fileType?: string;
    data?: Uint8Array;
  }

  export class SoundFile {
    fileType: string;
    data: Uint8Array;

    constructor(v?: ISoundFile) {
      this.fileType = v?.fileType || "";
      this.data = v?.data || new Uint8Array();
    }

    static encode(m: SoundFile, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.fileType.length) w.uint32(10).string(m.fileType);
      if (m.data.length) w.uint32(18).bytes(m.data);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): SoundFile {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new SoundFile();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.fileType = r.string();
          break;
          case 2:
          m.data = r.bytes();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type IHighlight = {
    alias?: string;
    peerKey?: Uint8Array;
  }

  export class Highlight {
    alias: string;
    peerKey: Uint8Array;

    constructor(v?: IHighlight) {
      this.alias = v?.alias || "";
      this.peerKey = v?.peerKey || new Uint8Array();
    }

    static encode(m: Highlight, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.alias.length) w.uint32(10).string(m.alias);
      if (m.peerKey.length) w.uint32(18).bytes(m.peerKey);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Highlight {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Highlight();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.alias = r.string();
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

  export type ITag = {
    alias?: string;
    peerKey?: Uint8Array;
    color?: string;
  }

  export class Tag {
    alias: string;
    peerKey: Uint8Array;
    color: string;

    constructor(v?: ITag) {
      this.alias = v?.alias || "";
      this.peerKey = v?.peerKey || new Uint8Array();
      this.color = v?.color || "";
    }

    static encode(m: Tag, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.alias.length) w.uint32(10).string(m.alias);
      if (m.peerKey.length) w.uint32(18).bytes(m.peerKey);
      if (m.color.length) w.uint32(26).string(m.color);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Tag {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Tag();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.alias = r.string();
          break;
          case 2:
          m.peerKey = r.bytes();
          break;
          case 3:
          m.color = r.string();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export type IIgnore = {
    alias?: string;
    peerKey?: Uint8Array;
    deadline?: bigint;
  }

  export class Ignore {
    alias: string;
    peerKey: Uint8Array;
    deadline: bigint;

    constructor(v?: IIgnore) {
      this.alias = v?.alias || "";
      this.peerKey = v?.peerKey || new Uint8Array();
      this.deadline = v?.deadline || BigInt(0);
    }

    static encode(m: Ignore, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.alias.length) w.uint32(10).string(m.alias);
      if (m.peerKey.length) w.uint32(18).bytes(m.peerKey);
      if (m.deadline) w.uint32(24).int64(m.deadline);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Ignore {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Ignore();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.alias = r.string();
          break;
          case 2:
          m.peerKey = r.bytes();
          break;
          case 3:
          m.deadline = r.int64();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export enum ShowRemoved {
    SHOW_REMOVED_REMOVE = 0,
    SHOW_REMOVED_CENSOR = 1,
    SHOW_REMOVED_DO_NOTHING = 2,
  }
  export enum ViewerStateIndicator {
    VIEWER_STATE_INDICATOR_DISABLED = 0,
    VIEWER_STATE_INDICATOR_BAR = 1,
    VIEWER_STATE_INDICATOR_DOT = 2,
    VIEWER_STATE_INDICATOR_ARRAY = 3,
  }
}

export type ICreateServerRequest = {
  networkKey?: Uint8Array;
  room?: IRoom;
}

export class CreateServerRequest {
  networkKey: Uint8Array;
  room: Room | undefined;

  constructor(v?: ICreateServerRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.room = v?.room && new Room(v.room);
  }

  static encode(m: CreateServerRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(18).bytes(m.networkKey);
    if (m.room) Room.encode(m.room, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateServerRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreateServerRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 2:
        m.networkKey = r.bytes();
        break;
        case 3:
        m.room = Room.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICreateServerResponse = {
  server?: IServer;
}

export class CreateServerResponse {
  server: Server | undefined;

  constructor(v?: ICreateServerResponse) {
    this.server = v?.server && new Server(v.server);
  }

  static encode(m: CreateServerResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.server) Server.encode(m.server, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateServerResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreateServerResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.server = Server.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IUpdateServerRequest = {
  id?: bigint;
  networkKey?: Uint8Array;
  room?: IRoom;
}

export class UpdateServerRequest {
  id: bigint;
  networkKey: Uint8Array;
  room: Room | undefined;

  constructor(v?: IUpdateServerRequest) {
    this.id = v?.id || BigInt(0);
    this.networkKey = v?.networkKey || new Uint8Array();
    this.room = v?.room && new Room(v.room);
  }

  static encode(m: UpdateServerRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.networkKey.length) w.uint32(18).bytes(m.networkKey);
    if (m.room) Room.encode(m.room, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateServerRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UpdateServerRequest();
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
        m.room = Room.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IUpdateServerResponse = {
  server?: IServer;
}

export class UpdateServerResponse {
  server: Server | undefined;

  constructor(v?: IUpdateServerResponse) {
    this.server = v?.server && new Server(v.server);
  }

  static encode(m: UpdateServerResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.server) Server.encode(m.server, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateServerResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UpdateServerResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.server = Server.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IDeleteServerRequest = {
  id?: bigint;
}

export class DeleteServerRequest {
  id: bigint;

  constructor(v?: IDeleteServerRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: DeleteServerRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeleteServerRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DeleteServerRequest();
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

export type IDeleteServerResponse = {
}

export class DeleteServerResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IDeleteServerResponse) {
  }

  static encode(m: DeleteServerResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeleteServerResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new DeleteServerResponse();
  }
}

export type IGetServerRequest = {
  id?: bigint;
}

export class GetServerRequest {
  id: bigint;

  constructor(v?: IGetServerRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: GetServerRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetServerRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetServerRequest();
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

export type IGetServerResponse = {
  server?: IServer;
}

export class GetServerResponse {
  server: Server | undefined;

  constructor(v?: IGetServerResponse) {
    this.server = v?.server && new Server(v.server);
  }

  static encode(m: GetServerResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.server) Server.encode(m.server, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetServerResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetServerResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.server = Server.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IListServersRequest = {
}

export class ListServersRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IListServersRequest) {
  }

  static encode(m: ListServersRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListServersRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new ListServersRequest();
  }
}

export type IListServersResponse = {
  servers?: IServer[];
}

export class ListServersResponse {
  servers: Server[];

  constructor(v?: IListServersResponse) {
    this.servers = v?.servers ? v.servers.map(v => new Server(v)) : [];
  }

  static encode(m: ListServersResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.servers) Server.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListServersResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ListServersResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.servers.push(Server.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICreateEmoteRequest = {
  serverId?: bigint;
  name?: string;
  images?: IEmoteImage[];
  css?: string;
  effects?: IEmoteEffect[];
  contributor?: IEmoteContributor;
}

export class CreateEmoteRequest {
  serverId: bigint;
  name: string;
  images: EmoteImage[];
  css: string;
  effects: EmoteEffect[];
  contributor: EmoteContributor | undefined;

  constructor(v?: ICreateEmoteRequest) {
    this.serverId = v?.serverId || BigInt(0);
    this.name = v?.name || "";
    this.images = v?.images ? v.images.map(v => new EmoteImage(v)) : [];
    this.css = v?.css || "";
    this.effects = v?.effects ? v.effects.map(v => new EmoteEffect(v)) : [];
    this.contributor = v?.contributor && new EmoteContributor(v.contributor);
  }

  static encode(m: CreateEmoteRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    if (m.name.length) w.uint32(18).string(m.name);
    for (const v of m.images) EmoteImage.encode(v, w.uint32(26).fork()).ldelim();
    if (m.css.length) w.uint32(34).string(m.css);
    for (const v of m.effects) EmoteEffect.encode(v, w.uint32(42).fork()).ldelim();
    if (m.contributor) EmoteContributor.encode(m.contributor, w.uint32(50).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateEmoteRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreateEmoteRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverId = r.uint64();
        break;
        case 2:
        m.name = r.string();
        break;
        case 3:
        m.images.push(EmoteImage.decode(r, r.uint32()));
        break;
        case 4:
        m.css = r.string();
        break;
        case 5:
        m.effects.push(EmoteEffect.decode(r, r.uint32()));
        break;
        case 6:
        m.contributor = EmoteContributor.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICreateEmoteResponse = {
  emote?: IEmote;
}

export class CreateEmoteResponse {
  emote: Emote | undefined;

  constructor(v?: ICreateEmoteResponse) {
    this.emote = v?.emote && new Emote(v.emote);
  }

  static encode(m: CreateEmoteResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.emote) Emote.encode(m.emote, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateEmoteResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreateEmoteResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.emote = Emote.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IUpdateEmoteRequest = {
  serverId?: bigint;
  id?: bigint;
  name?: string;
  images?: IEmoteImage[];
  css?: string;
  effects?: IEmoteEffect[];
  contributor?: IEmoteContributor;
}

export class UpdateEmoteRequest {
  serverId: bigint;
  id: bigint;
  name: string;
  images: EmoteImage[];
  css: string;
  effects: EmoteEffect[];
  contributor: EmoteContributor | undefined;

  constructor(v?: IUpdateEmoteRequest) {
    this.serverId = v?.serverId || BigInt(0);
    this.id = v?.id || BigInt(0);
    this.name = v?.name || "";
    this.images = v?.images ? v.images.map(v => new EmoteImage(v)) : [];
    this.css = v?.css || "";
    this.effects = v?.effects ? v.effects.map(v => new EmoteEffect(v)) : [];
    this.contributor = v?.contributor && new EmoteContributor(v.contributor);
  }

  static encode(m: UpdateEmoteRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    if (m.id) w.uint32(16).uint64(m.id);
    if (m.name.length) w.uint32(26).string(m.name);
    for (const v of m.images) EmoteImage.encode(v, w.uint32(34).fork()).ldelim();
    if (m.css.length) w.uint32(42).string(m.css);
    for (const v of m.effects) EmoteEffect.encode(v, w.uint32(50).fork()).ldelim();
    if (m.contributor) EmoteContributor.encode(m.contributor, w.uint32(58).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateEmoteRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UpdateEmoteRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverId = r.uint64();
        break;
        case 2:
        m.id = r.uint64();
        break;
        case 3:
        m.name = r.string();
        break;
        case 4:
        m.images.push(EmoteImage.decode(r, r.uint32()));
        break;
        case 5:
        m.css = r.string();
        break;
        case 6:
        m.effects.push(EmoteEffect.decode(r, r.uint32()));
        break;
        case 7:
        m.contributor = EmoteContributor.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IUpdateEmoteResponse = {
  emote?: IEmote;
}

export class UpdateEmoteResponse {
  emote: Emote | undefined;

  constructor(v?: IUpdateEmoteResponse) {
    this.emote = v?.emote && new Emote(v.emote);
  }

  static encode(m: UpdateEmoteResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.emote) Emote.encode(m.emote, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateEmoteResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UpdateEmoteResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.emote = Emote.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IDeleteEmoteRequest = {
  serverId?: bigint;
  id?: bigint;
}

export class DeleteEmoteRequest {
  serverId: bigint;
  id: bigint;

  constructor(v?: IDeleteEmoteRequest) {
    this.serverId = v?.serverId || BigInt(0);
    this.id = v?.id || BigInt(0);
  }

  static encode(m: DeleteEmoteRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    if (m.id) w.uint32(16).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeleteEmoteRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DeleteEmoteRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverId = r.uint64();
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

export type IDeleteEmoteResponse = {
}

export class DeleteEmoteResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IDeleteEmoteResponse) {
  }

  static encode(m: DeleteEmoteResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeleteEmoteResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new DeleteEmoteResponse();
  }
}

export type IGetEmoteRequest = {
  id?: bigint;
}

export class GetEmoteRequest {
  id: bigint;

  constructor(v?: IGetEmoteRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: GetEmoteRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetEmoteRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetEmoteRequest();
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

export type IGetEmoteResponse = {
  emote?: IEmote;
}

export class GetEmoteResponse {
  emote: Emote | undefined;

  constructor(v?: IGetEmoteResponse) {
    this.emote = v?.emote && new Emote(v.emote);
  }

  static encode(m: GetEmoteResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.emote) Emote.encode(m.emote, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetEmoteResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetEmoteResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.emote = Emote.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IListEmotesRequest = {
  serverId?: bigint;
}

export class ListEmotesRequest {
  serverId: bigint;

  constructor(v?: IListEmotesRequest) {
    this.serverId = v?.serverId || BigInt(0);
  }

  static encode(m: ListEmotesRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListEmotesRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ListEmotesRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverId = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IListEmotesResponse = {
  emotes?: IEmote[];
}

export class ListEmotesResponse {
  emotes: Emote[];

  constructor(v?: IListEmotesResponse) {
    this.emotes = v?.emotes ? v.emotes.map(v => new Emote(v)) : [];
  }

  static encode(m: ListEmotesResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.emotes) Emote.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListEmotesResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ListEmotesResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.emotes.push(Emote.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICreateModifierRequest = {
  serverId?: bigint;
  name?: string;
  priority?: number;
  internal?: boolean;
}

export class CreateModifierRequest {
  serverId: bigint;
  name: string;
  priority: number;
  internal: boolean;

  constructor(v?: ICreateModifierRequest) {
    this.serverId = v?.serverId || BigInt(0);
    this.name = v?.name || "";
    this.priority = v?.priority || 0;
    this.internal = v?.internal || false;
  }

  static encode(m: CreateModifierRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    if (m.name.length) w.uint32(18).string(m.name);
    if (m.priority) w.uint32(24).uint32(m.priority);
    if (m.internal) w.uint32(32).bool(m.internal);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateModifierRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreateModifierRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverId = r.uint64();
        break;
        case 2:
        m.name = r.string();
        break;
        case 3:
        m.priority = r.uint32();
        break;
        case 4:
        m.internal = r.bool();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICreateModifierResponse = {
  modifier?: IModifier;
}

export class CreateModifierResponse {
  modifier: Modifier | undefined;

  constructor(v?: ICreateModifierResponse) {
    this.modifier = v?.modifier && new Modifier(v.modifier);
  }

  static encode(m: CreateModifierResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.modifier) Modifier.encode(m.modifier, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateModifierResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreateModifierResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.modifier = Modifier.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IUpdateModifierRequest = {
  serverId?: bigint;
  id?: bigint;
  name?: string;
  priority?: number;
  internal?: boolean;
}

export class UpdateModifierRequest {
  serverId: bigint;
  id: bigint;
  name: string;
  priority: number;
  internal: boolean;

  constructor(v?: IUpdateModifierRequest) {
    this.serverId = v?.serverId || BigInt(0);
    this.id = v?.id || BigInt(0);
    this.name = v?.name || "";
    this.priority = v?.priority || 0;
    this.internal = v?.internal || false;
  }

  static encode(m: UpdateModifierRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    if (m.id) w.uint32(16).uint64(m.id);
    if (m.name.length) w.uint32(26).string(m.name);
    if (m.priority) w.uint32(32).uint32(m.priority);
    if (m.internal) w.uint32(40).bool(m.internal);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateModifierRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UpdateModifierRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverId = r.uint64();
        break;
        case 2:
        m.id = r.uint64();
        break;
        case 3:
        m.name = r.string();
        break;
        case 4:
        m.priority = r.uint32();
        break;
        case 5:
        m.internal = r.bool();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IUpdateModifierResponse = {
  modifier?: IModifier;
}

export class UpdateModifierResponse {
  modifier: Modifier | undefined;

  constructor(v?: IUpdateModifierResponse) {
    this.modifier = v?.modifier && new Modifier(v.modifier);
  }

  static encode(m: UpdateModifierResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.modifier) Modifier.encode(m.modifier, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateModifierResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UpdateModifierResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.modifier = Modifier.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IDeleteModifierRequest = {
  serverId?: bigint;
  id?: bigint;
}

export class DeleteModifierRequest {
  serverId: bigint;
  id: bigint;

  constructor(v?: IDeleteModifierRequest) {
    this.serverId = v?.serverId || BigInt(0);
    this.id = v?.id || BigInt(0);
  }

  static encode(m: DeleteModifierRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    if (m.id) w.uint32(16).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeleteModifierRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DeleteModifierRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverId = r.uint64();
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

export type IDeleteModifierResponse = {
}

export class DeleteModifierResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IDeleteModifierResponse) {
  }

  static encode(m: DeleteModifierResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeleteModifierResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new DeleteModifierResponse();
  }
}

export type IGetModifierRequest = {
  id?: bigint;
}

export class GetModifierRequest {
  id: bigint;

  constructor(v?: IGetModifierRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: GetModifierRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetModifierRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetModifierRequest();
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

export type IGetModifierResponse = {
  modifier?: IModifier;
}

export class GetModifierResponse {
  modifier: Modifier | undefined;

  constructor(v?: IGetModifierResponse) {
    this.modifier = v?.modifier && new Modifier(v.modifier);
  }

  static encode(m: GetModifierResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.modifier) Modifier.encode(m.modifier, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetModifierResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetModifierResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.modifier = Modifier.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IListModifiersRequest = {
  serverId?: bigint;
}

export class ListModifiersRequest {
  serverId: bigint;

  constructor(v?: IListModifiersRequest) {
    this.serverId = v?.serverId || BigInt(0);
  }

  static encode(m: ListModifiersRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListModifiersRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ListModifiersRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverId = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IListModifiersResponse = {
  modifiers?: IModifier[];
}

export class ListModifiersResponse {
  modifiers: Modifier[];

  constructor(v?: IListModifiersResponse) {
    this.modifiers = v?.modifiers ? v.modifiers.map(v => new Modifier(v)) : [];
  }

  static encode(m: ListModifiersResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.modifiers) Modifier.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListModifiersResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ListModifiersResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.modifiers.push(Modifier.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICreateTagRequest = {
  serverId?: bigint;
  name?: string;
  color?: string;
  sensitive?: boolean;
}

export class CreateTagRequest {
  serverId: bigint;
  name: string;
  color: string;
  sensitive: boolean;

  constructor(v?: ICreateTagRequest) {
    this.serverId = v?.serverId || BigInt(0);
    this.name = v?.name || "";
    this.color = v?.color || "";
    this.sensitive = v?.sensitive || false;
  }

  static encode(m: CreateTagRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    if (m.name.length) w.uint32(18).string(m.name);
    if (m.color.length) w.uint32(26).string(m.color);
    if (m.sensitive) w.uint32(32).bool(m.sensitive);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateTagRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreateTagRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverId = r.uint64();
        break;
        case 2:
        m.name = r.string();
        break;
        case 3:
        m.color = r.string();
        break;
        case 4:
        m.sensitive = r.bool();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICreateTagResponse = {
  tag?: ITag;
}

export class CreateTagResponse {
  tag: Tag | undefined;

  constructor(v?: ICreateTagResponse) {
    this.tag = v?.tag && new Tag(v.tag);
  }

  static encode(m: CreateTagResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.tag) Tag.encode(m.tag, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateTagResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreateTagResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.tag = Tag.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IUpdateTagRequest = {
  serverId?: bigint;
  id?: bigint;
  name?: string;
  color?: string;
  sensitive?: boolean;
}

export class UpdateTagRequest {
  serverId: bigint;
  id: bigint;
  name: string;
  color: string;
  sensitive: boolean;

  constructor(v?: IUpdateTagRequest) {
    this.serverId = v?.serverId || BigInt(0);
    this.id = v?.id || BigInt(0);
    this.name = v?.name || "";
    this.color = v?.color || "";
    this.sensitive = v?.sensitive || false;
  }

  static encode(m: UpdateTagRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    if (m.id) w.uint32(16).uint64(m.id);
    if (m.name.length) w.uint32(26).string(m.name);
    if (m.color.length) w.uint32(34).string(m.color);
    if (m.sensitive) w.uint32(40).bool(m.sensitive);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateTagRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UpdateTagRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverId = r.uint64();
        break;
        case 2:
        m.id = r.uint64();
        break;
        case 3:
        m.name = r.string();
        break;
        case 4:
        m.color = r.string();
        break;
        case 5:
        m.sensitive = r.bool();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IUpdateTagResponse = {
  tag?: ITag;
}

export class UpdateTagResponse {
  tag: Tag | undefined;

  constructor(v?: IUpdateTagResponse) {
    this.tag = v?.tag && new Tag(v.tag);
  }

  static encode(m: UpdateTagResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.tag) Tag.encode(m.tag, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateTagResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UpdateTagResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.tag = Tag.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IDeleteTagRequest = {
  serverId?: bigint;
  id?: bigint;
}

export class DeleteTagRequest {
  serverId: bigint;
  id: bigint;

  constructor(v?: IDeleteTagRequest) {
    this.serverId = v?.serverId || BigInt(0);
    this.id = v?.id || BigInt(0);
  }

  static encode(m: DeleteTagRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    if (m.id) w.uint32(16).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeleteTagRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DeleteTagRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverId = r.uint64();
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

export type IDeleteTagResponse = {
}

export class DeleteTagResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IDeleteTagResponse) {
  }

  static encode(m: DeleteTagResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeleteTagResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new DeleteTagResponse();
  }
}

export type IGetTagRequest = {
  id?: bigint;
}

export class GetTagRequest {
  id: bigint;

  constructor(v?: IGetTagRequest) {
    this.id = v?.id || BigInt(0);
  }

  static encode(m: GetTagRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetTagRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetTagRequest();
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

export type IGetTagResponse = {
  tag?: ITag;
}

export class GetTagResponse {
  tag: Tag | undefined;

  constructor(v?: IGetTagResponse) {
    this.tag = v?.tag && new Tag(v.tag);
  }

  static encode(m: GetTagResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.tag) Tag.encode(m.tag, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetTagResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetTagResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.tag = Tag.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IListTagsRequest = {
  serverId?: bigint;
}

export class ListTagsRequest {
  serverId: bigint;

  constructor(v?: IListTagsRequest) {
    this.serverId = v?.serverId || BigInt(0);
  }

  static encode(m: ListTagsRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListTagsRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ListTagsRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverId = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IListTagsResponse = {
  tags?: ITag[];
}

export class ListTagsResponse {
  tags: Tag[];

  constructor(v?: IListTagsResponse) {
    this.tags = v?.tags ? v.tags.map(v => new Tag(v)) : [];
  }

  static encode(m: ListTagsResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.tags) Tag.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListTagsResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ListTagsResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.tags.push(Tag.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ISyncAssetsRequest = {
  serverId?: bigint;
  forceUnifiedUpdate?: boolean;
}

export class SyncAssetsRequest {
  serverId: bigint;
  forceUnifiedUpdate: boolean;

  constructor(v?: ISyncAssetsRequest) {
    this.serverId = v?.serverId || BigInt(0);
    this.forceUnifiedUpdate = v?.forceUnifiedUpdate || false;
  }

  static encode(m: SyncAssetsRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    if (m.forceUnifiedUpdate) w.uint32(16).bool(m.forceUnifiedUpdate);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SyncAssetsRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new SyncAssetsRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverId = r.uint64();
        break;
        case 2:
        m.forceUnifiedUpdate = r.bool();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ISyncAssetsResponse = {
  version?: bigint;
  updateSize?: number;
}

export class SyncAssetsResponse {
  version: bigint;
  updateSize: number;

  constructor(v?: ISyncAssetsResponse) {
    this.version = v?.version || BigInt(0);
    this.updateSize = v?.updateSize || 0;
  }

  static encode(m: SyncAssetsResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.version) w.uint32(8).uint64(m.version);
    if (m.updateSize) w.uint32(16).uint32(m.updateSize);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SyncAssetsResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new SyncAssetsResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.version = r.uint64();
        break;
        case 2:
        m.updateSize = r.uint32();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IOpenClientRequest = {
  networkKey?: Uint8Array;
  serverKey?: Uint8Array;
}

export class OpenClientRequest {
  networkKey: Uint8Array;
  serverKey: Uint8Array;

  constructor(v?: IOpenClientRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.serverKey = v?.serverKey || new Uint8Array();
  }

  static encode(m: OpenClientRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    if (m.serverKey.length) w.uint32(18).bytes(m.serverKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): OpenClientRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new OpenClientRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkKey = r.bytes();
        break;
        case 2:
        m.serverKey = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IOpenClientResponse = {
  body?: OpenClientResponse.IBody
}

export class OpenClientResponse {
  body: OpenClientResponse.TBody;

  constructor(v?: IOpenClientResponse) {
    this.body = new OpenClientResponse.Body(v?.body);
  }

  static encode(m: OpenClientResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case OpenClientResponse.BodyCase.OPEN:
      OpenClientResponse.Open.encode(m.body.open, w.uint32(8010).fork()).ldelim();
      break;
      case OpenClientResponse.BodyCase.SERVER_EVENTS:
      OpenClientResponse.ServerEvents.encode(m.body.serverEvents, w.uint32(8018).fork()).ldelim();
      break;
      case OpenClientResponse.BodyCase.ASSET_BUNDLE:
      AssetBundle.encode(m.body.assetBundle, w.uint32(8026).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): OpenClientResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new OpenClientResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1001:
        m.body = new OpenClientResponse.Body({ open: OpenClientResponse.Open.decode(r, r.uint32()) });
        break;
        case 1002:
        m.body = new OpenClientResponse.Body({ serverEvents: OpenClientResponse.ServerEvents.decode(r, r.uint32()) });
        break;
        case 1003:
        m.body = new OpenClientResponse.Body({ assetBundle: AssetBundle.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace OpenClientResponse {
  export enum BodyCase {
    NOT_SET = 0,
    OPEN = 1001,
    SERVER_EVENTS = 1002,
    ASSET_BUNDLE = 1003,
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.OPEN, open: OpenClientResponse.IOpen }
  |{ case?: BodyCase.SERVER_EVENTS, serverEvents: OpenClientResponse.IServerEvents }
  |{ case?: BodyCase.ASSET_BUNDLE, assetBundle: IAssetBundle }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.OPEN, open: OpenClientResponse.Open }
  |{ case: BodyCase.SERVER_EVENTS, serverEvents: OpenClientResponse.ServerEvents }
  |{ case: BodyCase.ASSET_BUNDLE, assetBundle: AssetBundle }
  >;

  class BodyImpl {
    open: OpenClientResponse.Open;
    serverEvents: OpenClientResponse.ServerEvents;
    assetBundle: AssetBundle;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "open" in v) {
        this.case = BodyCase.OPEN;
        this.open = new OpenClientResponse.Open(v.open);
      } else
      if (v && "serverEvents" in v) {
        this.case = BodyCase.SERVER_EVENTS;
        this.serverEvents = new OpenClientResponse.ServerEvents(v.serverEvents);
      } else
      if (v && "assetBundle" in v) {
        this.case = BodyCase.ASSET_BUNDLE;
        this.assetBundle = new AssetBundle(v.assetBundle);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { open: OpenClientResponse.IOpen } ? { case: BodyCase.OPEN, open: OpenClientResponse.Open } :
    T extends { serverEvents: OpenClientResponse.IServerEvents } ? { case: BodyCase.SERVER_EVENTS, serverEvents: OpenClientResponse.ServerEvents } :
    T extends { assetBundle: IAssetBundle } ? { case: BodyCase.ASSET_BUNDLE, assetBundle: AssetBundle } :
    never
    >;
  };

  export type IOpen = {
  }

  export class Open {

    // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
    constructor(v?: IOpen) {
    }

    static encode(m: Open, w?: Writer): Writer {
      if (!w) w = new Writer();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Open {
      if (r instanceof Reader && length) r.skip(length);
      return new Open();
    }
  }

  export type IServerEvents = {
    events?: IServerEvent[];
  }

  export class ServerEvents {
    events: ServerEvent[];

    constructor(v?: IServerEvents) {
      this.events = v?.events ? v.events.map(v => new ServerEvent(v)) : [];
    }

    static encode(m: ServerEvents, w?: Writer): Writer {
      if (!w) w = new Writer();
      for (const v of m.events) ServerEvent.encode(v, w.uint32(10).fork()).ldelim();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): ServerEvents {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new ServerEvents();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.events.push(ServerEvent.decode(r, r.uint32()));
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

export type IClientSendMessageRequest = {
  networkKey?: Uint8Array;
  serverKey?: Uint8Array;
  body?: string;
}

export class ClientSendMessageRequest {
  networkKey: Uint8Array;
  serverKey: Uint8Array;
  body: string;

  constructor(v?: IClientSendMessageRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.serverKey = v?.serverKey || new Uint8Array();
    this.body = v?.body || "";
  }

  static encode(m: ClientSendMessageRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    if (m.serverKey.length) w.uint32(18).bytes(m.serverKey);
    if (m.body.length) w.uint32(26).string(m.body);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ClientSendMessageRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ClientSendMessageRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkKey = r.bytes();
        break;
        case 2:
        m.serverKey = r.bytes();
        break;
        case 3:
        m.body = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IClientSendMessageResponse = {
}

export class ClientSendMessageResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IClientSendMessageResponse) {
  }

  static encode(m: ClientSendMessageResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ClientSendMessageResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new ClientSendMessageResponse();
  }
}

export type IClientMuteRequest = {
  networkKey?: Uint8Array;
  serverKey?: Uint8Array;
  alias?: string;
  duration?: string;
  message?: string;
}

export class ClientMuteRequest {
  networkKey: Uint8Array;
  serverKey: Uint8Array;
  alias: string;
  duration: string;
  message: string;

  constructor(v?: IClientMuteRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.serverKey = v?.serverKey || new Uint8Array();
    this.alias = v?.alias || "";
    this.duration = v?.duration || "";
    this.message = v?.message || "";
  }

  static encode(m: ClientMuteRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    if (m.serverKey.length) w.uint32(18).bytes(m.serverKey);
    if (m.alias.length) w.uint32(26).string(m.alias);
    if (m.duration.length) w.uint32(34).string(m.duration);
    if (m.message.length) w.uint32(42).string(m.message);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ClientMuteRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ClientMuteRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkKey = r.bytes();
        break;
        case 2:
        m.serverKey = r.bytes();
        break;
        case 3:
        m.alias = r.string();
        break;
        case 4:
        m.duration = r.string();
        break;
        case 5:
        m.message = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IClientMuteResponse = {
}

export class ClientMuteResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IClientMuteResponse) {
  }

  static encode(m: ClientMuteResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ClientMuteResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new ClientMuteResponse();
  }
}

export type IClientUnmuteRequest = {
  networkKey?: Uint8Array;
  serverKey?: Uint8Array;
  alias?: string;
}

export class ClientUnmuteRequest {
  networkKey: Uint8Array;
  serverKey: Uint8Array;
  alias: string;

  constructor(v?: IClientUnmuteRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.serverKey = v?.serverKey || new Uint8Array();
    this.alias = v?.alias || "";
  }

  static encode(m: ClientUnmuteRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    if (m.serverKey.length) w.uint32(18).bytes(m.serverKey);
    if (m.alias.length) w.uint32(26).string(m.alias);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ClientUnmuteRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ClientUnmuteRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkKey = r.bytes();
        break;
        case 2:
        m.serverKey = r.bytes();
        break;
        case 3:
        m.alias = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IClientUnmuteResponse = {
}

export class ClientUnmuteResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IClientUnmuteResponse) {
  }

  static encode(m: ClientUnmuteResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ClientUnmuteResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new ClientUnmuteResponse();
  }
}

export type IClientGetMuteRequest = {
  networkKey?: Uint8Array;
  serverKey?: Uint8Array;
}

export class ClientGetMuteRequest {
  networkKey: Uint8Array;
  serverKey: Uint8Array;

  constructor(v?: IClientGetMuteRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.serverKey = v?.serverKey || new Uint8Array();
  }

  static encode(m: ClientGetMuteRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    if (m.serverKey.length) w.uint32(18).bytes(m.serverKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ClientGetMuteRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ClientGetMuteRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkKey = r.bytes();
        break;
        case 2:
        m.serverKey = r.bytes();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IClientGetMuteResponse = {
  endTime?: bigint;
  message?: string;
}

export class ClientGetMuteResponse {
  endTime: bigint;
  message: string;

  constructor(v?: IClientGetMuteResponse) {
    this.endTime = v?.endTime || BigInt(0);
    this.message = v?.message || "";
  }

  static encode(m: ClientGetMuteResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.endTime) w.uint32(8).int64(m.endTime);
    if (m.message.length) w.uint32(18).string(m.message);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ClientGetMuteResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ClientGetMuteResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.endTime = r.int64();
        break;
        case 2:
        m.message = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IWhisperRequest = {
  networkKey?: Uint8Array;
  serverKey?: Uint8Array;
  alias?: string;
  peerKey?: Uint8Array;
  body?: string;
}

export class WhisperRequest {
  networkKey: Uint8Array;
  serverKey: Uint8Array;
  alias: string;
  peerKey: Uint8Array;
  body: string;

  constructor(v?: IWhisperRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.serverKey = v?.serverKey || new Uint8Array();
    this.alias = v?.alias || "";
    this.peerKey = v?.peerKey || new Uint8Array();
    this.body = v?.body || "";
  }

  static encode(m: WhisperRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    if (m.serverKey.length) w.uint32(18).bytes(m.serverKey);
    if (m.alias.length) w.uint32(26).string(m.alias);
    if (m.peerKey.length) w.uint32(34).bytes(m.peerKey);
    if (m.body.length) w.uint32(42).string(m.body);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WhisperRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new WhisperRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkKey = r.bytes();
        break;
        case 2:
        m.serverKey = r.bytes();
        break;
        case 3:
        m.alias = r.string();
        break;
        case 4:
        m.peerKey = r.bytes();
        break;
        case 5:
        m.body = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IWhisperResponse = {
}

export class WhisperResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IWhisperResponse) {
  }

  static encode(m: WhisperResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WhisperResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new WhisperResponse();
  }
}

export type IListWhispersRequest = {
  peerKey?: Uint8Array;
}

export class ListWhispersRequest {
  peerKey: Uint8Array;

  constructor(v?: IListWhispersRequest) {
    this.peerKey = v?.peerKey || new Uint8Array();
  }

  static encode(m: ListWhispersRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.peerKey.length) w.uint32(10).bytes(m.peerKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListWhispersRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ListWhispersRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
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

export type IListWhispersResponse = {
  thread?: IWhisperThread;
  whispers?: IWhisperRecord[];
}

export class ListWhispersResponse {
  thread: WhisperThread | undefined;
  whispers: WhisperRecord[];

  constructor(v?: IListWhispersResponse) {
    this.thread = v?.thread && new WhisperThread(v.thread);
    this.whispers = v?.whispers ? v.whispers.map(v => new WhisperRecord(v)) : [];
  }

  static encode(m: ListWhispersResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.thread) WhisperThread.encode(m.thread, w.uint32(10).fork()).ldelim();
    for (const v of m.whispers) WhisperRecord.encode(v, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListWhispersResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ListWhispersResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.thread = WhisperThread.decode(r, r.uint32());
        break;
        case 2:
        m.whispers.push(WhisperRecord.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IWatchWhispersRequest = {
}

export class WatchWhispersRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IWatchWhispersRequest) {
  }

  static encode(m: WatchWhispersRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WatchWhispersRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new WatchWhispersRequest();
  }
}

export type IWatchWhispersResponse = {
  peerKey?: Uint8Array;
  body?: WatchWhispersResponse.IBody
}

export class WatchWhispersResponse {
  peerKey: Uint8Array;
  body: WatchWhispersResponse.TBody;

  constructor(v?: IWatchWhispersResponse) {
    this.peerKey = v?.peerKey || new Uint8Array();
    this.body = new WatchWhispersResponse.Body(v?.body);
  }

  static encode(m: WatchWhispersResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.peerKey.length) w.uint32(10).bytes(m.peerKey);
    switch (m.body.case) {
      case WatchWhispersResponse.BodyCase.THREAD_UPDATE:
      WhisperThread.encode(m.body.threadUpdate, w.uint32(8010).fork()).ldelim();
      break;
      case WatchWhispersResponse.BodyCase.WHISPER_UPDATE:
      WhisperRecord.encode(m.body.whisperUpdate, w.uint32(8018).fork()).ldelim();
      break;
      case WatchWhispersResponse.BodyCase.WHISPER_DELETE:
      WatchWhispersResponse.WhisperDelete.encode(m.body.whisperDelete, w.uint32(8026).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WatchWhispersResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new WatchWhispersResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.peerKey = r.bytes();
        break;
        case 1001:
        m.body = new WatchWhispersResponse.Body({ threadUpdate: WhisperThread.decode(r, r.uint32()) });
        break;
        case 1002:
        m.body = new WatchWhispersResponse.Body({ whisperUpdate: WhisperRecord.decode(r, r.uint32()) });
        break;
        case 1003:
        m.body = new WatchWhispersResponse.Body({ whisperDelete: WatchWhispersResponse.WhisperDelete.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace WatchWhispersResponse {
  export enum BodyCase {
    NOT_SET = 0,
    THREAD_UPDATE = 1001,
    WHISPER_UPDATE = 1002,
    WHISPER_DELETE = 1003,
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.THREAD_UPDATE, threadUpdate: IWhisperThread }
  |{ case?: BodyCase.WHISPER_UPDATE, whisperUpdate: IWhisperRecord }
  |{ case?: BodyCase.WHISPER_DELETE, whisperDelete: WatchWhispersResponse.IWhisperDelete }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.THREAD_UPDATE, threadUpdate: WhisperThread }
  |{ case: BodyCase.WHISPER_UPDATE, whisperUpdate: WhisperRecord }
  |{ case: BodyCase.WHISPER_DELETE, whisperDelete: WatchWhispersResponse.WhisperDelete }
  >;

  class BodyImpl {
    threadUpdate: WhisperThread;
    whisperUpdate: WhisperRecord;
    whisperDelete: WatchWhispersResponse.WhisperDelete;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "threadUpdate" in v) {
        this.case = BodyCase.THREAD_UPDATE;
        this.threadUpdate = new WhisperThread(v.threadUpdate);
      } else
      if (v && "whisperUpdate" in v) {
        this.case = BodyCase.WHISPER_UPDATE;
        this.whisperUpdate = new WhisperRecord(v.whisperUpdate);
      } else
      if (v && "whisperDelete" in v) {
        this.case = BodyCase.WHISPER_DELETE;
        this.whisperDelete = new WatchWhispersResponse.WhisperDelete(v.whisperDelete);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { threadUpdate: IWhisperThread } ? { case: BodyCase.THREAD_UPDATE, threadUpdate: WhisperThread } :
    T extends { whisperUpdate: IWhisperRecord } ? { case: BodyCase.WHISPER_UPDATE, whisperUpdate: WhisperRecord } :
    T extends { whisperDelete: WatchWhispersResponse.IWhisperDelete } ? { case: BodyCase.WHISPER_DELETE, whisperDelete: WatchWhispersResponse.WhisperDelete } :
    never
    >;
  };

  export type IWhisperDelete = {
    recordId?: bigint;
    threadId?: bigint;
  }

  export class WhisperDelete {
    recordId: bigint;
    threadId: bigint;

    constructor(v?: IWhisperDelete) {
      this.recordId = v?.recordId || BigInt(0);
      this.threadId = v?.threadId || BigInt(0);
    }

    static encode(m: WhisperDelete, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.recordId) w.uint32(8).uint64(m.recordId);
      if (m.threadId) w.uint32(16).uint64(m.threadId);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): WhisperDelete {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new WhisperDelete();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.recordId = r.uint64();
          break;
          case 2:
          m.threadId = r.uint64();
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

export type ISetUIConfigRequest = {
  uiConfig?: IUIConfig;
}

export class SetUIConfigRequest {
  uiConfig: UIConfig | undefined;

  constructor(v?: ISetUIConfigRequest) {
    this.uiConfig = v?.uiConfig && new UIConfig(v.uiConfig);
  }

  static encode(m: SetUIConfigRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.uiConfig) UIConfig.encode(m.uiConfig, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SetUIConfigRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new SetUIConfigRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.uiConfig = UIConfig.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ISetUIConfigResponse = {
}

export class SetUIConfigResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: ISetUIConfigResponse) {
  }

  static encode(m: SetUIConfigResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SetUIConfigResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new SetUIConfigResponse();
  }
}

export type IWatchUIConfigRequest = {
}

export class WatchUIConfigRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IWatchUIConfigRequest) {
  }

  static encode(m: WatchUIConfigRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WatchUIConfigRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new WatchUIConfigRequest();
  }
}

export type IWatchUIConfigResponse = {
  uiConfig?: IUIConfig;
}

export class WatchUIConfigResponse {
  uiConfig: UIConfig | undefined;

  constructor(v?: IWatchUIConfigResponse) {
    this.uiConfig = v?.uiConfig && new UIConfig(v.uiConfig);
  }

  static encode(m: WatchUIConfigResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.uiConfig) UIConfig.encode(m.uiConfig, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WatchUIConfigResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new WatchUIConfigResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.uiConfig = UIConfig.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IIgnoreRequest = {
  networkKey?: Uint8Array;
  alias?: string;
  duration?: string;
}

export class IgnoreRequest {
  networkKey: Uint8Array;
  alias: string;
  duration: string;

  constructor(v?: IIgnoreRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.alias = v?.alias || "";
    this.duration = v?.duration || "";
  }

  static encode(m: IgnoreRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    if (m.alias.length) w.uint32(18).string(m.alias);
    if (m.duration.length) w.uint32(26).string(m.duration);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): IgnoreRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new IgnoreRequest();
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
        m.duration = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IIgnoreResponse = {
}

export class IgnoreResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IIgnoreResponse) {
  }

  static encode(m: IgnoreResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): IgnoreResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new IgnoreResponse();
  }
}

export type IUnignoreRequest = {
  networkKey?: Uint8Array;
  alias?: string;
  peerKey?: Uint8Array;
}

export class UnignoreRequest {
  networkKey: Uint8Array;
  alias: string;
  peerKey: Uint8Array;

  constructor(v?: IUnignoreRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.alias = v?.alias || "";
    this.peerKey = v?.peerKey || new Uint8Array();
  }

  static encode(m: UnignoreRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    if (m.alias.length) w.uint32(18).string(m.alias);
    if (m.peerKey.length) w.uint32(26).bytes(m.peerKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UnignoreRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UnignoreRequest();
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

export type IUnignoreResponse = {
}

export class UnignoreResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IUnignoreResponse) {
  }

  static encode(m: UnignoreResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UnignoreResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new UnignoreResponse();
  }
}

export type IHighlightRequest = {
  networkKey?: Uint8Array;
  alias?: string;
}

export class HighlightRequest {
  networkKey: Uint8Array;
  alias: string;

  constructor(v?: IHighlightRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.alias = v?.alias || "";
  }

  static encode(m: HighlightRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    if (m.alias.length) w.uint32(18).string(m.alias);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): HighlightRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new HighlightRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkKey = r.bytes();
        break;
        case 2:
        m.alias = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IHighlightResponse = {
}

export class HighlightResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IHighlightResponse) {
  }

  static encode(m: HighlightResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): HighlightResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new HighlightResponse();
  }
}

export type IUnhighlightRequest = {
  networkKey?: Uint8Array;
  alias?: string;
  peerKey?: Uint8Array;
}

export class UnhighlightRequest {
  networkKey: Uint8Array;
  alias: string;
  peerKey: Uint8Array;

  constructor(v?: IUnhighlightRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.alias = v?.alias || "";
    this.peerKey = v?.peerKey || new Uint8Array();
  }

  static encode(m: UnhighlightRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    if (m.alias.length) w.uint32(18).string(m.alias);
    if (m.peerKey.length) w.uint32(26).bytes(m.peerKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UnhighlightRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UnhighlightRequest();
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

export type IUnhighlightResponse = {
}

export class UnhighlightResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IUnhighlightResponse) {
  }

  static encode(m: UnhighlightResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UnhighlightResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new UnhighlightResponse();
  }
}

export type ITagRequest = {
  networkKey?: Uint8Array;
  alias?: string;
  color?: string;
}

export class TagRequest {
  networkKey: Uint8Array;
  alias: string;
  color: string;

  constructor(v?: ITagRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.alias = v?.alias || "";
    this.color = v?.color || "";
  }

  static encode(m: TagRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    if (m.alias.length) w.uint32(18).string(m.alias);
    if (m.color.length) w.uint32(26).string(m.color);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): TagRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new TagRequest();
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
        m.color = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ITagResponse = {
}

export class TagResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: ITagResponse) {
  }

  static encode(m: TagResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): TagResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new TagResponse();
  }
}

export type IUntagRequest = {
  networkKey?: Uint8Array;
  alias?: string;
  peerKey?: Uint8Array;
}

export class UntagRequest {
  networkKey: Uint8Array;
  alias: string;
  peerKey: Uint8Array;

  constructor(v?: IUntagRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.alias = v?.alias || "";
    this.peerKey = v?.peerKey || new Uint8Array();
  }

  static encode(m: UntagRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    if (m.alias.length) w.uint32(18).string(m.alias);
    if (m.peerKey.length) w.uint32(26).bytes(m.peerKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UntagRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UntagRequest();
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

export type IUntagResponse = {
}

export class UntagResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IUntagResponse) {
  }

  static encode(m: UntagResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UntagResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new UntagResponse();
  }
}

export type ISendMessageRequest = {
  body?: string;
}

export class SendMessageRequest {
  body: string;

  constructor(v?: ISendMessageRequest) {
    this.body = v?.body || "";
  }

  static encode(m: SendMessageRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.body.length) w.uint32(10).string(m.body);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SendMessageRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new SendMessageRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.body = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ISendMessageResponse = {
}

export class SendMessageResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: ISendMessageResponse) {
  }

  static encode(m: SendMessageResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SendMessageResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new SendMessageResponse();
  }
}

export type IMuteRequest = {
  peerKey?: Uint8Array;
  durationSecs?: number;
  message?: string;
}

export class MuteRequest {
  peerKey: Uint8Array;
  durationSecs: number;
  message: string;

  constructor(v?: IMuteRequest) {
    this.peerKey = v?.peerKey || new Uint8Array();
    this.durationSecs = v?.durationSecs || 0;
    this.message = v?.message || "";
  }

  static encode(m: MuteRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.peerKey.length) w.uint32(10).bytes(m.peerKey);
    if (m.durationSecs) w.uint32(16).uint32(m.durationSecs);
    if (m.message.length) w.uint32(26).string(m.message);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): MuteRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new MuteRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.peerKey = r.bytes();
        break;
        case 2:
        m.durationSecs = r.uint32();
        break;
        case 3:
        m.message = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IMuteResponse = {
}

export class MuteResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IMuteResponse) {
  }

  static encode(m: MuteResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): MuteResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new MuteResponse();
  }
}

export type IUnmuteRequest = {
  peerKey?: Uint8Array;
}

export class UnmuteRequest {
  peerKey: Uint8Array;

  constructor(v?: IUnmuteRequest) {
    this.peerKey = v?.peerKey || new Uint8Array();
  }

  static encode(m: UnmuteRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.peerKey.length) w.uint32(10).bytes(m.peerKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UnmuteRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UnmuteRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
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

export type IUnmuteResponse = {
}

export class UnmuteResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IUnmuteResponse) {
  }

  static encode(m: UnmuteResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UnmuteResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new UnmuteResponse();
  }
}

export type IGetMuteRequest = {
}

export class GetMuteRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IGetMuteRequest) {
  }

  static encode(m: GetMuteRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetMuteRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new GetMuteRequest();
  }
}

export type IGetMuteResponse = {
  endTime?: bigint;
  message?: string;
}

export class GetMuteResponse {
  endTime: bigint;
  message: string;

  constructor(v?: IGetMuteResponse) {
    this.endTime = v?.endTime || BigInt(0);
    this.message = v?.message || "";
  }

  static encode(m: GetMuteResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.endTime) w.uint32(8).int64(m.endTime);
    if (m.message.length) w.uint32(18).string(m.message);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetMuteResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetMuteResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.endTime = r.int64();
        break;
        case 2:
        m.message = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IWhisperThread = {
  id?: bigint;
  peerKey?: Uint8Array;
  alias?: string;
  unreadCount?: number;
  lastReceiveTimes?: bigint[];
  lastMessageTime?: bigint;
  lastMessageId?: bigint;
}

export class WhisperThread {
  id: bigint;
  peerKey: Uint8Array;
  alias: string;
  unreadCount: number;
  lastReceiveTimes: bigint[];
  lastMessageTime: bigint;
  lastMessageId: bigint;

  constructor(v?: IWhisperThread) {
    this.id = v?.id || BigInt(0);
    this.peerKey = v?.peerKey || new Uint8Array();
    this.alias = v?.alias || "";
    this.unreadCount = v?.unreadCount || 0;
    this.lastReceiveTimes = v?.lastReceiveTimes ? v.lastReceiveTimes : [];
    this.lastMessageTime = v?.lastMessageTime || BigInt(0);
    this.lastMessageId = v?.lastMessageId || BigInt(0);
  }

  static encode(m: WhisperThread, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.peerKey.length) w.uint32(18).bytes(m.peerKey);
    if (m.alias.length) w.uint32(26).string(m.alias);
    if (m.unreadCount) w.uint32(32).uint32(m.unreadCount);
    m.lastReceiveTimes.reduce((w, v) => w.int64(v), w.uint32(42).fork()).ldelim();
    if (m.lastMessageTime) w.uint32(48).int64(m.lastMessageTime);
    if (m.lastMessageId) w.uint32(56).uint64(m.lastMessageId);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WhisperThread {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new WhisperThread();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.peerKey = r.bytes();
        break;
        case 3:
        m.alias = r.string();
        break;
        case 4:
        m.unreadCount = r.uint32();
        break;
        case 5:
        for (const flen = r.uint32(), fend = r.pos + flen; r.pos < fend;) m.lastReceiveTimes.push(r.int64());
        break;
        case 6:
        m.lastMessageTime = r.int64();
        break;
        case 7:
        m.lastMessageId = r.uint64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IWhisperRecord = {
  id?: bigint;
  threadId?: bigint;
  networkKey?: Uint8Array;
  serverKey?: Uint8Array;
  peerKey?: Uint8Array;
  state?: WhisperRecord.State;
  message?: IMessage;
}

export class WhisperRecord {
  id: bigint;
  threadId: bigint;
  networkKey: Uint8Array;
  serverKey: Uint8Array;
  peerKey: Uint8Array;
  state: WhisperRecord.State;
  message: Message | undefined;

  constructor(v?: IWhisperRecord) {
    this.id = v?.id || BigInt(0);
    this.threadId = v?.threadId || BigInt(0);
    this.networkKey = v?.networkKey || new Uint8Array();
    this.serverKey = v?.serverKey || new Uint8Array();
    this.peerKey = v?.peerKey || new Uint8Array();
    this.state = v?.state || 0;
    this.message = v?.message && new Message(v.message);
  }

  static encode(m: WhisperRecord, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.threadId) w.uint32(16).uint64(m.threadId);
    if (m.networkKey.length) w.uint32(26).bytes(m.networkKey);
    if (m.serverKey.length) w.uint32(34).bytes(m.serverKey);
    if (m.peerKey.length) w.uint32(42).bytes(m.peerKey);
    if (m.state) w.uint32(48).uint32(m.state);
    if (m.message) Message.encode(m.message, w.uint32(58).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WhisperRecord {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new WhisperRecord();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.threadId = r.uint64();
        break;
        case 3:
        m.networkKey = r.bytes();
        break;
        case 4:
        m.serverKey = r.bytes();
        break;
        case 5:
        m.peerKey = r.bytes();
        break;
        case 6:
        m.state = r.uint32();
        break;
        case 7:
        m.message = Message.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace WhisperRecord {
  export enum State {
    WHISPER_STATE_RECEIVED = 0,
    WHISPER_STATE_ENQUEUED = 1,
    WHISPER_STATE_DELIVERED = 2,
    WHISPER_STATE_FAILED = 3,
  }
}

export type IWhisperSendMessageRequest = {
  serverKey?: Uint8Array;
  body?: string;
}

export class WhisperSendMessageRequest {
  serverKey: Uint8Array;
  body: string;

  constructor(v?: IWhisperSendMessageRequest) {
    this.serverKey = v?.serverKey || new Uint8Array();
    this.body = v?.body || "";
  }

  static encode(m: WhisperSendMessageRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverKey.length) w.uint32(10).bytes(m.serverKey);
    if (m.body.length) w.uint32(18).string(m.body);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WhisperSendMessageRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new WhisperSendMessageRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverKey = r.bytes();
        break;
        case 2:
        m.body = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IWhisperSendMessageResponse = {
}

export class WhisperSendMessageResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IWhisperSendMessageResponse) {
  }

  static encode(m: WhisperSendMessageResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WhisperSendMessageResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new WhisperSendMessageResponse();
  }
}

export enum EmoteFileType {
  FILE_TYPE_UNDEFINED = 0,
  FILE_TYPE_PNG = 1,
  FILE_TYPE_GIF = 2,
}
export enum EmoteScale {
  EMOTE_SCALE_1X = 0,
  EMOTE_SCALE_2X = 1,
  EMOTE_SCALE_4X = 2,
}

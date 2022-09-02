import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  strims_type_Key,
  strims_type_IKey,
} from "../../type/key";
import {
  strims_type_Image,
  strims_type_IImage,
} from "../../type/image";
import {
  strims_network_v1_directory_Listing,
  strims_network_v1_directory_IListing,
} from "../../network/v1/directory/directory";

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
      strims_chat_v1_Message.encode(m.body.message, w.uint32(8010).fork()).ldelim();
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
        m.body = new ServerEvent.Body({ message: strims_chat_v1_Message.decode(r, r.uint32()) });
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
  |{ case?: BodyCase.MESSAGE, message: strims_chat_v1_IMessage }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.MESSAGE, message: strims_chat_v1_Message }
  >;

  class BodyImpl {
    message: strims_chat_v1_Message;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "message" in v) {
        this.case = BodyCase.MESSAGE;
        this.message = new strims_chat_v1_Message(v.message);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { message: strims_chat_v1_IMessage } ? { case: BodyCase.MESSAGE, message: strims_chat_v1_Message } :
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
  room?: strims_chat_v1_IRoom;
  adminPeerKeys?: Uint8Array[];
}

export class Server {
  id: bigint;
  networkKey: Uint8Array;
  key: strims_type_Key | undefined;
  room: strims_chat_v1_Room | undefined;
  adminPeerKeys: Uint8Array[];

  constructor(v?: IServer) {
    this.id = v?.id || BigInt(0);
    this.networkKey = v?.networkKey || new Uint8Array();
    this.key = v?.key && new strims_type_Key(v.key);
    this.room = v?.room && new strims_chat_v1_Room(v.room);
    this.adminPeerKeys = v?.adminPeerKeys ? v.adminPeerKeys : [];
  }

  static encode(m: Server, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.networkKey.length) w.uint32(18).bytes(m.networkKey);
    if (m.key) strims_type_Key.encode(m.key, w.uint32(26).fork()).ldelim();
    if (m.room) strims_chat_v1_Room.encode(m.room, w.uint32(34).fork()).ldelim();
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
        m.room = strims_chat_v1_Room.decode(r, r.uint32());
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

export type IServerIcon = {
  id?: bigint;
  serverId?: bigint;
  image?: strims_type_IImage;
}

export class ServerIcon {
  id: bigint;
  serverId: bigint;
  image: strims_type_Image | undefined;

  constructor(v?: IServerIcon) {
    this.id = v?.id || BigInt(0);
    this.serverId = v?.serverId || BigInt(0);
    this.image = v?.image && new strims_type_Image(v.image);
  }

  static encode(m: ServerIcon, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.serverId) w.uint32(16).uint64(m.serverId);
    if (m.image) strims_type_Image.encode(m.image, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ServerIcon {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ServerIcon();
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
        m.image = strims_type_Image.decode(r, r.uint32());
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
  fileType?: strims_chat_v1_EmoteFileType;
  height?: number;
  width?: number;
  scale?: strims_chat_v1_EmoteScale;
}

export class EmoteImage {
  data: Uint8Array;
  fileType: strims_chat_v1_EmoteFileType;
  height: number;
  width: number;
  scale: strims_chat_v1_EmoteScale;

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
      strims_chat_v1_EmoteEffect_CustomCSS.encode(m.effect.customCss, w.uint32(8010).fork()).ldelim();
      break;
      case EmoteEffect.EffectCase.SPRITE_ANIMATION:
      strims_chat_v1_EmoteEffect_SpriteAnimation.encode(m.effect.spriteAnimation, w.uint32(8018).fork()).ldelim();
      break;
      case EmoteEffect.EffectCase.DEFAULT_MODIFIERS:
      strims_chat_v1_EmoteEffect_DefaultModifiers.encode(m.effect.defaultModifiers, w.uint32(8026).fork()).ldelim();
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
        m.effect = new EmoteEffect.Effect({ customCss: strims_chat_v1_EmoteEffect_CustomCSS.decode(r, r.uint32()) });
        break;
        case 1002:
        m.effect = new EmoteEffect.Effect({ spriteAnimation: strims_chat_v1_EmoteEffect_SpriteAnimation.decode(r, r.uint32()) });
        break;
        case 1003:
        m.effect = new EmoteEffect.Effect({ defaultModifiers: strims_chat_v1_EmoteEffect_DefaultModifiers.decode(r, r.uint32()) });
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
  |{ case?: EffectCase.CUSTOM_CSS, customCss: strims_chat_v1_EmoteEffect_ICustomCSS }
  |{ case?: EffectCase.SPRITE_ANIMATION, spriteAnimation: strims_chat_v1_EmoteEffect_ISpriteAnimation }
  |{ case?: EffectCase.DEFAULT_MODIFIERS, defaultModifiers: strims_chat_v1_EmoteEffect_IDefaultModifiers }
  ;

  export type TEffect = Readonly<
  { case: EffectCase.NOT_SET }
  |{ case: EffectCase.CUSTOM_CSS, customCss: strims_chat_v1_EmoteEffect_CustomCSS }
  |{ case: EffectCase.SPRITE_ANIMATION, spriteAnimation: strims_chat_v1_EmoteEffect_SpriteAnimation }
  |{ case: EffectCase.DEFAULT_MODIFIERS, defaultModifiers: strims_chat_v1_EmoteEffect_DefaultModifiers }
  >;

  class EffectImpl {
    customCss: strims_chat_v1_EmoteEffect_CustomCSS;
    spriteAnimation: strims_chat_v1_EmoteEffect_SpriteAnimation;
    defaultModifiers: strims_chat_v1_EmoteEffect_DefaultModifiers;
    case: EffectCase = EffectCase.NOT_SET;

    constructor(v?: IEffect) {
      if (v && "customCss" in v) {
        this.case = EffectCase.CUSTOM_CSS;
        this.customCss = new strims_chat_v1_EmoteEffect_CustomCSS(v.customCss);
      } else
      if (v && "spriteAnimation" in v) {
        this.case = EffectCase.SPRITE_ANIMATION;
        this.spriteAnimation = new strims_chat_v1_EmoteEffect_SpriteAnimation(v.spriteAnimation);
      } else
      if (v && "defaultModifiers" in v) {
        this.case = EffectCase.DEFAULT_MODIFIERS;
        this.defaultModifiers = new strims_chat_v1_EmoteEffect_DefaultModifiers(v.defaultModifiers);
      }
    }
  }

  export const Effect = EffectImpl as {
    new (): Readonly<{ case: EffectCase.NOT_SET }>;
    new <T extends IEffect>(v: T): Readonly<
    T extends { customCss: strims_chat_v1_EmoteEffect_ICustomCSS } ? { case: EffectCase.CUSTOM_CSS, customCss: strims_chat_v1_EmoteEffect_CustomCSS } :
    T extends { spriteAnimation: strims_chat_v1_EmoteEffect_ISpriteAnimation } ? { case: EffectCase.SPRITE_ANIMATION, spriteAnimation: strims_chat_v1_EmoteEffect_SpriteAnimation } :
    T extends { defaultModifiers: strims_chat_v1_EmoteEffect_IDefaultModifiers } ? { case: EffectCase.DEFAULT_MODIFIERS, defaultModifiers: strims_chat_v1_EmoteEffect_DefaultModifiers } :
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
  images?: strims_chat_v1_IEmoteImage[];
  effects?: strims_chat_v1_IEmoteEffect[];
  contributor?: strims_chat_v1_IEmoteContributor;
}

export class Emote {
  id: bigint;
  serverId: bigint;
  name: string;
  images: strims_chat_v1_EmoteImage[];
  effects: strims_chat_v1_EmoteEffect[];
  contributor: strims_chat_v1_EmoteContributor | undefined;

  constructor(v?: IEmote) {
    this.id = v?.id || BigInt(0);
    this.serverId = v?.serverId || BigInt(0);
    this.name = v?.name || "";
    this.images = v?.images ? v.images.map(v => new strims_chat_v1_EmoteImage(v)) : [];
    this.effects = v?.effects ? v.effects.map(v => new strims_chat_v1_EmoteEffect(v)) : [];
    this.contributor = v?.contributor && new strims_chat_v1_EmoteContributor(v.contributor);
  }

  static encode(m: Emote, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.serverId) w.uint32(16).uint64(m.serverId);
    if (m.name.length) w.uint32(26).string(m.name);
    for (const v of m.images) strims_chat_v1_EmoteImage.encode(v, w.uint32(34).fork()).ldelim();
    for (const v of m.effects) strims_chat_v1_EmoteEffect.encode(v, w.uint32(42).fork()).ldelim();
    if (m.contributor) strims_chat_v1_EmoteContributor.encode(m.contributor, w.uint32(50).fork()).ldelim();
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
        m.images.push(strims_chat_v1_EmoteImage.decode(r, r.uint32()));
        break;
        case 5:
        m.effects.push(strims_chat_v1_EmoteEffect.decode(r, r.uint32()));
        break;
        case 6:
        m.contributor = strims_chat_v1_EmoteContributor.decode(r, r.uint32());
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
  procChance?: number;
}

export class Modifier {
  id: bigint;
  serverId: bigint;
  name: string;
  priority: number;
  internal: boolean;
  extraWrapCount: number;
  procChance: number;

  constructor(v?: IModifier) {
    this.id = v?.id || BigInt(0);
    this.serverId = v?.serverId || BigInt(0);
    this.name = v?.name || "";
    this.priority = v?.priority || 0;
    this.internal = v?.internal || false;
    this.extraWrapCount = v?.extraWrapCount || 0;
    this.procChance = v?.procChance || 0;
  }

  static encode(m: Modifier, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.serverId) w.uint32(16).uint64(m.serverId);
    if (m.name.length) w.uint32(26).string(m.name);
    if (m.priority) w.uint32(32).uint32(m.priority);
    if (m.internal) w.uint32(40).bool(m.internal);
    if (m.extraWrapCount) w.uint32(48).uint32(m.extraWrapCount);
    if (m.procChance) w.uint32(57).double(m.procChance);
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
        case 7:
        m.procChance = r.double();
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
  room?: strims_chat_v1_IRoom;
  emotes?: strims_chat_v1_IEmote[];
  modifiers?: strims_chat_v1_IModifier[];
  tags?: strims_chat_v1_ITag[];
  icon?: strims_type_IImage;
}

export class AssetBundle {
  isDelta: boolean;
  removedIds: bigint[];
  room: strims_chat_v1_Room | undefined;
  emotes: strims_chat_v1_Emote[];
  modifiers: strims_chat_v1_Modifier[];
  tags: strims_chat_v1_Tag[];
  icon: strims_type_Image | undefined;

  constructor(v?: IAssetBundle) {
    this.isDelta = v?.isDelta || false;
    this.removedIds = v?.removedIds ? v.removedIds : [];
    this.room = v?.room && new strims_chat_v1_Room(v.room);
    this.emotes = v?.emotes ? v.emotes.map(v => new strims_chat_v1_Emote(v)) : [];
    this.modifiers = v?.modifiers ? v.modifiers.map(v => new strims_chat_v1_Modifier(v)) : [];
    this.tags = v?.tags ? v.tags.map(v => new strims_chat_v1_Tag(v)) : [];
    this.icon = v?.icon && new strims_type_Image(v.icon);
  }

  static encode(m: AssetBundle, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.isDelta) w.uint32(8).bool(m.isDelta);
    m.removedIds.reduce((w, v) => w.uint64(v), w.uint32(18).fork()).ldelim();
    if (m.room) strims_chat_v1_Room.encode(m.room, w.uint32(26).fork()).ldelim();
    for (const v of m.emotes) strims_chat_v1_Emote.encode(v, w.uint32(34).fork()).ldelim();
    for (const v of m.modifiers) strims_chat_v1_Modifier.encode(v, w.uint32(42).fork()).ldelim();
    for (const v of m.tags) strims_chat_v1_Tag.encode(v, w.uint32(50).fork()).ldelim();
    if (m.icon) strims_type_Image.encode(m.icon, w.uint32(58).fork()).ldelim();
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
        m.room = strims_chat_v1_Room.decode(r, r.uint32());
        break;
        case 4:
        m.emotes.push(strims_chat_v1_Emote.decode(r, r.uint32()));
        break;
        case 5:
        m.modifiers.push(strims_chat_v1_Modifier.decode(r, r.uint32()));
        break;
        case 6:
        m.tags.push(strims_chat_v1_Tag.decode(r, r.uint32()));
        break;
        case 7:
        m.icon = strims_type_Image.decode(r, r.uint32());
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
  entities?: strims_chat_v1_Message_IEntities;
  viewedListing?: strims_chat_v1_Message_IDirectoryRef;
}

export class Message {
  serverTime: bigint;
  peerKey: Uint8Array;
  nick: string;
  body: string;
  entities: strims_chat_v1_Message_Entities | undefined;
  viewedListing: strims_chat_v1_Message_DirectoryRef | undefined;

  constructor(v?: IMessage) {
    this.serverTime = v?.serverTime || BigInt(0);
    this.peerKey = v?.peerKey || new Uint8Array();
    this.nick = v?.nick || "";
    this.body = v?.body || "";
    this.entities = v?.entities && new strims_chat_v1_Message_Entities(v.entities);
    this.viewedListing = v?.viewedListing && new strims_chat_v1_Message_DirectoryRef(v.viewedListing);
  }

  static encode(m: Message, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverTime) w.uint32(8).int64(m.serverTime);
    if (m.peerKey.length) w.uint32(18).bytes(m.peerKey);
    if (m.nick.length) w.uint32(26).string(m.nick);
    if (m.body.length) w.uint32(34).string(m.body);
    if (m.entities) strims_chat_v1_Message_Entities.encode(m.entities, w.uint32(42).fork()).ldelim();
    if (m.viewedListing) strims_chat_v1_Message_DirectoryRef.encode(m.viewedListing, w.uint32(74).fork()).ldelim();
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
        m.entities = strims_chat_v1_Message_Entities.decode(r, r.uint32());
        break;
        case 9:
        m.viewedListing = strims_chat_v1_Message_DirectoryRef.decode(r, r.uint32());
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
    links?: strims_chat_v1_Message_Entities_ILink[];
    emotes?: strims_chat_v1_Message_Entities_IEmote[];
    emojis?: strims_chat_v1_Message_Entities_IEmoji[];
    nicks?: strims_chat_v1_Message_Entities_INick[];
    tags?: strims_chat_v1_Message_Entities_ITag[];
    codeBlocks?: strims_chat_v1_Message_Entities_ICodeBlock[];
    spoilers?: strims_chat_v1_Message_Entities_ISpoiler[];
    greenText?: strims_chat_v1_Message_Entities_IGenericEntity;
    selfMessage?: strims_chat_v1_Message_Entities_IGenericEntity;
  }

  export class Entities {
    links: strims_chat_v1_Message_Entities_Link[];
    emotes: strims_chat_v1_Message_Entities_Emote[];
    emojis: strims_chat_v1_Message_Entities_Emoji[];
    nicks: strims_chat_v1_Message_Entities_Nick[];
    tags: strims_chat_v1_Message_Entities_Tag[];
    codeBlocks: strims_chat_v1_Message_Entities_CodeBlock[];
    spoilers: strims_chat_v1_Message_Entities_Spoiler[];
    greenText: strims_chat_v1_Message_Entities_GenericEntity | undefined;
    selfMessage: strims_chat_v1_Message_Entities_GenericEntity | undefined;

    constructor(v?: IEntities) {
      this.links = v?.links ? v.links.map(v => new strims_chat_v1_Message_Entities_Link(v)) : [];
      this.emotes = v?.emotes ? v.emotes.map(v => new strims_chat_v1_Message_Entities_Emote(v)) : [];
      this.emojis = v?.emojis ? v.emojis.map(v => new strims_chat_v1_Message_Entities_Emoji(v)) : [];
      this.nicks = v?.nicks ? v.nicks.map(v => new strims_chat_v1_Message_Entities_Nick(v)) : [];
      this.tags = v?.tags ? v.tags.map(v => new strims_chat_v1_Message_Entities_Tag(v)) : [];
      this.codeBlocks = v?.codeBlocks ? v.codeBlocks.map(v => new strims_chat_v1_Message_Entities_CodeBlock(v)) : [];
      this.spoilers = v?.spoilers ? v.spoilers.map(v => new strims_chat_v1_Message_Entities_Spoiler(v)) : [];
      this.greenText = v?.greenText && new strims_chat_v1_Message_Entities_GenericEntity(v.greenText);
      this.selfMessage = v?.selfMessage && new strims_chat_v1_Message_Entities_GenericEntity(v.selfMessage);
    }

    static encode(m: Entities, w?: Writer): Writer {
      if (!w) w = new Writer();
      for (const v of m.links) strims_chat_v1_Message_Entities_Link.encode(v, w.uint32(10).fork()).ldelim();
      for (const v of m.emotes) strims_chat_v1_Message_Entities_Emote.encode(v, w.uint32(18).fork()).ldelim();
      for (const v of m.emojis) strims_chat_v1_Message_Entities_Emoji.encode(v, w.uint32(26).fork()).ldelim();
      for (const v of m.nicks) strims_chat_v1_Message_Entities_Nick.encode(v, w.uint32(34).fork()).ldelim();
      for (const v of m.tags) strims_chat_v1_Message_Entities_Tag.encode(v, w.uint32(42).fork()).ldelim();
      for (const v of m.codeBlocks) strims_chat_v1_Message_Entities_CodeBlock.encode(v, w.uint32(50).fork()).ldelim();
      for (const v of m.spoilers) strims_chat_v1_Message_Entities_Spoiler.encode(v, w.uint32(58).fork()).ldelim();
      if (m.greenText) strims_chat_v1_Message_Entities_GenericEntity.encode(m.greenText, w.uint32(66).fork()).ldelim();
      if (m.selfMessage) strims_chat_v1_Message_Entities_GenericEntity.encode(m.selfMessage, w.uint32(74).fork()).ldelim();
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
          m.links.push(strims_chat_v1_Message_Entities_Link.decode(r, r.uint32()));
          break;
          case 2:
          m.emotes.push(strims_chat_v1_Message_Entities_Emote.decode(r, r.uint32()));
          break;
          case 3:
          m.emojis.push(strims_chat_v1_Message_Entities_Emoji.decode(r, r.uint32()));
          break;
          case 4:
          m.nicks.push(strims_chat_v1_Message_Entities_Nick.decode(r, r.uint32()));
          break;
          case 5:
          m.tags.push(strims_chat_v1_Message_Entities_Tag.decode(r, r.uint32()));
          break;
          case 6:
          m.codeBlocks.push(strims_chat_v1_Message_Entities_CodeBlock.decode(r, r.uint32()));
          break;
          case 7:
          m.spoilers.push(strims_chat_v1_Message_Entities_Spoiler.decode(r, r.uint32()));
          break;
          case 8:
          m.greenText = strims_chat_v1_Message_Entities_GenericEntity.decode(r, r.uint32());
          break;
          case 9:
          m.selfMessage = strims_chat_v1_Message_Entities_GenericEntity.decode(r, r.uint32());
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
      bounds?: strims_chat_v1_Message_Entities_IBounds;
      url?: string;
    }

    export class Link {
      bounds: strims_chat_v1_Message_Entities_Bounds | undefined;
      url: string;

      constructor(v?: ILink) {
        this.bounds = v?.bounds && new strims_chat_v1_Message_Entities_Bounds(v.bounds);
        this.url = v?.url || "";
      }

      static encode(m: Link, w?: Writer): Writer {
        if (!w) w = new Writer();
        if (m.bounds) strims_chat_v1_Message_Entities_Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
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
            m.bounds = strims_chat_v1_Message_Entities_Bounds.decode(r, r.uint32());
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
      bounds?: strims_chat_v1_Message_Entities_IBounds;
      name?: string;
      modifiers?: string[];
      combo?: number;
      canCombo?: boolean;
    }

    export class Emote {
      bounds: strims_chat_v1_Message_Entities_Bounds | undefined;
      name: string;
      modifiers: string[];
      combo: number;
      canCombo: boolean;

      constructor(v?: IEmote) {
        this.bounds = v?.bounds && new strims_chat_v1_Message_Entities_Bounds(v.bounds);
        this.name = v?.name || "";
        this.modifiers = v?.modifiers ? v.modifiers : [];
        this.combo = v?.combo || 0;
        this.canCombo = v?.canCombo || false;
      }

      static encode(m: Emote, w?: Writer): Writer {
        if (!w) w = new Writer();
        if (m.bounds) strims_chat_v1_Message_Entities_Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
        if (m.name.length) w.uint32(18).string(m.name);
        for (const v of m.modifiers) w.uint32(26).string(v);
        if (m.combo) w.uint32(32).uint32(m.combo);
        if (m.canCombo) w.uint32(40).bool(m.canCombo);
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
            m.bounds = strims_chat_v1_Message_Entities_Bounds.decode(r, r.uint32());
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
            case 5:
            m.canCombo = r.bool();
            break;
            default:
            r.skipType(tag & 7);
            break;
          }
        }
        return m;
      }
    }

    export type IEmoji = {
      bounds?: strims_chat_v1_Message_Entities_IBounds;
    }

    export class Emoji {
      bounds: strims_chat_v1_Message_Entities_Bounds | undefined;

      constructor(v?: IEmoji) {
        this.bounds = v?.bounds && new strims_chat_v1_Message_Entities_Bounds(v.bounds);
      }

      static encode(m: Emoji, w?: Writer): Writer {
        if (!w) w = new Writer();
        if (m.bounds) strims_chat_v1_Message_Entities_Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
        return w;
      }

      static decode(r: Reader | Uint8Array, length?: number): Emoji {
        r = r instanceof Reader ? r : new Reader(r);
        const end = length === undefined ? r.len : r.pos + length;
        const m = new Emoji();
        while (r.pos < end) {
          const tag = r.uint32();
          switch (tag >> 3) {
            case 1:
            m.bounds = strims_chat_v1_Message_Entities_Bounds.decode(r, r.uint32());
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
      bounds?: strims_chat_v1_Message_Entities_IBounds;
      nick?: string;
      peerKey?: Uint8Array;
      viewedListing?: strims_chat_v1_Message_IDirectoryRef;
    }

    export class Nick {
      bounds: strims_chat_v1_Message_Entities_Bounds | undefined;
      nick: string;
      peerKey: Uint8Array;
      viewedListing: strims_chat_v1_Message_DirectoryRef | undefined;

      constructor(v?: INick) {
        this.bounds = v?.bounds && new strims_chat_v1_Message_Entities_Bounds(v.bounds);
        this.nick = v?.nick || "";
        this.peerKey = v?.peerKey || new Uint8Array();
        this.viewedListing = v?.viewedListing && new strims_chat_v1_Message_DirectoryRef(v.viewedListing);
      }

      static encode(m: Nick, w?: Writer): Writer {
        if (!w) w = new Writer();
        if (m.bounds) strims_chat_v1_Message_Entities_Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
        if (m.nick.length) w.uint32(18).string(m.nick);
        if (m.peerKey.length) w.uint32(26).bytes(m.peerKey);
        if (m.viewedListing) strims_chat_v1_Message_DirectoryRef.encode(m.viewedListing, w.uint32(34).fork()).ldelim();
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
            m.bounds = strims_chat_v1_Message_Entities_Bounds.decode(r, r.uint32());
            break;
            case 2:
            m.nick = r.string();
            break;
            case 3:
            m.peerKey = r.bytes();
            break;
            case 4:
            m.viewedListing = strims_chat_v1_Message_DirectoryRef.decode(r, r.uint32());
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
      bounds?: strims_chat_v1_Message_Entities_IBounds;
      name?: string;
    }

    export class Tag {
      bounds: strims_chat_v1_Message_Entities_Bounds | undefined;
      name: string;

      constructor(v?: ITag) {
        this.bounds = v?.bounds && new strims_chat_v1_Message_Entities_Bounds(v.bounds);
        this.name = v?.name || "";
      }

      static encode(m: Tag, w?: Writer): Writer {
        if (!w) w = new Writer();
        if (m.bounds) strims_chat_v1_Message_Entities_Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
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
            m.bounds = strims_chat_v1_Message_Entities_Bounds.decode(r, r.uint32());
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
      bounds?: strims_chat_v1_Message_Entities_IBounds;
    }

    export class CodeBlock {
      bounds: strims_chat_v1_Message_Entities_Bounds | undefined;

      constructor(v?: ICodeBlock) {
        this.bounds = v?.bounds && new strims_chat_v1_Message_Entities_Bounds(v.bounds);
      }

      static encode(m: CodeBlock, w?: Writer): Writer {
        if (!w) w = new Writer();
        if (m.bounds) strims_chat_v1_Message_Entities_Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
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
            m.bounds = strims_chat_v1_Message_Entities_Bounds.decode(r, r.uint32());
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
      bounds?: strims_chat_v1_Message_Entities_IBounds;
    }

    export class Spoiler {
      bounds: strims_chat_v1_Message_Entities_Bounds | undefined;

      constructor(v?: ISpoiler) {
        this.bounds = v?.bounds && new strims_chat_v1_Message_Entities_Bounds(v.bounds);
      }

      static encode(m: Spoiler, w?: Writer): Writer {
        if (!w) w = new Writer();
        if (m.bounds) strims_chat_v1_Message_Entities_Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
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
            m.bounds = strims_chat_v1_Message_Entities_Bounds.decode(r, r.uint32());
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
      bounds?: strims_chat_v1_Message_Entities_IBounds;
    }

    export class GenericEntity {
      bounds: strims_chat_v1_Message_Entities_Bounds | undefined;

      constructor(v?: IGenericEntity) {
        this.bounds = v?.bounds && new strims_chat_v1_Message_Entities_Bounds(v.bounds);
      }

      static encode(m: GenericEntity, w?: Writer): Writer {
        if (!w) w = new Writer();
        if (m.bounds) strims_chat_v1_Message_Entities_Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
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
            m.bounds = strims_chat_v1_Message_Entities_Bounds.decode(r, r.uint32());
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

  export type IDirectoryRef = {
    directoryId?: bigint;
    networkKey?: Uint8Array;
    listing?: strims_network_v1_directory_IListing;
    themeColor?: number;
  }

  export class DirectoryRef {
    directoryId: bigint;
    networkKey: Uint8Array;
    listing: strims_network_v1_directory_Listing | undefined;
    themeColor: number;

    constructor(v?: IDirectoryRef) {
      this.directoryId = v?.directoryId || BigInt(0);
      this.networkKey = v?.networkKey || new Uint8Array();
      this.listing = v?.listing && new strims_network_v1_directory_Listing(v.listing);
      this.themeColor = v?.themeColor || 0;
    }

    static encode(m: DirectoryRef, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.directoryId) w.uint32(8).uint64(m.directoryId);
      if (m.networkKey.length) w.uint32(18).bytes(m.networkKey);
      if (m.listing) strims_network_v1_directory_Listing.encode(m.listing, w.uint32(26).fork()).ldelim();
      if (m.themeColor) w.uint32(32).uint32(m.themeColor);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): DirectoryRef {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new DirectoryRef();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.directoryId = r.uint64();
          break;
          case 2:
          m.networkKey = r.bytes();
          break;
          case 3:
          m.listing = strims_network_v1_directory_Listing.decode(r, r.uint32());
          break;
          case 4:
          m.themeColor = r.uint32();
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

export type IProfile = {
  id?: bigint;
  serverId?: bigint;
  peerKey?: Uint8Array;
  alias?: string;
  mutes?: strims_chat_v1_Profile_IMute[];
  muteDeadline?: bigint;
}

export class Profile {
  id: bigint;
  serverId: bigint;
  peerKey: Uint8Array;
  alias: string;
  mutes: strims_chat_v1_Profile_Mute[];
  muteDeadline: bigint;

  constructor(v?: IProfile) {
    this.id = v?.id || BigInt(0);
    this.serverId = v?.serverId || BigInt(0);
    this.peerKey = v?.peerKey || new Uint8Array();
    this.alias = v?.alias || "";
    this.mutes = v?.mutes ? v.mutes.map(v => new strims_chat_v1_Profile_Mute(v)) : [];
    this.muteDeadline = v?.muteDeadline || BigInt(0);
  }

  static encode(m: Profile, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.serverId) w.uint32(16).uint64(m.serverId);
    if (m.peerKey.length) w.uint32(26).bytes(m.peerKey);
    if (m.alias.length) w.uint32(34).string(m.alias);
    for (const v of m.mutes) strims_chat_v1_Profile_Mute.encode(v, w.uint32(42).fork()).ldelim();
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
        m.mutes.push(strims_chat_v1_Profile_Mute.decode(r, r.uint32()));
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
  notificationSoundFile?: strims_chat_v1_UIConfig_ISoundFile;
  highlight?: boolean;
  customHighlight?: string[];
  showRemoved?: strims_chat_v1_UIConfig_ShowRemoved;
  showWhispersInChat?: boolean;
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
  userPresenceIndicator?: strims_chat_v1_UIConfig_UserPresenceIndicator;
  hiddenEmotes?: string[];
  shortenLinks?: boolean;
  compactEmoteSpacing?: boolean;
  normalizeAliasCase?: boolean;
  emojiSkinTone?: string;
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
  notificationSoundFile: strims_chat_v1_UIConfig_SoundFile | undefined;
  highlight: boolean;
  customHighlight: string[];
  showRemoved: strims_chat_v1_UIConfig_ShowRemoved;
  showWhispersInChat: boolean;
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
  userPresenceIndicator: strims_chat_v1_UIConfig_UserPresenceIndicator;
  hiddenEmotes: string[];
  shortenLinks: boolean;
  compactEmoteSpacing: boolean;
  normalizeAliasCase: boolean;
  emojiSkinTone: string;

  constructor(v?: IUIConfig) {
    this.showTime = v?.showTime || false;
    this.showFlairIcons = v?.showFlairIcons || false;
    this.timestampFormat = v?.timestampFormat || "";
    this.maxLines = v?.maxLines || 0;
    this.notificationWhisper = v?.notificationWhisper || false;
    this.soundNotificationWhisper = v?.soundNotificationWhisper || false;
    this.notificationHighlight = v?.notificationHighlight || false;
    this.soundNotificationHighlight = v?.soundNotificationHighlight || false;
    this.notificationSoundFile = v?.notificationSoundFile && new strims_chat_v1_UIConfig_SoundFile(v.notificationSoundFile);
    this.highlight = v?.highlight || false;
    this.customHighlight = v?.customHighlight ? v.customHighlight : [];
    this.showRemoved = v?.showRemoved || 0;
    this.showWhispersInChat = v?.showWhispersInChat || false;
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
    this.userPresenceIndicator = v?.userPresenceIndicator || 0;
    this.hiddenEmotes = v?.hiddenEmotes ? v.hiddenEmotes : [];
    this.shortenLinks = v?.shortenLinks || false;
    this.compactEmoteSpacing = v?.compactEmoteSpacing || false;
    this.normalizeAliasCase = v?.normalizeAliasCase || false;
    this.emojiSkinTone = v?.emojiSkinTone || "";
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
    if (m.notificationSoundFile) strims_chat_v1_UIConfig_SoundFile.encode(m.notificationSoundFile, w.uint32(74).fork()).ldelim();
    if (m.highlight) w.uint32(80).bool(m.highlight);
    for (const v of m.customHighlight) w.uint32(90).string(v);
    if (m.showRemoved) w.uint32(112).uint32(m.showRemoved);
    if (m.showWhispersInChat) w.uint32(120).bool(m.showWhispersInChat);
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
    if (m.userPresenceIndicator) w.uint32(240).uint32(m.userPresenceIndicator);
    for (const v of m.hiddenEmotes) w.uint32(250).string(v);
    if (m.shortenLinks) w.uint32(256).bool(m.shortenLinks);
    if (m.compactEmoteSpacing) w.uint32(264).bool(m.compactEmoteSpacing);
    if (m.normalizeAliasCase) w.uint32(272).bool(m.normalizeAliasCase);
    if (m.emojiSkinTone.length) w.uint32(282).string(m.emojiSkinTone);
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
        m.notificationSoundFile = strims_chat_v1_UIConfig_SoundFile.decode(r, r.uint32());
        break;
        case 10:
        m.highlight = r.bool();
        break;
        case 11:
        m.customHighlight.push(r.string())
        break;
        case 14:
        m.showRemoved = r.uint32();
        break;
        case 15:
        m.showWhispersInChat = r.bool();
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
        m.userPresenceIndicator = r.uint32();
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
        case 35:
        m.emojiSkinTone = r.string();
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

  export enum ShowRemoved {
    SHOW_REMOVED_REMOVE = 0,
    SHOW_REMOVED_CENSOR = 1,
    SHOW_REMOVED_DO_NOTHING = 2,
  }
  export enum UserPresenceIndicator {
    USER_PRESENCE_INDICATOR_DISABLED = 0,
    USER_PRESENCE_INDICATOR_BAR = 1,
    USER_PRESENCE_INDICATOR_DOT = 2,
    USER_PRESENCE_INDICATOR_ARRAY = 3,
  }
}

export type IUIConfigHighlight = {
  id?: bigint;
  alias?: string;
  peerKey?: Uint8Array;
}

export class UIConfigHighlight {
  id: bigint;
  alias: string;
  peerKey: Uint8Array;

  constructor(v?: IUIConfigHighlight) {
    this.id = v?.id || BigInt(0);
    this.alias = v?.alias || "";
    this.peerKey = v?.peerKey || new Uint8Array();
  }

  static encode(m: UIConfigHighlight, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.alias.length) w.uint32(18).string(m.alias);
    if (m.peerKey.length) w.uint32(26).bytes(m.peerKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UIConfigHighlight {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UIConfigHighlight();
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

export type IUIConfigTag = {
  id?: bigint;
  alias?: string;
  peerKey?: Uint8Array;
  color?: string;
}

export class UIConfigTag {
  id: bigint;
  alias: string;
  peerKey: Uint8Array;
  color: string;

  constructor(v?: IUIConfigTag) {
    this.id = v?.id || BigInt(0);
    this.alias = v?.alias || "";
    this.peerKey = v?.peerKey || new Uint8Array();
    this.color = v?.color || "";
  }

  static encode(m: UIConfigTag, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.alias.length) w.uint32(18).string(m.alias);
    if (m.peerKey.length) w.uint32(26).bytes(m.peerKey);
    if (m.color.length) w.uint32(34).string(m.color);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UIConfigTag {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UIConfigTag();
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

export type IUIConfigIgnore = {
  id?: bigint;
  alias?: string;
  peerKey?: Uint8Array;
  deadline?: bigint;
}

export class UIConfigIgnore {
  id: bigint;
  alias: string;
  peerKey: Uint8Array;
  deadline: bigint;

  constructor(v?: IUIConfigIgnore) {
    this.id = v?.id || BigInt(0);
    this.alias = v?.alias || "";
    this.peerKey = v?.peerKey || new Uint8Array();
    this.deadline = v?.deadline || BigInt(0);
  }

  static encode(m: UIConfigIgnore, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.alias.length) w.uint32(18).string(m.alias);
    if (m.peerKey.length) w.uint32(26).bytes(m.peerKey);
    if (m.deadline) w.uint32(32).int64(m.deadline);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UIConfigIgnore {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UIConfigIgnore();
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

export type ICreateServerRequest = {
  networkKey?: Uint8Array;
  room?: strims_chat_v1_IRoom;
}

export class CreateServerRequest {
  networkKey: Uint8Array;
  room: strims_chat_v1_Room | undefined;

  constructor(v?: ICreateServerRequest) {
    this.networkKey = v?.networkKey || new Uint8Array();
    this.room = v?.room && new strims_chat_v1_Room(v.room);
  }

  static encode(m: CreateServerRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    if (m.room) strims_chat_v1_Room.encode(m.room, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateServerRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new CreateServerRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkKey = r.bytes();
        break;
        case 2:
        m.room = strims_chat_v1_Room.decode(r, r.uint32());
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
  server?: strims_chat_v1_IServer;
}

export class CreateServerResponse {
  server: strims_chat_v1_Server | undefined;

  constructor(v?: ICreateServerResponse) {
    this.server = v?.server && new strims_chat_v1_Server(v.server);
  }

  static encode(m: CreateServerResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.server) strims_chat_v1_Server.encode(m.server, w.uint32(10).fork()).ldelim();
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
        m.server = strims_chat_v1_Server.decode(r, r.uint32());
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
  room?: strims_chat_v1_IRoom;
}

export class UpdateServerRequest {
  id: bigint;
  networkKey: Uint8Array;
  room: strims_chat_v1_Room | undefined;

  constructor(v?: IUpdateServerRequest) {
    this.id = v?.id || BigInt(0);
    this.networkKey = v?.networkKey || new Uint8Array();
    this.room = v?.room && new strims_chat_v1_Room(v.room);
  }

  static encode(m: UpdateServerRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.networkKey.length) w.uint32(18).bytes(m.networkKey);
    if (m.room) strims_chat_v1_Room.encode(m.room, w.uint32(26).fork()).ldelim();
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
        m.room = strims_chat_v1_Room.decode(r, r.uint32());
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
  server?: strims_chat_v1_IServer;
}

export class UpdateServerResponse {
  server: strims_chat_v1_Server | undefined;

  constructor(v?: IUpdateServerResponse) {
    this.server = v?.server && new strims_chat_v1_Server(v.server);
  }

  static encode(m: UpdateServerResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.server) strims_chat_v1_Server.encode(m.server, w.uint32(10).fork()).ldelim();
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
        m.server = strims_chat_v1_Server.decode(r, r.uint32());
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

export type IDeleteServerResponse = Record<string, any>;

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
  server?: strims_chat_v1_IServer;
}

export class GetServerResponse {
  server: strims_chat_v1_Server | undefined;

  constructor(v?: IGetServerResponse) {
    this.server = v?.server && new strims_chat_v1_Server(v.server);
  }

  static encode(m: GetServerResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.server) strims_chat_v1_Server.encode(m.server, w.uint32(10).fork()).ldelim();
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
        m.server = strims_chat_v1_Server.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IListServersRequest = Record<string, any>;

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
  servers?: strims_chat_v1_IServer[];
}

export class ListServersResponse {
  servers: strims_chat_v1_Server[];

  constructor(v?: IListServersResponse) {
    this.servers = v?.servers ? v.servers.map(v => new strims_chat_v1_Server(v)) : [];
  }

  static encode(m: ListServersResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.servers) strims_chat_v1_Server.encode(v, w.uint32(10).fork()).ldelim();
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
        m.servers.push(strims_chat_v1_Server.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IUpdateServerIconRequest = {
  serverId?: bigint;
  image?: strims_type_IImage;
}

export class UpdateServerIconRequest {
  serverId: bigint;
  image: strims_type_Image | undefined;

  constructor(v?: IUpdateServerIconRequest) {
    this.serverId = v?.serverId || BigInt(0);
    this.image = v?.image && new strims_type_Image(v.image);
  }

  static encode(m: UpdateServerIconRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    if (m.image) strims_type_Image.encode(m.image, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateServerIconRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UpdateServerIconRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverId = r.uint64();
        break;
        case 2:
        m.image = strims_type_Image.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IUpdateServerIconResponse = {
  serverIcon?: strims_chat_v1_IServerIcon;
}

export class UpdateServerIconResponse {
  serverIcon: strims_chat_v1_ServerIcon | undefined;

  constructor(v?: IUpdateServerIconResponse) {
    this.serverIcon = v?.serverIcon && new strims_chat_v1_ServerIcon(v.serverIcon);
  }

  static encode(m: UpdateServerIconResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverIcon) strims_chat_v1_ServerIcon.encode(m.serverIcon, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateServerIconResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UpdateServerIconResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverIcon = strims_chat_v1_ServerIcon.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IGetServerIconRequest = {
  serverId?: bigint;
}

export class GetServerIconRequest {
  serverId: bigint;

  constructor(v?: IGetServerIconRequest) {
    this.serverId = v?.serverId || BigInt(0);
  }

  static encode(m: GetServerIconRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetServerIconRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetServerIconRequest();
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

export type IGetServerIconResponse = {
  serverIcon?: strims_chat_v1_IServerIcon;
}

export class GetServerIconResponse {
  serverIcon: strims_chat_v1_ServerIcon | undefined;

  constructor(v?: IGetServerIconResponse) {
    this.serverIcon = v?.serverIcon && new strims_chat_v1_ServerIcon(v.serverIcon);
  }

  static encode(m: GetServerIconResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverIcon) strims_chat_v1_ServerIcon.encode(m.serverIcon, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetServerIconResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetServerIconResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverIcon = strims_chat_v1_ServerIcon.decode(r, r.uint32());
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
  images?: strims_chat_v1_IEmoteImage[];
  css?: string;
  effects?: strims_chat_v1_IEmoteEffect[];
  contributor?: strims_chat_v1_IEmoteContributor;
}

export class CreateEmoteRequest {
  serverId: bigint;
  name: string;
  images: strims_chat_v1_EmoteImage[];
  css: string;
  effects: strims_chat_v1_EmoteEffect[];
  contributor: strims_chat_v1_EmoteContributor | undefined;

  constructor(v?: ICreateEmoteRequest) {
    this.serverId = v?.serverId || BigInt(0);
    this.name = v?.name || "";
    this.images = v?.images ? v.images.map(v => new strims_chat_v1_EmoteImage(v)) : [];
    this.css = v?.css || "";
    this.effects = v?.effects ? v.effects.map(v => new strims_chat_v1_EmoteEffect(v)) : [];
    this.contributor = v?.contributor && new strims_chat_v1_EmoteContributor(v.contributor);
  }

  static encode(m: CreateEmoteRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    if (m.name.length) w.uint32(18).string(m.name);
    for (const v of m.images) strims_chat_v1_EmoteImage.encode(v, w.uint32(26).fork()).ldelim();
    if (m.css.length) w.uint32(34).string(m.css);
    for (const v of m.effects) strims_chat_v1_EmoteEffect.encode(v, w.uint32(42).fork()).ldelim();
    if (m.contributor) strims_chat_v1_EmoteContributor.encode(m.contributor, w.uint32(50).fork()).ldelim();
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
        m.images.push(strims_chat_v1_EmoteImage.decode(r, r.uint32()));
        break;
        case 4:
        m.css = r.string();
        break;
        case 5:
        m.effects.push(strims_chat_v1_EmoteEffect.decode(r, r.uint32()));
        break;
        case 6:
        m.contributor = strims_chat_v1_EmoteContributor.decode(r, r.uint32());
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
  emote?: strims_chat_v1_IEmote;
}

export class CreateEmoteResponse {
  emote: strims_chat_v1_Emote | undefined;

  constructor(v?: ICreateEmoteResponse) {
    this.emote = v?.emote && new strims_chat_v1_Emote(v.emote);
  }

  static encode(m: CreateEmoteResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.emote) strims_chat_v1_Emote.encode(m.emote, w.uint32(10).fork()).ldelim();
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
        m.emote = strims_chat_v1_Emote.decode(r, r.uint32());
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
  images?: strims_chat_v1_IEmoteImage[];
  css?: string;
  effects?: strims_chat_v1_IEmoteEffect[];
  contributor?: strims_chat_v1_IEmoteContributor;
}

export class UpdateEmoteRequest {
  serverId: bigint;
  id: bigint;
  name: string;
  images: strims_chat_v1_EmoteImage[];
  css: string;
  effects: strims_chat_v1_EmoteEffect[];
  contributor: strims_chat_v1_EmoteContributor | undefined;

  constructor(v?: IUpdateEmoteRequest) {
    this.serverId = v?.serverId || BigInt(0);
    this.id = v?.id || BigInt(0);
    this.name = v?.name || "";
    this.images = v?.images ? v.images.map(v => new strims_chat_v1_EmoteImage(v)) : [];
    this.css = v?.css || "";
    this.effects = v?.effects ? v.effects.map(v => new strims_chat_v1_EmoteEffect(v)) : [];
    this.contributor = v?.contributor && new strims_chat_v1_EmoteContributor(v.contributor);
  }

  static encode(m: UpdateEmoteRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    if (m.id) w.uint32(16).uint64(m.id);
    if (m.name.length) w.uint32(26).string(m.name);
    for (const v of m.images) strims_chat_v1_EmoteImage.encode(v, w.uint32(34).fork()).ldelim();
    if (m.css.length) w.uint32(42).string(m.css);
    for (const v of m.effects) strims_chat_v1_EmoteEffect.encode(v, w.uint32(50).fork()).ldelim();
    if (m.contributor) strims_chat_v1_EmoteContributor.encode(m.contributor, w.uint32(58).fork()).ldelim();
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
        m.images.push(strims_chat_v1_EmoteImage.decode(r, r.uint32()));
        break;
        case 5:
        m.css = r.string();
        break;
        case 6:
        m.effects.push(strims_chat_v1_EmoteEffect.decode(r, r.uint32()));
        break;
        case 7:
        m.contributor = strims_chat_v1_EmoteContributor.decode(r, r.uint32());
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
  emote?: strims_chat_v1_IEmote;
}

export class UpdateEmoteResponse {
  emote: strims_chat_v1_Emote | undefined;

  constructor(v?: IUpdateEmoteResponse) {
    this.emote = v?.emote && new strims_chat_v1_Emote(v.emote);
  }

  static encode(m: UpdateEmoteResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.emote) strims_chat_v1_Emote.encode(m.emote, w.uint32(10).fork()).ldelim();
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
        m.emote = strims_chat_v1_Emote.decode(r, r.uint32());
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

export type IDeleteEmoteResponse = Record<string, any>;

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
  emote?: strims_chat_v1_IEmote;
}

export class GetEmoteResponse {
  emote: strims_chat_v1_Emote | undefined;

  constructor(v?: IGetEmoteResponse) {
    this.emote = v?.emote && new strims_chat_v1_Emote(v.emote);
  }

  static encode(m: GetEmoteResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.emote) strims_chat_v1_Emote.encode(m.emote, w.uint32(10).fork()).ldelim();
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
        m.emote = strims_chat_v1_Emote.decode(r, r.uint32());
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
  parts?: strims_chat_v1_ListEmotesRequest_Part[];
}

export class ListEmotesRequest {
  serverId: bigint;
  parts: strims_chat_v1_ListEmotesRequest_Part[];

  constructor(v?: IListEmotesRequest) {
    this.serverId = v?.serverId || BigInt(0);
    this.parts = v?.parts ? v.parts : [];
  }

  static encode(m: ListEmotesRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    m.parts.reduce((w, v) => w.uint32(v), w.uint32(18).fork()).ldelim();
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
        case 2:
        for (const flen = r.uint32(), fend = r.pos + flen; r.pos < fend;) m.parts.push(r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace ListEmotesRequest {
  export enum Part {
    PART_UNDEFINED = 0,
    PART_META = 1,
    PART_ASSETS = 2,
  }
}

export type IListEmotesResponse = {
  emotes?: strims_chat_v1_IEmote[];
}

export class ListEmotesResponse {
  emotes: strims_chat_v1_Emote[];

  constructor(v?: IListEmotesResponse) {
    this.emotes = v?.emotes ? v.emotes.map(v => new strims_chat_v1_Emote(v)) : [];
  }

  static encode(m: ListEmotesResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.emotes) strims_chat_v1_Emote.encode(v, w.uint32(10).fork()).ldelim();
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
        m.emotes.push(strims_chat_v1_Emote.decode(r, r.uint32()));
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
  extraWrapCount?: number;
  procChance?: number;
}

export class CreateModifierRequest {
  serverId: bigint;
  name: string;
  priority: number;
  internal: boolean;
  extraWrapCount: number;
  procChance: number;

  constructor(v?: ICreateModifierRequest) {
    this.serverId = v?.serverId || BigInt(0);
    this.name = v?.name || "";
    this.priority = v?.priority || 0;
    this.internal = v?.internal || false;
    this.extraWrapCount = v?.extraWrapCount || 0;
    this.procChance = v?.procChance || 0;
  }

  static encode(m: CreateModifierRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    if (m.name.length) w.uint32(18).string(m.name);
    if (m.priority) w.uint32(24).uint32(m.priority);
    if (m.internal) w.uint32(32).bool(m.internal);
    if (m.extraWrapCount) w.uint32(40).uint32(m.extraWrapCount);
    if (m.procChance) w.uint32(49).double(m.procChance);
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
        case 5:
        m.extraWrapCount = r.uint32();
        break;
        case 6:
        m.procChance = r.double();
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
  modifier?: strims_chat_v1_IModifier;
}

export class CreateModifierResponse {
  modifier: strims_chat_v1_Modifier | undefined;

  constructor(v?: ICreateModifierResponse) {
    this.modifier = v?.modifier && new strims_chat_v1_Modifier(v.modifier);
  }

  static encode(m: CreateModifierResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.modifier) strims_chat_v1_Modifier.encode(m.modifier, w.uint32(10).fork()).ldelim();
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
        m.modifier = strims_chat_v1_Modifier.decode(r, r.uint32());
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
  extraWrapCount?: number;
  procChance?: number;
}

export class UpdateModifierRequest {
  serverId: bigint;
  id: bigint;
  name: string;
  priority: number;
  internal: boolean;
  extraWrapCount: number;
  procChance: number;

  constructor(v?: IUpdateModifierRequest) {
    this.serverId = v?.serverId || BigInt(0);
    this.id = v?.id || BigInt(0);
    this.name = v?.name || "";
    this.priority = v?.priority || 0;
    this.internal = v?.internal || false;
    this.extraWrapCount = v?.extraWrapCount || 0;
    this.procChance = v?.procChance || 0;
  }

  static encode(m: UpdateModifierRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    if (m.id) w.uint32(16).uint64(m.id);
    if (m.name.length) w.uint32(26).string(m.name);
    if (m.priority) w.uint32(32).uint32(m.priority);
    if (m.internal) w.uint32(40).bool(m.internal);
    if (m.extraWrapCount) w.uint32(48).uint32(m.extraWrapCount);
    if (m.procChance) w.uint32(57).double(m.procChance);
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
        case 6:
        m.extraWrapCount = r.uint32();
        break;
        case 7:
        m.procChance = r.double();
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
  modifier?: strims_chat_v1_IModifier;
}

export class UpdateModifierResponse {
  modifier: strims_chat_v1_Modifier | undefined;

  constructor(v?: IUpdateModifierResponse) {
    this.modifier = v?.modifier && new strims_chat_v1_Modifier(v.modifier);
  }

  static encode(m: UpdateModifierResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.modifier) strims_chat_v1_Modifier.encode(m.modifier, w.uint32(10).fork()).ldelim();
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
        m.modifier = strims_chat_v1_Modifier.decode(r, r.uint32());
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

export type IDeleteModifierResponse = Record<string, any>;

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
  modifier?: strims_chat_v1_IModifier;
}

export class GetModifierResponse {
  modifier: strims_chat_v1_Modifier | undefined;

  constructor(v?: IGetModifierResponse) {
    this.modifier = v?.modifier && new strims_chat_v1_Modifier(v.modifier);
  }

  static encode(m: GetModifierResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.modifier) strims_chat_v1_Modifier.encode(m.modifier, w.uint32(10).fork()).ldelim();
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
        m.modifier = strims_chat_v1_Modifier.decode(r, r.uint32());
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
  modifiers?: strims_chat_v1_IModifier[];
}

export class ListModifiersResponse {
  modifiers: strims_chat_v1_Modifier[];

  constructor(v?: IListModifiersResponse) {
    this.modifiers = v?.modifiers ? v.modifiers.map(v => new strims_chat_v1_Modifier(v)) : [];
  }

  static encode(m: ListModifiersResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.modifiers) strims_chat_v1_Modifier.encode(v, w.uint32(10).fork()).ldelim();
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
        m.modifiers.push(strims_chat_v1_Modifier.decode(r, r.uint32()));
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
  tag?: strims_chat_v1_ITag;
}

export class CreateTagResponse {
  tag: strims_chat_v1_Tag | undefined;

  constructor(v?: ICreateTagResponse) {
    this.tag = v?.tag && new strims_chat_v1_Tag(v.tag);
  }

  static encode(m: CreateTagResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.tag) strims_chat_v1_Tag.encode(m.tag, w.uint32(10).fork()).ldelim();
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
        m.tag = strims_chat_v1_Tag.decode(r, r.uint32());
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
  tag?: strims_chat_v1_ITag;
}

export class UpdateTagResponse {
  tag: strims_chat_v1_Tag | undefined;

  constructor(v?: IUpdateTagResponse) {
    this.tag = v?.tag && new strims_chat_v1_Tag(v.tag);
  }

  static encode(m: UpdateTagResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.tag) strims_chat_v1_Tag.encode(m.tag, w.uint32(10).fork()).ldelim();
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
        m.tag = strims_chat_v1_Tag.decode(r, r.uint32());
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

export type IDeleteTagResponse = Record<string, any>;

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
  tag?: strims_chat_v1_ITag;
}

export class GetTagResponse {
  tag: strims_chat_v1_Tag | undefined;

  constructor(v?: IGetTagResponse) {
    this.tag = v?.tag && new strims_chat_v1_Tag(v.tag);
  }

  static encode(m: GetTagResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.tag) strims_chat_v1_Tag.encode(m.tag, w.uint32(10).fork()).ldelim();
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
        m.tag = strims_chat_v1_Tag.decode(r, r.uint32());
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
  tags?: strims_chat_v1_ITag[];
}

export class ListTagsResponse {
  tags: strims_chat_v1_Tag[];

  constructor(v?: IListTagsResponse) {
    this.tags = v?.tags ? v.tags.map(v => new strims_chat_v1_Tag(v)) : [];
  }

  static encode(m: ListTagsResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.tags) strims_chat_v1_Tag.encode(v, w.uint32(10).fork()).ldelim();
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
        m.tags.push(strims_chat_v1_Tag.decode(r, r.uint32()));
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
      strims_chat_v1_OpenClientResponse_Open.encode(m.body.open, w.uint32(8010).fork()).ldelim();
      break;
      case OpenClientResponse.BodyCase.SERVER_EVENTS:
      strims_chat_v1_OpenClientResponse_ServerEvents.encode(m.body.serverEvents, w.uint32(8018).fork()).ldelim();
      break;
      case OpenClientResponse.BodyCase.ASSET_BUNDLE:
      strims_chat_v1_AssetBundle.encode(m.body.assetBundle, w.uint32(8026).fork()).ldelim();
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
        m.body = new OpenClientResponse.Body({ open: strims_chat_v1_OpenClientResponse_Open.decode(r, r.uint32()) });
        break;
        case 1002:
        m.body = new OpenClientResponse.Body({ serverEvents: strims_chat_v1_OpenClientResponse_ServerEvents.decode(r, r.uint32()) });
        break;
        case 1003:
        m.body = new OpenClientResponse.Body({ assetBundle: strims_chat_v1_AssetBundle.decode(r, r.uint32()) });
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
  |{ case?: BodyCase.OPEN, open: strims_chat_v1_OpenClientResponse_IOpen }
  |{ case?: BodyCase.SERVER_EVENTS, serverEvents: strims_chat_v1_OpenClientResponse_IServerEvents }
  |{ case?: BodyCase.ASSET_BUNDLE, assetBundle: strims_chat_v1_IAssetBundle }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.OPEN, open: strims_chat_v1_OpenClientResponse_Open }
  |{ case: BodyCase.SERVER_EVENTS, serverEvents: strims_chat_v1_OpenClientResponse_ServerEvents }
  |{ case: BodyCase.ASSET_BUNDLE, assetBundle: strims_chat_v1_AssetBundle }
  >;

  class BodyImpl {
    open: strims_chat_v1_OpenClientResponse_Open;
    serverEvents: strims_chat_v1_OpenClientResponse_ServerEvents;
    assetBundle: strims_chat_v1_AssetBundle;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "open" in v) {
        this.case = BodyCase.OPEN;
        this.open = new strims_chat_v1_OpenClientResponse_Open(v.open);
      } else
      if (v && "serverEvents" in v) {
        this.case = BodyCase.SERVER_EVENTS;
        this.serverEvents = new strims_chat_v1_OpenClientResponse_ServerEvents(v.serverEvents);
      } else
      if (v && "assetBundle" in v) {
        this.case = BodyCase.ASSET_BUNDLE;
        this.assetBundle = new strims_chat_v1_AssetBundle(v.assetBundle);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { open: strims_chat_v1_OpenClientResponse_IOpen } ? { case: BodyCase.OPEN, open: strims_chat_v1_OpenClientResponse_Open } :
    T extends { serverEvents: strims_chat_v1_OpenClientResponse_IServerEvents } ? { case: BodyCase.SERVER_EVENTS, serverEvents: strims_chat_v1_OpenClientResponse_ServerEvents } :
    T extends { assetBundle: strims_chat_v1_IAssetBundle } ? { case: BodyCase.ASSET_BUNDLE, assetBundle: strims_chat_v1_AssetBundle } :
    never
    >;
  };

  export type IOpen = Record<string, any>;

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
    events?: strims_chat_v1_IServerEvent[];
  }

  export class ServerEvents {
    events: strims_chat_v1_ServerEvent[];

    constructor(v?: IServerEvents) {
      this.events = v?.events ? v.events.map(v => new strims_chat_v1_ServerEvent(v)) : [];
    }

    static encode(m: ServerEvents, w?: Writer): Writer {
      if (!w) w = new Writer();
      for (const v of m.events) strims_chat_v1_ServerEvent.encode(v, w.uint32(10).fork()).ldelim();
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
          m.events.push(strims_chat_v1_ServerEvent.decode(r, r.uint32()));
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

export type IClientSendMessageResponse = Record<string, any>;

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

export type IClientMuteResponse = Record<string, any>;

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

export type IClientUnmuteResponse = Record<string, any>;

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

export type IWhisperResponse = Record<string, any>;

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
  thread?: strims_chat_v1_IWhisperThread;
  whispers?: strims_chat_v1_IWhisperRecord[];
}

export class ListWhispersResponse {
  thread: strims_chat_v1_WhisperThread | undefined;
  whispers: strims_chat_v1_WhisperRecord[];

  constructor(v?: IListWhispersResponse) {
    this.thread = v?.thread && new strims_chat_v1_WhisperThread(v.thread);
    this.whispers = v?.whispers ? v.whispers.map(v => new strims_chat_v1_WhisperRecord(v)) : [];
  }

  static encode(m: ListWhispersResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.thread) strims_chat_v1_WhisperThread.encode(m.thread, w.uint32(10).fork()).ldelim();
    for (const v of m.whispers) strims_chat_v1_WhisperRecord.encode(v, w.uint32(18).fork()).ldelim();
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
        m.thread = strims_chat_v1_WhisperThread.decode(r, r.uint32());
        break;
        case 2:
        m.whispers.push(strims_chat_v1_WhisperRecord.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IWatchWhispersRequest = Record<string, any>;

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
      strims_chat_v1_WhisperThread.encode(m.body.threadUpdate, w.uint32(8010).fork()).ldelim();
      break;
      case WatchWhispersResponse.BodyCase.THREAD_DELETE:
      strims_chat_v1_WatchWhispersResponse_WhisperThreadDelete.encode(m.body.threadDelete, w.uint32(8018).fork()).ldelim();
      break;
      case WatchWhispersResponse.BodyCase.WHISPER_UPDATE:
      strims_chat_v1_WhisperRecord.encode(m.body.whisperUpdate, w.uint32(8026).fork()).ldelim();
      break;
      case WatchWhispersResponse.BodyCase.WHISPER_DELETE:
      strims_chat_v1_WatchWhispersResponse_WhisperDelete.encode(m.body.whisperDelete, w.uint32(8034).fork()).ldelim();
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
        m.body = new WatchWhispersResponse.Body({ threadUpdate: strims_chat_v1_WhisperThread.decode(r, r.uint32()) });
        break;
        case 1002:
        m.body = new WatchWhispersResponse.Body({ threadDelete: strims_chat_v1_WatchWhispersResponse_WhisperThreadDelete.decode(r, r.uint32()) });
        break;
        case 1003:
        m.body = new WatchWhispersResponse.Body({ whisperUpdate: strims_chat_v1_WhisperRecord.decode(r, r.uint32()) });
        break;
        case 1004:
        m.body = new WatchWhispersResponse.Body({ whisperDelete: strims_chat_v1_WatchWhispersResponse_WhisperDelete.decode(r, r.uint32()) });
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
    THREAD_DELETE = 1002,
    WHISPER_UPDATE = 1003,
    WHISPER_DELETE = 1004,
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.THREAD_UPDATE, threadUpdate: strims_chat_v1_IWhisperThread }
  |{ case?: BodyCase.THREAD_DELETE, threadDelete: strims_chat_v1_WatchWhispersResponse_IWhisperThreadDelete }
  |{ case?: BodyCase.WHISPER_UPDATE, whisperUpdate: strims_chat_v1_IWhisperRecord }
  |{ case?: BodyCase.WHISPER_DELETE, whisperDelete: strims_chat_v1_WatchWhispersResponse_IWhisperDelete }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.THREAD_UPDATE, threadUpdate: strims_chat_v1_WhisperThread }
  |{ case: BodyCase.THREAD_DELETE, threadDelete: strims_chat_v1_WatchWhispersResponse_WhisperThreadDelete }
  |{ case: BodyCase.WHISPER_UPDATE, whisperUpdate: strims_chat_v1_WhisperRecord }
  |{ case: BodyCase.WHISPER_DELETE, whisperDelete: strims_chat_v1_WatchWhispersResponse_WhisperDelete }
  >;

  class BodyImpl {
    threadUpdate: strims_chat_v1_WhisperThread;
    threadDelete: strims_chat_v1_WatchWhispersResponse_WhisperThreadDelete;
    whisperUpdate: strims_chat_v1_WhisperRecord;
    whisperDelete: strims_chat_v1_WatchWhispersResponse_WhisperDelete;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "threadUpdate" in v) {
        this.case = BodyCase.THREAD_UPDATE;
        this.threadUpdate = new strims_chat_v1_WhisperThread(v.threadUpdate);
      } else
      if (v && "threadDelete" in v) {
        this.case = BodyCase.THREAD_DELETE;
        this.threadDelete = new strims_chat_v1_WatchWhispersResponse_WhisperThreadDelete(v.threadDelete);
      } else
      if (v && "whisperUpdate" in v) {
        this.case = BodyCase.WHISPER_UPDATE;
        this.whisperUpdate = new strims_chat_v1_WhisperRecord(v.whisperUpdate);
      } else
      if (v && "whisperDelete" in v) {
        this.case = BodyCase.WHISPER_DELETE;
        this.whisperDelete = new strims_chat_v1_WatchWhispersResponse_WhisperDelete(v.whisperDelete);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { threadUpdate: strims_chat_v1_IWhisperThread } ? { case: BodyCase.THREAD_UPDATE, threadUpdate: strims_chat_v1_WhisperThread } :
    T extends { threadDelete: strims_chat_v1_WatchWhispersResponse_IWhisperThreadDelete } ? { case: BodyCase.THREAD_DELETE, threadDelete: strims_chat_v1_WatchWhispersResponse_WhisperThreadDelete } :
    T extends { whisperUpdate: strims_chat_v1_IWhisperRecord } ? { case: BodyCase.WHISPER_UPDATE, whisperUpdate: strims_chat_v1_WhisperRecord } :
    T extends { whisperDelete: strims_chat_v1_WatchWhispersResponse_IWhisperDelete } ? { case: BodyCase.WHISPER_DELETE, whisperDelete: strims_chat_v1_WatchWhispersResponse_WhisperDelete } :
    never
    >;
  };

  export type IWhisperThreadDelete = {
    threadId?: bigint;
  }

  export class WhisperThreadDelete {
    threadId: bigint;

    constructor(v?: IWhisperThreadDelete) {
      this.threadId = v?.threadId || BigInt(0);
    }

    static encode(m: WhisperThreadDelete, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.threadId) w.uint32(8).uint64(m.threadId);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): WhisperThreadDelete {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new WhisperThreadDelete();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
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

export type IMarkWhispersReadRequest = {
  threadId?: bigint;
}

export class MarkWhispersReadRequest {
  threadId: bigint;

  constructor(v?: IMarkWhispersReadRequest) {
    this.threadId = v?.threadId || BigInt(0);
  }

  static encode(m: MarkWhispersReadRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.threadId) w.uint32(8).uint64(m.threadId);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): MarkWhispersReadRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new MarkWhispersReadRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
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

export type IMarkWhispersReadResponse = Record<string, any>;

export class MarkWhispersReadResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IMarkWhispersReadResponse) {
  }

  static encode(m: MarkWhispersReadResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): MarkWhispersReadResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new MarkWhispersReadResponse();
  }
}

export type IDeleteWhisperThreadRequest = {
  threadId?: bigint;
}

export class DeleteWhisperThreadRequest {
  threadId: bigint;

  constructor(v?: IDeleteWhisperThreadRequest) {
    this.threadId = v?.threadId || BigInt(0);
  }

  static encode(m: DeleteWhisperThreadRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.threadId) w.uint32(8).uint64(m.threadId);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeleteWhisperThreadRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DeleteWhisperThreadRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
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

export type IDeleteWhisperThreadResponse = Record<string, any>;

export class DeleteWhisperThreadResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IDeleteWhisperThreadResponse) {
  }

  static encode(m: DeleteWhisperThreadResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeleteWhisperThreadResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new DeleteWhisperThreadResponse();
  }
}

export type ISetUIConfigRequest = {
  uiConfig?: strims_chat_v1_IUIConfig;
}

export class SetUIConfigRequest {
  uiConfig: strims_chat_v1_UIConfig | undefined;

  constructor(v?: ISetUIConfigRequest) {
    this.uiConfig = v?.uiConfig && new strims_chat_v1_UIConfig(v.uiConfig);
  }

  static encode(m: SetUIConfigRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.uiConfig) strims_chat_v1_UIConfig.encode(m.uiConfig, w.uint32(10).fork()).ldelim();
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
        m.uiConfig = strims_chat_v1_UIConfig.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ISetUIConfigResponse = Record<string, any>;

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

export type IWatchUIConfigRequest = Record<string, any>;

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
  config?: WatchUIConfigResponse.IConfig
}

export class WatchUIConfigResponse {
  config: WatchUIConfigResponse.TConfig;

  constructor(v?: IWatchUIConfigResponse) {
    this.config = new WatchUIConfigResponse.Config(v?.config);
  }

  static encode(m: WatchUIConfigResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.config.case) {
      case WatchUIConfigResponse.ConfigCase.UI_CONFIG:
      strims_chat_v1_UIConfig.encode(m.config.uiConfig, w.uint32(8010).fork()).ldelim();
      break;
      case WatchUIConfigResponse.ConfigCase.UI_CONFIG_HIGHLIGHT:
      strims_chat_v1_UIConfigHighlight.encode(m.config.uiConfigHighlight, w.uint32(8018).fork()).ldelim();
      break;
      case WatchUIConfigResponse.ConfigCase.UI_CONFIG_HIGHLIGHT_DELETE:
      strims_chat_v1_UIConfigHighlight.encode(m.config.uiConfigHighlightDelete, w.uint32(8026).fork()).ldelim();
      break;
      case WatchUIConfigResponse.ConfigCase.UI_CONFIG_TAG:
      strims_chat_v1_UIConfigTag.encode(m.config.uiConfigTag, w.uint32(8034).fork()).ldelim();
      break;
      case WatchUIConfigResponse.ConfigCase.UI_CONFIG_TAG_DELETE:
      strims_chat_v1_UIConfigTag.encode(m.config.uiConfigTagDelete, w.uint32(8042).fork()).ldelim();
      break;
      case WatchUIConfigResponse.ConfigCase.UI_CONFIG_IGNORE:
      strims_chat_v1_UIConfigIgnore.encode(m.config.uiConfigIgnore, w.uint32(8050).fork()).ldelim();
      break;
      case WatchUIConfigResponse.ConfigCase.UI_CONFIG_IGNORE_DELETE:
      strims_chat_v1_UIConfigIgnore.encode(m.config.uiConfigIgnoreDelete, w.uint32(8058).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WatchUIConfigResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new WatchUIConfigResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1001:
        m.config = new WatchUIConfigResponse.Config({ uiConfig: strims_chat_v1_UIConfig.decode(r, r.uint32()) });
        break;
        case 1002:
        m.config = new WatchUIConfigResponse.Config({ uiConfigHighlight: strims_chat_v1_UIConfigHighlight.decode(r, r.uint32()) });
        break;
        case 1003:
        m.config = new WatchUIConfigResponse.Config({ uiConfigHighlightDelete: strims_chat_v1_UIConfigHighlight.decode(r, r.uint32()) });
        break;
        case 1004:
        m.config = new WatchUIConfigResponse.Config({ uiConfigTag: strims_chat_v1_UIConfigTag.decode(r, r.uint32()) });
        break;
        case 1005:
        m.config = new WatchUIConfigResponse.Config({ uiConfigTagDelete: strims_chat_v1_UIConfigTag.decode(r, r.uint32()) });
        break;
        case 1006:
        m.config = new WatchUIConfigResponse.Config({ uiConfigIgnore: strims_chat_v1_UIConfigIgnore.decode(r, r.uint32()) });
        break;
        case 1007:
        m.config = new WatchUIConfigResponse.Config({ uiConfigIgnoreDelete: strims_chat_v1_UIConfigIgnore.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace WatchUIConfigResponse {
  export enum ConfigCase {
    NOT_SET = 0,
    UI_CONFIG = 1001,
    UI_CONFIG_HIGHLIGHT = 1002,
    UI_CONFIG_HIGHLIGHT_DELETE = 1003,
    UI_CONFIG_TAG = 1004,
    UI_CONFIG_TAG_DELETE = 1005,
    UI_CONFIG_IGNORE = 1006,
    UI_CONFIG_IGNORE_DELETE = 1007,
  }

  export type IConfig =
  { case?: ConfigCase.NOT_SET }
  |{ case?: ConfigCase.UI_CONFIG, uiConfig: strims_chat_v1_IUIConfig }
  |{ case?: ConfigCase.UI_CONFIG_HIGHLIGHT, uiConfigHighlight: strims_chat_v1_IUIConfigHighlight }
  |{ case?: ConfigCase.UI_CONFIG_HIGHLIGHT_DELETE, uiConfigHighlightDelete: strims_chat_v1_IUIConfigHighlight }
  |{ case?: ConfigCase.UI_CONFIG_TAG, uiConfigTag: strims_chat_v1_IUIConfigTag }
  |{ case?: ConfigCase.UI_CONFIG_TAG_DELETE, uiConfigTagDelete: strims_chat_v1_IUIConfigTag }
  |{ case?: ConfigCase.UI_CONFIG_IGNORE, uiConfigIgnore: strims_chat_v1_IUIConfigIgnore }
  |{ case?: ConfigCase.UI_CONFIG_IGNORE_DELETE, uiConfigIgnoreDelete: strims_chat_v1_IUIConfigIgnore }
  ;

  export type TConfig = Readonly<
  { case: ConfigCase.NOT_SET }
  |{ case: ConfigCase.UI_CONFIG, uiConfig: strims_chat_v1_UIConfig }
  |{ case: ConfigCase.UI_CONFIG_HIGHLIGHT, uiConfigHighlight: strims_chat_v1_UIConfigHighlight }
  |{ case: ConfigCase.UI_CONFIG_HIGHLIGHT_DELETE, uiConfigHighlightDelete: strims_chat_v1_UIConfigHighlight }
  |{ case: ConfigCase.UI_CONFIG_TAG, uiConfigTag: strims_chat_v1_UIConfigTag }
  |{ case: ConfigCase.UI_CONFIG_TAG_DELETE, uiConfigTagDelete: strims_chat_v1_UIConfigTag }
  |{ case: ConfigCase.UI_CONFIG_IGNORE, uiConfigIgnore: strims_chat_v1_UIConfigIgnore }
  |{ case: ConfigCase.UI_CONFIG_IGNORE_DELETE, uiConfigIgnoreDelete: strims_chat_v1_UIConfigIgnore }
  >;

  class ConfigImpl {
    uiConfig: strims_chat_v1_UIConfig;
    uiConfigHighlight: strims_chat_v1_UIConfigHighlight;
    uiConfigHighlightDelete: strims_chat_v1_UIConfigHighlight;
    uiConfigTag: strims_chat_v1_UIConfigTag;
    uiConfigTagDelete: strims_chat_v1_UIConfigTag;
    uiConfigIgnore: strims_chat_v1_UIConfigIgnore;
    uiConfigIgnoreDelete: strims_chat_v1_UIConfigIgnore;
    case: ConfigCase = ConfigCase.NOT_SET;

    constructor(v?: IConfig) {
      if (v && "uiConfig" in v) {
        this.case = ConfigCase.UI_CONFIG;
        this.uiConfig = new strims_chat_v1_UIConfig(v.uiConfig);
      } else
      if (v && "uiConfigHighlight" in v) {
        this.case = ConfigCase.UI_CONFIG_HIGHLIGHT;
        this.uiConfigHighlight = new strims_chat_v1_UIConfigHighlight(v.uiConfigHighlight);
      } else
      if (v && "uiConfigHighlightDelete" in v) {
        this.case = ConfigCase.UI_CONFIG_HIGHLIGHT_DELETE;
        this.uiConfigHighlightDelete = new strims_chat_v1_UIConfigHighlight(v.uiConfigHighlightDelete);
      } else
      if (v && "uiConfigTag" in v) {
        this.case = ConfigCase.UI_CONFIG_TAG;
        this.uiConfigTag = new strims_chat_v1_UIConfigTag(v.uiConfigTag);
      } else
      if (v && "uiConfigTagDelete" in v) {
        this.case = ConfigCase.UI_CONFIG_TAG_DELETE;
        this.uiConfigTagDelete = new strims_chat_v1_UIConfigTag(v.uiConfigTagDelete);
      } else
      if (v && "uiConfigIgnore" in v) {
        this.case = ConfigCase.UI_CONFIG_IGNORE;
        this.uiConfigIgnore = new strims_chat_v1_UIConfigIgnore(v.uiConfigIgnore);
      } else
      if (v && "uiConfigIgnoreDelete" in v) {
        this.case = ConfigCase.UI_CONFIG_IGNORE_DELETE;
        this.uiConfigIgnoreDelete = new strims_chat_v1_UIConfigIgnore(v.uiConfigIgnoreDelete);
      }
    }
  }

  export const Config = ConfigImpl as {
    new (): Readonly<{ case: ConfigCase.NOT_SET }>;
    new <T extends IConfig>(v: T): Readonly<
    T extends { uiConfig: strims_chat_v1_IUIConfig } ? { case: ConfigCase.UI_CONFIG, uiConfig: strims_chat_v1_UIConfig } :
    T extends { uiConfigHighlight: strims_chat_v1_IUIConfigHighlight } ? { case: ConfigCase.UI_CONFIG_HIGHLIGHT, uiConfigHighlight: strims_chat_v1_UIConfigHighlight } :
    T extends { uiConfigHighlightDelete: strims_chat_v1_IUIConfigHighlight } ? { case: ConfigCase.UI_CONFIG_HIGHLIGHT_DELETE, uiConfigHighlightDelete: strims_chat_v1_UIConfigHighlight } :
    T extends { uiConfigTag: strims_chat_v1_IUIConfigTag } ? { case: ConfigCase.UI_CONFIG_TAG, uiConfigTag: strims_chat_v1_UIConfigTag } :
    T extends { uiConfigTagDelete: strims_chat_v1_IUIConfigTag } ? { case: ConfigCase.UI_CONFIG_TAG_DELETE, uiConfigTagDelete: strims_chat_v1_UIConfigTag } :
    T extends { uiConfigIgnore: strims_chat_v1_IUIConfigIgnore } ? { case: ConfigCase.UI_CONFIG_IGNORE, uiConfigIgnore: strims_chat_v1_UIConfigIgnore } :
    T extends { uiConfigIgnoreDelete: strims_chat_v1_IUIConfigIgnore } ? { case: ConfigCase.UI_CONFIG_IGNORE_DELETE, uiConfigIgnoreDelete: strims_chat_v1_UIConfigIgnore } :
    never
    >;
  };

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

export type IIgnoreResponse = Record<string, any>;

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

export type IUnignoreResponse = Record<string, any>;

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

export type IHighlightResponse = Record<string, any>;

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

export type IUnhighlightResponse = Record<string, any>;

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

export type ITagResponse = Record<string, any>;

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

export type IUntagResponse = Record<string, any>;

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

export type ISendMessageResponse = Record<string, any>;

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

export type IMuteResponse = Record<string, any>;

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

export type IUnmuteResponse = Record<string, any>;

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

export type IGetMuteRequest = Record<string, any>;

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
  state?: strims_chat_v1_WhisperRecord_State;
  message?: strims_chat_v1_IMessage;
}

export class WhisperRecord {
  id: bigint;
  threadId: bigint;
  networkKey: Uint8Array;
  serverKey: Uint8Array;
  peerKey: Uint8Array;
  state: strims_chat_v1_WhisperRecord_State;
  message: strims_chat_v1_Message | undefined;

  constructor(v?: IWhisperRecord) {
    this.id = v?.id || BigInt(0);
    this.threadId = v?.threadId || BigInt(0);
    this.networkKey = v?.networkKey || new Uint8Array();
    this.serverKey = v?.serverKey || new Uint8Array();
    this.peerKey = v?.peerKey || new Uint8Array();
    this.state = v?.state || 0;
    this.message = v?.message && new strims_chat_v1_Message(v.message);
  }

  static encode(m: WhisperRecord, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.threadId) w.uint32(16).uint64(m.threadId);
    if (m.networkKey.length) w.uint32(26).bytes(m.networkKey);
    if (m.serverKey.length) w.uint32(34).bytes(m.serverKey);
    if (m.peerKey.length) w.uint32(42).bytes(m.peerKey);
    if (m.state) w.uint32(48).uint32(m.state);
    if (m.message) strims_chat_v1_Message.encode(m.message, w.uint32(58).fork()).ldelim();
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
        m.message = strims_chat_v1_Message.decode(r, r.uint32());
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

export type IWhisperSendMessageResponse = Record<string, any>;

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
/* @internal */
export const strims_chat_v1_ServerEvent = ServerEvent;
/* @internal */
export type strims_chat_v1_ServerEvent = ServerEvent;
/* @internal */
export type strims_chat_v1_IServerEvent = IServerEvent;
/* @internal */
export const strims_chat_v1_Room = Room;
/* @internal */
export type strims_chat_v1_Room = Room;
/* @internal */
export type strims_chat_v1_IRoom = IRoom;
/* @internal */
export const strims_chat_v1_Server = Server;
/* @internal */
export type strims_chat_v1_Server = Server;
/* @internal */
export type strims_chat_v1_IServer = IServer;
/* @internal */
export const strims_chat_v1_ServerIcon = ServerIcon;
/* @internal */
export type strims_chat_v1_ServerIcon = ServerIcon;
/* @internal */
export type strims_chat_v1_IServerIcon = IServerIcon;
/* @internal */
export const strims_chat_v1_EmoteImage = EmoteImage;
/* @internal */
export type strims_chat_v1_EmoteImage = EmoteImage;
/* @internal */
export type strims_chat_v1_IEmoteImage = IEmoteImage;
/* @internal */
export const strims_chat_v1_EmoteEffect = EmoteEffect;
/* @internal */
export type strims_chat_v1_EmoteEffect = EmoteEffect;
/* @internal */
export type strims_chat_v1_IEmoteEffect = IEmoteEffect;
/* @internal */
export const strims_chat_v1_EmoteContributor = EmoteContributor;
/* @internal */
export type strims_chat_v1_EmoteContributor = EmoteContributor;
/* @internal */
export type strims_chat_v1_IEmoteContributor = IEmoteContributor;
/* @internal */
export const strims_chat_v1_Emote = Emote;
/* @internal */
export type strims_chat_v1_Emote = Emote;
/* @internal */
export type strims_chat_v1_IEmote = IEmote;
/* @internal */
export const strims_chat_v1_Modifier = Modifier;
/* @internal */
export type strims_chat_v1_Modifier = Modifier;
/* @internal */
export type strims_chat_v1_IModifier = IModifier;
/* @internal */
export const strims_chat_v1_Tag = Tag;
/* @internal */
export type strims_chat_v1_Tag = Tag;
/* @internal */
export type strims_chat_v1_ITag = ITag;
/* @internal */
export const strims_chat_v1_AssetBundle = AssetBundle;
/* @internal */
export type strims_chat_v1_AssetBundle = AssetBundle;
/* @internal */
export type strims_chat_v1_IAssetBundle = IAssetBundle;
/* @internal */
export const strims_chat_v1_Message = Message;
/* @internal */
export type strims_chat_v1_Message = Message;
/* @internal */
export type strims_chat_v1_IMessage = IMessage;
/* @internal */
export const strims_chat_v1_Profile = Profile;
/* @internal */
export type strims_chat_v1_Profile = Profile;
/* @internal */
export type strims_chat_v1_IProfile = IProfile;
/* @internal */
export const strims_chat_v1_UIConfig = UIConfig;
/* @internal */
export type strims_chat_v1_UIConfig = UIConfig;
/* @internal */
export type strims_chat_v1_IUIConfig = IUIConfig;
/* @internal */
export const strims_chat_v1_UIConfigHighlight = UIConfigHighlight;
/* @internal */
export type strims_chat_v1_UIConfigHighlight = UIConfigHighlight;
/* @internal */
export type strims_chat_v1_IUIConfigHighlight = IUIConfigHighlight;
/* @internal */
export const strims_chat_v1_UIConfigTag = UIConfigTag;
/* @internal */
export type strims_chat_v1_UIConfigTag = UIConfigTag;
/* @internal */
export type strims_chat_v1_IUIConfigTag = IUIConfigTag;
/* @internal */
export const strims_chat_v1_UIConfigIgnore = UIConfigIgnore;
/* @internal */
export type strims_chat_v1_UIConfigIgnore = UIConfigIgnore;
/* @internal */
export type strims_chat_v1_IUIConfigIgnore = IUIConfigIgnore;
/* @internal */
export const strims_chat_v1_CreateServerRequest = CreateServerRequest;
/* @internal */
export type strims_chat_v1_CreateServerRequest = CreateServerRequest;
/* @internal */
export type strims_chat_v1_ICreateServerRequest = ICreateServerRequest;
/* @internal */
export const strims_chat_v1_CreateServerResponse = CreateServerResponse;
/* @internal */
export type strims_chat_v1_CreateServerResponse = CreateServerResponse;
/* @internal */
export type strims_chat_v1_ICreateServerResponse = ICreateServerResponse;
/* @internal */
export const strims_chat_v1_UpdateServerRequest = UpdateServerRequest;
/* @internal */
export type strims_chat_v1_UpdateServerRequest = UpdateServerRequest;
/* @internal */
export type strims_chat_v1_IUpdateServerRequest = IUpdateServerRequest;
/* @internal */
export const strims_chat_v1_UpdateServerResponse = UpdateServerResponse;
/* @internal */
export type strims_chat_v1_UpdateServerResponse = UpdateServerResponse;
/* @internal */
export type strims_chat_v1_IUpdateServerResponse = IUpdateServerResponse;
/* @internal */
export const strims_chat_v1_DeleteServerRequest = DeleteServerRequest;
/* @internal */
export type strims_chat_v1_DeleteServerRequest = DeleteServerRequest;
/* @internal */
export type strims_chat_v1_IDeleteServerRequest = IDeleteServerRequest;
/* @internal */
export const strims_chat_v1_DeleteServerResponse = DeleteServerResponse;
/* @internal */
export type strims_chat_v1_DeleteServerResponse = DeleteServerResponse;
/* @internal */
export type strims_chat_v1_IDeleteServerResponse = IDeleteServerResponse;
/* @internal */
export const strims_chat_v1_GetServerRequest = GetServerRequest;
/* @internal */
export type strims_chat_v1_GetServerRequest = GetServerRequest;
/* @internal */
export type strims_chat_v1_IGetServerRequest = IGetServerRequest;
/* @internal */
export const strims_chat_v1_GetServerResponse = GetServerResponse;
/* @internal */
export type strims_chat_v1_GetServerResponse = GetServerResponse;
/* @internal */
export type strims_chat_v1_IGetServerResponse = IGetServerResponse;
/* @internal */
export const strims_chat_v1_ListServersRequest = ListServersRequest;
/* @internal */
export type strims_chat_v1_ListServersRequest = ListServersRequest;
/* @internal */
export type strims_chat_v1_IListServersRequest = IListServersRequest;
/* @internal */
export const strims_chat_v1_ListServersResponse = ListServersResponse;
/* @internal */
export type strims_chat_v1_ListServersResponse = ListServersResponse;
/* @internal */
export type strims_chat_v1_IListServersResponse = IListServersResponse;
/* @internal */
export const strims_chat_v1_UpdateServerIconRequest = UpdateServerIconRequest;
/* @internal */
export type strims_chat_v1_UpdateServerIconRequest = UpdateServerIconRequest;
/* @internal */
export type strims_chat_v1_IUpdateServerIconRequest = IUpdateServerIconRequest;
/* @internal */
export const strims_chat_v1_UpdateServerIconResponse = UpdateServerIconResponse;
/* @internal */
export type strims_chat_v1_UpdateServerIconResponse = UpdateServerIconResponse;
/* @internal */
export type strims_chat_v1_IUpdateServerIconResponse = IUpdateServerIconResponse;
/* @internal */
export const strims_chat_v1_GetServerIconRequest = GetServerIconRequest;
/* @internal */
export type strims_chat_v1_GetServerIconRequest = GetServerIconRequest;
/* @internal */
export type strims_chat_v1_IGetServerIconRequest = IGetServerIconRequest;
/* @internal */
export const strims_chat_v1_GetServerIconResponse = GetServerIconResponse;
/* @internal */
export type strims_chat_v1_GetServerIconResponse = GetServerIconResponse;
/* @internal */
export type strims_chat_v1_IGetServerIconResponse = IGetServerIconResponse;
/* @internal */
export const strims_chat_v1_CreateEmoteRequest = CreateEmoteRequest;
/* @internal */
export type strims_chat_v1_CreateEmoteRequest = CreateEmoteRequest;
/* @internal */
export type strims_chat_v1_ICreateEmoteRequest = ICreateEmoteRequest;
/* @internal */
export const strims_chat_v1_CreateEmoteResponse = CreateEmoteResponse;
/* @internal */
export type strims_chat_v1_CreateEmoteResponse = CreateEmoteResponse;
/* @internal */
export type strims_chat_v1_ICreateEmoteResponse = ICreateEmoteResponse;
/* @internal */
export const strims_chat_v1_UpdateEmoteRequest = UpdateEmoteRequest;
/* @internal */
export type strims_chat_v1_UpdateEmoteRequest = UpdateEmoteRequest;
/* @internal */
export type strims_chat_v1_IUpdateEmoteRequest = IUpdateEmoteRequest;
/* @internal */
export const strims_chat_v1_UpdateEmoteResponse = UpdateEmoteResponse;
/* @internal */
export type strims_chat_v1_UpdateEmoteResponse = UpdateEmoteResponse;
/* @internal */
export type strims_chat_v1_IUpdateEmoteResponse = IUpdateEmoteResponse;
/* @internal */
export const strims_chat_v1_DeleteEmoteRequest = DeleteEmoteRequest;
/* @internal */
export type strims_chat_v1_DeleteEmoteRequest = DeleteEmoteRequest;
/* @internal */
export type strims_chat_v1_IDeleteEmoteRequest = IDeleteEmoteRequest;
/* @internal */
export const strims_chat_v1_DeleteEmoteResponse = DeleteEmoteResponse;
/* @internal */
export type strims_chat_v1_DeleteEmoteResponse = DeleteEmoteResponse;
/* @internal */
export type strims_chat_v1_IDeleteEmoteResponse = IDeleteEmoteResponse;
/* @internal */
export const strims_chat_v1_GetEmoteRequest = GetEmoteRequest;
/* @internal */
export type strims_chat_v1_GetEmoteRequest = GetEmoteRequest;
/* @internal */
export type strims_chat_v1_IGetEmoteRequest = IGetEmoteRequest;
/* @internal */
export const strims_chat_v1_GetEmoteResponse = GetEmoteResponse;
/* @internal */
export type strims_chat_v1_GetEmoteResponse = GetEmoteResponse;
/* @internal */
export type strims_chat_v1_IGetEmoteResponse = IGetEmoteResponse;
/* @internal */
export const strims_chat_v1_ListEmotesRequest = ListEmotesRequest;
/* @internal */
export type strims_chat_v1_ListEmotesRequest = ListEmotesRequest;
/* @internal */
export type strims_chat_v1_IListEmotesRequest = IListEmotesRequest;
/* @internal */
export const strims_chat_v1_ListEmotesResponse = ListEmotesResponse;
/* @internal */
export type strims_chat_v1_ListEmotesResponse = ListEmotesResponse;
/* @internal */
export type strims_chat_v1_IListEmotesResponse = IListEmotesResponse;
/* @internal */
export const strims_chat_v1_CreateModifierRequest = CreateModifierRequest;
/* @internal */
export type strims_chat_v1_CreateModifierRequest = CreateModifierRequest;
/* @internal */
export type strims_chat_v1_ICreateModifierRequest = ICreateModifierRequest;
/* @internal */
export const strims_chat_v1_CreateModifierResponse = CreateModifierResponse;
/* @internal */
export type strims_chat_v1_CreateModifierResponse = CreateModifierResponse;
/* @internal */
export type strims_chat_v1_ICreateModifierResponse = ICreateModifierResponse;
/* @internal */
export const strims_chat_v1_UpdateModifierRequest = UpdateModifierRequest;
/* @internal */
export type strims_chat_v1_UpdateModifierRequest = UpdateModifierRequest;
/* @internal */
export type strims_chat_v1_IUpdateModifierRequest = IUpdateModifierRequest;
/* @internal */
export const strims_chat_v1_UpdateModifierResponse = UpdateModifierResponse;
/* @internal */
export type strims_chat_v1_UpdateModifierResponse = UpdateModifierResponse;
/* @internal */
export type strims_chat_v1_IUpdateModifierResponse = IUpdateModifierResponse;
/* @internal */
export const strims_chat_v1_DeleteModifierRequest = DeleteModifierRequest;
/* @internal */
export type strims_chat_v1_DeleteModifierRequest = DeleteModifierRequest;
/* @internal */
export type strims_chat_v1_IDeleteModifierRequest = IDeleteModifierRequest;
/* @internal */
export const strims_chat_v1_DeleteModifierResponse = DeleteModifierResponse;
/* @internal */
export type strims_chat_v1_DeleteModifierResponse = DeleteModifierResponse;
/* @internal */
export type strims_chat_v1_IDeleteModifierResponse = IDeleteModifierResponse;
/* @internal */
export const strims_chat_v1_GetModifierRequest = GetModifierRequest;
/* @internal */
export type strims_chat_v1_GetModifierRequest = GetModifierRequest;
/* @internal */
export type strims_chat_v1_IGetModifierRequest = IGetModifierRequest;
/* @internal */
export const strims_chat_v1_GetModifierResponse = GetModifierResponse;
/* @internal */
export type strims_chat_v1_GetModifierResponse = GetModifierResponse;
/* @internal */
export type strims_chat_v1_IGetModifierResponse = IGetModifierResponse;
/* @internal */
export const strims_chat_v1_ListModifiersRequest = ListModifiersRequest;
/* @internal */
export type strims_chat_v1_ListModifiersRequest = ListModifiersRequest;
/* @internal */
export type strims_chat_v1_IListModifiersRequest = IListModifiersRequest;
/* @internal */
export const strims_chat_v1_ListModifiersResponse = ListModifiersResponse;
/* @internal */
export type strims_chat_v1_ListModifiersResponse = ListModifiersResponse;
/* @internal */
export type strims_chat_v1_IListModifiersResponse = IListModifiersResponse;
/* @internal */
export const strims_chat_v1_CreateTagRequest = CreateTagRequest;
/* @internal */
export type strims_chat_v1_CreateTagRequest = CreateTagRequest;
/* @internal */
export type strims_chat_v1_ICreateTagRequest = ICreateTagRequest;
/* @internal */
export const strims_chat_v1_CreateTagResponse = CreateTagResponse;
/* @internal */
export type strims_chat_v1_CreateTagResponse = CreateTagResponse;
/* @internal */
export type strims_chat_v1_ICreateTagResponse = ICreateTagResponse;
/* @internal */
export const strims_chat_v1_UpdateTagRequest = UpdateTagRequest;
/* @internal */
export type strims_chat_v1_UpdateTagRequest = UpdateTagRequest;
/* @internal */
export type strims_chat_v1_IUpdateTagRequest = IUpdateTagRequest;
/* @internal */
export const strims_chat_v1_UpdateTagResponse = UpdateTagResponse;
/* @internal */
export type strims_chat_v1_UpdateTagResponse = UpdateTagResponse;
/* @internal */
export type strims_chat_v1_IUpdateTagResponse = IUpdateTagResponse;
/* @internal */
export const strims_chat_v1_DeleteTagRequest = DeleteTagRequest;
/* @internal */
export type strims_chat_v1_DeleteTagRequest = DeleteTagRequest;
/* @internal */
export type strims_chat_v1_IDeleteTagRequest = IDeleteTagRequest;
/* @internal */
export const strims_chat_v1_DeleteTagResponse = DeleteTagResponse;
/* @internal */
export type strims_chat_v1_DeleteTagResponse = DeleteTagResponse;
/* @internal */
export type strims_chat_v1_IDeleteTagResponse = IDeleteTagResponse;
/* @internal */
export const strims_chat_v1_GetTagRequest = GetTagRequest;
/* @internal */
export type strims_chat_v1_GetTagRequest = GetTagRequest;
/* @internal */
export type strims_chat_v1_IGetTagRequest = IGetTagRequest;
/* @internal */
export const strims_chat_v1_GetTagResponse = GetTagResponse;
/* @internal */
export type strims_chat_v1_GetTagResponse = GetTagResponse;
/* @internal */
export type strims_chat_v1_IGetTagResponse = IGetTagResponse;
/* @internal */
export const strims_chat_v1_ListTagsRequest = ListTagsRequest;
/* @internal */
export type strims_chat_v1_ListTagsRequest = ListTagsRequest;
/* @internal */
export type strims_chat_v1_IListTagsRequest = IListTagsRequest;
/* @internal */
export const strims_chat_v1_ListTagsResponse = ListTagsResponse;
/* @internal */
export type strims_chat_v1_ListTagsResponse = ListTagsResponse;
/* @internal */
export type strims_chat_v1_IListTagsResponse = IListTagsResponse;
/* @internal */
export const strims_chat_v1_SyncAssetsRequest = SyncAssetsRequest;
/* @internal */
export type strims_chat_v1_SyncAssetsRequest = SyncAssetsRequest;
/* @internal */
export type strims_chat_v1_ISyncAssetsRequest = ISyncAssetsRequest;
/* @internal */
export const strims_chat_v1_SyncAssetsResponse = SyncAssetsResponse;
/* @internal */
export type strims_chat_v1_SyncAssetsResponse = SyncAssetsResponse;
/* @internal */
export type strims_chat_v1_ISyncAssetsResponse = ISyncAssetsResponse;
/* @internal */
export const strims_chat_v1_OpenClientRequest = OpenClientRequest;
/* @internal */
export type strims_chat_v1_OpenClientRequest = OpenClientRequest;
/* @internal */
export type strims_chat_v1_IOpenClientRequest = IOpenClientRequest;
/* @internal */
export const strims_chat_v1_OpenClientResponse = OpenClientResponse;
/* @internal */
export type strims_chat_v1_OpenClientResponse = OpenClientResponse;
/* @internal */
export type strims_chat_v1_IOpenClientResponse = IOpenClientResponse;
/* @internal */
export const strims_chat_v1_ClientSendMessageRequest = ClientSendMessageRequest;
/* @internal */
export type strims_chat_v1_ClientSendMessageRequest = ClientSendMessageRequest;
/* @internal */
export type strims_chat_v1_IClientSendMessageRequest = IClientSendMessageRequest;
/* @internal */
export const strims_chat_v1_ClientSendMessageResponse = ClientSendMessageResponse;
/* @internal */
export type strims_chat_v1_ClientSendMessageResponse = ClientSendMessageResponse;
/* @internal */
export type strims_chat_v1_IClientSendMessageResponse = IClientSendMessageResponse;
/* @internal */
export const strims_chat_v1_ClientMuteRequest = ClientMuteRequest;
/* @internal */
export type strims_chat_v1_ClientMuteRequest = ClientMuteRequest;
/* @internal */
export type strims_chat_v1_IClientMuteRequest = IClientMuteRequest;
/* @internal */
export const strims_chat_v1_ClientMuteResponse = ClientMuteResponse;
/* @internal */
export type strims_chat_v1_ClientMuteResponse = ClientMuteResponse;
/* @internal */
export type strims_chat_v1_IClientMuteResponse = IClientMuteResponse;
/* @internal */
export const strims_chat_v1_ClientUnmuteRequest = ClientUnmuteRequest;
/* @internal */
export type strims_chat_v1_ClientUnmuteRequest = ClientUnmuteRequest;
/* @internal */
export type strims_chat_v1_IClientUnmuteRequest = IClientUnmuteRequest;
/* @internal */
export const strims_chat_v1_ClientUnmuteResponse = ClientUnmuteResponse;
/* @internal */
export type strims_chat_v1_ClientUnmuteResponse = ClientUnmuteResponse;
/* @internal */
export type strims_chat_v1_IClientUnmuteResponse = IClientUnmuteResponse;
/* @internal */
export const strims_chat_v1_ClientGetMuteRequest = ClientGetMuteRequest;
/* @internal */
export type strims_chat_v1_ClientGetMuteRequest = ClientGetMuteRequest;
/* @internal */
export type strims_chat_v1_IClientGetMuteRequest = IClientGetMuteRequest;
/* @internal */
export const strims_chat_v1_ClientGetMuteResponse = ClientGetMuteResponse;
/* @internal */
export type strims_chat_v1_ClientGetMuteResponse = ClientGetMuteResponse;
/* @internal */
export type strims_chat_v1_IClientGetMuteResponse = IClientGetMuteResponse;
/* @internal */
export const strims_chat_v1_WhisperRequest = WhisperRequest;
/* @internal */
export type strims_chat_v1_WhisperRequest = WhisperRequest;
/* @internal */
export type strims_chat_v1_IWhisperRequest = IWhisperRequest;
/* @internal */
export const strims_chat_v1_WhisperResponse = WhisperResponse;
/* @internal */
export type strims_chat_v1_WhisperResponse = WhisperResponse;
/* @internal */
export type strims_chat_v1_IWhisperResponse = IWhisperResponse;
/* @internal */
export const strims_chat_v1_ListWhispersRequest = ListWhispersRequest;
/* @internal */
export type strims_chat_v1_ListWhispersRequest = ListWhispersRequest;
/* @internal */
export type strims_chat_v1_IListWhispersRequest = IListWhispersRequest;
/* @internal */
export const strims_chat_v1_ListWhispersResponse = ListWhispersResponse;
/* @internal */
export type strims_chat_v1_ListWhispersResponse = ListWhispersResponse;
/* @internal */
export type strims_chat_v1_IListWhispersResponse = IListWhispersResponse;
/* @internal */
export const strims_chat_v1_WatchWhispersRequest = WatchWhispersRequest;
/* @internal */
export type strims_chat_v1_WatchWhispersRequest = WatchWhispersRequest;
/* @internal */
export type strims_chat_v1_IWatchWhispersRequest = IWatchWhispersRequest;
/* @internal */
export const strims_chat_v1_WatchWhispersResponse = WatchWhispersResponse;
/* @internal */
export type strims_chat_v1_WatchWhispersResponse = WatchWhispersResponse;
/* @internal */
export type strims_chat_v1_IWatchWhispersResponse = IWatchWhispersResponse;
/* @internal */
export const strims_chat_v1_MarkWhispersReadRequest = MarkWhispersReadRequest;
/* @internal */
export type strims_chat_v1_MarkWhispersReadRequest = MarkWhispersReadRequest;
/* @internal */
export type strims_chat_v1_IMarkWhispersReadRequest = IMarkWhispersReadRequest;
/* @internal */
export const strims_chat_v1_MarkWhispersReadResponse = MarkWhispersReadResponse;
/* @internal */
export type strims_chat_v1_MarkWhispersReadResponse = MarkWhispersReadResponse;
/* @internal */
export type strims_chat_v1_IMarkWhispersReadResponse = IMarkWhispersReadResponse;
/* @internal */
export const strims_chat_v1_DeleteWhisperThreadRequest = DeleteWhisperThreadRequest;
/* @internal */
export type strims_chat_v1_DeleteWhisperThreadRequest = DeleteWhisperThreadRequest;
/* @internal */
export type strims_chat_v1_IDeleteWhisperThreadRequest = IDeleteWhisperThreadRequest;
/* @internal */
export const strims_chat_v1_DeleteWhisperThreadResponse = DeleteWhisperThreadResponse;
/* @internal */
export type strims_chat_v1_DeleteWhisperThreadResponse = DeleteWhisperThreadResponse;
/* @internal */
export type strims_chat_v1_IDeleteWhisperThreadResponse = IDeleteWhisperThreadResponse;
/* @internal */
export const strims_chat_v1_SetUIConfigRequest = SetUIConfigRequest;
/* @internal */
export type strims_chat_v1_SetUIConfigRequest = SetUIConfigRequest;
/* @internal */
export type strims_chat_v1_ISetUIConfigRequest = ISetUIConfigRequest;
/* @internal */
export const strims_chat_v1_SetUIConfigResponse = SetUIConfigResponse;
/* @internal */
export type strims_chat_v1_SetUIConfigResponse = SetUIConfigResponse;
/* @internal */
export type strims_chat_v1_ISetUIConfigResponse = ISetUIConfigResponse;
/* @internal */
export const strims_chat_v1_WatchUIConfigRequest = WatchUIConfigRequest;
/* @internal */
export type strims_chat_v1_WatchUIConfigRequest = WatchUIConfigRequest;
/* @internal */
export type strims_chat_v1_IWatchUIConfigRequest = IWatchUIConfigRequest;
/* @internal */
export const strims_chat_v1_WatchUIConfigResponse = WatchUIConfigResponse;
/* @internal */
export type strims_chat_v1_WatchUIConfigResponse = WatchUIConfigResponse;
/* @internal */
export type strims_chat_v1_IWatchUIConfigResponse = IWatchUIConfigResponse;
/* @internal */
export const strims_chat_v1_IgnoreRequest = IgnoreRequest;
/* @internal */
export type strims_chat_v1_IgnoreRequest = IgnoreRequest;
/* @internal */
export type strims_chat_v1_IIgnoreRequest = IIgnoreRequest;
/* @internal */
export const strims_chat_v1_IgnoreResponse = IgnoreResponse;
/* @internal */
export type strims_chat_v1_IgnoreResponse = IgnoreResponse;
/* @internal */
export type strims_chat_v1_IIgnoreResponse = IIgnoreResponse;
/* @internal */
export const strims_chat_v1_UnignoreRequest = UnignoreRequest;
/* @internal */
export type strims_chat_v1_UnignoreRequest = UnignoreRequest;
/* @internal */
export type strims_chat_v1_IUnignoreRequest = IUnignoreRequest;
/* @internal */
export const strims_chat_v1_UnignoreResponse = UnignoreResponse;
/* @internal */
export type strims_chat_v1_UnignoreResponse = UnignoreResponse;
/* @internal */
export type strims_chat_v1_IUnignoreResponse = IUnignoreResponse;
/* @internal */
export const strims_chat_v1_HighlightRequest = HighlightRequest;
/* @internal */
export type strims_chat_v1_HighlightRequest = HighlightRequest;
/* @internal */
export type strims_chat_v1_IHighlightRequest = IHighlightRequest;
/* @internal */
export const strims_chat_v1_HighlightResponse = HighlightResponse;
/* @internal */
export type strims_chat_v1_HighlightResponse = HighlightResponse;
/* @internal */
export type strims_chat_v1_IHighlightResponse = IHighlightResponse;
/* @internal */
export const strims_chat_v1_UnhighlightRequest = UnhighlightRequest;
/* @internal */
export type strims_chat_v1_UnhighlightRequest = UnhighlightRequest;
/* @internal */
export type strims_chat_v1_IUnhighlightRequest = IUnhighlightRequest;
/* @internal */
export const strims_chat_v1_UnhighlightResponse = UnhighlightResponse;
/* @internal */
export type strims_chat_v1_UnhighlightResponse = UnhighlightResponse;
/* @internal */
export type strims_chat_v1_IUnhighlightResponse = IUnhighlightResponse;
/* @internal */
export const strims_chat_v1_TagRequest = TagRequest;
/* @internal */
export type strims_chat_v1_TagRequest = TagRequest;
/* @internal */
export type strims_chat_v1_ITagRequest = ITagRequest;
/* @internal */
export const strims_chat_v1_TagResponse = TagResponse;
/* @internal */
export type strims_chat_v1_TagResponse = TagResponse;
/* @internal */
export type strims_chat_v1_ITagResponse = ITagResponse;
/* @internal */
export const strims_chat_v1_UntagRequest = UntagRequest;
/* @internal */
export type strims_chat_v1_UntagRequest = UntagRequest;
/* @internal */
export type strims_chat_v1_IUntagRequest = IUntagRequest;
/* @internal */
export const strims_chat_v1_UntagResponse = UntagResponse;
/* @internal */
export type strims_chat_v1_UntagResponse = UntagResponse;
/* @internal */
export type strims_chat_v1_IUntagResponse = IUntagResponse;
/* @internal */
export const strims_chat_v1_SendMessageRequest = SendMessageRequest;
/* @internal */
export type strims_chat_v1_SendMessageRequest = SendMessageRequest;
/* @internal */
export type strims_chat_v1_ISendMessageRequest = ISendMessageRequest;
/* @internal */
export const strims_chat_v1_SendMessageResponse = SendMessageResponse;
/* @internal */
export type strims_chat_v1_SendMessageResponse = SendMessageResponse;
/* @internal */
export type strims_chat_v1_ISendMessageResponse = ISendMessageResponse;
/* @internal */
export const strims_chat_v1_MuteRequest = MuteRequest;
/* @internal */
export type strims_chat_v1_MuteRequest = MuteRequest;
/* @internal */
export type strims_chat_v1_IMuteRequest = IMuteRequest;
/* @internal */
export const strims_chat_v1_MuteResponse = MuteResponse;
/* @internal */
export type strims_chat_v1_MuteResponse = MuteResponse;
/* @internal */
export type strims_chat_v1_IMuteResponse = IMuteResponse;
/* @internal */
export const strims_chat_v1_UnmuteRequest = UnmuteRequest;
/* @internal */
export type strims_chat_v1_UnmuteRequest = UnmuteRequest;
/* @internal */
export type strims_chat_v1_IUnmuteRequest = IUnmuteRequest;
/* @internal */
export const strims_chat_v1_UnmuteResponse = UnmuteResponse;
/* @internal */
export type strims_chat_v1_UnmuteResponse = UnmuteResponse;
/* @internal */
export type strims_chat_v1_IUnmuteResponse = IUnmuteResponse;
/* @internal */
export const strims_chat_v1_GetMuteRequest = GetMuteRequest;
/* @internal */
export type strims_chat_v1_GetMuteRequest = GetMuteRequest;
/* @internal */
export type strims_chat_v1_IGetMuteRequest = IGetMuteRequest;
/* @internal */
export const strims_chat_v1_GetMuteResponse = GetMuteResponse;
/* @internal */
export type strims_chat_v1_GetMuteResponse = GetMuteResponse;
/* @internal */
export type strims_chat_v1_IGetMuteResponse = IGetMuteResponse;
/* @internal */
export const strims_chat_v1_WhisperThread = WhisperThread;
/* @internal */
export type strims_chat_v1_WhisperThread = WhisperThread;
/* @internal */
export type strims_chat_v1_IWhisperThread = IWhisperThread;
/* @internal */
export const strims_chat_v1_WhisperRecord = WhisperRecord;
/* @internal */
export type strims_chat_v1_WhisperRecord = WhisperRecord;
/* @internal */
export type strims_chat_v1_IWhisperRecord = IWhisperRecord;
/* @internal */
export const strims_chat_v1_WhisperSendMessageRequest = WhisperSendMessageRequest;
/* @internal */
export type strims_chat_v1_WhisperSendMessageRequest = WhisperSendMessageRequest;
/* @internal */
export type strims_chat_v1_IWhisperSendMessageRequest = IWhisperSendMessageRequest;
/* @internal */
export const strims_chat_v1_WhisperSendMessageResponse = WhisperSendMessageResponse;
/* @internal */
export type strims_chat_v1_WhisperSendMessageResponse = WhisperSendMessageResponse;
/* @internal */
export type strims_chat_v1_IWhisperSendMessageResponse = IWhisperSendMessageResponse;
/* @internal */
export const strims_chat_v1_EmoteEffect_CustomCSS = EmoteEffect.CustomCSS;
/* @internal */
export type strims_chat_v1_EmoteEffect_CustomCSS = EmoteEffect.CustomCSS;
/* @internal */
export type strims_chat_v1_EmoteEffect_ICustomCSS = EmoteEffect.ICustomCSS;
/* @internal */
export const strims_chat_v1_EmoteEffect_SpriteAnimation = EmoteEffect.SpriteAnimation;
/* @internal */
export type strims_chat_v1_EmoteEffect_SpriteAnimation = EmoteEffect.SpriteAnimation;
/* @internal */
export type strims_chat_v1_EmoteEffect_ISpriteAnimation = EmoteEffect.ISpriteAnimation;
/* @internal */
export const strims_chat_v1_EmoteEffect_DefaultModifiers = EmoteEffect.DefaultModifiers;
/* @internal */
export type strims_chat_v1_EmoteEffect_DefaultModifiers = EmoteEffect.DefaultModifiers;
/* @internal */
export type strims_chat_v1_EmoteEffect_IDefaultModifiers = EmoteEffect.IDefaultModifiers;
/* @internal */
export const strims_chat_v1_Message_Entities = Message.Entities;
/* @internal */
export type strims_chat_v1_Message_Entities = Message.Entities;
/* @internal */
export type strims_chat_v1_Message_IEntities = Message.IEntities;
/* @internal */
export const strims_chat_v1_Message_DirectoryRef = Message.DirectoryRef;
/* @internal */
export type strims_chat_v1_Message_DirectoryRef = Message.DirectoryRef;
/* @internal */
export type strims_chat_v1_Message_IDirectoryRef = Message.IDirectoryRef;
/* @internal */
export const strims_chat_v1_Message_Entities_Bounds = Message.Entities.Bounds;
/* @internal */
export type strims_chat_v1_Message_Entities_Bounds = Message.Entities.Bounds;
/* @internal */
export type strims_chat_v1_Message_Entities_IBounds = Message.Entities.IBounds;
/* @internal */
export const strims_chat_v1_Message_Entities_Link = Message.Entities.Link;
/* @internal */
export type strims_chat_v1_Message_Entities_Link = Message.Entities.Link;
/* @internal */
export type strims_chat_v1_Message_Entities_ILink = Message.Entities.ILink;
/* @internal */
export const strims_chat_v1_Message_Entities_Emote = Message.Entities.Emote;
/* @internal */
export type strims_chat_v1_Message_Entities_Emote = Message.Entities.Emote;
/* @internal */
export type strims_chat_v1_Message_Entities_IEmote = Message.Entities.IEmote;
/* @internal */
export const strims_chat_v1_Message_Entities_Emoji = Message.Entities.Emoji;
/* @internal */
export type strims_chat_v1_Message_Entities_Emoji = Message.Entities.Emoji;
/* @internal */
export type strims_chat_v1_Message_Entities_IEmoji = Message.Entities.IEmoji;
/* @internal */
export const strims_chat_v1_Message_Entities_Nick = Message.Entities.Nick;
/* @internal */
export type strims_chat_v1_Message_Entities_Nick = Message.Entities.Nick;
/* @internal */
export type strims_chat_v1_Message_Entities_INick = Message.Entities.INick;
/* @internal */
export const strims_chat_v1_Message_Entities_Tag = Message.Entities.Tag;
/* @internal */
export type strims_chat_v1_Message_Entities_Tag = Message.Entities.Tag;
/* @internal */
export type strims_chat_v1_Message_Entities_ITag = Message.Entities.ITag;
/* @internal */
export const strims_chat_v1_Message_Entities_CodeBlock = Message.Entities.CodeBlock;
/* @internal */
export type strims_chat_v1_Message_Entities_CodeBlock = Message.Entities.CodeBlock;
/* @internal */
export type strims_chat_v1_Message_Entities_ICodeBlock = Message.Entities.ICodeBlock;
/* @internal */
export const strims_chat_v1_Message_Entities_Spoiler = Message.Entities.Spoiler;
/* @internal */
export type strims_chat_v1_Message_Entities_Spoiler = Message.Entities.Spoiler;
/* @internal */
export type strims_chat_v1_Message_Entities_ISpoiler = Message.Entities.ISpoiler;
/* @internal */
export const strims_chat_v1_Message_Entities_GenericEntity = Message.Entities.GenericEntity;
/* @internal */
export type strims_chat_v1_Message_Entities_GenericEntity = Message.Entities.GenericEntity;
/* @internal */
export type strims_chat_v1_Message_Entities_IGenericEntity = Message.Entities.IGenericEntity;
/* @internal */
export const strims_chat_v1_Profile_Mute = Profile.Mute;
/* @internal */
export type strims_chat_v1_Profile_Mute = Profile.Mute;
/* @internal */
export type strims_chat_v1_Profile_IMute = Profile.IMute;
/* @internal */
export const strims_chat_v1_UIConfig_SoundFile = UIConfig.SoundFile;
/* @internal */
export type strims_chat_v1_UIConfig_SoundFile = UIConfig.SoundFile;
/* @internal */
export type strims_chat_v1_UIConfig_ISoundFile = UIConfig.ISoundFile;
/* @internal */
export const strims_chat_v1_OpenClientResponse_Open = OpenClientResponse.Open;
/* @internal */
export type strims_chat_v1_OpenClientResponse_Open = OpenClientResponse.Open;
/* @internal */
export type strims_chat_v1_OpenClientResponse_IOpen = OpenClientResponse.IOpen;
/* @internal */
export const strims_chat_v1_OpenClientResponse_ServerEvents = OpenClientResponse.ServerEvents;
/* @internal */
export type strims_chat_v1_OpenClientResponse_ServerEvents = OpenClientResponse.ServerEvents;
/* @internal */
export type strims_chat_v1_OpenClientResponse_IServerEvents = OpenClientResponse.IServerEvents;
/* @internal */
export const strims_chat_v1_WatchWhispersResponse_WhisperThreadDelete = WatchWhispersResponse.WhisperThreadDelete;
/* @internal */
export type strims_chat_v1_WatchWhispersResponse_WhisperThreadDelete = WatchWhispersResponse.WhisperThreadDelete;
/* @internal */
export type strims_chat_v1_WatchWhispersResponse_IWhisperThreadDelete = WatchWhispersResponse.IWhisperThreadDelete;
/* @internal */
export const strims_chat_v1_WatchWhispersResponse_WhisperDelete = WatchWhispersResponse.WhisperDelete;
/* @internal */
export type strims_chat_v1_WatchWhispersResponse_WhisperDelete = WatchWhispersResponse.WhisperDelete;
/* @internal */
export type strims_chat_v1_WatchWhispersResponse_IWhisperDelete = WatchWhispersResponse.IWhisperDelete;
/* @internal */
export const strims_chat_v1_EmoteFileType = EmoteFileType;
/* @internal */
export type strims_chat_v1_EmoteFileType = EmoteFileType;
/* @internal */
export const strims_chat_v1_EmoteScale = EmoteScale;
/* @internal */
export type strims_chat_v1_EmoteScale = EmoteScale;
/* @internal */
export const strims_chat_v1_UIConfig_ShowRemoved = UIConfig.ShowRemoved;
/* @internal */
export type strims_chat_v1_UIConfig_ShowRemoved = UIConfig.ShowRemoved;
/* @internal */
export const strims_chat_v1_UIConfig_UserPresenceIndicator = UIConfig.UserPresenceIndicator;
/* @internal */
export type strims_chat_v1_UIConfig_UserPresenceIndicator = UIConfig.UserPresenceIndicator;
/* @internal */
export const strims_chat_v1_ListEmotesRequest_Part = ListEmotesRequest.Part;
/* @internal */
export type strims_chat_v1_ListEmotesRequest_Part = ListEmotesRequest.Part;
/* @internal */
export const strims_chat_v1_WhisperRecord_State = WhisperRecord.State;
/* @internal */
export type strims_chat_v1_WhisperRecord_State = WhisperRecord.State;

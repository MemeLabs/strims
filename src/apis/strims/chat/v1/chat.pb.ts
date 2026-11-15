// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  Key as strims_type_Key,
  type IKey as strims_type_IKey,
} from "../../type/key.pb";
import {
  Image as strims_type_Image,
  type IImage as strims_type_IImage,
} from "../../type/image.pb";
import {
  Listing as strims_network_v1_directory_Listing,
  type IListing as strims_network_v1_directory_IListing,
} from "../../network/v1/directory/directory.pb";
import {
  VersionVector as strims_dao_v1_VersionVector,
  type IVersionVector as strims_dao_v1_IVersionVector,
} from "../../dao/v1/dao.pb";
import {
  BoolValue as google_protobuf_BoolValue,
  type IBoolValue as google_protobuf_IBoolValue,
} from "../../../google/protobuf/wrappers.pb";

export type IServerEvent = {
  body?: ServerEvent_IBody
}

export class ServerEvent {
  body: ServerEvent_TBody;

  constructor(v?: IServerEvent) {
    this.body = v?.body ? new ServerEvent_Body(v.body) : new ServerEvent_Body();
  }

  static encode(m: ServerEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case ServerEvent_BodyCase.MESSAGE:
      Message.encode(m.body.message, w.uint32(8010).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ServerEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ServerEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1001:
        m.body = new ServerEvent_Body({ message: Message.decode(r, r.uint32()) });
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

export enum ServerEvent_BodyCase {
  NOT_SET = 0,
  MESSAGE = 1001,
}

export type ServerEvent_IBody =
{ case?: ServerEvent_BodyCase.NOT_SET }
|{ case?: ServerEvent_BodyCase.MESSAGE, message: IMessage }
;

export type ServerEvent_TBody = Readonly<
{ case: ServerEvent_BodyCase.NOT_SET }
|{ case: ServerEvent_BodyCase.MESSAGE, message: Message }
>;

class ServerEvent_BodyImpl {
  declare message?: Message;
  case: ServerEvent_BodyCase = ServerEvent_BodyCase.NOT_SET;

  constructor(v?: ServerEvent_IBody) {
    if (v && "message" in v) {
      this.case = ServerEvent_BodyCase.MESSAGE;
      this.message = new Message(v.message);
    }
  }
}

export const ServerEvent_Body = ServerEvent_BodyImpl as {
  new (): Readonly<{ case: ServerEvent_BodyCase.NOT_SET }>;
  new <T extends ServerEvent_IBody>(v: T): Readonly<
  T extends { message: IMessage } ? { case: ServerEvent_BodyCase.MESSAGE, message: Message } :
  never
  >;
};

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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IServerIcon = {
  id?: bigint;
  image?: strims_type_IImage;
}

export class ServerIcon {
  id: bigint;
  image: strims_type_Image | undefined;

  constructor(v?: IServerIcon) {
    this.id = v?.id || BigInt(0);
    this.image = v?.image && new strims_type_Image(v.image);
  }

  static encode(m: ServerIcon, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.image) strims_type_Image.encode(m.image, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ServerIcon {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ServerIcon();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 3:
        m.image = strims_type_Image.decode(r, r.uint32());
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

export type IStyleSheet = {
  scss?: string;
  css?: string;
  assets?: StyleSheet_IAsset[];
}

export class StyleSheet {
  scss: string;
  css: string;
  assets: StyleSheet_Asset[];

  constructor(v?: IStyleSheet) {
    this.scss = v?.scss || "";
    this.css = v?.css || "";
    this.assets = v?.assets ? v.assets.map(v => new StyleSheet_Asset(v)) : [];
  }

  static encode(m: StyleSheet, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.scss.length) w.uint32(10).string(m.scss);
    if (m.css.length) w.uint32(18).string(m.css);
    for (const v of m.assets) StyleSheet_Asset.encode(v, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): StyleSheet {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new StyleSheet();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.scss = r.string();
        break;
        case 2:
        m.css = r.string();
        break;
        case 3:
        m.assets.push(StyleSheet_Asset.decode(r, r.uint32()));
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

export type StyleSheet_IAsset = {
  name?: string;
  image?: strims_type_IImage;
}

export class StyleSheet_Asset {
  name: string;
  image: strims_type_Image | undefined;

  constructor(v?: StyleSheet_IAsset) {
    this.name = v?.name || "";
    this.image = v?.image && new strims_type_Image(v.image);
  }

  static encode(m: StyleSheet_Asset, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.name.length) w.uint32(10).string(m.name);
    if (m.image) strims_type_Image.encode(m.image, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): StyleSheet_Asset {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new StyleSheet_Asset();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.name = r.string();
        break;
        case 2:
        m.image = strims_type_Image.decode(r, r.uint32());
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IEmoteEffect = {
  effect?: EmoteEffect_IEffect
}

export class EmoteEffect {
  effect: EmoteEffect_TEffect;

  constructor(v?: IEmoteEffect) {
    this.effect = v?.effect ? new EmoteEffect_Effect(v.effect) : new EmoteEffect_Effect();
  }

  static encode(m: EmoteEffect, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.effect.case) {
      case EmoteEffect_EffectCase.CUSTOM_CSS:
      EmoteEffect_CustomCSS.encode(m.effect.customCss, w.uint32(8010).fork()).ldelim();
      break;
      case EmoteEffect_EffectCase.SPRITE_ANIMATION:
      EmoteEffect_SpriteAnimation.encode(m.effect.spriteAnimation, w.uint32(8018).fork()).ldelim();
      break;
      case EmoteEffect_EffectCase.DEFAULT_MODIFIERS:
      EmoteEffect_DefaultModifiers.encode(m.effect.defaultModifiers, w.uint32(8026).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): EmoteEffect {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new EmoteEffect();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1001:
        m.effect = new EmoteEffect_Effect({ customCss: EmoteEffect_CustomCSS.decode(r, r.uint32()) });
        break;
        case 1002:
        m.effect = new EmoteEffect_Effect({ spriteAnimation: EmoteEffect_SpriteAnimation.decode(r, r.uint32()) });
        break;
        case 1003:
        m.effect = new EmoteEffect_Effect({ defaultModifiers: EmoteEffect_DefaultModifiers.decode(r, r.uint32()) });
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

export enum EmoteEffect_EffectCase {
  NOT_SET = 0,
  CUSTOM_CSS = 1001,
  SPRITE_ANIMATION = 1002,
  DEFAULT_MODIFIERS = 1003,
}

export type EmoteEffect_IEffect =
{ case?: EmoteEffect_EffectCase.NOT_SET }
|{ case?: EmoteEffect_EffectCase.CUSTOM_CSS, customCss: EmoteEffect_ICustomCSS }
|{ case?: EmoteEffect_EffectCase.SPRITE_ANIMATION, spriteAnimation: EmoteEffect_ISpriteAnimation }
|{ case?: EmoteEffect_EffectCase.DEFAULT_MODIFIERS, defaultModifiers: EmoteEffect_IDefaultModifiers }
;

export type EmoteEffect_TEffect = Readonly<
{ case: EmoteEffect_EffectCase.NOT_SET }
|{ case: EmoteEffect_EffectCase.CUSTOM_CSS, customCss: EmoteEffect_CustomCSS }
|{ case: EmoteEffect_EffectCase.SPRITE_ANIMATION, spriteAnimation: EmoteEffect_SpriteAnimation }
|{ case: EmoteEffect_EffectCase.DEFAULT_MODIFIERS, defaultModifiers: EmoteEffect_DefaultModifiers }
>;

class EmoteEffect_EffectImpl {
  declare customCss?: EmoteEffect_CustomCSS;
  declare spriteAnimation?: EmoteEffect_SpriteAnimation;
  declare defaultModifiers?: EmoteEffect_DefaultModifiers;
  case: EmoteEffect_EffectCase = EmoteEffect_EffectCase.NOT_SET;

  constructor(v?: EmoteEffect_IEffect) {
    if (v && "customCss" in v) {
      this.case = EmoteEffect_EffectCase.CUSTOM_CSS;
      this.customCss = new EmoteEffect_CustomCSS(v.customCss);
    } else
    if (v && "spriteAnimation" in v) {
      this.case = EmoteEffect_EffectCase.SPRITE_ANIMATION;
      this.spriteAnimation = new EmoteEffect_SpriteAnimation(v.spriteAnimation);
    } else
    if (v && "defaultModifiers" in v) {
      this.case = EmoteEffect_EffectCase.DEFAULT_MODIFIERS;
      this.defaultModifiers = new EmoteEffect_DefaultModifiers(v.defaultModifiers);
    }
  }
}

export const EmoteEffect_Effect = EmoteEffect_EffectImpl as {
  new (): Readonly<{ case: EmoteEffect_EffectCase.NOT_SET }>;
  new <T extends EmoteEffect_IEffect>(v: T): Readonly<
  T extends { customCss: EmoteEffect_ICustomCSS } ? { case: EmoteEffect_EffectCase.CUSTOM_CSS, customCss: EmoteEffect_CustomCSS } :
  T extends { spriteAnimation: EmoteEffect_ISpriteAnimation } ? { case: EmoteEffect_EffectCase.SPRITE_ANIMATION, spriteAnimation: EmoteEffect_SpriteAnimation } :
  T extends { defaultModifiers: EmoteEffect_IDefaultModifiers } ? { case: EmoteEffect_EffectCase.DEFAULT_MODIFIERS, defaultModifiers: EmoteEffect_DefaultModifiers } :
  never
  >;
};

export type EmoteEffect_ICustomCSS = {
  styleSheet?: IStyleSheet;
  extraWrapCount?: number;
  wrapAdjacent?: boolean;
}

export class EmoteEffect_CustomCSS {
  styleSheet: StyleSheet | undefined;
  extraWrapCount: number;
  wrapAdjacent: boolean;

  constructor(v?: EmoteEffect_ICustomCSS) {
    this.styleSheet = v?.styleSheet && new StyleSheet(v.styleSheet);
    this.extraWrapCount = v?.extraWrapCount || 0;
    this.wrapAdjacent = v?.wrapAdjacent || false;
  }

  static encode(m: EmoteEffect_CustomCSS, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.styleSheet) StyleSheet.encode(m.styleSheet, w.uint32(10).fork()).ldelim();
    if (m.extraWrapCount) w.uint32(16).uint32(m.extraWrapCount);
    if (m.wrapAdjacent) w.uint32(24).bool(m.wrapAdjacent);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): EmoteEffect_CustomCSS {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new EmoteEffect_CustomCSS();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.styleSheet = StyleSheet.decode(r, r.uint32());
        break;
        case 2:
        m.extraWrapCount = r.uint32();
        break;
        case 3:
        m.wrapAdjacent = r.bool();
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

export type EmoteEffect_ISpriteAnimation = {
  frameCount?: number;
  durationMs?: number;
  iterationCount?: number;
  endOnFrame?: number;
  loopForever?: boolean;
  alternateDirection?: boolean;
}

export class EmoteEffect_SpriteAnimation {
  frameCount: number;
  durationMs: number;
  iterationCount: number;
  endOnFrame: number;
  loopForever: boolean;
  alternateDirection: boolean;

  constructor(v?: EmoteEffect_ISpriteAnimation) {
    this.frameCount = v?.frameCount || 0;
    this.durationMs = v?.durationMs || 0;
    this.iterationCount = v?.iterationCount || 0;
    this.endOnFrame = v?.endOnFrame || 0;
    this.loopForever = v?.loopForever || false;
    this.alternateDirection = v?.alternateDirection || false;
  }

  static encode(m: EmoteEffect_SpriteAnimation, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.frameCount) w.uint32(8).uint32(m.frameCount);
    if (m.durationMs) w.uint32(16).uint32(m.durationMs);
    if (m.iterationCount) w.uint32(24).uint32(m.iterationCount);
    if (m.endOnFrame) w.uint32(32).uint32(m.endOnFrame);
    if (m.loopForever) w.uint32(40).bool(m.loopForever);
    if (m.alternateDirection) w.uint32(48).bool(m.alternateDirection);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): EmoteEffect_SpriteAnimation {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new EmoteEffect_SpriteAnimation();
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type EmoteEffect_IDefaultModifiers = {
  modifiers?: string[];
}

export class EmoteEffect_DefaultModifiers {
  modifiers: string[];

  constructor(v?: EmoteEffect_IDefaultModifiers) {
    this.modifiers = v?.modifiers ? v.modifiers : [];
  }

  static encode(m: EmoteEffect_DefaultModifiers, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.modifiers) w.uint32(10).string(v);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): EmoteEffect_DefaultModifiers {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new EmoteEffect_DefaultModifiers();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.modifiers.push(r.string())
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
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
  labels?: string[];
  enable?: boolean;
}

export class Emote {
  id: bigint;
  serverId: bigint;
  name: string;
  images: EmoteImage[];
  effects: EmoteEffect[];
  contributor: EmoteContributor | undefined;
  labels: string[];
  enable: boolean;

  constructor(v?: IEmote) {
    this.id = v?.id || BigInt(0);
    this.serverId = v?.serverId || BigInt(0);
    this.name = v?.name || "";
    this.images = v?.images ? v.images.map(v => new EmoteImage(v)) : [];
    this.effects = v?.effects ? v.effects.map(v => new EmoteEffect(v)) : [];
    this.contributor = v?.contributor && new EmoteContributor(v.contributor);
    this.labels = v?.labels ? v.labels : [];
    this.enable = v?.enable || false;
  }

  static encode(m: Emote, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.serverId) w.uint32(16).uint64(m.serverId);
    if (m.name.length) w.uint32(26).string(m.name);
    for (const v of m.images) EmoteImage.encode(v, w.uint32(34).fork()).ldelim();
    for (const v of m.effects) EmoteEffect.encode(v, w.uint32(42).fork()).ldelim();
    if (m.contributor) EmoteContributor.encode(m.contributor, w.uint32(50).fork()).ldelim();
    for (const v of m.labels) w.uint32(58).string(v);
    if (m.enable) w.uint32(64).bool(m.enable);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Emote {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
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
        case 7:
        m.labels.push(r.string())
        break;
        case 8:
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

export type IModifier = {
  id?: bigint;
  serverId?: bigint;
  name?: string;
  priority?: number;
  internal?: boolean;
  extraWrapCount?: number;
  procChance?: number;
  styleSheet?: IStyleSheet;
}

export class Modifier {
  id: bigint;
  serverId: bigint;
  name: string;
  priority: number;
  internal: boolean;
  extraWrapCount: number;
  procChance: number;
  styleSheet: StyleSheet | undefined;

  constructor(v?: IModifier) {
    this.id = v?.id || BigInt(0);
    this.serverId = v?.serverId || BigInt(0);
    this.name = v?.name || "";
    this.priority = v?.priority || 0;
    this.internal = v?.internal || false;
    this.extraWrapCount = v?.extraWrapCount || 0;
    this.procChance = v?.procChance || 0;
    this.styleSheet = v?.styleSheet && new StyleSheet(v.styleSheet);
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
    if (m.styleSheet) StyleSheet.encode(m.styleSheet, w.uint32(66).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Modifier {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
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
        case 8:
        m.styleSheet = StyleSheet.decode(r, r.uint32());
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
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
  icon?: strims_type_IImage;
}

export class AssetBundle {
  isDelta: boolean;
  removedIds: bigint[];
  room: Room | undefined;
  emotes: Emote[];
  modifiers: Modifier[];
  tags: Tag[];
  icon: strims_type_Image | undefined;

  constructor(v?: IAssetBundle) {
    this.isDelta = v?.isDelta || false;
    this.removedIds = v?.removedIds ? v.removedIds : [];
    this.room = v?.room && new Room(v.room);
    this.emotes = v?.emotes ? v.emotes.map(v => new Emote(v)) : [];
    this.modifiers = v?.modifiers ? v.modifiers.map(v => new Modifier(v)) : [];
    this.tags = v?.tags ? v.tags.map(v => new Tag(v)) : [];
    this.icon = v?.icon && new strims_type_Image(v.icon);
  }

  static encode(m: AssetBundle, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.isDelta) w.uint32(8).bool(m.isDelta);
    m.removedIds.reduce((w, v) => w.uint64(v), w.uint32(18).fork()).ldelim();
    if (m.room) Room.encode(m.room, w.uint32(26).fork()).ldelim();
    for (const v of m.emotes) Emote.encode(v, w.uint32(34).fork()).ldelim();
    for (const v of m.modifiers) Modifier.encode(v, w.uint32(42).fork()).ldelim();
    for (const v of m.tags) Tag.encode(v, w.uint32(50).fork()).ldelim();
    if (m.icon) strims_type_Image.encode(m.icon, w.uint32(58).fork()).ldelim();
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
        case 7:
        m.icon = strims_type_Image.decode(r, r.uint32());
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

export type IMessage = {
  serverTime?: bigint;
  peerKey?: Uint8Array;
  nick?: string;
  body?: string;
  entities?: Message_IEntities;
  viewedListing?: Message_IDirectoryRef;
}

export class Message {
  serverTime: bigint;
  peerKey: Uint8Array;
  nick: string;
  body: string;
  entities: Message_Entities | undefined;
  viewedListing: Message_DirectoryRef | undefined;

  constructor(v?: IMessage) {
    this.serverTime = v?.serverTime || BigInt(0);
    this.peerKey = v?.peerKey || new Uint8Array();
    this.nick = v?.nick || "";
    this.body = v?.body || "";
    this.entities = v?.entities && new Message_Entities(v.entities);
    this.viewedListing = v?.viewedListing && new Message_DirectoryRef(v.viewedListing);
  }

  static encode(m: Message, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverTime) w.uint32(8).int64(m.serverTime);
    if (m.peerKey.length) w.uint32(18).bytes(m.peerKey);
    if (m.nick.length) w.uint32(26).string(m.nick);
    if (m.body.length) w.uint32(34).string(m.body);
    if (m.entities) Message_Entities.encode(m.entities, w.uint32(42).fork()).ldelim();
    if (m.viewedListing) Message_DirectoryRef.encode(m.viewedListing, w.uint32(74).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Message {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
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
        m.entities = Message_Entities.decode(r, r.uint32());
        break;
        case 9:
        m.viewedListing = Message_DirectoryRef.decode(r, r.uint32());
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

export type Message_IEntities = {
  links?: Message_Entities_ILink[];
  emotes?: Message_Entities_IEmote[];
  emojis?: Message_Entities_IEmoji[];
  nicks?: Message_Entities_INick[];
  tags?: Message_Entities_ITag[];
  codeBlocks?: Message_Entities_ICodeBlock[];
  spoilers?: Message_Entities_ISpoiler[];
  greenText?: Message_Entities_IGenericEntity;
  selfMessage?: Message_Entities_IGenericEntity;
}

export class Message_Entities {
  links: Message_Entities_Link[];
  emotes: Message_Entities_Emote[];
  emojis: Message_Entities_Emoji[];
  nicks: Message_Entities_Nick[];
  tags: Message_Entities_Tag[];
  codeBlocks: Message_Entities_CodeBlock[];
  spoilers: Message_Entities_Spoiler[];
  greenText: Message_Entities_GenericEntity | undefined;
  selfMessage: Message_Entities_GenericEntity | undefined;

  constructor(v?: Message_IEntities) {
    this.links = v?.links ? v.links.map(v => new Message_Entities_Link(v)) : [];
    this.emotes = v?.emotes ? v.emotes.map(v => new Message_Entities_Emote(v)) : [];
    this.emojis = v?.emojis ? v.emojis.map(v => new Message_Entities_Emoji(v)) : [];
    this.nicks = v?.nicks ? v.nicks.map(v => new Message_Entities_Nick(v)) : [];
    this.tags = v?.tags ? v.tags.map(v => new Message_Entities_Tag(v)) : [];
    this.codeBlocks = v?.codeBlocks ? v.codeBlocks.map(v => new Message_Entities_CodeBlock(v)) : [];
    this.spoilers = v?.spoilers ? v.spoilers.map(v => new Message_Entities_Spoiler(v)) : [];
    this.greenText = v?.greenText && new Message_Entities_GenericEntity(v.greenText);
    this.selfMessage = v?.selfMessage && new Message_Entities_GenericEntity(v.selfMessage);
  }

  static encode(m: Message_Entities, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.links) Message_Entities_Link.encode(v, w.uint32(10).fork()).ldelim();
    for (const v of m.emotes) Message_Entities_Emote.encode(v, w.uint32(18).fork()).ldelim();
    for (const v of m.emojis) Message_Entities_Emoji.encode(v, w.uint32(26).fork()).ldelim();
    for (const v of m.nicks) Message_Entities_Nick.encode(v, w.uint32(34).fork()).ldelim();
    for (const v of m.tags) Message_Entities_Tag.encode(v, w.uint32(42).fork()).ldelim();
    for (const v of m.codeBlocks) Message_Entities_CodeBlock.encode(v, w.uint32(50).fork()).ldelim();
    for (const v of m.spoilers) Message_Entities_Spoiler.encode(v, w.uint32(58).fork()).ldelim();
    if (m.greenText) Message_Entities_GenericEntity.encode(m.greenText, w.uint32(66).fork()).ldelim();
    if (m.selfMessage) Message_Entities_GenericEntity.encode(m.selfMessage, w.uint32(74).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Message_Entities {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Message_Entities();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.links.push(Message_Entities_Link.decode(r, r.uint32()));
        break;
        case 2:
        m.emotes.push(Message_Entities_Emote.decode(r, r.uint32()));
        break;
        case 3:
        m.emojis.push(Message_Entities_Emoji.decode(r, r.uint32()));
        break;
        case 4:
        m.nicks.push(Message_Entities_Nick.decode(r, r.uint32()));
        break;
        case 5:
        m.tags.push(Message_Entities_Tag.decode(r, r.uint32()));
        break;
        case 6:
        m.codeBlocks.push(Message_Entities_CodeBlock.decode(r, r.uint32()));
        break;
        case 7:
        m.spoilers.push(Message_Entities_Spoiler.decode(r, r.uint32()));
        break;
        case 8:
        m.greenText = Message_Entities_GenericEntity.decode(r, r.uint32());
        break;
        case 9:
        m.selfMessage = Message_Entities_GenericEntity.decode(r, r.uint32());
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

export type Message_Entities_IBounds = {
  start?: number;
  end?: number;
}

export class Message_Entities_Bounds {
  start: number;
  end: number;

  constructor(v?: Message_Entities_IBounds) {
    this.start = v?.start || 0;
    this.end = v?.end || 0;
  }

  static encode(m: Message_Entities_Bounds, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.start) w.uint32(8).uint32(m.start);
    if (m.end) w.uint32(16).uint32(m.end);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Message_Entities_Bounds {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Message_Entities_Bounds();
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type Message_Entities_ILink = {
  bounds?: Message_Entities_IBounds;
  url?: string;
}

export class Message_Entities_Link {
  bounds: Message_Entities_Bounds | undefined;
  url: string;

  constructor(v?: Message_Entities_ILink) {
    this.bounds = v?.bounds && new Message_Entities_Bounds(v.bounds);
    this.url = v?.url || "";
  }

  static encode(m: Message_Entities_Link, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.bounds) Message_Entities_Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
    if (m.url.length) w.uint32(18).string(m.url);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Message_Entities_Link {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Message_Entities_Link();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.bounds = Message_Entities_Bounds.decode(r, r.uint32());
        break;
        case 2:
        m.url = r.string();
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

export type Message_Entities_IEmote = {
  bounds?: Message_Entities_IBounds;
  name?: string;
  modifiers?: string[];
  combo?: number;
  canCombo?: boolean;
}

export class Message_Entities_Emote {
  bounds: Message_Entities_Bounds | undefined;
  name: string;
  modifiers: string[];
  combo: number;
  canCombo: boolean;

  constructor(v?: Message_Entities_IEmote) {
    this.bounds = v?.bounds && new Message_Entities_Bounds(v.bounds);
    this.name = v?.name || "";
    this.modifiers = v?.modifiers ? v.modifiers : [];
    this.combo = v?.combo || 0;
    this.canCombo = v?.canCombo || false;
  }

  static encode(m: Message_Entities_Emote, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.bounds) Message_Entities_Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
    if (m.name.length) w.uint32(18).string(m.name);
    for (const v of m.modifiers) w.uint32(26).string(v);
    if (m.combo) w.uint32(32).uint32(m.combo);
    if (m.canCombo) w.uint32(40).bool(m.canCombo);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Message_Entities_Emote {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Message_Entities_Emote();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.bounds = Message_Entities_Bounds.decode(r, r.uint32());
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type Message_Entities_IEmoji = {
  bounds?: Message_Entities_IBounds;
}

export class Message_Entities_Emoji {
  bounds: Message_Entities_Bounds | undefined;

  constructor(v?: Message_Entities_IEmoji) {
    this.bounds = v?.bounds && new Message_Entities_Bounds(v.bounds);
  }

  static encode(m: Message_Entities_Emoji, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.bounds) Message_Entities_Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Message_Entities_Emoji {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Message_Entities_Emoji();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.bounds = Message_Entities_Bounds.decode(r, r.uint32());
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

export type Message_Entities_INick = {
  bounds?: Message_Entities_IBounds;
  nick?: string;
  peerKey?: Uint8Array;
  viewedListing?: Message_IDirectoryRef;
}

export class Message_Entities_Nick {
  bounds: Message_Entities_Bounds | undefined;
  nick: string;
  peerKey: Uint8Array;
  viewedListing: Message_DirectoryRef | undefined;

  constructor(v?: Message_Entities_INick) {
    this.bounds = v?.bounds && new Message_Entities_Bounds(v.bounds);
    this.nick = v?.nick || "";
    this.peerKey = v?.peerKey || new Uint8Array();
    this.viewedListing = v?.viewedListing && new Message_DirectoryRef(v.viewedListing);
  }

  static encode(m: Message_Entities_Nick, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.bounds) Message_Entities_Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
    if (m.nick.length) w.uint32(18).string(m.nick);
    if (m.peerKey.length) w.uint32(26).bytes(m.peerKey);
    if (m.viewedListing) Message_DirectoryRef.encode(m.viewedListing, w.uint32(34).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Message_Entities_Nick {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Message_Entities_Nick();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.bounds = Message_Entities_Bounds.decode(r, r.uint32());
        break;
        case 2:
        m.nick = r.string();
        break;
        case 3:
        m.peerKey = r.bytes();
        break;
        case 4:
        m.viewedListing = Message_DirectoryRef.decode(r, r.uint32());
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

export type Message_Entities_ITag = {
  bounds?: Message_Entities_IBounds;
  name?: string;
}

export class Message_Entities_Tag {
  bounds: Message_Entities_Bounds | undefined;
  name: string;

  constructor(v?: Message_Entities_ITag) {
    this.bounds = v?.bounds && new Message_Entities_Bounds(v.bounds);
    this.name = v?.name || "";
  }

  static encode(m: Message_Entities_Tag, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.bounds) Message_Entities_Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
    if (m.name.length) w.uint32(18).string(m.name);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Message_Entities_Tag {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Message_Entities_Tag();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.bounds = Message_Entities_Bounds.decode(r, r.uint32());
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

export type Message_Entities_ICodeBlock = {
  bounds?: Message_Entities_IBounds;
}

export class Message_Entities_CodeBlock {
  bounds: Message_Entities_Bounds | undefined;

  constructor(v?: Message_Entities_ICodeBlock) {
    this.bounds = v?.bounds && new Message_Entities_Bounds(v.bounds);
  }

  static encode(m: Message_Entities_CodeBlock, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.bounds) Message_Entities_Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Message_Entities_CodeBlock {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Message_Entities_CodeBlock();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.bounds = Message_Entities_Bounds.decode(r, r.uint32());
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

export type Message_Entities_ISpoiler = {
  bounds?: Message_Entities_IBounds;
}

export class Message_Entities_Spoiler {
  bounds: Message_Entities_Bounds | undefined;

  constructor(v?: Message_Entities_ISpoiler) {
    this.bounds = v?.bounds && new Message_Entities_Bounds(v.bounds);
  }

  static encode(m: Message_Entities_Spoiler, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.bounds) Message_Entities_Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Message_Entities_Spoiler {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Message_Entities_Spoiler();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.bounds = Message_Entities_Bounds.decode(r, r.uint32());
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

export type Message_Entities_IGenericEntity = {
  bounds?: Message_Entities_IBounds;
}

export class Message_Entities_GenericEntity {
  bounds: Message_Entities_Bounds | undefined;

  constructor(v?: Message_Entities_IGenericEntity) {
    this.bounds = v?.bounds && new Message_Entities_Bounds(v.bounds);
  }

  static encode(m: Message_Entities_GenericEntity, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.bounds) Message_Entities_Bounds.encode(m.bounds, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Message_Entities_GenericEntity {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Message_Entities_GenericEntity();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.bounds = Message_Entities_Bounds.decode(r, r.uint32());
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

export type Message_IDirectoryRef = {
  directoryId?: bigint;
  networkKey?: Uint8Array;
  listing?: strims_network_v1_directory_IListing;
  themeColor?: number;
}

export class Message_DirectoryRef {
  directoryId: bigint;
  networkKey: Uint8Array;
  listing: strims_network_v1_directory_Listing | undefined;
  themeColor: number;

  constructor(v?: Message_IDirectoryRef) {
    this.directoryId = v?.directoryId || BigInt(0);
    this.networkKey = v?.networkKey || new Uint8Array();
    this.listing = v?.listing && new strims_network_v1_directory_Listing(v.listing);
    this.themeColor = v?.themeColor || 0;
  }

  static encode(m: Message_DirectoryRef, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.directoryId) w.uint32(8).uint64(m.directoryId);
    if (m.networkKey.length) w.uint32(18).bytes(m.networkKey);
    if (m.listing) strims_network_v1_directory_Listing.encode(m.listing, w.uint32(26).fork()).ldelim();
    if (m.themeColor) w.uint32(32).uint32(m.themeColor);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Message_DirectoryRef {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Message_DirectoryRef();
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
  serverId?: bigint;
  peerKey?: Uint8Array;
  alias?: string;
  mutes?: Profile_IMute[];
  muteDeadline?: bigint;
}

export class Profile {
  id: bigint;
  serverId: bigint;
  peerKey: Uint8Array;
  alias: string;
  mutes: Profile_Mute[];
  muteDeadline: bigint;

  constructor(v?: IProfile) {
    this.id = v?.id || BigInt(0);
    this.serverId = v?.serverId || BigInt(0);
    this.peerKey = v?.peerKey || new Uint8Array();
    this.alias = v?.alias || "";
    this.mutes = v?.mutes ? v.mutes.map(v => new Profile_Mute(v)) : [];
    this.muteDeadline = v?.muteDeadline || BigInt(0);
  }

  static encode(m: Profile, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.serverId) w.uint32(16).uint64(m.serverId);
    if (m.peerKey.length) w.uint32(26).bytes(m.peerKey);
    if (m.alias.length) w.uint32(34).string(m.alias);
    for (const v of m.mutes) Profile_Mute.encode(v, w.uint32(42).fork()).ldelim();
    if (m.muteDeadline) w.uint32(48).int64(m.muteDeadline);
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
        m.serverId = r.uint64();
        break;
        case 3:
        m.peerKey = r.bytes();
        break;
        case 4:
        m.alias = r.string();
        break;
        case 5:
        m.mutes.push(Profile_Mute.decode(r, r.uint32()));
        break;
        case 6:
        m.muteDeadline = r.int64();
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

export type Profile_IMute = {
  createdAt?: bigint;
  durationSecs?: number;
  message?: string;
  moderatorPeerKey?: Uint8Array;
}

export class Profile_Mute {
  createdAt: bigint;
  durationSecs: number;
  message: string;
  moderatorPeerKey: Uint8Array;

  constructor(v?: Profile_IMute) {
    this.createdAt = v?.createdAt || BigInt(0);
    this.durationSecs = v?.durationSecs || 0;
    this.message = v?.message || "";
    this.moderatorPeerKey = v?.moderatorPeerKey || new Uint8Array();
  }

  static encode(m: Profile_Mute, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.createdAt) w.uint32(8).int64(m.createdAt);
    if (m.durationSecs) w.uint32(16).uint32(m.durationSecs);
    if (m.message.length) w.uint32(26).string(m.message);
    if (m.moderatorPeerKey.length) w.uint32(34).bytes(m.moderatorPeerKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Profile_Mute {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Profile_Mute();
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IUIConfig = {
  version?: strims_dao_v1_IVersionVector;
  showTime?: boolean;
  showFlairIcons?: boolean;
  timestampFormat?: string;
  maxLines?: number;
  notificationWhisper?: boolean;
  soundNotificationWhisper?: boolean;
  notificationHighlight?: boolean;
  soundNotificationHighlight?: boolean;
  notificationSoundFile?: UIConfig_ISoundFile;
  highlight?: boolean;
  customHighlight?: string[];
  showRemoved?: UIConfig_ShowRemoved;
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
  userPresenceIndicator?: UIConfig_UserPresenceIndicator;
  hiddenEmotes?: string[];
  shortenLinks?: boolean;
  compactEmoteSpacing?: boolean;
  normalizeAliasCase?: boolean;
  emojiSkinTone?: string;
  replicate?: boolean;
}

export class UIConfig {
  version: strims_dao_v1_VersionVector | undefined;
  showTime: boolean;
  showFlairIcons: boolean;
  timestampFormat: string;
  maxLines: number;
  notificationWhisper: boolean;
  soundNotificationWhisper: boolean;
  notificationHighlight: boolean;
  soundNotificationHighlight: boolean;
  notificationSoundFile: UIConfig_SoundFile | undefined;
  highlight: boolean;
  customHighlight: string[];
  showRemoved: UIConfig_ShowRemoved;
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
  userPresenceIndicator: UIConfig_UserPresenceIndicator;
  hiddenEmotes: string[];
  shortenLinks: boolean;
  compactEmoteSpacing: boolean;
  normalizeAliasCase: boolean;
  emojiSkinTone: string;
  replicate: boolean;

  constructor(v?: IUIConfig) {
    this.version = v?.version && new strims_dao_v1_VersionVector(v.version);
    this.showTime = v?.showTime || false;
    this.showFlairIcons = v?.showFlairIcons || false;
    this.timestampFormat = v?.timestampFormat || "";
    this.maxLines = v?.maxLines || 0;
    this.notificationWhisper = v?.notificationWhisper || false;
    this.soundNotificationWhisper = v?.soundNotificationWhisper || false;
    this.notificationHighlight = v?.notificationHighlight || false;
    this.soundNotificationHighlight = v?.soundNotificationHighlight || false;
    this.notificationSoundFile = v?.notificationSoundFile && new UIConfig_SoundFile(v.notificationSoundFile);
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
    this.replicate = v?.replicate || false;
  }

  static encode(m: UIConfig, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.version) strims_dao_v1_VersionVector.encode(m.version, w.uint32(290).fork()).ldelim();
    if (m.showTime) w.uint32(8).bool(m.showTime);
    if (m.showFlairIcons) w.uint32(16).bool(m.showFlairIcons);
    if (m.timestampFormat.length) w.uint32(26).string(m.timestampFormat);
    if (m.maxLines) w.uint32(32).uint32(m.maxLines);
    if (m.notificationWhisper) w.uint32(40).bool(m.notificationWhisper);
    if (m.soundNotificationWhisper) w.uint32(48).bool(m.soundNotificationWhisper);
    if (m.notificationHighlight) w.uint32(56).bool(m.notificationHighlight);
    if (m.soundNotificationHighlight) w.uint32(64).bool(m.soundNotificationHighlight);
    if (m.notificationSoundFile) UIConfig_SoundFile.encode(m.notificationSoundFile, w.uint32(74).fork()).ldelim();
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
    if (m.replicate) w.uint32(296).bool(m.replicate);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UIConfig {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UIConfig();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 36:
        m.version = strims_dao_v1_VersionVector.decode(r, r.uint32());
        break;
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
        m.notificationSoundFile = UIConfig_SoundFile.decode(r, r.uint32());
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
        case 37:
        m.replicate = r.bool();
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

export type UIConfig_ISoundFile = {
  fileType?: string;
  data?: Uint8Array;
}

export class UIConfig_SoundFile {
  fileType: string;
  data: Uint8Array;

  constructor(v?: UIConfig_ISoundFile) {
    this.fileType = v?.fileType || "";
    this.data = v?.data || new Uint8Array();
  }

  static encode(m: UIConfig_SoundFile, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.fileType.length) w.uint32(10).string(m.fileType);
    if (m.data.length) w.uint32(18).bytes(m.data);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UIConfig_SoundFile {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UIConfig_SoundFile();
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export enum UIConfig_ShowRemoved {
  SHOW_REMOVED_REMOVE = 0,
  SHOW_REMOVED_CENSOR = 1,
  SHOW_REMOVED_DO_NOTHING = 2,
}

export enum UIConfig_UserPresenceIndicator {
  USER_PRESENCE_INDICATOR_DISABLED = 0,
  USER_PRESENCE_INDICATOR_BAR = 1,
  USER_PRESENCE_INDICATOR_DOT = 2,
  USER_PRESENCE_INDICATOR_ARRAY = 3,
}

export type IUIConfigHighlight = {
  id?: bigint;
  version?: strims_dao_v1_IVersionVector;
  alias?: string;
  peerKey?: Uint8Array;
}

export class UIConfigHighlight {
  id: bigint;
  version: strims_dao_v1_VersionVector | undefined;
  alias: string;
  peerKey: Uint8Array;

  constructor(v?: IUIConfigHighlight) {
    this.id = v?.id || BigInt(0);
    this.version = v?.version && new strims_dao_v1_VersionVector(v.version);
    this.alias = v?.alias || "";
    this.peerKey = v?.peerKey || new Uint8Array();
  }

  static encode(m: UIConfigHighlight, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.version) strims_dao_v1_VersionVector.encode(m.version, w.uint32(34).fork()).ldelim();
    if (m.alias.length) w.uint32(18).string(m.alias);
    if (m.peerKey.length) w.uint32(26).bytes(m.peerKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UIConfigHighlight {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UIConfigHighlight();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 4:
        m.version = strims_dao_v1_VersionVector.decode(r, r.uint32());
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

export type IUIConfigTag = {
  id?: bigint;
  version?: strims_dao_v1_IVersionVector;
  alias?: string;
  peerKey?: Uint8Array;
  color?: string;
}

export class UIConfigTag {
  id: bigint;
  version: strims_dao_v1_VersionVector | undefined;
  alias: string;
  peerKey: Uint8Array;
  color: string;

  constructor(v?: IUIConfigTag) {
    this.id = v?.id || BigInt(0);
    this.version = v?.version && new strims_dao_v1_VersionVector(v.version);
    this.alias = v?.alias || "";
    this.peerKey = v?.peerKey || new Uint8Array();
    this.color = v?.color || "";
  }

  static encode(m: UIConfigTag, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.version) strims_dao_v1_VersionVector.encode(m.version, w.uint32(42).fork()).ldelim();
    if (m.alias.length) w.uint32(18).string(m.alias);
    if (m.peerKey.length) w.uint32(26).bytes(m.peerKey);
    if (m.color.length) w.uint32(34).string(m.color);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UIConfigTag {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UIConfigTag();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 5:
        m.version = strims_dao_v1_VersionVector.decode(r, r.uint32());
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IUIConfigIgnore = {
  id?: bigint;
  version?: strims_dao_v1_IVersionVector;
  alias?: string;
  peerKey?: Uint8Array;
  deadline?: bigint;
}

export class UIConfigIgnore {
  id: bigint;
  version: strims_dao_v1_VersionVector | undefined;
  alias: string;
  peerKey: Uint8Array;
  deadline: bigint;

  constructor(v?: IUIConfigIgnore) {
    this.id = v?.id || BigInt(0);
    this.version = v?.version && new strims_dao_v1_VersionVector(v.version);
    this.alias = v?.alias || "";
    this.peerKey = v?.peerKey || new Uint8Array();
    this.deadline = v?.deadline || BigInt(0);
  }

  static encode(m: UIConfigIgnore, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.version) strims_dao_v1_VersionVector.encode(m.version, w.uint32(42).fork()).ldelim();
    if (m.alias.length) w.uint32(18).string(m.alias);
    if (m.peerKey.length) w.uint32(26).bytes(m.peerKey);
    if (m.deadline) w.uint32(32).int64(m.deadline);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UIConfigIgnore {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UIConfigIgnore();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 5:
        m.version = strims_dao_v1_VersionVector.decode(r, r.uint32());
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
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
    if (m.networkKey.length) w.uint32(10).bytes(m.networkKey);
    if (m.room) Room.encode(m.room, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateServerRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new CreateServerRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.networkKey = r.bytes();
        break;
        case 2:
        m.room = Room.decode(r, r.uint32());
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new CreateServerResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.server = Server.decode(r, r.uint32());
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UpdateServerResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.server = Server.decode(r, r.uint32());
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new DeleteServerRequest();
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

export type IDeleteServerResponse = object;

export class DeleteServerResponse {

  constructor(v?: IDeleteServerResponse) {
    void v; // an empty message has no fields to set
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new GetServerRequest();
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new GetServerResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.server = Server.decode(r, r.uint32());
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

export type IListServersRequest = object;

export class ListServersRequest {

  constructor(v?: IListServersRequest) {
    void v; // an empty message has no fields to set
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ListServersResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.servers.push(Server.decode(r, r.uint32()));
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IUpdateServerIconResponse = {
  serverIcon?: IServerIcon;
}

export class UpdateServerIconResponse {
  serverIcon: ServerIcon | undefined;

  constructor(v?: IUpdateServerIconResponse) {
    this.serverIcon = v?.serverIcon && new ServerIcon(v.serverIcon);
  }

  static encode(m: UpdateServerIconResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverIcon) ServerIcon.encode(m.serverIcon, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateServerIconResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UpdateServerIconResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverIcon = ServerIcon.decode(r, r.uint32());
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new GetServerIconRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverId = r.uint64();
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

export type IGetServerIconResponse = {
  serverIcon?: IServerIcon;
}

export class GetServerIconResponse {
  serverIcon: ServerIcon | undefined;

  constructor(v?: IGetServerIconResponse) {
    this.serverIcon = v?.serverIcon && new ServerIcon(v.serverIcon);
  }

  static encode(m: GetServerIconResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverIcon) ServerIcon.encode(m.serverIcon, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetServerIconResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new GetServerIconResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverIcon = ServerIcon.decode(r, r.uint32());
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

export type ICreateEmoteRequest = {
  serverId?: bigint;
  name?: string;
  images?: IEmoteImage[];
  css?: string;
  effects?: IEmoteEffect[];
  contributor?: IEmoteContributor;
  labels?: string[];
  enable?: boolean;
}

export class CreateEmoteRequest {
  serverId: bigint;
  name: string;
  images: EmoteImage[];
  css: string;
  effects: EmoteEffect[];
  contributor: EmoteContributor | undefined;
  labels: string[];
  enable: boolean;

  constructor(v?: ICreateEmoteRequest) {
    this.serverId = v?.serverId || BigInt(0);
    this.name = v?.name || "";
    this.images = v?.images ? v.images.map(v => new EmoteImage(v)) : [];
    this.css = v?.css || "";
    this.effects = v?.effects ? v.effects.map(v => new EmoteEffect(v)) : [];
    this.contributor = v?.contributor && new EmoteContributor(v.contributor);
    this.labels = v?.labels ? v.labels : [];
    this.enable = v?.enable || false;
  }

  static encode(m: CreateEmoteRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    if (m.name.length) w.uint32(18).string(m.name);
    for (const v of m.images) EmoteImage.encode(v, w.uint32(26).fork()).ldelim();
    if (m.css.length) w.uint32(34).string(m.css);
    for (const v of m.effects) EmoteEffect.encode(v, w.uint32(42).fork()).ldelim();
    if (m.contributor) EmoteContributor.encode(m.contributor, w.uint32(50).fork()).ldelim();
    for (const v of m.labels) w.uint32(58).string(v);
    if (m.enable) w.uint32(64).bool(m.enable);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateEmoteRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
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
        case 7:
        m.labels.push(r.string())
        break;
        case 8:
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new CreateEmoteResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.emote = Emote.decode(r, r.uint32());
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

export type IUpdateEmoteRequest = {
  serverId?: bigint;
  id?: bigint;
  name?: string;
  images?: IEmoteImage[];
  css?: string;
  effects?: IEmoteEffect[];
  contributor?: IEmoteContributor;
  labels?: string[];
  enable?: boolean;
}

export class UpdateEmoteRequest {
  serverId: bigint;
  id: bigint;
  name: string;
  images: EmoteImage[];
  css: string;
  effects: EmoteEffect[];
  contributor: EmoteContributor | undefined;
  labels: string[];
  enable: boolean;

  constructor(v?: IUpdateEmoteRequest) {
    this.serverId = v?.serverId || BigInt(0);
    this.id = v?.id || BigInt(0);
    this.name = v?.name || "";
    this.images = v?.images ? v.images.map(v => new EmoteImage(v)) : [];
    this.css = v?.css || "";
    this.effects = v?.effects ? v.effects.map(v => new EmoteEffect(v)) : [];
    this.contributor = v?.contributor && new EmoteContributor(v.contributor);
    this.labels = v?.labels ? v.labels : [];
    this.enable = v?.enable || false;
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
    for (const v of m.labels) w.uint32(66).string(v);
    if (m.enable) w.uint32(72).bool(m.enable);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateEmoteRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
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
        case 8:
        m.labels.push(r.string())
        break;
        case 9:
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UpdateEmoteResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.emote = Emote.decode(r, r.uint32());
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IDeleteEmoteResponse = object;

export class DeleteEmoteResponse {

  constructor(v?: IDeleteEmoteResponse) {
    void v; // an empty message has no fields to set
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new GetEmoteRequest();
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new GetEmoteResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.emote = Emote.decode(r, r.uint32());
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

export type IListEmotesRequest = {
  serverId?: bigint;
  parts?: ListEmotesRequest_Part[];
}

export class ListEmotesRequest {
  serverId: bigint;
  parts: ListEmotesRequest_Part[];

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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export enum ListEmotesRequest_Part {
  PART_UNDEFINED = 0,
  PART_META = 1,
  PART_ASSETS = 2,
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ListEmotesResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.emotes.push(Emote.decode(r, r.uint32()));
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

export type IUpdateEmotesRequest = {
  serverId?: bigint;
  ids?: bigint[];
  addLabels?: string[];
  removeLabels?: string[];
  enable?: google_protobuf_IBoolValue;
}

export class UpdateEmotesRequest {
  serverId: bigint;
  ids: bigint[];
  addLabels: string[];
  removeLabels: string[];
  enable: google_protobuf_BoolValue | undefined;

  constructor(v?: IUpdateEmotesRequest) {
    this.serverId = v?.serverId || BigInt(0);
    this.ids = v?.ids ? v.ids : [];
    this.addLabels = v?.addLabels ? v.addLabels : [];
    this.removeLabels = v?.removeLabels ? v.removeLabels : [];
    this.enable = v?.enable && new google_protobuf_BoolValue(v.enable);
  }

  static encode(m: UpdateEmotesRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    m.ids.reduce((w, v) => w.uint64(v), w.uint32(18).fork()).ldelim();
    for (const v of m.addLabels) w.uint32(26).string(v);
    for (const v of m.removeLabels) w.uint32(34).string(v);
    if (m.enable) google_protobuf_BoolValue.encode(m.enable, w.uint32(42).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateEmotesRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UpdateEmotesRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverId = r.uint64();
        break;
        case 2:
        for (const flen = r.uint32(), fend = r.pos + flen; r.pos < fend;) m.ids.push(r.uint64());
        break;
        case 3:
        m.addLabels.push(r.string())
        break;
        case 4:
        m.removeLabels.push(r.string())
        break;
        case 5:
        m.enable = google_protobuf_BoolValue.decode(r, r.uint32());
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

export type IUpdateEmotesResponse = object;

export class UpdateEmotesResponse {

  constructor(v?: IUpdateEmotesResponse) {
    void v; // an empty message has no fields to set
  }

  static encode(m: UpdateEmotesResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateEmotesResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new UpdateEmotesResponse();
  }
}

export type ICreateModifierRequest = {
  serverId?: bigint;
  name?: string;
  priority?: number;
  internal?: boolean;
  extraWrapCount?: number;
  procChance?: number;
  styleSheet?: IStyleSheet;
}

export class CreateModifierRequest {
  serverId: bigint;
  name: string;
  priority: number;
  internal: boolean;
  extraWrapCount: number;
  procChance: number;
  styleSheet: StyleSheet | undefined;

  constructor(v?: ICreateModifierRequest) {
    this.serverId = v?.serverId || BigInt(0);
    this.name = v?.name || "";
    this.priority = v?.priority || 0;
    this.internal = v?.internal || false;
    this.extraWrapCount = v?.extraWrapCount || 0;
    this.procChance = v?.procChance || 0;
    this.styleSheet = v?.styleSheet && new StyleSheet(v.styleSheet);
  }

  static encode(m: CreateModifierRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    if (m.name.length) w.uint32(18).string(m.name);
    if (m.priority) w.uint32(24).uint32(m.priority);
    if (m.internal) w.uint32(32).bool(m.internal);
    if (m.extraWrapCount) w.uint32(40).uint32(m.extraWrapCount);
    if (m.procChance) w.uint32(49).double(m.procChance);
    if (m.styleSheet) StyleSheet.encode(m.styleSheet, w.uint32(58).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): CreateModifierRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
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
        case 7:
        m.styleSheet = StyleSheet.decode(r, r.uint32());
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new CreateModifierResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.modifier = Modifier.decode(r, r.uint32());
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

export type IUpdateModifierRequest = {
  serverId?: bigint;
  id?: bigint;
  name?: string;
  priority?: number;
  internal?: boolean;
  extraWrapCount?: number;
  procChance?: number;
  styleSheet?: IStyleSheet;
}

export class UpdateModifierRequest {
  serverId: bigint;
  id: bigint;
  name: string;
  priority: number;
  internal: boolean;
  extraWrapCount: number;
  procChance: number;
  styleSheet: StyleSheet | undefined;

  constructor(v?: IUpdateModifierRequest) {
    this.serverId = v?.serverId || BigInt(0);
    this.id = v?.id || BigInt(0);
    this.name = v?.name || "";
    this.priority = v?.priority || 0;
    this.internal = v?.internal || false;
    this.extraWrapCount = v?.extraWrapCount || 0;
    this.procChance = v?.procChance || 0;
    this.styleSheet = v?.styleSheet && new StyleSheet(v.styleSheet);
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
    if (m.styleSheet) StyleSheet.encode(m.styleSheet, w.uint32(66).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UpdateModifierRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
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
        case 8:
        m.styleSheet = StyleSheet.decode(r, r.uint32());
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UpdateModifierResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.modifier = Modifier.decode(r, r.uint32());
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IDeleteModifierResponse = object;

export class DeleteModifierResponse {

  constructor(v?: IDeleteModifierResponse) {
    void v; // an empty message has no fields to set
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new GetModifierRequest();
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new GetModifierResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.modifier = Modifier.decode(r, r.uint32());
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ListModifiersRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverId = r.uint64();
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ListModifiersResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.modifiers.push(Modifier.decode(r, r.uint32()));
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new CreateTagResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.tag = Tag.decode(r, r.uint32());
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UpdateTagResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.tag = Tag.decode(r, r.uint32());
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IDeleteTagResponse = object;

export class DeleteTagResponse {

  constructor(v?: IDeleteTagResponse) {
    void v; // an empty message has no fields to set
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new GetTagRequest();
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new GetTagResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.tag = Tag.decode(r, r.uint32());
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ListTagsRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverId = r.uint64();
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ListTagsResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.tags.push(Tag.decode(r, r.uint32()));
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IListEmoteLabelsRequest = {
  serverId?: bigint;
}

export class ListEmoteLabelsRequest {
  serverId: bigint;

  constructor(v?: IListEmoteLabelsRequest) {
    this.serverId = v?.serverId || BigInt(0);
  }

  static encode(m: ListEmoteLabelsRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListEmoteLabelsRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ListEmoteLabelsRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverId = r.uint64();
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

export type IListEmoteLabelsResponse = {
  labels?: string[];
}

export class ListEmoteLabelsResponse {
  labels: string[];

  constructor(v?: IListEmoteLabelsResponse) {
    this.labels = v?.labels ? v.labels : [];
  }

  static encode(m: ListEmoteLabelsResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.labels) w.uint32(10).string(v);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ListEmoteLabelsResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ListEmoteLabelsResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.labels.push(r.string())
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IOpenClientResponse = {
  body?: OpenClientResponse_IBody
}

export class OpenClientResponse {
  body: OpenClientResponse_TBody;

  constructor(v?: IOpenClientResponse) {
    this.body = v?.body ? new OpenClientResponse_Body(v.body) : new OpenClientResponse_Body();
  }

  static encode(m: OpenClientResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case OpenClientResponse_BodyCase.OPEN:
      OpenClientResponse_Open.encode(m.body.open, w.uint32(8010).fork()).ldelim();
      break;
      case OpenClientResponse_BodyCase.SERVER_EVENTS:
      OpenClientResponse_ServerEvents.encode(m.body.serverEvents, w.uint32(8018).fork()).ldelim();
      break;
      case OpenClientResponse_BodyCase.ASSET_BUNDLE:
      AssetBundle.encode(m.body.assetBundle, w.uint32(8026).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): OpenClientResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new OpenClientResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1001:
        m.body = new OpenClientResponse_Body({ open: OpenClientResponse_Open.decode(r, r.uint32()) });
        break;
        case 1002:
        m.body = new OpenClientResponse_Body({ serverEvents: OpenClientResponse_ServerEvents.decode(r, r.uint32()) });
        break;
        case 1003:
        m.body = new OpenClientResponse_Body({ assetBundle: AssetBundle.decode(r, r.uint32()) });
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

export enum OpenClientResponse_BodyCase {
  NOT_SET = 0,
  OPEN = 1001,
  SERVER_EVENTS = 1002,
  ASSET_BUNDLE = 1003,
}

export type OpenClientResponse_IBody =
{ case?: OpenClientResponse_BodyCase.NOT_SET }
|{ case?: OpenClientResponse_BodyCase.OPEN, open: OpenClientResponse_IOpen }
|{ case?: OpenClientResponse_BodyCase.SERVER_EVENTS, serverEvents: OpenClientResponse_IServerEvents }
|{ case?: OpenClientResponse_BodyCase.ASSET_BUNDLE, assetBundle: IAssetBundle }
;

export type OpenClientResponse_TBody = Readonly<
{ case: OpenClientResponse_BodyCase.NOT_SET }
|{ case: OpenClientResponse_BodyCase.OPEN, open: OpenClientResponse_Open }
|{ case: OpenClientResponse_BodyCase.SERVER_EVENTS, serverEvents: OpenClientResponse_ServerEvents }
|{ case: OpenClientResponse_BodyCase.ASSET_BUNDLE, assetBundle: AssetBundle }
>;

class OpenClientResponse_BodyImpl {
  declare open?: OpenClientResponse_Open;
  declare serverEvents?: OpenClientResponse_ServerEvents;
  declare assetBundle?: AssetBundle;
  case: OpenClientResponse_BodyCase = OpenClientResponse_BodyCase.NOT_SET;

  constructor(v?: OpenClientResponse_IBody) {
    if (v && "open" in v) {
      this.case = OpenClientResponse_BodyCase.OPEN;
      this.open = new OpenClientResponse_Open(v.open);
    } else
    if (v && "serverEvents" in v) {
      this.case = OpenClientResponse_BodyCase.SERVER_EVENTS;
      this.serverEvents = new OpenClientResponse_ServerEvents(v.serverEvents);
    } else
    if (v && "assetBundle" in v) {
      this.case = OpenClientResponse_BodyCase.ASSET_BUNDLE;
      this.assetBundle = new AssetBundle(v.assetBundle);
    }
  }
}

export const OpenClientResponse_Body = OpenClientResponse_BodyImpl as {
  new (): Readonly<{ case: OpenClientResponse_BodyCase.NOT_SET }>;
  new <T extends OpenClientResponse_IBody>(v: T): Readonly<
  T extends { open: OpenClientResponse_IOpen } ? { case: OpenClientResponse_BodyCase.OPEN, open: OpenClientResponse_Open } :
  T extends { serverEvents: OpenClientResponse_IServerEvents } ? { case: OpenClientResponse_BodyCase.SERVER_EVENTS, serverEvents: OpenClientResponse_ServerEvents } :
  T extends { assetBundle: IAssetBundle } ? { case: OpenClientResponse_BodyCase.ASSET_BUNDLE, assetBundle: AssetBundle } :
  never
  >;
};

export type OpenClientResponse_IOpen = object;

export class OpenClientResponse_Open {

  constructor(v?: OpenClientResponse_IOpen) {
    void v; // an empty message has no fields to set
  }

  static encode(m: OpenClientResponse_Open, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): OpenClientResponse_Open {
    if (r instanceof Reader && length) r.skip(length);
    return new OpenClientResponse_Open();
  }
}

export type OpenClientResponse_IServerEvents = {
  events?: IServerEvent[];
}

export class OpenClientResponse_ServerEvents {
  events: ServerEvent[];

  constructor(v?: OpenClientResponse_IServerEvents) {
    this.events = v?.events ? v.events.map(v => new ServerEvent(v)) : [];
  }

  static encode(m: OpenClientResponse_ServerEvents, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.events) ServerEvent.encode(v, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): OpenClientResponse_ServerEvents {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new OpenClientResponse_ServerEvents();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.events.push(ServerEvent.decode(r, r.uint32()));
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IClientSendMessageResponse = object;

export class ClientSendMessageResponse {

  constructor(v?: IClientSendMessageResponse) {
    void v; // an empty message has no fields to set
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IClientMuteResponse = object;

export class ClientMuteResponse {

  constructor(v?: IClientMuteResponse) {
    void v; // an empty message has no fields to set
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IClientUnmuteResponse = object;

export class ClientUnmuteResponse {

  constructor(v?: IClientUnmuteResponse) {
    void v; // an empty message has no fields to set
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IWhisperResponse = object;

export class WhisperResponse {

  constructor(v?: IWhisperResponse) {
    void v; // an empty message has no fields to set
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ListWhispersRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IWatchWhispersRequest = object;

export class WatchWhispersRequest {

  constructor(v?: IWatchWhispersRequest) {
    void v; // an empty message has no fields to set
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
  body?: WatchWhispersResponse_IBody
}

export class WatchWhispersResponse {
  peerKey: Uint8Array;
  body: WatchWhispersResponse_TBody;

  constructor(v?: IWatchWhispersResponse) {
    this.peerKey = v?.peerKey || new Uint8Array();
    this.body = v?.body ? new WatchWhispersResponse_Body(v.body) : new WatchWhispersResponse_Body();
  }

  static encode(m: WatchWhispersResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.peerKey.length) w.uint32(10).bytes(m.peerKey);
    switch (m.body.case) {
      case WatchWhispersResponse_BodyCase.THREAD_UPDATE:
      WhisperThread.encode(m.body.threadUpdate, w.uint32(8010).fork()).ldelim();
      break;
      case WatchWhispersResponse_BodyCase.THREAD_DELETE:
      WatchWhispersResponse_WhisperThreadDelete.encode(m.body.threadDelete, w.uint32(8018).fork()).ldelim();
      break;
      case WatchWhispersResponse_BodyCase.WHISPER_UPDATE:
      WhisperRecord.encode(m.body.whisperUpdate, w.uint32(8026).fork()).ldelim();
      break;
      case WatchWhispersResponse_BodyCase.WHISPER_DELETE:
      WatchWhispersResponse_WhisperDelete.encode(m.body.whisperDelete, w.uint32(8034).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WatchWhispersResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new WatchWhispersResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.peerKey = r.bytes();
        break;
        case 1001:
        m.body = new WatchWhispersResponse_Body({ threadUpdate: WhisperThread.decode(r, r.uint32()) });
        break;
        case 1002:
        m.body = new WatchWhispersResponse_Body({ threadDelete: WatchWhispersResponse_WhisperThreadDelete.decode(r, r.uint32()) });
        break;
        case 1003:
        m.body = new WatchWhispersResponse_Body({ whisperUpdate: WhisperRecord.decode(r, r.uint32()) });
        break;
        case 1004:
        m.body = new WatchWhispersResponse_Body({ whisperDelete: WatchWhispersResponse_WhisperDelete.decode(r, r.uint32()) });
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

export enum WatchWhispersResponse_BodyCase {
  NOT_SET = 0,
  THREAD_UPDATE = 1001,
  THREAD_DELETE = 1002,
  WHISPER_UPDATE = 1003,
  WHISPER_DELETE = 1004,
}

export type WatchWhispersResponse_IBody =
{ case?: WatchWhispersResponse_BodyCase.NOT_SET }
|{ case?: WatchWhispersResponse_BodyCase.THREAD_UPDATE, threadUpdate: IWhisperThread }
|{ case?: WatchWhispersResponse_BodyCase.THREAD_DELETE, threadDelete: WatchWhispersResponse_IWhisperThreadDelete }
|{ case?: WatchWhispersResponse_BodyCase.WHISPER_UPDATE, whisperUpdate: IWhisperRecord }
|{ case?: WatchWhispersResponse_BodyCase.WHISPER_DELETE, whisperDelete: WatchWhispersResponse_IWhisperDelete }
;

export type WatchWhispersResponse_TBody = Readonly<
{ case: WatchWhispersResponse_BodyCase.NOT_SET }
|{ case: WatchWhispersResponse_BodyCase.THREAD_UPDATE, threadUpdate: WhisperThread }
|{ case: WatchWhispersResponse_BodyCase.THREAD_DELETE, threadDelete: WatchWhispersResponse_WhisperThreadDelete }
|{ case: WatchWhispersResponse_BodyCase.WHISPER_UPDATE, whisperUpdate: WhisperRecord }
|{ case: WatchWhispersResponse_BodyCase.WHISPER_DELETE, whisperDelete: WatchWhispersResponse_WhisperDelete }
>;

class WatchWhispersResponse_BodyImpl {
  declare threadUpdate?: WhisperThread;
  declare threadDelete?: WatchWhispersResponse_WhisperThreadDelete;
  declare whisperUpdate?: WhisperRecord;
  declare whisperDelete?: WatchWhispersResponse_WhisperDelete;
  case: WatchWhispersResponse_BodyCase = WatchWhispersResponse_BodyCase.NOT_SET;

  constructor(v?: WatchWhispersResponse_IBody) {
    if (v && "threadUpdate" in v) {
      this.case = WatchWhispersResponse_BodyCase.THREAD_UPDATE;
      this.threadUpdate = new WhisperThread(v.threadUpdate);
    } else
    if (v && "threadDelete" in v) {
      this.case = WatchWhispersResponse_BodyCase.THREAD_DELETE;
      this.threadDelete = new WatchWhispersResponse_WhisperThreadDelete(v.threadDelete);
    } else
    if (v && "whisperUpdate" in v) {
      this.case = WatchWhispersResponse_BodyCase.WHISPER_UPDATE;
      this.whisperUpdate = new WhisperRecord(v.whisperUpdate);
    } else
    if (v && "whisperDelete" in v) {
      this.case = WatchWhispersResponse_BodyCase.WHISPER_DELETE;
      this.whisperDelete = new WatchWhispersResponse_WhisperDelete(v.whisperDelete);
    }
  }
}

export const WatchWhispersResponse_Body = WatchWhispersResponse_BodyImpl as {
  new (): Readonly<{ case: WatchWhispersResponse_BodyCase.NOT_SET }>;
  new <T extends WatchWhispersResponse_IBody>(v: T): Readonly<
  T extends { threadUpdate: IWhisperThread } ? { case: WatchWhispersResponse_BodyCase.THREAD_UPDATE, threadUpdate: WhisperThread } :
  T extends { threadDelete: WatchWhispersResponse_IWhisperThreadDelete } ? { case: WatchWhispersResponse_BodyCase.THREAD_DELETE, threadDelete: WatchWhispersResponse_WhisperThreadDelete } :
  T extends { whisperUpdate: IWhisperRecord } ? { case: WatchWhispersResponse_BodyCase.WHISPER_UPDATE, whisperUpdate: WhisperRecord } :
  T extends { whisperDelete: WatchWhispersResponse_IWhisperDelete } ? { case: WatchWhispersResponse_BodyCase.WHISPER_DELETE, whisperDelete: WatchWhispersResponse_WhisperDelete } :
  never
  >;
};

export type WatchWhispersResponse_IWhisperThreadDelete = object;

export class WatchWhispersResponse_WhisperThreadDelete {

  constructor(v?: WatchWhispersResponse_IWhisperThreadDelete) {
    void v; // an empty message has no fields to set
  }

  static encode(m: WatchWhispersResponse_WhisperThreadDelete, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WatchWhispersResponse_WhisperThreadDelete {
    if (r instanceof Reader && length) r.skip(length);
    return new WatchWhispersResponse_WhisperThreadDelete();
  }
}

export type WatchWhispersResponse_IWhisperDelete = {
  recordId?: bigint;
}

export class WatchWhispersResponse_WhisperDelete {
  recordId: bigint;

  constructor(v?: WatchWhispersResponse_IWhisperDelete) {
    this.recordId = v?.recordId || BigInt(0);
  }

  static encode(m: WatchWhispersResponse_WhisperDelete, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.recordId) w.uint32(8).uint64(m.recordId);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WatchWhispersResponse_WhisperDelete {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new WatchWhispersResponse_WhisperDelete();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.recordId = r.uint64();
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

export type IMarkWhispersReadRequest = {
  peerKey?: Uint8Array;
}

export class MarkWhispersReadRequest {
  peerKey: Uint8Array;

  constructor(v?: IMarkWhispersReadRequest) {
    this.peerKey = v?.peerKey || new Uint8Array();
  }

  static encode(m: MarkWhispersReadRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.peerKey.length) w.uint32(10).bytes(m.peerKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): MarkWhispersReadRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new MarkWhispersReadRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
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

export type IMarkWhispersReadResponse = object;

export class MarkWhispersReadResponse {

  constructor(v?: IMarkWhispersReadResponse) {
    void v; // an empty message has no fields to set
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
  peerKey?: Uint8Array;
}

export class DeleteWhisperThreadRequest {
  peerKey: Uint8Array;

  constructor(v?: IDeleteWhisperThreadRequest) {
    this.peerKey = v?.peerKey || new Uint8Array();
  }

  static encode(m: DeleteWhisperThreadRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.peerKey.length) w.uint32(10).bytes(m.peerKey);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DeleteWhisperThreadRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new DeleteWhisperThreadRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
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

export type IDeleteWhisperThreadResponse = object;

export class DeleteWhisperThreadResponse {

  constructor(v?: IDeleteWhisperThreadResponse) {
    void v; // an empty message has no fields to set
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new SetUIConfigRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.uiConfig = UIConfig.decode(r, r.uint32());
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

export type ISetUIConfigResponse = object;

export class SetUIConfigResponse {

  constructor(v?: ISetUIConfigResponse) {
    void v; // an empty message has no fields to set
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

export type IWatchUIConfigRequest = object;

export class WatchUIConfigRequest {

  constructor(v?: IWatchUIConfigRequest) {
    void v; // an empty message has no fields to set
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
  config?: WatchUIConfigResponse_IConfig
}

export class WatchUIConfigResponse {
  config: WatchUIConfigResponse_TConfig;

  constructor(v?: IWatchUIConfigResponse) {
    this.config = v?.config ? new WatchUIConfigResponse_Config(v.config) : new WatchUIConfigResponse_Config();
  }

  static encode(m: WatchUIConfigResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.config.case) {
      case WatchUIConfigResponse_ConfigCase.UI_CONFIG:
      UIConfig.encode(m.config.uiConfig, w.uint32(8010).fork()).ldelim();
      break;
      case WatchUIConfigResponse_ConfigCase.UI_CONFIG_HIGHLIGHT:
      UIConfigHighlight.encode(m.config.uiConfigHighlight, w.uint32(8018).fork()).ldelim();
      break;
      case WatchUIConfigResponse_ConfigCase.UI_CONFIG_HIGHLIGHT_DELETE:
      UIConfigHighlight.encode(m.config.uiConfigHighlightDelete, w.uint32(8026).fork()).ldelim();
      break;
      case WatchUIConfigResponse_ConfigCase.UI_CONFIG_TAG:
      UIConfigTag.encode(m.config.uiConfigTag, w.uint32(8034).fork()).ldelim();
      break;
      case WatchUIConfigResponse_ConfigCase.UI_CONFIG_TAG_DELETE:
      UIConfigTag.encode(m.config.uiConfigTagDelete, w.uint32(8042).fork()).ldelim();
      break;
      case WatchUIConfigResponse_ConfigCase.UI_CONFIG_IGNORE:
      UIConfigIgnore.encode(m.config.uiConfigIgnore, w.uint32(8050).fork()).ldelim();
      break;
      case WatchUIConfigResponse_ConfigCase.UI_CONFIG_IGNORE_DELETE:
      UIConfigIgnore.encode(m.config.uiConfigIgnoreDelete, w.uint32(8058).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WatchUIConfigResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new WatchUIConfigResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1001:
        m.config = new WatchUIConfigResponse_Config({ uiConfig: UIConfig.decode(r, r.uint32()) });
        break;
        case 1002:
        m.config = new WatchUIConfigResponse_Config({ uiConfigHighlight: UIConfigHighlight.decode(r, r.uint32()) });
        break;
        case 1003:
        m.config = new WatchUIConfigResponse_Config({ uiConfigHighlightDelete: UIConfigHighlight.decode(r, r.uint32()) });
        break;
        case 1004:
        m.config = new WatchUIConfigResponse_Config({ uiConfigTag: UIConfigTag.decode(r, r.uint32()) });
        break;
        case 1005:
        m.config = new WatchUIConfigResponse_Config({ uiConfigTagDelete: UIConfigTag.decode(r, r.uint32()) });
        break;
        case 1006:
        m.config = new WatchUIConfigResponse_Config({ uiConfigIgnore: UIConfigIgnore.decode(r, r.uint32()) });
        break;
        case 1007:
        m.config = new WatchUIConfigResponse_Config({ uiConfigIgnoreDelete: UIConfigIgnore.decode(r, r.uint32()) });
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

export enum WatchUIConfigResponse_ConfigCase {
  NOT_SET = 0,
  UI_CONFIG = 1001,
  UI_CONFIG_HIGHLIGHT = 1002,
  UI_CONFIG_HIGHLIGHT_DELETE = 1003,
  UI_CONFIG_TAG = 1004,
  UI_CONFIG_TAG_DELETE = 1005,
  UI_CONFIG_IGNORE = 1006,
  UI_CONFIG_IGNORE_DELETE = 1007,
}

export type WatchUIConfigResponse_IConfig =
{ case?: WatchUIConfigResponse_ConfigCase.NOT_SET }
|{ case?: WatchUIConfigResponse_ConfigCase.UI_CONFIG, uiConfig: IUIConfig }
|{ case?: WatchUIConfigResponse_ConfigCase.UI_CONFIG_HIGHLIGHT, uiConfigHighlight: IUIConfigHighlight }
|{ case?: WatchUIConfigResponse_ConfigCase.UI_CONFIG_HIGHLIGHT_DELETE, uiConfigHighlightDelete: IUIConfigHighlight }
|{ case?: WatchUIConfigResponse_ConfigCase.UI_CONFIG_TAG, uiConfigTag: IUIConfigTag }
|{ case?: WatchUIConfigResponse_ConfigCase.UI_CONFIG_TAG_DELETE, uiConfigTagDelete: IUIConfigTag }
|{ case?: WatchUIConfigResponse_ConfigCase.UI_CONFIG_IGNORE, uiConfigIgnore: IUIConfigIgnore }
|{ case?: WatchUIConfigResponse_ConfigCase.UI_CONFIG_IGNORE_DELETE, uiConfigIgnoreDelete: IUIConfigIgnore }
;

export type WatchUIConfigResponse_TConfig = Readonly<
{ case: WatchUIConfigResponse_ConfigCase.NOT_SET }
|{ case: WatchUIConfigResponse_ConfigCase.UI_CONFIG, uiConfig: UIConfig }
|{ case: WatchUIConfigResponse_ConfigCase.UI_CONFIG_HIGHLIGHT, uiConfigHighlight: UIConfigHighlight }
|{ case: WatchUIConfigResponse_ConfigCase.UI_CONFIG_HIGHLIGHT_DELETE, uiConfigHighlightDelete: UIConfigHighlight }
|{ case: WatchUIConfigResponse_ConfigCase.UI_CONFIG_TAG, uiConfigTag: UIConfigTag }
|{ case: WatchUIConfigResponse_ConfigCase.UI_CONFIG_TAG_DELETE, uiConfigTagDelete: UIConfigTag }
|{ case: WatchUIConfigResponse_ConfigCase.UI_CONFIG_IGNORE, uiConfigIgnore: UIConfigIgnore }
|{ case: WatchUIConfigResponse_ConfigCase.UI_CONFIG_IGNORE_DELETE, uiConfigIgnoreDelete: UIConfigIgnore }
>;

class WatchUIConfigResponse_ConfigImpl {
  declare uiConfig?: UIConfig;
  declare uiConfigHighlight?: UIConfigHighlight;
  declare uiConfigHighlightDelete?: UIConfigHighlight;
  declare uiConfigTag?: UIConfigTag;
  declare uiConfigTagDelete?: UIConfigTag;
  declare uiConfigIgnore?: UIConfigIgnore;
  declare uiConfigIgnoreDelete?: UIConfigIgnore;
  case: WatchUIConfigResponse_ConfigCase = WatchUIConfigResponse_ConfigCase.NOT_SET;

  constructor(v?: WatchUIConfigResponse_IConfig) {
    if (v && "uiConfig" in v) {
      this.case = WatchUIConfigResponse_ConfigCase.UI_CONFIG;
      this.uiConfig = new UIConfig(v.uiConfig);
    } else
    if (v && "uiConfigHighlight" in v) {
      this.case = WatchUIConfigResponse_ConfigCase.UI_CONFIG_HIGHLIGHT;
      this.uiConfigHighlight = new UIConfigHighlight(v.uiConfigHighlight);
    } else
    if (v && "uiConfigHighlightDelete" in v) {
      this.case = WatchUIConfigResponse_ConfigCase.UI_CONFIG_HIGHLIGHT_DELETE;
      this.uiConfigHighlightDelete = new UIConfigHighlight(v.uiConfigHighlightDelete);
    } else
    if (v && "uiConfigTag" in v) {
      this.case = WatchUIConfigResponse_ConfigCase.UI_CONFIG_TAG;
      this.uiConfigTag = new UIConfigTag(v.uiConfigTag);
    } else
    if (v && "uiConfigTagDelete" in v) {
      this.case = WatchUIConfigResponse_ConfigCase.UI_CONFIG_TAG_DELETE;
      this.uiConfigTagDelete = new UIConfigTag(v.uiConfigTagDelete);
    } else
    if (v && "uiConfigIgnore" in v) {
      this.case = WatchUIConfigResponse_ConfigCase.UI_CONFIG_IGNORE;
      this.uiConfigIgnore = new UIConfigIgnore(v.uiConfigIgnore);
    } else
    if (v && "uiConfigIgnoreDelete" in v) {
      this.case = WatchUIConfigResponse_ConfigCase.UI_CONFIG_IGNORE_DELETE;
      this.uiConfigIgnoreDelete = new UIConfigIgnore(v.uiConfigIgnoreDelete);
    }
  }
}

export const WatchUIConfigResponse_Config = WatchUIConfigResponse_ConfigImpl as {
  new (): Readonly<{ case: WatchUIConfigResponse_ConfigCase.NOT_SET }>;
  new <T extends WatchUIConfigResponse_IConfig>(v: T): Readonly<
  T extends { uiConfig: IUIConfig } ? { case: WatchUIConfigResponse_ConfigCase.UI_CONFIG, uiConfig: UIConfig } :
  T extends { uiConfigHighlight: IUIConfigHighlight } ? { case: WatchUIConfigResponse_ConfigCase.UI_CONFIG_HIGHLIGHT, uiConfigHighlight: UIConfigHighlight } :
  T extends { uiConfigHighlightDelete: IUIConfigHighlight } ? { case: WatchUIConfigResponse_ConfigCase.UI_CONFIG_HIGHLIGHT_DELETE, uiConfigHighlightDelete: UIConfigHighlight } :
  T extends { uiConfigTag: IUIConfigTag } ? { case: WatchUIConfigResponse_ConfigCase.UI_CONFIG_TAG, uiConfigTag: UIConfigTag } :
  T extends { uiConfigTagDelete: IUIConfigTag } ? { case: WatchUIConfigResponse_ConfigCase.UI_CONFIG_TAG_DELETE, uiConfigTagDelete: UIConfigTag } :
  T extends { uiConfigIgnore: IUIConfigIgnore } ? { case: WatchUIConfigResponse_ConfigCase.UI_CONFIG_IGNORE, uiConfigIgnore: UIConfigIgnore } :
  T extends { uiConfigIgnoreDelete: IUIConfigIgnore } ? { case: WatchUIConfigResponse_ConfigCase.UI_CONFIG_IGNORE_DELETE, uiConfigIgnoreDelete: UIConfigIgnore } :
  never
  >;
};

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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IIgnoreResponse = object;

export class IgnoreResponse {

  constructor(v?: IIgnoreResponse) {
    void v; // an empty message has no fields to set
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IUnignoreResponse = object;

export class UnignoreResponse {

  constructor(v?: IUnignoreResponse) {
    void v; // an empty message has no fields to set
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IHighlightResponse = object;

export class HighlightResponse {

  constructor(v?: IHighlightResponse) {
    void v; // an empty message has no fields to set
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IUnhighlightResponse = object;

export class UnhighlightResponse {

  constructor(v?: IUnhighlightResponse) {
    void v; // an empty message has no fields to set
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type ITagResponse = object;

export class TagResponse {

  constructor(v?: ITagResponse) {
    void v; // an empty message has no fields to set
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IUntagResponse = object;

export class UntagResponse {

  constructor(v?: IUntagResponse) {
    void v; // an empty message has no fields to set
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new SendMessageRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.body = r.string();
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

export type ISendMessageResponse = object;

export class SendMessageResponse {

  constructor(v?: ISendMessageResponse) {
    void v; // an empty message has no fields to set
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IMuteResponse = object;

export class MuteResponse {

  constructor(v?: IMuteResponse) {
    void v; // an empty message has no fields to set
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UnmuteRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
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

export type IUnmuteResponse = object;

export class UnmuteResponse {

  constructor(v?: IUnmuteResponse) {
    void v; // an empty message has no fields to set
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

export type IGetMuteRequest = object;

export class GetMuteRequest {

  constructor(v?: IGetMuteRequest) {
    void v; // an empty message has no fields to set
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IWhisperThread = {
  id?: bigint;
  version?: strims_dao_v1_IVersionVector;
  peerKey?: Uint8Array;
  alias?: string;
  unreadCount?: number;
  lastMessageTime?: bigint;
  lastMessageId?: bigint;
  hasUnread?: boolean;
}

export class WhisperThread {
  id: bigint;
  version: strims_dao_v1_VersionVector | undefined;
  peerKey: Uint8Array;
  alias: string;
  unreadCount: number;
  lastMessageTime: bigint;
  lastMessageId: bigint;
  hasUnread: boolean;

  constructor(v?: IWhisperThread) {
    this.id = v?.id || BigInt(0);
    this.version = v?.version && new strims_dao_v1_VersionVector(v.version);
    this.peerKey = v?.peerKey || new Uint8Array();
    this.alias = v?.alias || "";
    this.unreadCount = v?.unreadCount || 0;
    this.lastMessageTime = v?.lastMessageTime || BigInt(0);
    this.lastMessageId = v?.lastMessageId || BigInt(0);
    this.hasUnread = v?.hasUnread || false;
  }

  static encode(m: WhisperThread, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.version) strims_dao_v1_VersionVector.encode(m.version, w.uint32(66).fork()).ldelim();
    if (m.peerKey.length) w.uint32(18).bytes(m.peerKey);
    if (m.alias.length) w.uint32(26).string(m.alias);
    if (m.unreadCount) w.uint32(32).uint32(m.unreadCount);
    if (m.lastMessageTime) w.uint32(48).int64(m.lastMessageTime);
    if (m.lastMessageId) w.uint32(56).uint64(m.lastMessageId);
    if (m.hasUnread) w.uint32(72).bool(m.hasUnread);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WhisperThread {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new WhisperThread();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 8:
        m.version = strims_dao_v1_VersionVector.decode(r, r.uint32());
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
        case 6:
        m.lastMessageTime = r.int64();
        break;
        case 7:
        m.lastMessageId = r.uint64();
        break;
        case 9:
        m.hasUnread = r.bool();
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

export type IWhisperRecord = {
  id?: bigint;
  version?: strims_dao_v1_IVersionVector;
  threadId?: bigint;
  networkKey?: Uint8Array;
  serverKey?: Uint8Array;
  peerKey?: Uint8Array;
  state?: MessageState;
  message?: IMessage;
  peerMessageId?: bigint;
}

export class WhisperRecord {
  id: bigint;
  version: strims_dao_v1_VersionVector | undefined;
  threadId: bigint;
  networkKey: Uint8Array;
  serverKey: Uint8Array;
  peerKey: Uint8Array;
  state: MessageState;
  message: Message | undefined;
  peerMessageId: bigint;

  constructor(v?: IWhisperRecord) {
    this.id = v?.id || BigInt(0);
    this.version = v?.version && new strims_dao_v1_VersionVector(v.version);
    this.threadId = v?.threadId || BigInt(0);
    this.networkKey = v?.networkKey || new Uint8Array();
    this.serverKey = v?.serverKey || new Uint8Array();
    this.peerKey = v?.peerKey || new Uint8Array();
    this.state = v?.state || 0;
    this.message = v?.message && new Message(v.message);
    this.peerMessageId = v?.peerMessageId || BigInt(0);
  }

  static encode(m: WhisperRecord, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.version) strims_dao_v1_VersionVector.encode(m.version, w.uint32(66).fork()).ldelim();
    if (m.threadId) w.uint32(16).uint64(m.threadId);
    if (m.networkKey.length) w.uint32(26).bytes(m.networkKey);
    if (m.serverKey.length) w.uint32(34).bytes(m.serverKey);
    if (m.peerKey.length) w.uint32(42).bytes(m.peerKey);
    if (m.state) w.uint32(48).uint32(m.state);
    if (m.message) Message.encode(m.message, w.uint32(58).fork()).ldelim();
    if (m.peerMessageId) w.uint32(72).uint64(m.peerMessageId);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WhisperRecord {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new WhisperRecord();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 8:
        m.version = strims_dao_v1_VersionVector.decode(r, r.uint32());
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
        case 9:
        m.peerMessageId = r.uint64();
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

export type IWhisperSendMessageRequest = {
  serverKey?: Uint8Array;
  body?: string;
  id?: bigint;
}

export class WhisperSendMessageRequest {
  serverKey: Uint8Array;
  body: string;
  id: bigint;

  constructor(v?: IWhisperSendMessageRequest) {
    this.serverKey = v?.serverKey || new Uint8Array();
    this.body = v?.body || "";
    this.id = v?.id || BigInt(0);
  }

  static encode(m: WhisperSendMessageRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverKey.length) w.uint32(10).bytes(m.serverKey);
    if (m.body.length) w.uint32(18).string(m.body);
    if (m.id) w.uint32(24).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WhisperSendMessageRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
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
        case 3:
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

export type IWhisperSendMessageResponse = object;

export class WhisperSendMessageResponse {

  constructor(v?: IWhisperSendMessageResponse) {
    void v; // an empty message has no fields to set
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

export enum MessageState {
  MESSAGE_STATE_UNREAD = 0,
  MESSAGE_STATE_ENQUEUED = 1,
  MESSAGE_STATE_DELIVERED = 2,
  MESSAGE_STATE_FAILED = 3,
  MESSAGE_STATE_READ = 4,
}


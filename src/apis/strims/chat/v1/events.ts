import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  Emote as strims_chat_v1_Emote,
  IEmote as strims_chat_v1_IEmote,
  Modifier as strims_chat_v1_Modifier,
  IModifier as strims_chat_v1_IModifier,
  Server as strims_chat_v1_Server,
  IServer as strims_chat_v1_IServer,
  Tag as strims_chat_v1_Tag,
  ITag as strims_chat_v1_ITag,
  UIConfig as strims_chat_v1_UIConfig,
  IUIConfig as strims_chat_v1_IUIConfig,
} from "./chat";

export type IServerChangeEvent = {
  server?: strims_chat_v1_IServer;
}

export class ServerChangeEvent {
  server: strims_chat_v1_Server | undefined;

  constructor(v?: IServerChangeEvent) {
    this.server = v?.server && new strims_chat_v1_Server(v.server);
  }

  static encode(m: ServerChangeEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.server) strims_chat_v1_Server.encode(m.server, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ServerChangeEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ServerChangeEvent();
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

export type IServerDeleteEvent = {
  server?: strims_chat_v1_IServer;
}

export class ServerDeleteEvent {
  server: strims_chat_v1_Server | undefined;

  constructor(v?: IServerDeleteEvent) {
    this.server = v?.server && new strims_chat_v1_Server(v.server);
  }

  static encode(m: ServerDeleteEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.server) strims_chat_v1_Server.encode(m.server, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ServerDeleteEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ServerDeleteEvent();
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

export type IEmoteChangeEvent = {
  emote?: strims_chat_v1_IEmote;
}

export class EmoteChangeEvent {
  emote: strims_chat_v1_Emote | undefined;

  constructor(v?: IEmoteChangeEvent) {
    this.emote = v?.emote && new strims_chat_v1_Emote(v.emote);
  }

  static encode(m: EmoteChangeEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.emote) strims_chat_v1_Emote.encode(m.emote, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): EmoteChangeEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new EmoteChangeEvent();
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

export type IEmoteDeleteEvent = {
  emote?: strims_chat_v1_IEmote;
}

export class EmoteDeleteEvent {
  emote: strims_chat_v1_Emote | undefined;

  constructor(v?: IEmoteDeleteEvent) {
    this.emote = v?.emote && new strims_chat_v1_Emote(v.emote);
  }

  static encode(m: EmoteDeleteEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.emote) strims_chat_v1_Emote.encode(m.emote, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): EmoteDeleteEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new EmoteDeleteEvent();
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

export type IModifierChangeEvent = {
  modifier?: strims_chat_v1_IModifier;
}

export class ModifierChangeEvent {
  modifier: strims_chat_v1_Modifier | undefined;

  constructor(v?: IModifierChangeEvent) {
    this.modifier = v?.modifier && new strims_chat_v1_Modifier(v.modifier);
  }

  static encode(m: ModifierChangeEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.modifier) strims_chat_v1_Modifier.encode(m.modifier, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ModifierChangeEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ModifierChangeEvent();
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

export type IModifierDeleteEvent = {
  modifier?: strims_chat_v1_IModifier;
}

export class ModifierDeleteEvent {
  modifier: strims_chat_v1_Modifier | undefined;

  constructor(v?: IModifierDeleteEvent) {
    this.modifier = v?.modifier && new strims_chat_v1_Modifier(v.modifier);
  }

  static encode(m: ModifierDeleteEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.modifier) strims_chat_v1_Modifier.encode(m.modifier, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ModifierDeleteEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new ModifierDeleteEvent();
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

export type ITagChangeEvent = {
  tag?: strims_chat_v1_ITag;
}

export class TagChangeEvent {
  tag: strims_chat_v1_Tag | undefined;

  constructor(v?: ITagChangeEvent) {
    this.tag = v?.tag && new strims_chat_v1_Tag(v.tag);
  }

  static encode(m: TagChangeEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.tag) strims_chat_v1_Tag.encode(m.tag, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): TagChangeEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new TagChangeEvent();
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

export type ITagDeleteEvent = {
  tag?: strims_chat_v1_ITag;
}

export class TagDeleteEvent {
  tag: strims_chat_v1_Tag | undefined;

  constructor(v?: ITagDeleteEvent) {
    this.tag = v?.tag && new strims_chat_v1_Tag(v.tag);
  }

  static encode(m: TagDeleteEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.tag) strims_chat_v1_Tag.encode(m.tag, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): TagDeleteEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new TagDeleteEvent();
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

export type IUIConfigChangeEvent = {
  uiConfig?: strims_chat_v1_IUIConfig;
}

export class UIConfigChangeEvent {
  uiConfig: strims_chat_v1_UIConfig | undefined;

  constructor(v?: IUIConfigChangeEvent) {
    this.uiConfig = v?.uiConfig && new strims_chat_v1_UIConfig(v.uiConfig);
  }

  static encode(m: UIConfigChangeEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.uiConfig) strims_chat_v1_UIConfig.encode(m.uiConfig, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UIConfigChangeEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new UIConfigChangeEvent();
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

export type ISyncAssetsEvent = {
  serverId?: bigint;
  forceUnifiedUpdate?: boolean;
}

export class SyncAssetsEvent {
  serverId: bigint;
  forceUnifiedUpdate: boolean;

  constructor(v?: ISyncAssetsEvent) {
    this.serverId = v?.serverId || BigInt(0);
    this.forceUnifiedUpdate = v?.forceUnifiedUpdate || false;
  }

  static encode(m: SyncAssetsEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverId) w.uint32(8).uint64(m.serverId);
    if (m.forceUnifiedUpdate) w.uint32(16).bool(m.forceUnifiedUpdate);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): SyncAssetsEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new SyncAssetsEvent();
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


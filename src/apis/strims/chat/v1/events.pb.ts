// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  Emote as strims_chat_v1_Emote,
  type IEmote as strims_chat_v1_IEmote,
  Modifier as strims_chat_v1_Modifier,
  type IModifier as strims_chat_v1_IModifier,
  Server as strims_chat_v1_Server,
  type IServer as strims_chat_v1_IServer,
  ServerIcon as strims_chat_v1_ServerIcon,
  type IServerIcon as strims_chat_v1_IServerIcon,
  Tag as strims_chat_v1_Tag,
  type ITag as strims_chat_v1_ITag,
  UIConfig as strims_chat_v1_UIConfig,
  type IUIConfig as strims_chat_v1_IUIConfig,
  UIConfigHighlight as strims_chat_v1_UIConfigHighlight,
  type IUIConfigHighlight as strims_chat_v1_IUIConfigHighlight,
  UIConfigIgnore as strims_chat_v1_UIConfigIgnore,
  type IUIConfigIgnore as strims_chat_v1_IUIConfigIgnore,
  UIConfigTag as strims_chat_v1_UIConfigTag,
  type IUIConfigTag as strims_chat_v1_IUIConfigTag,
  WhisperRecord as strims_chat_v1_WhisperRecord,
  type IWhisperRecord as strims_chat_v1_IWhisperRecord,
  WhisperThread as strims_chat_v1_WhisperThread,
  type IWhisperThread as strims_chat_v1_IWhisperThread,
} from "./chat.pb";

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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ServerChangeEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.server = strims_chat_v1_Server.decode(r, r.uint32());
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ServerDeleteEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.server = strims_chat_v1_Server.decode(r, r.uint32());
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

export type IServerIconChangeEvent = {
  serverIcon?: strims_chat_v1_IServerIcon;
}

export class ServerIconChangeEvent {
  serverIcon: strims_chat_v1_ServerIcon | undefined;

  constructor(v?: IServerIconChangeEvent) {
    this.serverIcon = v?.serverIcon && new strims_chat_v1_ServerIcon(v.serverIcon);
  }

  static encode(m: ServerIconChangeEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.serverIcon) strims_chat_v1_ServerIcon.encode(m.serverIcon, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): ServerIconChangeEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ServerIconChangeEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.serverIcon = strims_chat_v1_ServerIcon.decode(r, r.uint32());
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new EmoteChangeEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.emote = strims_chat_v1_Emote.decode(r, r.uint32());
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new EmoteDeleteEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.emote = strims_chat_v1_Emote.decode(r, r.uint32());
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ModifierChangeEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.modifier = strims_chat_v1_Modifier.decode(r, r.uint32());
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new ModifierDeleteEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.modifier = strims_chat_v1_Modifier.decode(r, r.uint32());
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new TagChangeEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.tag = strims_chat_v1_Tag.decode(r, r.uint32());
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new TagDeleteEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.tag = strims_chat_v1_Tag.decode(r, r.uint32());
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
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UIConfigChangeEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.uiConfig = strims_chat_v1_UIConfig.decode(r, r.uint32());
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

export type IUIConfigHighlightChangeEvent = {
  uiConfigHighlight?: strims_chat_v1_IUIConfigHighlight;
}

export class UIConfigHighlightChangeEvent {
  uiConfigHighlight: strims_chat_v1_UIConfigHighlight | undefined;

  constructor(v?: IUIConfigHighlightChangeEvent) {
    this.uiConfigHighlight = v?.uiConfigHighlight && new strims_chat_v1_UIConfigHighlight(v.uiConfigHighlight);
  }

  static encode(m: UIConfigHighlightChangeEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.uiConfigHighlight) strims_chat_v1_UIConfigHighlight.encode(m.uiConfigHighlight, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UIConfigHighlightChangeEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UIConfigHighlightChangeEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.uiConfigHighlight = strims_chat_v1_UIConfigHighlight.decode(r, r.uint32());
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

export type IUIConfigHighlightDeleteEvent = {
  uiConfigHighlight?: strims_chat_v1_IUIConfigHighlight;
}

export class UIConfigHighlightDeleteEvent {
  uiConfigHighlight: strims_chat_v1_UIConfigHighlight | undefined;

  constructor(v?: IUIConfigHighlightDeleteEvent) {
    this.uiConfigHighlight = v?.uiConfigHighlight && new strims_chat_v1_UIConfigHighlight(v.uiConfigHighlight);
  }

  static encode(m: UIConfigHighlightDeleteEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.uiConfigHighlight) strims_chat_v1_UIConfigHighlight.encode(m.uiConfigHighlight, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UIConfigHighlightDeleteEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UIConfigHighlightDeleteEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.uiConfigHighlight = strims_chat_v1_UIConfigHighlight.decode(r, r.uint32());
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

export type IUIConfigTagChangeEvent = {
  uiConfigTag?: strims_chat_v1_IUIConfigTag;
}

export class UIConfigTagChangeEvent {
  uiConfigTag: strims_chat_v1_UIConfigTag | undefined;

  constructor(v?: IUIConfigTagChangeEvent) {
    this.uiConfigTag = v?.uiConfigTag && new strims_chat_v1_UIConfigTag(v.uiConfigTag);
  }

  static encode(m: UIConfigTagChangeEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.uiConfigTag) strims_chat_v1_UIConfigTag.encode(m.uiConfigTag, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UIConfigTagChangeEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UIConfigTagChangeEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.uiConfigTag = strims_chat_v1_UIConfigTag.decode(r, r.uint32());
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

export type IUIConfigTagDeleteEvent = {
  uiConfigTag?: strims_chat_v1_IUIConfigTag;
}

export class UIConfigTagDeleteEvent {
  uiConfigTag: strims_chat_v1_UIConfigTag | undefined;

  constructor(v?: IUIConfigTagDeleteEvent) {
    this.uiConfigTag = v?.uiConfigTag && new strims_chat_v1_UIConfigTag(v.uiConfigTag);
  }

  static encode(m: UIConfigTagDeleteEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.uiConfigTag) strims_chat_v1_UIConfigTag.encode(m.uiConfigTag, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UIConfigTagDeleteEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UIConfigTagDeleteEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.uiConfigTag = strims_chat_v1_UIConfigTag.decode(r, r.uint32());
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

export type IUIConfigIgnoreChangeEvent = {
  uiConfigIgnore?: strims_chat_v1_IUIConfigIgnore;
}

export class UIConfigIgnoreChangeEvent {
  uiConfigIgnore: strims_chat_v1_UIConfigIgnore | undefined;

  constructor(v?: IUIConfigIgnoreChangeEvent) {
    this.uiConfigIgnore = v?.uiConfigIgnore && new strims_chat_v1_UIConfigIgnore(v.uiConfigIgnore);
  }

  static encode(m: UIConfigIgnoreChangeEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.uiConfigIgnore) strims_chat_v1_UIConfigIgnore.encode(m.uiConfigIgnore, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UIConfigIgnoreChangeEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UIConfigIgnoreChangeEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.uiConfigIgnore = strims_chat_v1_UIConfigIgnore.decode(r, r.uint32());
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

export type IUIConfigIgnoreDeleteEvent = {
  uiConfigIgnore?: strims_chat_v1_IUIConfigIgnore;
}

export class UIConfigIgnoreDeleteEvent {
  uiConfigIgnore: strims_chat_v1_UIConfigIgnore | undefined;

  constructor(v?: IUIConfigIgnoreDeleteEvent) {
    this.uiConfigIgnore = v?.uiConfigIgnore && new strims_chat_v1_UIConfigIgnore(v.uiConfigIgnore);
  }

  static encode(m: UIConfigIgnoreDeleteEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.uiConfigIgnore) strims_chat_v1_UIConfigIgnore.encode(m.uiConfigIgnore, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): UIConfigIgnoreDeleteEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new UIConfigIgnoreDeleteEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.uiConfigIgnore = strims_chat_v1_UIConfigIgnore.decode(r, r.uint32());
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
    const parentEnd = r.enter(length);
    const end = r.end;
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
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type IWhisperThreadChangeEvent = {
  whisperThread?: strims_chat_v1_IWhisperThread;
}

export class WhisperThreadChangeEvent {
  whisperThread: strims_chat_v1_WhisperThread | undefined;

  constructor(v?: IWhisperThreadChangeEvent) {
    this.whisperThread = v?.whisperThread && new strims_chat_v1_WhisperThread(v.whisperThread);
  }

  static encode(m: WhisperThreadChangeEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.whisperThread) strims_chat_v1_WhisperThread.encode(m.whisperThread, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WhisperThreadChangeEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new WhisperThreadChangeEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.whisperThread = strims_chat_v1_WhisperThread.decode(r, r.uint32());
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

export type IWhisperThreadDeleteEvent = {
  whisperThread?: strims_chat_v1_IWhisperThread;
}

export class WhisperThreadDeleteEvent {
  whisperThread: strims_chat_v1_WhisperThread | undefined;

  constructor(v?: IWhisperThreadDeleteEvent) {
    this.whisperThread = v?.whisperThread && new strims_chat_v1_WhisperThread(v.whisperThread);
  }

  static encode(m: WhisperThreadDeleteEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.whisperThread) strims_chat_v1_WhisperThread.encode(m.whisperThread, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WhisperThreadDeleteEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new WhisperThreadDeleteEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.whisperThread = strims_chat_v1_WhisperThread.decode(r, r.uint32());
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

export type IWhisperRecordChangeEvent = {
  whisperRecord?: strims_chat_v1_IWhisperRecord;
}

export class WhisperRecordChangeEvent {
  whisperRecord: strims_chat_v1_WhisperRecord | undefined;

  constructor(v?: IWhisperRecordChangeEvent) {
    this.whisperRecord = v?.whisperRecord && new strims_chat_v1_WhisperRecord(v.whisperRecord);
  }

  static encode(m: WhisperRecordChangeEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.whisperRecord) strims_chat_v1_WhisperRecord.encode(m.whisperRecord, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WhisperRecordChangeEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new WhisperRecordChangeEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.whisperRecord = strims_chat_v1_WhisperRecord.decode(r, r.uint32());
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

export type IWhisperRecordDeleteEvent = {
  whisperRecord?: strims_chat_v1_IWhisperRecord;
}

export class WhisperRecordDeleteEvent {
  whisperRecord: strims_chat_v1_WhisperRecord | undefined;

  constructor(v?: IWhisperRecordDeleteEvent) {
    this.whisperRecord = v?.whisperRecord && new strims_chat_v1_WhisperRecord(v.whisperRecord);
  }

  static encode(m: WhisperRecordDeleteEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.whisperRecord) strims_chat_v1_WhisperRecord.encode(m.whisperRecord, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WhisperRecordDeleteEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new WhisperRecordDeleteEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.whisperRecord = strims_chat_v1_WhisperRecord.decode(r, r.uint32());
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


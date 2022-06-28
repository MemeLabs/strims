import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  strims_chat_v1_Emote,
  strims_chat_v1_IEmote,
  strims_chat_v1_Modifier,
  strims_chat_v1_IModifier,
  strims_chat_v1_Server,
  strims_chat_v1_IServer,
  strims_chat_v1_Tag,
  strims_chat_v1_ITag,
  strims_chat_v1_UIConfig,
  strims_chat_v1_IUIConfig,
  strims_chat_v1_WhisperRecord,
  strims_chat_v1_IWhisperRecord,
  strims_chat_v1_WhisperThread,
  strims_chat_v1_IWhisperThread,
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new WhisperThreadChangeEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.whisperThread = strims_chat_v1_WhisperThread.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new WhisperRecordChangeEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.whisperRecord = strims_chat_v1_WhisperRecord.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new WhisperRecordDeleteEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.whisperRecord = strims_chat_v1_WhisperRecord.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

/* @internal */
export const strims_chat_v1_ServerChangeEvent = ServerChangeEvent;
/* @internal */
export type strims_chat_v1_ServerChangeEvent = ServerChangeEvent;
/* @internal */
export type strims_chat_v1_IServerChangeEvent = IServerChangeEvent;
/* @internal */
export const strims_chat_v1_ServerDeleteEvent = ServerDeleteEvent;
/* @internal */
export type strims_chat_v1_ServerDeleteEvent = ServerDeleteEvent;
/* @internal */
export type strims_chat_v1_IServerDeleteEvent = IServerDeleteEvent;
/* @internal */
export const strims_chat_v1_EmoteChangeEvent = EmoteChangeEvent;
/* @internal */
export type strims_chat_v1_EmoteChangeEvent = EmoteChangeEvent;
/* @internal */
export type strims_chat_v1_IEmoteChangeEvent = IEmoteChangeEvent;
/* @internal */
export const strims_chat_v1_EmoteDeleteEvent = EmoteDeleteEvent;
/* @internal */
export type strims_chat_v1_EmoteDeleteEvent = EmoteDeleteEvent;
/* @internal */
export type strims_chat_v1_IEmoteDeleteEvent = IEmoteDeleteEvent;
/* @internal */
export const strims_chat_v1_ModifierChangeEvent = ModifierChangeEvent;
/* @internal */
export type strims_chat_v1_ModifierChangeEvent = ModifierChangeEvent;
/* @internal */
export type strims_chat_v1_IModifierChangeEvent = IModifierChangeEvent;
/* @internal */
export const strims_chat_v1_ModifierDeleteEvent = ModifierDeleteEvent;
/* @internal */
export type strims_chat_v1_ModifierDeleteEvent = ModifierDeleteEvent;
/* @internal */
export type strims_chat_v1_IModifierDeleteEvent = IModifierDeleteEvent;
/* @internal */
export const strims_chat_v1_TagChangeEvent = TagChangeEvent;
/* @internal */
export type strims_chat_v1_TagChangeEvent = TagChangeEvent;
/* @internal */
export type strims_chat_v1_ITagChangeEvent = ITagChangeEvent;
/* @internal */
export const strims_chat_v1_TagDeleteEvent = TagDeleteEvent;
/* @internal */
export type strims_chat_v1_TagDeleteEvent = TagDeleteEvent;
/* @internal */
export type strims_chat_v1_ITagDeleteEvent = ITagDeleteEvent;
/* @internal */
export const strims_chat_v1_UIConfigChangeEvent = UIConfigChangeEvent;
/* @internal */
export type strims_chat_v1_UIConfigChangeEvent = UIConfigChangeEvent;
/* @internal */
export type strims_chat_v1_IUIConfigChangeEvent = IUIConfigChangeEvent;
/* @internal */
export const strims_chat_v1_SyncAssetsEvent = SyncAssetsEvent;
/* @internal */
export type strims_chat_v1_SyncAssetsEvent = SyncAssetsEvent;
/* @internal */
export type strims_chat_v1_ISyncAssetsEvent = ISyncAssetsEvent;
/* @internal */
export const strims_chat_v1_WhisperThreadChangeEvent = WhisperThreadChangeEvent;
/* @internal */
export type strims_chat_v1_WhisperThreadChangeEvent = WhisperThreadChangeEvent;
/* @internal */
export type strims_chat_v1_IWhisperThreadChangeEvent = IWhisperThreadChangeEvent;
/* @internal */
export const strims_chat_v1_WhisperRecordChangeEvent = WhisperRecordChangeEvent;
/* @internal */
export type strims_chat_v1_WhisperRecordChangeEvent = WhisperRecordChangeEvent;
/* @internal */
export type strims_chat_v1_IWhisperRecordChangeEvent = IWhisperRecordChangeEvent;
/* @internal */
export const strims_chat_v1_WhisperRecordDeleteEvent = WhisperRecordDeleteEvent;
/* @internal */
export type strims_chat_v1_WhisperRecordDeleteEvent = WhisperRecordDeleteEvent;
/* @internal */
export type strims_chat_v1_IWhisperRecordDeleteEvent = IWhisperRecordDeleteEvent;

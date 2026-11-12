// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { EventEmitter } from "events";
import { PassThrough } from "stream";

import { Readable } from "@memelabs/protobuf/lib/rpc/stream";
import { Base64 } from "js-base64";

import * as chatv1 from "../../../apis/strims/chat/v1/chat";
import {
  ChatFrontendService,
  ChatServerFrontendService,
  UnimplementedChatFrontendService,
  UnimplementedChatServerFrontendService,
} from "../../../apis/strims/chat/v1/chat_rpc";
import MessageEmitter, { peerKey } from "../../mocks/chat/MessageEmitter";
import assetBundle, { modifiers } from "../../mocks/chat/assetBundle";
import { profileKey } from "../../storybook/fixtures/profile";

/** The chat UI's defaults, as the Chat provider starts with them. */
export const defaultUIConfig: chatv1.IUIConfig = {
  showTime: false,
  showFlairIcons: true,
  timestampFormat: "HH:mm",
  maxLines: 1024,
  notificationWhisper: true,
  soundNotificationWhisper: false,
  notificationHighlight: true,
  soundNotificationHighlight: false,
  highlight: true,
  showRemoved: chatv1.UIConfig.ShowRemoved.SHOW_REMOVED_REMOVE,
  showWhispersInChat: true,
  focusMentioned: false,
  notificationTimeout: true,
  ignoreMentions: false,
  autocompleteHelper: true,
  autocompleteEmotePreview: true,
  taggedVisibility: false,
  hideNsfw: false,
  animateForever: true,
  formatterGreen: true,
  formatterEmote: true,
  formatterCombo: true,
  emoteModifiers: true,
  disableSpoilers: false,
  shortenLinks: true,
  compactEmoteSpacing: false,
  normalizeAliasCase: true,
  userPresenceIndicator: chatv1.UIConfig.UserPresenceIndicator.USER_PRESENCE_INDICATOR_BAR,
  emojiSkinTone: "",
  replicate: true,
};

/** A whisper thread the mock holds, with its messages. */
export interface MockWhisperThread {
  alias: string;
  unreadCount?: number;
  /** Milliseconds since the epoch. */
  lastMessageTime?: bigint;
  records?: { message: chatv1.Message; state?: chatv1.MessageState }[];
}

export interface ChatServiceOptions {
  /**
   * Each room's asset bundle; default the mock's emotes, modifiers and tags.
   * null holds it back, the room never getting its emotes.
   */
  assetBundle?: (() => Promise<chatv1.AssetBundle>) | null;
  /**
   * A delta bundle (a room's name, its icon) after the first, or a function
   * giving it, by the room's server key (URL-safe base64).
   */
  roomBundles?: Record<string, chatv1.IAssetBundle | (() => Promise<chatv1.IAssetBundle>)>;
  /** Whether openClient's stream says the room is open (default true). */
  open?: boolean;
  /** Fields over defaultUIConfig, which watchUIConfig then answers first. */
  uiConfig?: Partial<chatv1.IUIConfig>;
  /** Aliases highlighted, tagged (with a colour) and ignored. */
  highlights?: string[];
  tags?: Record<string, string>;
  ignores?: string[];
  /** Whisper threads, which watchWhispers answers first and listWhispers by peer key (peerKey(alias)). */
  whispers?: MockWhisperThread[];
}

const uiConfigEvent = (config: chatv1.IWatchUIConfigResponse["config"]) =>
  new chatv1.WatchUIConfigResponse({ config });

// A chat server (ChatFrontend and ChatServerFrontend) over a message emitter.
// Each openClient gets the emitter's messages and the asset bundle;
// setUIConfig, highlight, tag and ignore (and their undoing) go to every
// watchUIConfig; whisper, deleteWhisperThread to every watchWhispers.
class ChatService {
  messages: Readable<chatv1.Message[]>;
  assetBundles: PassThrough;
  uiConfigs: PassThrough;
  whisperEvents: PassThrough;
  options: ChatServiceOptions;
  private bundle: Promise<chatv1.AssetBundle>;
  private nextId = 1;

  constructor(messages = new MessageEmitter({ ivl: 1000 }), options: ChatServiceOptions = {}) {
    this.messages = messages;
    this.options = options;
    this.assetBundles = new PassThrough({ objectMode: true });
    this.uiConfigs = new PassThrough({ objectMode: true });
    this.whisperEvents = new PassThrough({ objectMode: true });
    // Several rooms' streams each listen.
    for (const s of [this.messages, this.assetBundles, this.uiConfigs, this.whisperEvents]) {
      s.setMaxListeners(100);
    }
  }

  destroy(): void {
    this.messages.destroy();
    this.assetBundles.destroy();
    this.uiConfigs.destroy();
    this.whisperEvents.destroy();
  }

  emitMessage(message: chatv1.Message): void {
    this.messages.push([message]);
  }

  emitAssetBundle(bundle: chatv1.AssetBundle): void {
    this.assetBundles.push(bundle);
  }

  // Updates whisper thread `i` of the options (its unread count, say) in
  // every watchWhispers.
  emitWhisperThread(i: number, update: Partial<MockWhisperThread> = {}): void {
    const thread = this.whisperThread({ ...this.options.whispers[i], ...update }, i);
    this.whisperEvents.push(
      new chatv1.WatchWhispersResponse({ peerKey: thread.peerKey, body: { threadUpdate: thread } })
    );
  }

  // Forwards `source`'s data to `ch` while ch is open.
  private forward<T>(source: EventEmitter, ch: PassThrough, map: (v: T) => unknown) {
    const listener = (v: T) => ch.push(map(v));
    source.on("data", listener);
    ch.on("close", () => source.off("data", listener));
  }

  openClient(req: chatv1.OpenClientRequest): Readable<chatv1.OpenClientResponse> {
    const ch = new PassThrough({ objectMode: true });
    const { assetBundle: loadBundle = assetBundle, roomBundles = {}, open = true } = this.options;

    if (open) {
      window.setTimeout(() =>
        ch.push(new chatv1.OpenClientResponse({ body: { open: { clientId: BigInt(1) } } }))
      );
    }

    const pushBundle = (assetBundle: chatv1.IAssetBundle) =>
      ch.push(new chatv1.OpenClientResponse({ body: { assetBundle } }));
    this.forward(
      this.assetBundles,
      ch,
      (assetBundle: chatv1.AssetBundle) => new chatv1.OpenClientResponse({ body: { assetBundle } })
    );
    const room = roomBundles[Base64.fromUint8Array(req.serverKey, true)];
    if (loadBundle) {
      this.bundle ??= loadBundle();
    }
    void (async () => {
      if (loadBundle) {
        pushBundle(await this.bundle);
      }
      if (room) {
        pushBundle({ isDelta: true, ...(typeof room === "function" ? await room() : room) });
      }
    })();

    this.forward(
      this.messages,
      ch,
      (messages: chatv1.Message[]) =>
        new chatv1.OpenClientResponse({
          body: {
            serverEvents: {
              events: messages.map((message) => ({ body: { message } })),
            },
          },
        })
    );

    return ch;
  }

  clientSendMessage(req: chatv1.ClientSendMessageRequest): chatv1.ClientSendMessageResponse {
    this.messages.push([
      new chatv1.Message({
        nick: "test_user",
        peerKey: profileKey,
        serverTime: BigInt(Date.now()),
        body: req.body,
        entities: new chatv1.Message.Entities(),
      }),
    ]);
    return new chatv1.ClientSendMessageResponse();
  }

  clientMute(): chatv1.ClientMuteResponse {
    return new chatv1.ClientMuteResponse();
  }

  clientUnmute(): chatv1.ClientUnmuteResponse {
    return new chatv1.ClientUnmuteResponse();
  }

  setUIConfig(req: chatv1.SetUIConfigRequest): chatv1.SetUIConfigResponse {
    this.uiConfigs.push(uiConfigEvent({ uiConfig: req.uiConfig }));
    return new chatv1.SetUIConfigResponse();
  }

  watchUIConfig(): Readable<chatv1.WatchUIConfigResponse> {
    const ch = new PassThrough({ objectMode: true });
    const { uiConfig, highlights = [], tags = {}, ignores = [] } = this.options;

    if (uiConfig) {
      ch.push(uiConfigEvent({ uiConfig: { ...defaultUIConfig, ...uiConfig } }));
    }
    for (const alias of highlights) {
      ch.push(uiConfigEvent({ uiConfigHighlight: this.highlightOf(alias) }));
    }
    for (const [alias, color] of Object.entries(tags)) {
      ch.push(uiConfigEvent({ uiConfigTag: { ...this.highlightOf(alias), color } }));
    }
    for (const alias of ignores) {
      ch.push(uiConfigEvent({ uiConfigIgnore: this.highlightOf(alias) }));
    }
    this.forward(this.uiConfigs, ch, (e: chatv1.WatchUIConfigResponse) => e);

    return ch;
  }

  private highlightOf(alias: string) {
    return { id: BigInt(this.nextId++), alias, peerKey: peerKey(alias) };
  }

  highlight(req: chatv1.HighlightRequest): chatv1.HighlightResponse {
    this.uiConfigs.push(uiConfigEvent({ uiConfigHighlight: this.highlightOf(req.alias) }));
    return new chatv1.HighlightResponse();
  }

  unhighlight(req: chatv1.UnhighlightRequest): chatv1.UnhighlightResponse {
    this.uiConfigs.push(uiConfigEvent({ uiConfigHighlightDelete: { peerKey: req.peerKey } }));
    return new chatv1.UnhighlightResponse();
  }

  tag(req: chatv1.TagRequest): chatv1.TagResponse {
    this.uiConfigs.push(
      uiConfigEvent({ uiConfigTag: { ...this.highlightOf(req.alias), color: req.color } })
    );
    return new chatv1.TagResponse();
  }

  untag(req: chatv1.UntagRequest): chatv1.UntagResponse {
    this.uiConfigs.push(uiConfigEvent({ uiConfigTagDelete: { peerKey: req.peerKey } }));
    return new chatv1.UntagResponse();
  }

  ignore(req: chatv1.IgnoreRequest): chatv1.IgnoreResponse {
    this.uiConfigs.push(uiConfigEvent({ uiConfigIgnore: this.highlightOf(req.alias) }));
    return new chatv1.IgnoreResponse();
  }

  unignore(req: chatv1.UnignoreRequest): chatv1.UnignoreResponse {
    this.uiConfigs.push(uiConfigEvent({ uiConfigIgnoreDelete: { peerKey: req.peerKey } }));
    return new chatv1.UnignoreResponse();
  }

  private whisperThread(t: MockWhisperThread, i: number): chatv1.WhisperThread {
    return new chatv1.WhisperThread({
      id: BigInt(i + 1),
      peerKey: peerKey(t.alias),
      alias: t.alias,
      unreadCount: t.unreadCount ?? 0,
      hasUnread: (t.unreadCount ?? 0) > 0,
      lastMessageTime:
        t.lastMessageTime ?? t.records?.[t.records.length - 1]?.message.serverTime ?? BigInt(0),
    });
  }

  private findWhisperThread(key: Uint8Array): [MockWhisperThread, number] {
    const b64 = Base64.fromUint8Array(key, true);
    const i = (this.options.whispers ?? []).findIndex(
      (t) => Base64.fromUint8Array(peerKey(t.alias), true) === b64
    );
    return [this.options.whispers?.[i], i];
  }

  // A thread's records; an unknown peer's thread is empty.
  listWhispers(req: chatv1.ListWhispersRequest): chatv1.ListWhispersResponse {
    const [t, i] = this.findWhisperThread(req.peerKey);
    if (!t) {
      return new chatv1.ListWhispersResponse({
        thread: new chatv1.WhisperThread({ peerKey: req.peerKey, alias: `peer_${req.peerKey[0]}` }),
        whispers: [],
      });
    }
    return new chatv1.ListWhispersResponse({
      thread: this.whisperThread(t, i),
      whispers: (t.records ?? []).map(
        ({ message, state = chatv1.MessageState.MESSAGE_STATE_DELIVERED }, j) =>
          new chatv1.WhisperRecord({
            id: BigInt((i + 1) * 1000 + j),
            threadId: BigInt(i + 1),
            peerKey: req.peerKey,
            state,
            message,
          })
      ),
    });
  }

  watchWhispers(): Readable<chatv1.WatchWhispersResponse> {
    const ch = new PassThrough({ objectMode: true });
    (this.options.whispers ?? []).forEach((t, i) => {
      const thread = this.whisperThread(t, i);
      ch.push(
        new chatv1.WatchWhispersResponse({
          peerKey: thread.peerKey,
          body: { threadUpdate: thread },
        })
      );
    });
    this.forward(this.whisperEvents, ch, (e: chatv1.WatchWhispersResponse) => e);
    return ch;
  }

  markWhispersRead(): chatv1.MarkWhispersReadResponse {
    return new chatv1.MarkWhispersReadResponse();
  }

  // Sends to a known thread's peer, delivered at once.
  whisper(req: chatv1.WhisperRequest): chatv1.WhisperResponse {
    const key = req.peerKey.length ? req.peerKey : peerKey(req.alias);
    const [, i] = this.findWhisperThread(key);
    this.whisperEvents.push(
      new chatv1.WatchWhispersResponse({
        peerKey: key,
        body: {
          whisperUpdate: {
            id: BigInt(this.nextId++ + 1000000),
            threadId: BigInt(i + 1),
            peerKey: key,
            state: chatv1.MessageState.MESSAGE_STATE_DELIVERED,
            message: {
              nick: "test_user",
              peerKey: profileKey,
              serverTime: BigInt(Date.now()),
              body: req.body,
              entities: {},
            },
          },
        },
      })
    );
    return new chatv1.WhisperResponse();
  }

  deleteWhisperThread(req: chatv1.DeleteWhisperThreadRequest): chatv1.DeleteWhisperThreadResponse {
    this.whisperEvents.push(
      new chatv1.WatchWhispersResponse({ peerKey: req.peerKey, body: { threadDelete: {} } })
    );
    return new chatv1.DeleteWhisperThreadResponse();
  }

  // ChatServerFrontendService

  listModifiers(): chatv1.ListModifiersResponse {
    return new chatv1.ListModifiersResponse({ modifiers });
  }

  listEmoteLabels(): chatv1.ListEmoteLabelsResponse {
    return new chatv1.ListEmoteLabelsResponse({ labels: ["animated", "classic", "seasonal"] });
  }
}

interface ChatService extends ChatFrontendService, ChatServerFrontendService {}

Object.defineProperties(ChatService.prototype, {
  ...Object.getOwnPropertyDescriptors(UnimplementedChatFrontendService.prototype),
  ...Object.getOwnPropertyDescriptors(UnimplementedChatServerFrontendService.prototype),
  ...Object.getOwnPropertyDescriptors(ChatService.prototype),
});

export default ChatService;

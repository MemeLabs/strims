// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Readable } from "@memelabs/protobuf/lib/rpc/stream";
import { CompactEmoji, MessagesDataset, ShortcodesDataset } from "emojibase";
import { Base64 } from "js-base64";
import { isEqual } from "lodash";
import React, {
  ReactNode,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";

import { FrontendClient } from "../apis/client";
import {
  AssetBundle,
  Emote,
  EmoteEffect,
  IUIConfig,
  Message,
  Modifier,
  OpenClientResponse,
  Room,
  ServerEvent,
  Tag,
  UIConfig,
  UIConfigHighlight,
  UIConfigIgnore,
  UIConfigTag,
  WatchUIConfigResponse,
  WatchWhispersResponse,
  WhisperRecord,
  WhisperThread,
} from "../apis/strims/chat/v1/chat";
import { FrontendJoinResponse } from "../apis/strims/network/v1/directory/directory";
import { Image } from "../apis/strims/type/image";
import { useUserList } from "../hooks/chat";
import curryDispatchActions from "../lib/curryDispatchActions";
import MessageSizeCache from "../lib/MessageSizeCache";
import { applyActionInStateMap, deleteFromStateMap, setInStateMap } from "../lib/stateMap";
import { useClient } from "./FrontendApi";

export interface Style {
  name: string;
  animated: boolean;
  modifiers: string[];
}

export interface ChatStyles {
  emotes: Map<string, Style>;
  modifiers: Map<string, Modifier>;
  tags: Tag[];
  selectedPeers: Set<string>;
}

export const enum ThreadInitState {
  NEW,
  INITIALIZED,
  OPEN,
  CLOSED,
}

export interface ThreadState {
  id: number;
  topic: Topic;
  messages: Message[];
  messageGCEnabled: boolean;
  messageSizeCache: MessageSizeCache;
  assetBundles: AssetBundle[];
  liveEmotes: Emote[];
  styles: ChatStyles;
  emotes: string[];
  modifiers: string[];
  nicks: string[];
  tags: string[];
  commands: string[];
  errors: Error[];
  state: ThreadInitState;
  label: string;
  icon: Image;
  unreadCount: number;
  visible: boolean;
}

export interface WhisperThreadState extends ThreadState {
  networkKeys: Uint8Array[];
  peerKey: Uint8Array;
  thread: WhisperThread;
  messageIndex: Map<bigint, Message>;
}

export interface RoomThreadState extends ThreadState {
  networkKey: Uint8Array;
  serverKey: Uint8Array;
  serverEvents: Readable<OpenClientResponse>;
  directoryJoinRes: Promise<FrontendJoinResponse>;
  room: Room;
}

export interface Topic {
  type: "ROOM" | "WHISPER";
  topicKey: Uint8Array;
}

const topicThreadKeys = {
  "WHISPER": "whispers",
  "ROOM": "rooms",
} as const;

export interface State {
  nextId: number;
  uiConfig: UIConfig;
  uiConfigHighlights: Map<string, UIConfigHighlight>;
  uiConfigTags: Map<string, UIConfigTag>;
  uiConfigIgnores: Map<string, UIConfigIgnore>;
  config: {
    messageGCThreshold: number;
  };
  rooms: Map<string, RoomThreadState>;
  whispers: Map<string, WhisperThreadState>;
  whisperThreads: Map<string, WhisperThread>;
  popoutTopics: Topic[];
  popoutTopicCapacity: number;
  mainTopics: Topic[];
  mainActiveTopic?: Topic;
  emoji?: {
    emoji: CompactEmoji[];
    messages: MessagesDataset;
    shortcodes: ShortcodesDataset;
  };
}

type StateDispatcher = React.Dispatch<React.SetStateAction<State>>;
type ThreadStateActionApplier<T extends ThreadState> = (
  state: State,
  action: (thread: T, state: State) => T
) => State;

const initialRoomState: ThreadState = {
  id: 0,
  topic: null,
  messages: [],
  messageGCEnabled: true,
  messageSizeCache: null,
  assetBundles: [],
  liveEmotes: [],
  styles: {
    emotes: new Map(),
    modifiers: new Map(),
    tags: [],
    selectedPeers: new Set(),
  },
  emotes: [],
  modifiers: [],
  nicks: [],
  tags: [],
  commands: [],
  errors: [],
  state: ThreadInitState.NEW,
  label: "",
  icon: null,
  unreadCount: 0,
  visible: true,
};

const initialState: State = {
  nextId: 1,
  uiConfig: new UIConfig({
    showTime: false,
    showFlairIcons: true,
    timestampFormat: "HH:mm",
    maxLines: 1024,
    notificationWhisper: true,
    soundNotificationWhisper: false,
    notificationHighlight: true,
    soundNotificationHighlight: false,
    highlight: true,
    showRemoved: UIConfig.ShowRemoved.SHOW_REMOVED_REMOVE,
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
    userPresenceIndicator: UIConfig.UserPresenceIndicator.USER_PRESENCE_INDICATOR_BAR,
    emojiSkinTone: "",
  }),
  uiConfigHighlights: new Map(),
  uiConfigTags: new Map(),
  uiConfigIgnores: new Map(),
  config: {
    messageGCThreshold: 1024,
  },
  rooms: new Map(),
  whispers: new Map(),
  whisperThreads: new Map(),
  popoutTopics: [],
  popoutTopicCapacity: 0,
  mainTopics: [],
};

const initialMessageHeight = 30;

const roomCommands = [
  "help",
  "emotes",
  "me",
  "message",
  "msg",
  "ignore",
  "unignore",
  "highlight",
  "unhighlight",
  "maxlines",
  "mute",
  "unmute",
  "subonly",
  "ban",
  "unban",
  "timestampformat",
  "tag",
  "untag",
  "resetconfig",
  "exit",
  "hideemote",
  "unhideemote",
  "spoiler",
];

type ChatActions = {
  mergeUIConfig: (config: Partial<IUIConfig>) => void;
  openRoom: (serverKey: Uint8Array, networkKey: Uint8Array) => void;
  openWhispers: (peerKey: Uint8Array, networkKeys?: Uint8Array[], alias?: string) => void;
  openTopicPopout: (topic: Topic) => void;
  setPopoutTopicCapacity: (popoutTopicCapacity: number) => void;
  closeTopic: (topic: Topic) => void;
  setMainActiveTopic: (topic: Topic) => void;
  resetTopicUnreadCount: (topic: Topic) => void;
};

const ChatContext = React.createContext<[State, ChatActions, StateDispatcher]>(null);

type ThreadActions = {
  sendMessage: (body: string) => void;
  getMessage: (index: number) => Message;
  getMessageCount: () => number;
  getMessageIsContinued: (index: number) => boolean;
  getNetworkKeys: () => Uint8Array[];
  toggleMessageGC: (messageGCEnabled: boolean) => void;
  toggleSelectedPeer: (peerKey: Uint8Array, state?: boolean) => void;
  resetSelectedPeers: () => void;
  toggleVisible: (visible: boolean) => void;
};

const ThreadContext = React.createContext<[ThreadState, ThreadActions]>(null);

const formatKey = (key: Uint8Array) => Base64.fromUint8Array(key, true);

export const selectWhispers = (state: State, key: Uint8Array) => state.whispers.get(formatKey(key));
export const selectRoom = (state: State, key: Uint8Array) => state.rooms.get(formatKey(key));

const reduceMessage = <T extends ThreadState>(room: T, state: State, message: Message): T => {
  const messages = [...room.messages];
  if (
    messages.length !== 0 &&
    message.entities.emotes.length === 1 &&
    message.entities.emotes[0].combo > 0
  ) {
    messages[messages.length - 1] = message;
    room.messageSizeCache.unset(messages.length - 1);
  } else {
    messages.push(message);
  }

  const messageOverflow = messages.length - state.uiConfig.maxLines;
  if (room.messageGCEnabled && messageOverflow > state.config.messageGCThreshold) {
    messages.splice(0, messageOverflow);
    room.messageSizeCache.prune(messageOverflow);
  }

  return {
    ...room,
    messages,
    unreadCount: room.visible ? 0 : room.unreadCount + 1,
  };
};

const createGlobalActions = (client: FrontendClient, setState: StateDispatcher) => {
  const applyTopicThreadAction = (
    state: State,
    topic: Topic,
    action: (thread: ThreadState, state: State) => ThreadState
  ) => applyActionInStateMap(state, topicThreadKeys[topic.type], formatKey(topic.topicKey), action);

  const selectThread = (state: State, topic: Topic) =>
    state[topicThreadKeys[topic.type]].get(formatKey(topic.topicKey));

  const openRoom = (state: State, serverKey: Uint8Array, networkKey: Uint8Array) => {
    const key = formatKey(serverKey);
    if (state.rooms.has(key)) {
      return state;
    }

    const roomActions = curryDispatchActions(
      setState,
      createRoomActions(client, setState, serverKey)
    );

    const serverEvents = client.chat.openClient({ networkKey, serverKey });
    serverEvents.on("data", roomActions.handleClientData);
    serverEvents.on("error", roomActions.handleClientError);
    serverEvents.on("close", roomActions.handleClientClose);

    const directoryJoinRes = client.directory.join({
      networkKey,
      query: { query: { listing: { content: { chat: { key: serverKey } } } } },
    });

    const topic: Topic = { type: "ROOM", topicKey: serverKey };

    return {
      ...state,
      nextId: state.nextId + 1,
      rooms: new Map(state.rooms).set(key, {
        ...initialRoomState,
        commands: roomCommands,
        id: state.nextId,
        topic,
        messageSizeCache: new MessageSizeCache(
          state.uiConfig.maxLines + state.config.messageGCThreshold,
          initialMessageHeight
        ),
        networkKey,
        serverKey,
        serverEvents,
        directoryJoinRes,
        room: new Room(),
      }),
      mainTopics: [...state.mainTopics, topic],
      mainActiveTopic: topic,
    };
  };

  const openWhispers = (
    state: State,
    peerKey: Uint8Array,
    networkKeys?: Uint8Array[],
    alias?: string
  ) => {
    const key = formatKey(peerKey);
    if (state.whispers.has(key)) {
      return state;
    }

    const whisperActions = curryDispatchActions(
      setState,
      createWhisperActions(client, setState, peerKey)
    );

    client.chat
      .listWhispers({ peerKey })
      .then(({ thread, whispers }) => whisperActions.setWhisperMessages(thread, whispers))
      .catch((error: Error) => whisperActions.handleWhisperError(error));

    const thread = state.whisperThreads.get(key);
    const topic: Topic = { type: "WHISPER", topicKey: peerKey };

    return {
      ...state,
      nextId: state.nextId + 1,
      whispers: new Map(state.whispers).set(key, {
        ...initialRoomState,
        id: state.nextId,
        topic,
        label: alias,
        messageSizeCache: new MessageSizeCache(
          state.uiConfig.maxLines + state.config.messageGCThreshold,
          initialMessageHeight
        ),
        peerKey: peerKey,
        networkKeys,
        thread: thread ?? new WhisperThread({ alias }),
        state: thread ? ThreadInitState.OPEN : ThreadInitState.INITIALIZED,
        messageIndex: new Map(),
        visible: true,
      }),
      mainTopics: [...state.mainTopics, topic],
      mainActiveTopic: topic,
    };
  };

  const closeRoom = (room: RoomThreadState) => {
    const { networkKey } = room;
    room.serverEvents.destroy();
    void room.directoryJoinRes.then(({ id }) => client.directory.part({ networkKey, id }));
  };

  const closeTopic = (state: State, topic: Topic) => {
    let { mainActiveTopic } = state;
    if (isEqual(mainActiveTopic, topic)) {
      const i = state.mainTopics.findIndex((t) => isEqual(t, topic));
      mainActiveTopic = state.mainTopics[i + 1] ?? state.mainTopics[i - 1];
    }

    const mainTopics = state.mainTopics.filter((t) => !isEqual(t, topic));
    const popoutTopics = state.popoutTopics.filter((t) => !isEqual(t, topic));

    state = { ...state, mainTopics, mainActiveTopic, popoutTopics };

    const key = formatKey(topic.topicKey);
    switch (topic.type) {
      case "ROOM":
        return applyActionInStateMap(state, "rooms", key, closeRoom);
      case "WHISPER":
        return deleteFromStateMap(state, "whispers", key);
    }
  };

  const setMainActiveTopic = (state: State, topic: Topic) => {
    const mainActiveTopic = state.mainTopics.find((t) => isEqual(t, topic));
    if (!mainActiveTopic) {
      return state;
    }
    return { ...state, mainActiveTopic };
  };

  const resetTopicUnreadCount = (state: State, topic: Topic) =>
    applyTopicThreadAction(state, topic, (thread) => {
      if (topic.type === "WHISPER") {
        const threadId = selectWhispers(state, topic.topicKey).thread.id;
        void client.chat.markWhispersRead({ threadId });
      }

      return { ...thread, unreadCount: 0 };
    });

  const openTopicPopout = (state: State, topic: Topic) => {
    const mainTopics = [...state.mainTopics];
    const mainTopicIndex = mainTopics.findIndex((t) => isEqual(t, topic));
    if (mainTopicIndex === -1) {
      return state;
    }
    mainTopics.splice(mainTopicIndex, 1);

    let { mainActiveTopic } = state;
    if (isEqual(mainActiveTopic, topic)) {
      mainActiveTopic = mainTopics[mainTopicIndex] ?? mainTopics[mainTopicIndex - 1];
    }

    const popoutTopics = [topic, ...state.popoutTopics];
    const overflow = popoutTopics.splice(state.popoutTopicCapacity);
    mainTopics.push(...overflow);

    for (const movedTopic of [topic, ...overflow]) {
      selectThread(state, movedTopic)?.messageSizeCache.reset();
    }

    return {
      ...state,
      mainTopics,
      mainActiveTopic,
      popoutTopics,
    };
  };

  const setPopoutTopicCapacity = (state: State, popoutTopicCapacity: number) => {
    const mainTopics = [...state.mainTopics];
    const popoutTopics = [...state.popoutTopics];
    const overflow = popoutTopics.splice(popoutTopicCapacity);
    mainTopics.push(...overflow);

    for (const movedTopic of overflow) {
      selectThread(state, movedTopic)?.messageSizeCache.reset();
    }

    return {
      ...state,
      mainTopics,
      mainActiveTopic: state.mainActiveTopic ?? mainTopics[0],
      popoutTopics,
      popoutTopicCapacity,
    };
  };

  const reduceUIConfigEvent = (state: State, res: WatchUIConfigResponse) => {
    for (const thread of state.rooms.values()) {
      thread.messageSizeCache.reset();
    }
    for (const thread of state.whispers.values()) {
      thread.messageSizeCache.reset();
    }

    switch (res.config.case) {
      case WatchUIConfigResponse.ConfigCase.UI_CONFIG:
        return {
          ...state,
          UIConfig: res.config.uiConfig,
          config: {
            ...state.config,
            messageGCThreshold: res.config.uiConfig.maxLines,
          },
        };
      case WatchUIConfigResponse.ConfigCase.UI_CONFIG_HIGHLIGHT: {
        const key = formatKey(res.config.uiConfigHighlight.peerKey);
        return setInStateMap(state, "uiConfigHighlights", key, res.config.uiConfigHighlight);
      }
      case WatchUIConfigResponse.ConfigCase.UI_CONFIG_HIGHLIGHT_DELETE: {
        const key = formatKey(res.config.uiConfigHighlightDelete.peerKey);
        return deleteFromStateMap(state, "uiConfigHighlights", key);
      }
      case WatchUIConfigResponse.ConfigCase.UI_CONFIG_TAG: {
        const key = formatKey(res.config.uiConfigTag.peerKey);
        return setInStateMap(state, "uiConfigTags", key, res.config.uiConfigTag);
      }
      case WatchUIConfigResponse.ConfigCase.UI_CONFIG_TAG_DELETE: {
        const key = formatKey(res.config.uiConfigTagDelete.peerKey);
        return deleteFromStateMap(state, "uiConfigTags", key);
      }
      case WatchUIConfigResponse.ConfigCase.UI_CONFIG_IGNORE: {
        const key = formatKey(res.config.uiConfigIgnore.peerKey);
        return setInStateMap(state, "uiConfigIgnores", key, res.config.uiConfigIgnore);
      }
      case WatchUIConfigResponse.ConfigCase.UI_CONFIG_IGNORE_DELETE: {
        const key = formatKey(res.config.uiConfigIgnoreDelete.peerKey);
        return deleteFromStateMap(state, "uiConfigIgnores", key);
      }
    }
  };

  const reduceWhisperEvent = (
    thread: WhisperThreadState,
    state: State,
    res: WatchWhispersResponse
  ): WhisperThreadState => {
    switch (res.body.case) {
      case WatchWhispersResponse.BodyCase.THREAD_UPDATE:
        return {
          ...thread,
          thread: res.body.threadUpdate,
          label: res.body.threadUpdate.alias,
          unreadCount: res.body.threadUpdate.unreadCount,
          state: ThreadInitState.OPEN,
        };
      case WatchWhispersResponse.BodyCase.THREAD_DELETE:
        return undefined;
      case WatchWhispersResponse.BodyCase.WHISPER_UPDATE: {
        if (thread.visible) {
          void client.chat.markWhispersRead({ threadId: thread.thread.id });
        }

        const { id, message } = res.body.whisperUpdate;
        const messageIndex = new Map(thread.messageIndex).set(id, message);
        return {
          ...thread,
          messageIndex,
          messages: Array.from(messageIndex.values()),
        };
      }
      case WatchWhispersResponse.BodyCase.WHISPER_DELETE: {
        const messageIndex = new Map(thread.messageIndex);
        messageIndex.delete(res.body.whisperDelete.recordId);
        return {
          ...thread,
          messageIndex,
          messages: Array.from(messageIndex.values()),
        };
      }
      default:
        return thread;
    }
  };

  const handleWhisperEvent = (state: State, res: WatchWhispersResponse) => {
    const key = formatKey(res.peerKey);

    if (res.body.case === WatchWhispersResponse.BodyCase.THREAD_UPDATE) {
      state = setInStateMap(state, "whisperThreads", key, res.body.threadUpdate);
    } else if (res.body.case === WatchWhispersResponse.BodyCase.THREAD_DELETE) {
      state = deleteFromStateMap(state, "whisperThreads", key);
      state = closeTopic(state, {
        type: "WHISPER",
        topicKey: res.peerKey,
      });
    }

    return applyActionInStateMap(state, "whispers", key, reduceWhisperEvent, res);
  };

  const mergeUIConfig = (state: State, values: Partial<IUIConfig>) => {
    void client.chat.setUIConfig({
      uiConfig: {
        ...state.uiConfig,
        ...values,
      },
    });

    // TODO: api req state?
    return state;
  };

  return {
    openRoom,
    openWhispers,
    reduceUIConfigEvent,
    handleWhisperEvent,
    openTopicPopout,
    setPopoutTopicCapacity,
    closeTopic,
    setMainActiveTopic,
    resetTopicUnreadCount,
    mergeUIConfig,
  };
};

const createWhisperActions = (
  client: FrontendClient,
  setState: StateDispatcher,
  peerKey: Uint8Array
) => {
  const applyWhisperAction = (
    state: State,
    action: (thread: WhisperThreadState, state: State) => WhisperThreadState
  ) => applyActionInStateMap(state, "whispers", formatKey(peerKey), action);

  const setWhisperMessages = (state: State, thread: WhisperThread, whispers: WhisperRecord[]) =>
    applyWhisperAction(state, (whisper) => ({
      ...whisper,
      thread,
      label: thread.alias,
      messageIndex: new Map(whispers.map(({ id, message }) => [id, message])),
      messages: whispers.map(({ message }) => message),
      state: ThreadInitState.OPEN,
    }));

  const handleWhisperError = (state: State, error: Error) =>
    applyWhisperAction(state, (whisper) => ({
      ...whisper,
      errors: [...whisper.errors, error],
    }));

  const sendMessage = (state: State, body: string) =>
    applyWhisperAction(state, (whisper) => {
      const { networkKeys } = whisper;

      client.chat
        .whisper({
          networkKey: networkKeys[0],
          peerKey,
          body,
        })
        .then((res) => console.log("send message res", res))
        .catch((err) => console.log("send message err", err));

      return whisper;
    });

  return {
    ...createThreadActions(applyWhisperAction),
    setWhisperMessages,
    handleWhisperError,
    sendMessage,
  };
};

const createThreadActions = <T extends ThreadState>(applyAction: ThreadStateActionApplier<T>) => {
  const toggleMessageGC = (state: State, messageGCEnabled: boolean) =>
    applyAction(state, (thread) => ({
      ...thread,
      messageGCEnabled,
    }));

  const toggleSelectedPeer = (state: State, peerKey: Uint8Array, value?: boolean) =>
    applyAction(state, (thread) => {
      const key = Base64.fromUint8Array(peerKey, true);
      const selectedPeers = new Set(thread.styles.selectedPeers);

      if (value === true) {
        selectedPeers.add(key);
      } else if (value === false) {
        selectedPeers.delete(key);
      } else if (selectedPeers.has(key)) {
        selectedPeers.delete(key);
      } else {
        selectedPeers.add(key);
      }

      return {
        ...thread,
        styles: {
          ...thread.styles,
          selectedPeers,
        },
      };
    });

  const resetSelectedPeers = (state: State) =>
    applyAction(state, (thread) => {
      if (thread.styles.selectedPeers.size === 0) {
        return thread;
      }
      return {
        ...thread,
        styles: {
          ...thread.styles,
          selectedPeers: new Set(),
        },
      };
    });

  const toggleVisible = (state: State, visible: boolean) =>
    applyAction(state, (thread) => ({ ...thread, visible }));

  return {
    toggleMessageGC,
    toggleSelectedPeer,
    resetSelectedPeers,
    toggleVisible,
  };
};

const createRoomActions = (
  client: FrontendClient,
  setState: StateDispatcher,
  serverKey: Uint8Array
) => {
  const applyRoomAction = (
    state: State,
    action: (thread: RoomThreadState, state: State) => RoomThreadState
  ) => applyActionInStateMap(state, "rooms", formatKey(serverKey), action);

  const reduceServerEvent = (
    state: State,
    room: RoomThreadState,
    events: ServerEvent[]
  ): RoomThreadState => {
    for (const event of events) {
      switch (event.body.case) {
        case ServerEvent.BodyCase.MESSAGE:
          room = reduceMessage(room, state, event.body.message);
      }
    }
    return room;
  };

  const toNames = (vs: { name: string }[]): string[] => vs.map(({ name }) => name).sort();

  const reduceAssetBundle = (state: RoomThreadState, bundle: AssetBundle): RoomThreadState => {
    state.messageSizeCache.reset();

    const assetBundles = bundle.isDelta ? [...state.assetBundles, bundle] : [bundle];
    const liveEmoteMap = new Map<bigint, Emote>();
    const liveModifierMap = new Map<bigint, Modifier>();
    const liveTagMap = new Map<bigint, Tag>();
    let { room, icon } = state;
    for (const b of assetBundles) {
      for (const id of b.removedIds) {
        liveEmoteMap.delete(id);
        liveModifierMap.delete(id);
        liveTagMap.delete(id);
      }
      b.emotes.forEach((e) => liveEmoteMap.set(e.id, e));
      b.modifiers.forEach((e) => liveModifierMap.set(e.id, e));
      b.tags.forEach((e) => liveTagMap.set(e.id, e));
      room = b.room ?? room;
      icon = b.icon ?? icon;
    }
    const liveEmotes = Array.from(liveEmoteMap.values()).sort((a, b) =>
      a.name.localeCompare(b.name)
    );
    const emoteStyles = new Map(
      liveEmotes.map(({ id, name, effects }) => {
        const style: Style = {
          name: `e_${name}_${state.id}_${id}`,
          animated: false,
          modifiers: [],
        };
        effects.forEach((e) => {
          switch (e.effect.case) {
            case EmoteEffect.EffectCase.SPRITE_ANIMATION:
              style.animated = true;
              break;
            case EmoteEffect.EffectCase.DEFAULT_MODIFIERS:
              style.modifiers = e.effect.defaultModifiers.modifiers;
              break;
          }
        });
        return [name, style];
      })
    );
    const liveModifiers = Array.from(liveModifierMap.values());
    const liveTags = Array.from(liveTagMap.values());

    return {
      ...state,
      room,
      label: room.name,
      icon,
      assetBundles,
      liveEmotes,
      styles: {
        ...state.styles,
        emotes: emoteStyles,
        modifiers: new Map(liveModifiers.map((m) => [m.name, m])),
        tags: liveTags,
      },
      emotes: toNames(liveEmotes),
      modifiers: toNames(liveModifiers.filter(({ internal }) => !internal)),
      tags: toNames(liveTags),
    };
  };

  const handleClientData = (state: State, event: OpenClientResponse) =>
    applyRoomAction(state, (room) => {
      switch (event.body.case) {
        case OpenClientResponse.BodyCase.OPEN:
          return {
            ...room,
            state: ThreadInitState.OPEN,
          };
        case OpenClientResponse.BodyCase.SERVER_EVENTS:
          return reduceServerEvent(state, room, event.body.serverEvents.events);
        case OpenClientResponse.BodyCase.ASSET_BUNDLE:
          return reduceAssetBundle(room, event.body.assetBundle);
        default:
          return room;
      }
    });

  const handleClientError = (state: State, error: Error) =>
    applyRoomAction(state, (room) => ({
      ...room,
      errors: [...room.errors, error],
    }));

  const handleClientClose = (state: State) =>
    applyRoomAction(state, (room) => ({
      ...room,
      state: ThreadInitState.CLOSED,
    }));

  const { mergeUIConfig, closeTopic } = createGlobalActions(client, setState);

  const sendMessage = (state: State, body: string) => {
    const { networkKey } = state.rooms.get(formatKey(serverKey));

    // TODO: handle rpc errors
    const commandFuncs: { [key: string]: (...args: string[]) => void } = {
      help: () => {
        console.log("help");
      },
      ignore: (alias: string, duration: string) => {
        if (alias) {
          void client.chat.ignore({ networkKey, alias, duration });
        } else {
          console.log("ignore");
        }
      },
      unignore: (alias: string) => {
        void client.chat.unignore({ networkKey, alias });
      },
      highlight: (alias: string) => {
        void client.chat.highlight({ networkKey, alias });
      },
      unhighlight: (alias: string) => {
        void client.chat.unhighlight({ networkKey, alias });
      },
      maxlines: (n: string) => {
        state = mergeUIConfig(state, { maxLines: parseInt(n) });
      },
      mute: (alias: string, duration: string, message: string) => {
        void client.chat.clientMute({ networkKey, serverKey, alias, duration, message });
      },
      unmute: (alias: string) => {
        void client.chat.clientUnmute({ networkKey, serverKey, alias });
      },
      timestampformat: (timestampFormat: string) => {
        state = mergeUIConfig(state, { timestampFormat });
      },
      tag: (alias: string, color: string) => {
        void client.chat.tag({ networkKey, alias, color });
      },
      untag: (alias: string) => {
        void client.chat.untag({ networkKey, alias });
      },
      whisper: (alias: string, body: string) => {
        void client.chat.whisper({ networkKey, alias, body });
      },
      exit: () => {
        state = closeTopic(state, { type: "ROOM", topicKey: serverKey });
      },
      hideemote: (name: string) => {
        const hiddenEmotes = Array.from(new Set(state.uiConfig.hiddenEmotes).add(name));
        state = mergeUIConfig(state, { hiddenEmotes });
      },
      unhideemote: (name: string) => {
        const hiddenEmotes = state.uiConfig.hiddenEmotes.filter((e) => e !== name);
        state = mergeUIConfig(state, { hiddenEmotes });
      },
      resetconfig: () => {
        state = mergeUIConfig(state, initialState.uiConfig);
      },
      me: (body: string) => {
        void client.chat.clientSendMessage({
          networkKey,
          serverKey,
          body: `/me ${body}`,
        });
      },
      spoiler: (body: string) => {
        void client.chat.clientSendMessage({
          networkKey,
          serverKey,
          body: `|| ${body} ||`,
        });
      },
    };

    const commandAliases: { [key: string]: string } = {
      "w": "whisper",
      "message": "whisper",
      "msg": "whisper",
      "tell": "whisper",
      "t": "whisper",
      "notify": "whisper",
    };

    if (body.startsWith("/")) {
      const command = body.split(" ", 1).pop().toLowerCase().substring(1);
      const func = commandFuncs[commandAliases[command] ?? command];
      if (func) {
        const args = body.split(" ", func.length);
        func(...[...args.slice(1), body.substring(args.reduce((n, a) => n + a.length + 1, 0))]);
      } else {
        // TODO: invalid command feedback
      }
    } else {
      void client.chat.clientSendMessage({ networkKey, serverKey, body });
    }

    // TODO: pending send state...
    return state;
  };

  return {
    ...createThreadActions(applyRoomAction),
    handleClientData,
    handleClientError,
    handleClientClose,
    sendMessage,
  };
};

interface ProviderProps {
  children: ReactNode;
}

export const Provider: React.FC<ProviderProps> = ({ children }) => {
  const client = useClient();
  const [state, setState] = useState(initialState);
  const actions = useMemo(
    () => curryDispatchActions(setState, createGlobalActions(client, setState)),
    [client]
  );

  useEffect(() => {
    const langExists = (lang: string) => EMOJI_LANG.includes(lang);

    // TODO: app language preference
    const langFull = navigator.language;
    const lang2Code = langFull.substring(0, 2);
    const lang = langExists(langFull) ? langFull : langExists(lang2Code) ? lang2Code : "en";

    void Promise.all([
      fetch(`/emoji/${lang}/compact.json`).then((res) => res.json()),
      fetch(`/emoji/${lang}/messages.json`).then((res) => res.json()),
      fetch(`/emoji/${lang}/shortcodes/cldr.json`).then((res) => res.json()),
    ]).then(([emoji, messages, shortcodes]: [CompactEmoji[], MessagesDataset, ShortcodesDataset]) =>
      setState((prev) => ({ ...prev, emoji: { emoji, messages, shortcodes } }))
    );
  }, []);

  useEffect(() => {
    const uiConfigEvents = client.chat.watchUIConfig();
    uiConfigEvents.on("data", actions.reduceUIConfigEvent);
    const whisperEvents = client.chat.watchWhispers();
    whisperEvents.on("data", actions.handleWhisperEvent);

    return () => {
      uiConfigEvents.destroy();
      whisperEvents.destroy();
    };
  }, [client]);

  const value = useMemo<[State, ChatActions, StateDispatcher]>(
    () => [state, actions, setState],
    [state]
  );
  return <ChatContext.Provider value={value}>{children}</ChatContext.Provider>;
};

Provider.displayName = "Chat.Provider";

export const useChat = (): [State, ChatActions, StateDispatcher] => useContext(ChatContext);

export const ChatConsumer = ChatContext.Consumer;

export interface ThreadProviderProps extends Topic {
  children: ReactNode;
}

export const ThreadProvider: React.FC<ThreadProviderProps> = (props) => {
  switch (props.type) {
    case "ROOM":
      return <RoomThreadProvider {...props} />;
    case "WHISPER":
      return <WhisperThreadProvider {...props} />;
  }
};

ThreadProvider.displayName = "Thread.Provider";

const useMessageAccessors = (messages: Message[]) => {
  const ref = useRef<Message[]>();
  ref.current = messages;

  const getMessage = useCallback((index: number): Message => ref.current[index], []);
  const getMessageCount = useCallback((): number => ref.current.length, []);

  const getMessageIsContinued = useCallback(
    (index: number): boolean =>
      isEqual(ref.current[index].peerKey, ref.current[index + 1]?.peerKey),
    []
  );

  return { getMessage, getMessageCount, getMessageIsContinued };
};

const RoomThreadProvider: React.FC<ThreadProviderProps> = ({ children, ...props }) => {
  const client = useClient();
  const [state, , setState] = useChat();

  const thread = state.rooms.get(formatKey(props.topicKey));
  const actions = useMemo(
    () => curryDispatchActions(setState, createRoomActions(client, setState, props.topicKey)),
    [props.type, props.topicKey]
  );

  const nicks = useUserList(thread.networkKey, thread.serverKey);
  const messageAccessors = useMessageAccessors(thread.messages);

  const getNetworkKeys = useCallback(() => [thread.networkKey], [thread.networkKey]);

  const value = useMemo<[ThreadState, ThreadActions]>(
    () => [
      { ...thread, nicks },
      { ...actions, ...messageAccessors, getNetworkKeys },
    ],
    [thread, nicks]
  );
  return <ThreadContext.Provider value={value}>{children}</ThreadContext.Provider>;
};

const WhisperThreadProvider: React.FC<ThreadProviderProps> = ({ children, ...props }) => {
  const client = useClient();
  const [state, , setState] = useChat();

  const thread = state.whispers.get(formatKey(props.topicKey));
  const actions = useMemo(
    () => curryDispatchActions(setState, createWhisperActions(client, setState, props.topicKey)),
    [props.type, props.topicKey]
  );

  const messageAccessors = useMessageAccessors(thread.messages);

  const getNetworkKeys = useCallback(() => thread.networkKeys, [thread.networkKeys]);

  const value = useMemo<[ThreadState, ThreadActions]>(
    () => [thread, { ...actions, ...messageAccessors, getNetworkKeys }],
    [thread]
  );
  return <ThreadContext.Provider value={value}>{children}</ThreadContext.Provider>;
};

export const useRoom = (): [ThreadState, ThreadActions] => useContext(ThreadContext);

export const ThreadConsumer = ThreadContext.Consumer;

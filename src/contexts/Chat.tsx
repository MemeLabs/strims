// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Readable } from "@memelabs/protobuf/lib/rpc/stream";
import { Base64 } from "js-base64";
import { isEqual } from "lodash";
import React, { useCallback, useContext, useEffect, useMemo, useRef, useState } from "react";

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
  WatchWhispersResponse,
  WhisperRecord,
  WhisperThread,
} from "../apis/strims/chat/v1/chat";
import { useStableCallback, useStableCallbacks } from "../hooks/useStableCallback";
import ChatCellMeasurerCache from "../lib/ChatCellMeasurerCache";
import { updateInStateMap } from "../lib/setInStateMap";
import { DirectoryListing } from "./Directory";
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

export interface UserMeta {
  alias: string;
  listing: DirectoryListing;
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
  messageSizeCache: ChatCellMeasurerCache;
  assetBundles: AssetBundle[];
  liveEmotes: Emote[];
  styles: ChatStyles;
  emotes: string[];
  modifiers: string[];
  nicks: string[];
  tags: string[];
  users: Map<string, UserMeta>;
  errors: Error[];
  state: ThreadInitState;
  label: string;
  unreadCount: number;
  visible: boolean;
}

export interface WhisperThreadState extends ThreadState {
  networkKeys: Uint8Array[];
  peerKey: Uint8Array;
  thread: WhisperThread;
}

export interface RoomThreadState extends ThreadState {
  networkKey: Uint8Array;
  serverKey: Uint8Array;
  serverEvents?: Readable<OpenClientResponse>;
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
}

type StateDispatcher = React.Dispatch<React.SetStateAction<State>>;
type ThreadStateDispatcher<T extends ThreadState> = (action: (room: T, state: State) => T) => void;

const initialRoomState: ThreadState = {
  id: 0,
  topic: null,
  messages: [],
  messageGCEnabled: true,
  messageSizeCache: new ChatCellMeasurerCache(),
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
  users: new Map(),
  errors: [],
  state: ThreadInitState.NEW,
  label: "",
  unreadCount: 0,
  visible: true,
};

const initialState: State = {
  nextId: 1,
  uiConfig: new UIConfig({
    showTime: false,
    showFlairIcons: true,
    timestampFormat: "HH:mm",
    maxLines: 250,
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
    viewerStateIndicator: UIConfig.ViewerStateIndicator.VIEWER_STATE_INDICATOR_BAR,
  }),
  config: {
    messageGCThreshold: 250,
  },
  rooms: new Map(),
  whispers: new Map(),
  whisperThreads: new Map(),
  popoutTopics: [],
  popoutTopicCapacity: 0,
  mainTopics: [],
};

type ChatActions = {
  mergeUIConfig: (config: Partial<IUIConfig>) => void;
  openRoom: (serverKey: Uint8Array, networkKey: Uint8Array) => void;
  openWhispers: (peerKey: Uint8Array, networkKeys?: Uint8Array[]) => void;
  openTopicPopout: (topic: Topic) => void;
  setPopoutTopicCapacity: (popoutTopicCapacity: number) => void;
  closeTopic: (topic: Topic) => void;
  setMainActiveTopic: (topic: Topic) => void;
  toggleTopicVisible: (topic: Topic, visible: boolean) => void;
  resetTopicUnreadCount: (topic: Topic) => void;
};

const ChatContext = React.createContext<[State, ChatActions, StateDispatcher]>(null);

type RoomActions = {
  sendMessage: (body: string) => void;
  getMessage: (index: number) => Message;
  getMessageCount: () => number;
  toggleMessageGC: (messageGCEnabled: boolean) => void;
  toggleSelectedPeer: (peerKey: Uint8Array, state?: boolean) => void;
  resetSelectedPeers: () => void;
};

const RoomContext = React.createContext<[ThreadState, RoomActions]>(null);

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
    room.messageSizeCache.clear(messages.length - 1, 0);
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
  const setTopicThread = (
    topic: Topic,
    action: (thread: ThreadState, state: State) => ThreadState
  ) => updateInStateMap(setState, topicThreadKeys[topic.type], formatKey(topic.topicKey), action);

  const openRoom = (serverKey: Uint8Array, networkKey: Uint8Array) =>
    setState((state) => {
      const key = formatKey(serverKey);
      if (state.rooms.has(key)) {
        return state;
      }

      const roomActions = createRoomActions(client, setState, serverKey);

      const serverEvents = client.chat.openClient({ networkKey, serverKey });
      serverEvents.on("data", roomActions.handleClientData);
      serverEvents.on("error", roomActions.handleClientError);
      serverEvents.on("close", roomActions.handleClientClose);

      const topic: Topic = { type: "ROOM", topicKey: serverKey };

      return {
        ...state,
        nextId: state.nextId + 1,
        rooms: new Map(state.rooms).set(key, {
          ...initialRoomState,
          id: state.nextId,
          topic,
          messageSizeCache: new ChatCellMeasurerCache(),
          networkKey,
          serverKey,
          serverEvents,
          room: new Room(),
        }),
        mainTopics: [...state.mainTopics, topic],
        mainActiveTopic: topic,
      };
    });

  const openWhispers = (peerKey: Uint8Array, networkKeys: Uint8Array[]) =>
    setState((state) => {
      const key = formatKey(peerKey);
      if (state.whispers.has(key)) {
        return state;
      }

      const whisperActions = createWhisperActions(client, setState, peerKey);

      client.chat
        .listWhispers({ peerKey })
        .then(({ thread, whispers }) => whisperActions.setWhisperMessages(thread, whispers))
        .catch((error: Error) => whisperActions.handleWhisperError(error));

      const topic: Topic = { type: "WHISPER", topicKey: peerKey };

      return {
        ...state,
        nextId: state.nextId + 1,
        whispers: new Map(state.whispers).set(key, {
          ...initialRoomState,
          id: state.nextId,
          topic,
          messageSizeCache: new ChatCellMeasurerCache(),
          peerKey: peerKey,
          networkKeys: networkKeys,
          thread: new WhisperThread(),
        }),
        mainTopics: [...state.mainTopics, topic],
        mainActiveTopic: topic,
      };
    });

  const closeRoom = (serverKey: Uint8Array) =>
    setState((state) => {
      const key = formatKey(serverKey);
      if (!state.rooms.has(key)) {
        return state;
      }

      state.rooms.get(key).serverEvents.destroy();

      const rooms = new Map(state.rooms);
      rooms.delete(key);
      return { ...state, rooms };
    });

  const closeTopic = (topic: Topic) => {
    setState((state) => {
      let { mainActiveTopic } = state;
      if (isEqual(mainActiveTopic, topic)) {
        const i = state.mainTopics.findIndex((t) => isEqual(t, topic));
        mainActiveTopic = state.mainTopics[i + 1] ?? state.mainTopics[i - 1];
      }

      const mainTopics = state.mainTopics.filter((t) => !isEqual(t, topic));
      const popoutTopics = state.popoutTopics.filter((t) => !isEqual(t, topic));
      return { ...state, mainTopics, mainActiveTopic, popoutTopics };
    });

    switch (topic.type) {
      case "ROOM":
        return closeRoom(topic.topicKey);
      case "WHISPER":
        return closeWhispers(topic.topicKey);
    }
  };

  const setMainActiveTopic = (topic: Topic) =>
    setState((state) => {
      const mainActiveTopic = state.mainTopics.find((t) => isEqual(t, topic));
      if (!mainActiveTopic) {
        return state;
      }
      return { ...state, mainActiveTopic };
    });

  const toggleTopicVisible = (topic: Topic, visible: boolean) =>
    setTopicThread(topic, (thread) => ({ ...thread, visible }));

  const resetTopicUnreadCount = (topic: Topic) =>
    setTopicThread(topic, (thread, state) => {
      if (topic.type === "WHISPER") {
        const threadId = selectWhispers(state, topic.topicKey).thread.id;
        void client.chat.markWhispersRead({ threadId });
      }

      return { ...thread, unreadCount: 0 };
    });

  const openTopicPopout = (topic: Topic) =>
    setState((state) => {
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
      if (popoutTopics.length > state.popoutTopicCapacity) {
        mainTopics.push(...popoutTopics.splice(state.popoutTopicCapacity));
      }

      return {
        ...state,
        mainTopics,
        mainActiveTopic,
        popoutTopics,
      };
    });

  const setPopoutTopicCapacity = (popoutTopicCapacity: number) =>
    setState((state) => {
      const mainTopics = [...state.mainTopics];
      const popoutTopics = [...state.popoutTopics];
      if (popoutTopics.length > popoutTopicCapacity) {
        mainTopics.push(...popoutTopics.splice(popoutTopicCapacity));
      }

      return {
        ...state,
        mainTopics,
        popoutTopics,
        popoutTopicCapacity,
      };
    });

  const closeWhispers = (peerKey: Uint8Array) =>
    setState((state) => {
      const whispers = new Map(state.whispers);
      whispers.delete(formatKey(peerKey));
      return { ...state, whispers };
    });

  const setUiConfig = (uiConfig: UIConfig) =>
    setState((state) => ({
      ...state,
      uiConfig,
    }));

  const reduceWhisperEvent = (
    state: State,
    thread: WhisperThreadState,
    res: WatchWhispersResponse
  ): WhisperThreadState => {
    switch (res.body.case) {
      case WatchWhispersResponse.BodyCase.THREAD_UPDATE:
        return {
          ...thread,
          thread: res.body.threadUpdate,
          label: res.body.threadUpdate.alias,
        };
      case WatchWhispersResponse.BodyCase.WHISPER_UPDATE:
        return reduceMessage(thread, state, res.body.whisperUpdate.message);
      case WatchWhispersResponse.BodyCase.WHISPER_DELETE:
      // TODO fallthrough
      default:
        return thread;
    }
  };

  const handleWhisperEvent = (res: WatchWhispersResponse) =>
    setState((state) => {
      const key = formatKey(res.peerKey);

      if (res.body.case === WatchWhispersResponse.BodyCase.THREAD_UPDATE) {
        state = {
          ...state,
          whisperThreads: new Map(state.whisperThreads).set(key, res.body.threadUpdate),
        };
      }

      if (!state.whispers.has(key)) {
        return state;
      }
      return {
        ...state,
        whispers: new Map(state.whispers).set(
          key,
          reduceWhisperEvent(state, state.whispers.get(key), res)
        ),
      };
    });

  return {
    openRoom,
    openWhispers,
    setUiConfig,
    handleWhisperEvent,
    openTopicPopout,
    setPopoutTopicCapacity,
    closeTopic,
    setMainActiveTopic,
    toggleTopicVisible,
    resetTopicUnreadCount,
  };
};

const reduceThreadState = <T extends ThreadState>(
  state: State,
  m: Map<string, T>,
  k: Uint8Array,
  action: (thread: T, state: State) => T
) => {
  const key = formatKey(k);
  const thread = m.get(key);
  return thread ? new Map(m).set(key, action(thread, state)) : m;
};

const createWhisperActions = (
  client: FrontendClient,
  setState: StateDispatcher,
  peerKey: Uint8Array
) => {
  const setWhisperState: ThreadStateDispatcher<WhisperThreadState> = (action) =>
    setState((state) => ({
      ...state,
      whispers: reduceThreadState(state, state.whispers, peerKey, action),
    }));

  const setWhisperMessages = (thread: WhisperThread, whispers: WhisperRecord[]) =>
    setWhisperState((whisper) => ({
      ...whisper,
      thread,
      label: thread.alias,
      messages: whispers.map(({ message }) => message),
      state: ThreadInitState.OPEN,
    }));

  const handleWhisperError = (error: Error) =>
    setWhisperState((whisper) => ({
      ...whisper,
      errors: [...whisper.errors, error],
    }));

  const sendMessage = (body: string) =>
    setWhisperState((whisper) => {
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
    ...createThreadActions(setWhisperState),
    setWhisperMessages,
    handleWhisperError,
    sendMessage,
  };
};

const createThreadActions = <T extends ThreadState>(setState: ThreadStateDispatcher<T>) => {
  const toggleMessageGC = (messageGCEnabled: boolean) =>
    setState((thread) => ({
      ...thread,
      messageGCEnabled,
    }));

  const toggleSelectedPeer = (peerKey: Uint8Array, state?: boolean) =>
    setState((thread) => {
      const key = Base64.fromUint8Array(peerKey, true);
      const selectedPeers = new Set(thread.styles.selectedPeers);

      if (state === true) {
        selectedPeers.add(key);
      } else if (state === false) {
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

  const resetSelectedPeers = () =>
    setState((thread) => {
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

  return {
    toggleMessageGC,
    toggleSelectedPeer,
    resetSelectedPeers,
  };
};

const createRoomActions = (
  client: FrontendClient,
  setState: StateDispatcher,
  serverKey: Uint8Array
) => {
  const setRoomState: ThreadStateDispatcher<RoomThreadState> = (action) =>
    setState((state) => ({
      ...state,
      rooms: reduceThreadState(state, state.rooms, serverKey, action),
    }));

  const syncUsers = (nicks: string[], users: Map<string, UserMeta>) =>
    setRoomState((room) => ({
      ...room,
      nicks,
      users,
    }));

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

  const reduceAssetBundle = <T extends ThreadState>(state: T, bundle: AssetBundle): T => {
    state.messageSizeCache.clearAll();

    const assetBundles = bundle.isDelta ? [...state.assetBundles, bundle] : [bundle];
    const liveEmoteMap = new Map<bigint, Emote>();
    const liveModifierMap = new Map<bigint, Modifier>();
    const liveTagMap = new Map<bigint, Tag>();
    let room: Room;
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
    }
    const liveEmotes = Array.from(liveEmoteMap.values());
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

  const handleClientData = (event: OpenClientResponse) =>
    setRoomState((room, state) => {
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

  const handleClientError = (error: Error) =>
    setRoomState((room) => ({
      ...room,
      errors: [...room.errors, error],
    }));

  const handleClientClose = () =>
    setRoomState((room) => ({
      ...room,
      state: ThreadInitState.CLOSED,
    }));

  const sendMessage = (body: string) =>
    setRoomState((room) => {
      const { networkKey } = room;

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
          console.log("maxlines", { n });
        },
        mute: (alias: string, duration: string, message: string) => {
          void client.chat.clientMute({ networkKey, serverKey, alias, duration, message });
        },
        unmute: (alias: string) => {
          void client.chat.clientUnmute({ networkKey, serverKey, alias });
        },
        timestampformat: (format: string) => {
          console.log("timestampformat", { format });
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
          console.log("exit");
        },
        hideemote: (name: string) => {
          console.log("hideemote", { name });
        },
        unhideemote: (name: string) => {
          console.log("unhideemote", { name });
        },
        me: (body: string) => {
          void client.chat.clientSendMessage({
            networkKey,
            serverKey,
            body: `/me ${body}`,
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
      return room;
    });

  return {
    ...createThreadActions(setRoomState),
    syncUsers,
    handleClientData,
    handleClientError,
    handleClientClose,
    sendMessage,
  };
};

export const Provider: React.FC = ({ children }) => {
  const client = useClient();
  const [state, setState] = useState(initialState);
  const actions = useStableCallbacks(createGlobalActions(client, setState));

  useEffect(() => {
    const uiConfigEvents = client.chat.watchUIConfig();
    uiConfigEvents.on("data", ({ uiConfig }) => actions.setUiConfig(uiConfig));
    const whisperEvents = client.chat.watchWhispers();
    whisperEvents.on("data", actions.handleWhisperEvent);
    return () => {
      uiConfigEvents.destroy();
      whisperEvents.destroy();
    };
  }, [client]);

  const mergeUIConfig = useStableCallback((values: Partial<IUIConfig>) => {
    void client.chat.setUIConfig({
      uiConfig: {
        ...state.uiConfig,
        ...values,
      },
    });
  });

  const value = useMemo<[State, ChatActions, StateDispatcher]>(
    () => [state, { mergeUIConfig, ...actions }, setState],
    [state]
  );
  return <ChatContext.Provider value={value}>{children}</ChatContext.Provider>;
};

Provider.displayName = "Chat.Provider";

export const useChat = (): [State, ChatActions, StateDispatcher] => useContext(ChatContext);

export const ChatConsumer = ChatContext.Consumer;

export type RoomProviderProps = Topic;

export const RoomProvider: React.FC<RoomProviderProps> = ({ children, ...props }) => {
  const client = useClient();
  const [state, , setState] = useChat();

  let thread: ThreadState;
  let actions: ReturnType<typeof createRoomActions> | ReturnType<typeof createWhisperActions>;
  switch (props.type) {
    case "ROOM":
      thread = state.rooms.get(formatKey(props.topicKey));
      actions = useStableCallbacks(createRoomActions(client, setState, props.topicKey));
      break;
    case "WHISPER":
      thread = state.whispers.get(formatKey(props.topicKey));
      actions = useStableCallbacks(createWhisperActions(client, setState, props.topicKey));
      break;
  }

  // const listing = useDirectoryListing(networkKey, directoryListingId);
  // const directory = useDirectory(networkKey);
  // useReady(() => {
  //   const nicks: string[] = [];
  //   const users = new Map<string, UserMeta>();
  //   for (const user of listing.viewers.values()) {
  //     nicks.push(user.alias);
  //     users.set(formatKey(user.peerKey), {
  //       alias: user.alias,
  //       listing: findUserMediaListing(directory, user),
  //     });
  //   }
  //   dispatch({ type: "SYNC_USERS", nicks, users });
  // }, [listing, directory]);

  // useEffect(() => {
  //   if (directoryListingId === BigInt(0)) {
  //     return;
  //   }

  //   const req = {
  //     networkKey,
  //     id: directoryListingId,
  //   };
  //   void client.directory.join(req);
  //   return () => void client.directory.part(req);
  // }, [networkKey, directoryListingId]);

  const messages = useRef<Message[]>();
  messages.current = thread.messages;

  const getMessage = useCallback((index: number): Message => messages.current[index], []);
  const getMessageCount = useCallback((): number => messages.current.length, []);

  const value = useMemo<[ThreadState, RoomActions]>(
    () => [thread, { getMessage, getMessageCount, ...actions }],
    [thread]
  );
  return <RoomContext.Provider value={value}>{children}</RoomContext.Provider>;
};

RoomProvider.displayName = "Room.Provider";

export const useRoom = (): [ThreadState, RoomActions] => useContext(RoomContext);

export const RoomConsumer = RoomContext.Consumer;

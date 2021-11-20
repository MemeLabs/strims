import { Readable } from "@memelabs/protobuf/lib/rpc/stream";
import { Base64 } from "js-base64";
import React, { ReactNode, useCallback, useContext, useMemo, useRef, useState } from "react";
import { useEffect } from "react";

import {
  AssetBundle,
  Emote,
  EmoteEffect,
  IUIConfig,
  Message,
  Modifier,
  OpenClientResponse,
  Room,
  Tag,
  UIConfig,
} from "../apis/strims/chat/v1/chat";
import ChatCellMeasurerCache from "../lib/ChatCellMeasurerCache";
import { useClient } from "./FrontendApi";

type RoomAction =
  | {
      type: "INIT";
      chatClient: Readable<OpenClientResponse>;
    }
  | {
      type: "TOGGLE_MESSAGE_GC";
      state: boolean;
    }
  | {
      type: "CLIENT_DATA";
      data: OpenClientResponse;
    }
  | {
      type: "CLIENT_ERROR";
      error: Error;
    }
  | {
      type: "CLIENT_CLOSE";
    };

type Action = {
  type: "SET_UI_CONFIG";
  uiConfig: UIConfig;
};

export interface Style {
  name: string;
  animated: boolean;
  modifiers: string[];
}

export type EmoteStyleMap = {
  [key: string]: Style;
};

export type ModifierMap = {
  [key: string]: Modifier;
};

export interface ChatStyles {
  emotes: EmoteStyleMap;
  modifiers: ModifierMap;
  tags: Tag[];
}

const enum RoomInitState {
  NEW,
  INITIALIZED,
  OPEN,
  CLOSED,
}

export interface RoomState {
  chatClient: Readable<OpenClientResponse>;
  id: number;
  clientId?: bigint;
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
  errors: Error[];
  state: RoomInitState;
}

export interface State {
  uiConfig: UIConfig;
  config: {
    messageGCThreshold: number;
  };
  rooms: {
    [key: string]: RoomState;
  };
}

type StateDispatcher = React.Dispatch<React.SetStateAction<State>>;

const initialRoomState: RoomState = {
  chatClient: null,
  id: 0,
  messages: [],
  messageGCEnabled: true,
  messageSizeCache: new ChatCellMeasurerCache(),
  assetBundles: [],
  liveEmotes: [],
  styles: {
    emotes: {},
    modifiers: {},
    tags: [],
  },
  emotes: [],
  modifiers: [],
  nicks: [],
  tags: [],
  errors: [],
  state: RoomInitState.NEW,
};

let nextId = 0;
const initializeRoomState = (
  state: RoomState,
  chatClient: Readable<OpenClientResponse>
): RoomState => ({
  ...state,
  chatClient,
  id: nextId++,
  messageSizeCache: new ChatCellMeasurerCache(),
  state: RoomInitState.INITIALIZED,
});

const initialState: State = {
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
    viewerStateIndicator: UIConfig.ViewerStateIndicator.VIEWER_STATE_INDICATOR_BAR,
  }),
  config: {
    messageGCThreshold: 250,
  },
  rooms: {},
};

type ChatActions = {
  mergeUIConfig: (config: Partial<IUIConfig>) => void;
};

const ChatContext = React.createContext<[State, ChatActions, StateDispatcher]>(null);

type RoomActions = {
  sendMessage: (body: string) => void;
  getMessage: (index: number) => Message;
  getMessageCount: () => number;
  toggleMessageGC: (state: boolean) => void;
};

const RoomContext = React.createContext<[RoomState, RoomActions]>(null);

const useStateReducer = (setState: StateDispatcher) => (action: Action) =>
  setState((state) => stateReducer(state, action));

const useRoomStateReducer = (setState: StateDispatcher, key: string) => (action: RoomAction) =>
  setState((state) => ({
    ...state,
    rooms: {
      ...state.rooms,
      [key]: roomReducer(state, state.rooms[key] ?? initialRoomState, action),
    },
  }));

const stateReducer = (state: State, action: Action): State => {
  switch (action.type) {
    case "SET_UI_CONFIG":
      return {
        ...state,
        uiConfig: action.uiConfig,
      };
    default:
      return state;
  }
};

const roomReducer = (state: State, room: RoomState, action: RoomAction): RoomState => {
  switch (action.type) {
    case "INIT":
      return initializeRoomState(room, action.chatClient);
    case "CLIENT_DATA":
      return chatClientDataReducer(state, room, action.data);
    case "CLIENT_ERROR":
      return {
        ...room,
        errors: [...room.errors, action.error],
      };
    case "CLIENT_CLOSE":
      return {
        ...room,
        state: RoomInitState.CLOSED,
      };
    case "TOGGLE_MESSAGE_GC":
      return {
        ...room,
        messageGCEnabled: action.state,
      };
    default:
      return room;
  }
};

const chatClientDataReducer = (
  state: State,
  room: RoomState,
  event: OpenClientResponse
): RoomState => {
  switch (event.body.case) {
    case OpenClientResponse.BodyCase.OPEN:
      return {
        ...room,
        clientId: event.body.open.clientId,
        state: RoomInitState.OPEN,
      };
    case OpenClientResponse.BodyCase.MESSAGE:
      return messageReducer(state, room, event.body.message);
    case OpenClientResponse.BodyCase.ASSET_BUNDLE:
      return assetBundleReducer(room, event.body.assetBundle);
    default:
      return room;
  }
};

const messageReducer = (state: State, room: RoomState, message: Message): RoomState => {
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
  };
};

interface Named {
  name: string;
}
const toNames = <T extends Named>(vs: T[]): string[] => vs.map(({ name }) => name).sort();

const assetBundleReducer = (state: RoomState, bundle: AssetBundle): RoomState => {
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
    room = b.room || room;
  }
  const liveEmotes = Array.from(liveEmoteMap.values());
  const emoteStyles = Object.fromEntries(
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
    assetBundles,
    liveEmotes,
    styles: {
      emotes: emoteStyles,
      modifiers: Object.fromEntries(liveModifiers.map((m) => [m.name, m])),
      tags: liveTags,
    },
    emotes: toNames(liveEmotes),
    modifiers: toNames(liveModifiers.filter(({ internal }) => !internal)),
    tags: toNames(liveTags),
  };
};

export const Provider: React.FC = ({ children }) => {
  const [state, setState] = useState(initialState);
  const dispatch = useStateReducer(setState);

  const client = useClient();

  useEffect(() => {
    void (async () => {
      const { uiConfig } = await client.chat.getUIConfig();
      if (uiConfig) {
        dispatch({ type: "SET_UI_CONFIG", uiConfig });
      }
    })();
  }, []);

  const mergeUIConfig = useCallback(
    (values: Partial<IUIConfig>) => {
      const uiConfig = new UIConfig({
        ...state.uiConfig,
        ...values,
      });
      dispatch({ type: "SET_UI_CONFIG", uiConfig });
      void client.chat.setUIConfig({ uiConfig });
    },
    [client, state.uiConfig]
  );

  const value = useMemo<[State, ChatActions, StateDispatcher]>(
    () => [state, { mergeUIConfig }, setState],
    [state]
  );
  return <ChatContext.Provider value={value}>{children}</ChatContext.Provider>;
};

Provider.displayName = "Chat.Provider";

export const useChat = (): [State, ChatActions, StateDispatcher] => useContext(ChatContext);

export const ChatConsumer = ChatContext.Consumer;

interface RoomProviderProps {
  networkKey: Uint8Array;
  serverKey: Uint8Array;
}

export const RoomProvider: React.FC<RoomProviderProps> = ({ networkKey, serverKey, children }) => {
  const key = useMemo(
    () => Base64.fromUint8Array(networkKey) + ":" + Base64.fromUint8Array(serverKey),
    [networkKey, serverKey]
  );

  const [state, , setState] = React.useContext(ChatContext);
  const dispatch = useRoomStateReducer(setState, key);

  const client = useClient();

  const room = state.rooms[key] ?? initialRoomState;
  useEffect(() => {
    if (room.state > RoomInitState.NEW) {
      return;
    }

    const chatClient = client.chat.openClient({ networkKey, serverKey });
    chatClient.on("data", (data) => dispatch({ type: "CLIENT_DATA", data }));
    chatClient.on("error", (error) => dispatch({ type: "CLIENT_ERROR", error }));
    chatClient.on("close", () => dispatch({ type: "CLIENT_CLOSE" }));
    dispatch({ type: "INIT", chatClient });
  }, [networkKey, serverKey]);

  const sendMessage = useCallback(
    (body: string) =>
      client.chat.clientSendMessage({
        clientId: room.clientId,
        body,
      }),
    [client, room.clientId]
  );

  const messages = useRef<Message[]>();
  messages.current = room.messages;

  const getMessage = useCallback((index: number): Message => messages.current[index], []);
  const getMessageCount = useCallback((): number => messages.current.length, []);

  const toggleMessageGC = useCallback(
    (state: boolean): void => dispatch({ type: "TOGGLE_MESSAGE_GC", state }),
    []
  );

  const value = useMemo<[RoomState, RoomActions]>(
    () => [room, { sendMessage, getMessage, getMessageCount, toggleMessageGC }],
    [room]
  );
  return <RoomContext.Provider value={value}>{children}</RoomContext.Provider>;
};

RoomProvider.displayName = "Room.Provider";

export const useRoom = (): [RoomState, RoomActions] => useContext(RoomContext);

export const RoomConsumer = RoomContext.Consumer;

import { Readable } from "@memelabs/protobuf/lib/rpc/stream";
import { Base64 } from "js-base64";
import React, { useCallback, useContext, useMemo, useRef, useState } from "react";
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
  ServerEvent,
  Tag,
  UIConfig,
} from "../apis/strims/chat/v1/chat";
import { Listing } from "../apis/strims/network/v1/directory/directory";
import ChatCellMeasurerCache from "../lib/ChatCellMeasurerCache";
import { useDirectoryListing } from "./Directory";
import { useClient } from "./FrontendApi";

type RoomAction =
  | {
      type: "INIT";
      serverEvents: Readable<OpenClientResponse>;
      networkKey: Uint8Array;
      serverKey: Uint8Array;
    }
  | {
      type: "TOGGLE_MESSAGE_GC";
      state: boolean;
    }
  | {
      type: "SYNC_NICKS";
      nicks: string[];
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

export interface ChatStyles {
  emotes: Map<string, Style>;
  modifiers: Map<string, Modifier>;
  tags: Tag[];
}

export interface UserMeta {
  alias: string;
  tagColor: string;
  stream: {
    listing: Listing;
    color: string;
  };
}

const enum RoomInitState {
  NEW,
  INITIALIZED,
  OPEN,
  CLOSED,
}

export interface RoomState {
  serverEvents: Readable<OpenClientResponse>;
  networkKey: Uint8Array;
  serverKey: Uint8Array;
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
  serverEvents: null,
  networkKey: null,
  serverKey: null,
  id: 0,
  messages: [],
  messageGCEnabled: true,
  messageSizeCache: new ChatCellMeasurerCache(),
  assetBundles: [],
  liveEmotes: [],
  styles: {
    emotes: new Map(),
    modifiers: new Map(),
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
  serverEvents: Readable<OpenClientResponse>,
  networkKey: Uint8Array,
  serverKey: Uint8Array
): RoomState => ({
  ...state,
  serverEvents,
  networkKey,
  serverKey,
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
    normalizeAliasCase: true,
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
      return initializeRoomState(room, action.serverEvents, action.networkKey, action.serverKey);
    case "CLIENT_DATA":
      return serverEventsDataReducer(state, room, action.data);
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
    case "SYNC_NICKS":
      return {
        ...room,
        nicks: action.nicks,
      };
    default:
      return room;
  }
};

const serverEventsDataReducer = (
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
    case OpenClientResponse.BodyCase.SERVER_EVENTS:
      return serverEventsReducer(state, room, event.body.serverEvents.events);
    case OpenClientResponse.BodyCase.ASSET_BUNDLE:
      return assetBundleReducer(room, event.body.assetBundle);
    default:
      return room;
  }
};

const serverEventsReducer = (state: State, room: RoomState, events: ServerEvent[]): RoomState => {
  for (const event of events) {
    switch (event.body.case) {
      case ServerEvent.BodyCase.MESSAGE:
        room = messageReducer(state, room, event.body.message);
        break;
    }
  }

  return room;
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

const toNames = (vs: { name: string }[]): string[] => vs.map(({ name }) => name).sort();

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
    assetBundles,
    liveEmotes,
    styles: {
      emotes: emoteStyles,
      modifiers: new Map(liveModifiers.map((m) => [m.name, m])),
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
    const events = client.chat.watchUIConfig();
    events.on("data", ({ uiConfig }) => dispatch({ type: "SET_UI_CONFIG", uiConfig }));
    return () => events.destroy();
  }, []);

  const mergeUIConfig = useCallback(
    (values: Partial<IUIConfig>) => {
      void client.chat.setUIConfig({
        uiConfig: {
          ...state.uiConfig,
          ...values,
        },
      });
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
  directoryListingId?: bigint;
}

export const RoomProvider: React.FC<RoomProviderProps> = ({
  networkKey,
  serverKey,
  directoryListingId,
  children,
}) => {
  const key = useMemo(
    () => Base64.fromUint8Array(networkKey) + ":" + Base64.fromUint8Array(serverKey),
    [networkKey, serverKey]
  );

  const [state, { mergeUIConfig }, setState] = React.useContext(ChatContext);
  const dispatch = useRoomStateReducer(setState, key);

  const client = useClient();

  const room = state.rooms[key] ?? initialRoomState;
  useEffect(() => {
    if (room.state > RoomInitState.NEW) {
      return;
    }

    const serverEvents = client.chat.openClient({ networkKey, serverKey });
    serverEvents.on("data", (data) => dispatch({ type: "CLIENT_DATA", data }));
    serverEvents.on("error", (error) => dispatch({ type: "CLIENT_ERROR", error }));
    serverEvents.on("close", () => dispatch({ type: "CLIENT_CLOSE" }));
    dispatch({ type: "INIT", serverEvents, networkKey, serverKey });
  }, [networkKey, serverKey]);

  const listing = useDirectoryListing(networkKey, directoryListingId);
  useEffect(() => {
    if (!listing) {
      return;
    }

    const nicks: string[] = [];
    const metas = new Map<string, UserMeta>();
    for (const { alias } of listing.viewers.values()) {
      nicks.push(alias);

      const meta = { alias };
    }
    dispatch({ type: "SYNC_NICKS", nicks });
  }, [listing]);

  useEffect(() => {
    if (directoryListingId === BigInt(0)) {
      return;
    }

    const req = {
      networkKey,
      id: directoryListingId,
    };
    void client.directory.join(req);
    return () => void client.directory.part(req);
  }, [networkKey, directoryListingId]);

  const sendMessage = useMemo(() => {
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

    return (body: string) => {
      if (body.startsWith("/")) {
        const command = body.split(" ", 1).pop().toLowerCase().substring(1);
        const func = commandFuncs[commandAliases[command] ?? command];
        if (func) {
          const args = body.split(" ", func.length);
          func(...[...args.slice(1), body.substring(args.reduce((n, a) => n + a.length + 1, 0))]);
        } else {
          // invalid command
        }
      } else {
        void client.chat.clientSendMessage({ networkKey, serverKey, body });
      }
    };
  }, [client, room.clientId]);

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

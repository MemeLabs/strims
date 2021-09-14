import clsx from "clsx";
import React, { useCallback, useRef } from "react";
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

type Action =
  | {
      type: "SET_UI_CONFIG";
      uiConfig: UIConfig;
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

export interface Style {
  name: string;
  animated: boolean;
  modifiers: string[];
}

export type EmoteStyleMap = {
  [key: string]: Style;
};

export interface ChatStyles {
  emotes: EmoteStyleMap;
  modifiers: Modifier[];
  tags: Tag[];
}

export interface State {
  id: number;
  clientId?: bigint;
  uiConfig: UIConfig;
  config: {
    messageGCThreshold: number;
  };
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
  state: "new" | "open" | "closed";
}

const initialState: State = {
  id: 0,
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
    viewerStateIndicator: UIConfig.ViewerStateIndicator.VIEWER_STATE_INDICATOR_BAR,
  }),
  config: {
    messageGCThreshold: 250,
  },
  messages: [],
  messageGCEnabled: true,
  messageSizeCache: null,
  assetBundles: [],
  liveEmotes: [],
  styles: {
    emotes: {},
    modifiers: [],
    tags: [],
  },
  emotes: [],
  modifiers: [],
  nicks: [],
  tags: [],
  errors: [],
  state: "new",
};

let nextId = 0;
const initializeState = (state: State): State => ({
  ...state,
  id: nextId++,
  messageSizeCache: new ChatCellMeasurerCache(),
});

const ChatContext = React.createContext<[State, (action: Action) => void]>(null);

const chatReducer = (state: State, action: Action): State => {
  switch (action.type) {
    case "SET_UI_CONFIG":
      return {
        ...state,
        uiConfig: action.uiConfig,
      };
    case "CLIENT_DATA":
      return chatClientDataReducer(state, action.data);
    case "CLIENT_ERROR":
      return {
        ...state,
        errors: [...state.errors, action.error],
      };
    case "CLIENT_CLOSE":
      return {
        ...state,
        state: "closed",
      };
    case "TOGGLE_MESSAGE_GC":
      return {
        ...state,
        messageGCEnabled: action.state,
      };
    default:
      return state;
  }
};
const chatClientDataReducer = (state: State, event: OpenClientResponse): State => {
  switch (event.body.case) {
    case OpenClientResponse.BodyCase.OPEN:
      return {
        ...state,
        clientId: event.body.open.clientId,
        state: "open",
      };
    case OpenClientResponse.BodyCase.MESSAGE:
      return messageReducer(state, event.body.message);
    case OpenClientResponse.BodyCase.ASSET_BUNDLE:
      return assetBundleReducer(state, event.body.assetBundle);
    default:
      return state;
  }
};

const messageReducer = (state: State, message: Message): State => {
  const messages = [...state.messages];
  if (
    messages.length !== 0 &&
    message.entities.emotes.length === 1 &&
    message.entities.emotes[0].combo > 0
  ) {
    messages[messages.length - 1] = message;
    state.messageSizeCache.clear(messages.length - 1, 0);
  } else {
    messages.push(message);
  }

  const messageOverflow = messages.length - state.uiConfig.maxLines;
  if (state.messageGCEnabled && messageOverflow > state.config.messageGCThreshold) {
    messages.splice(0, messageOverflow);
    state.messageSizeCache.prune(messageOverflow);
  }

  return {
    ...state,
    messages,
  };
};

interface Named {
  name: string;
}
const toNames = <T extends Named>(vs: T[]): string[] => vs.map(({ name }) => name).sort();

const assetBundleReducer = (state: State, bundle: AssetBundle): State => {
  const assetBundles = bundle.isDelta ? [...state.assetBundles, bundle] : [bundle];
  const liveEmoteMap = new Map<bigint, Emote>();
  const liveModifierMap = new Map<bigint, Modifier>();
  const liveTagMap = new Map<bigint, Tag>();
  let room: Room;
  assetBundles.forEach((b) => {
    b.removedIds.forEach((id) => {
      liveEmoteMap.delete(id);
      liveModifierMap.delete(id);
      liveTagMap.delete(id);
    });
    b.emotes.forEach((e) => liveEmoteMap.set(e.id, e));
    b.modifiers.forEach((e) => liveModifierMap.set(e.id, e));
    b.tags.forEach((e) => liveTagMap.set(e.id, e));
    room = b.room || room;
  });
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
      modifiers: liveModifiers,
      tags: liveTags,
    },
    emotes: toNames(liveEmotes),
    modifiers: toNames(liveModifiers.filter(({ internal }) => !internal)),
    tags: toNames(liveTags),
  };
};

type Actions = {
  sendMessage: (body: string) => void;
  mergeUIConfig: (config: Partial<IUIConfig>) => void;
  getMessage: (index: number) => Message;
  getMessageCount: () => number;
  toggleMessageGC: (state: boolean) => void;
};

export const useChat = (): [State, Actions] => {
  const [state, dispatch] = React.useContext(ChatContext);
  const client = useClient();

  const sendMessage = useCallback(
    (body: string) =>
      client.chat.clientSendMessage({
        clientId: state.clientId,
        body,
      }),
    [client, state.clientId]
  );

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

  const messages = useRef<Message[]>();
  messages.current = state.messages;

  const getMessage = useCallback((index: number): Message => messages.current[index], []);
  const getMessageCount = useCallback((): number => messages.current.length, []);

  const toggleMessageGC = useCallback(
    (state: boolean): void => dispatch({ type: "TOGGLE_MESSAGE_GC", state }),
    []
  );

  const actions = {
    sendMessage,
    mergeUIConfig,
    getMessage,
    getMessageCount,
    toggleMessageGC,
  };
  return [state, actions];
};

interface ProviderProps {
  networkKey: Uint8Array;
  serverKey: Uint8Array;
}

export const Provider: React.FC<ProviderProps> = ({ networkKey, serverKey, children }) => {
  const [state, dispatch] = React.useReducer(chatReducer, initialState, initializeState);
  const client = useClient();

  useEffect(() => {
    void (async () => {
      const { uiConfig } = await client.chat.getUIConfig();
      if (uiConfig) {
        dispatch({ type: "SET_UI_CONFIG", uiConfig });
      }
    })();

    const events = client.chat.openClient({ networkKey, serverKey });
    events.on("data", (data) => dispatch({ type: "CLIENT_DATA", data }));
    events.on("error", (error) => dispatch({ type: "CLIENT_ERROR", error }));

    const handleClose = () => dispatch({ type: "CLIENT_CLOSE" });
    events.on("close", handleClose);
    return () => {
      console.log("closed chat thing...");
      events.off("close", handleClose);
      events.destroy();
    };
  }, []);

  return (
    <ChatContext.Provider value={[state, dispatch]}>
      <div className={clsx("chat")}>{children}</div>
    </ChatContext.Provider>
  );
};

Provider.displayName = "Chat.Provider";

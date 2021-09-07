import clsx from "clsx";
import React, { useCallback } from "react";
import { useEffect } from "react";
import { CellMeasurerCache } from "react-virtualized";

import {
  AssetBundle,
  Emote,
  IUIConfig,
  Message,
  OpenClientResponse,
  Room,
  UIConfig,
} from "../apis/strims/chat/v1/chat";
import { useClient } from "./FrontendApi";

type Action =
  | {
      type: "SET_UI_CONFIG";
      uiConfig: UIConfig;
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

interface State {
  id: number;
  clientId?: bigint;
  uiConfig: UIConfig;
  config: {
    messageHistorySize: number;
    messageGCThreshold: number;
  };
  messages: Message[];
  messageSizeCache: CellMeasurerCache;
  assetBundles: AssetBundle[];
  liveEmotes: Emote[];
  styles: {
    [key: string]: string;
  };
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
    messageHistorySize: 250,
    messageGCThreshold: 250,
  },
  messages: [],
  messageSizeCache: new CellMeasurerCache(),
  assetBundles: [],
  liveEmotes: [],
  styles: {},
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

  // TODO: pause gc while ui is scrolled
  if (messages.length >= state.config.messageHistorySize + state.config.messageGCThreshold) {
    messages.splice(0, state.config.messageGCThreshold);
    state.messageSizeCache.clearAll();
  }

  return {
    ...state,
    messages,
  };
};

const assetBundleReducer = (state: State, bundle: AssetBundle): State => {
  const assetBundles = bundle.isDelta ? [...state.assetBundles, bundle] : [bundle];
  const liveEmoteMap = new Map<bigint, Emote>();
  let room: Room;
  assetBundles.forEach((b) => {
    b.removedEmotes.forEach((id) => liveEmoteMap.delete(id));
    b.emotes.forEach((e) => liveEmoteMap.set(e.id, e));
    room = b.room || room;
  });
  const liveEmotes = Array.from(liveEmoteMap.values());
  const styles = Object.fromEntries(
    liveEmotes.map(({ id, name }) => [name, `e_${name}_${state.id}_${id}`])
  );

  return {
    ...state,
    assetBundles,
    liveEmotes,
    styles,
    emotes: Object.keys(styles).sort(),
    modifiers: room.modifiers,
    tags: room.tags,
  };
};

type Actions = {
  sendMessage: (body: string) => void;
  mergeUIConfig: (config: Partial<IUIConfig>) => void;
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

  // TODO: load server config

  const actions = {
    sendMessage,
    mergeUIConfig,
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

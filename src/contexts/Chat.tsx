import { StyleDeclaration, StyleSheet, css, resetInjectedStyle } from "aphrodite/no-important";
import clsx from "clsx";
import React from "react";
import { useEffect } from "react";
import { CellMeasurerCache } from "react-virtualized";

import {
  AssetBundle,
  EmoteFileType,
  EmoteScale,
  IUIConfig,
  Message,
  OpenClientResponse,
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

interface BundleMeta {
  emoteIdObjectUrls: Map<bigint, string[]>;
  emoteIdNames: Map<bigint, string>;
}

interface State {
  clientId?: bigint;
  uiConfig: UIConfig;
  config: {
    messageHistorySize: number;
    messageGCThreshold: number;
  };
  messages: Message[];
  messageSizeCache: CellMeasurerCache;
  bundleMeta: BundleMeta;
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
  bundleMeta: {
    emoteIdObjectUrls: new Map(),
    emoteIdNames: new Map(),
  },
  styles: {},
  emotes: [],
  modifiers: [],
  nicks: [],
  tags: [],
  errors: [],
  state: "new",
};

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

const fileTypeToImageType = (fileType: EmoteFileType): string => {
  switch (fileType) {
    case EmoteFileType.FILE_TYPE_PNG:
      return "image/png";
    case EmoteFileType.FILE_TYPE_GIF:
      return "image/gif";
  }
};

const scaleToImageScale = (scale: EmoteScale): number => {
  switch (scale) {
    case EmoteScale.EMOTE_SCALE_1X:
      return 1;
    case EmoteScale.EMOTE_SCALE_2X:
      return 2;
    case EmoteScale.EMOTE_SCALE_4X:
      return 4;
  }
};

const assetBundleReducer = (state: State, bundle: AssetBundle): State => {
  const styles = { ...state.styles };
  const resetIds = [...bundle.removedEmotes, ...bundle.emotes.map(({ id }) => id)];
  const emoteIdObjectUrls = new Map(Array.from(state.bundleMeta.emoteIdObjectUrls.entries()));
  const emoteIdNames = new Map(
    Array.from(state.bundleMeta.emoteIdNames.entries()).filter(([id, name]) => {
      const reset = resetIds.includes(id);
      if (reset) {
        resetInjectedStyle(styles[name]);
        delete styles[name];
        emoteIdObjectUrls.get(id)?.forEach((url) => URL.revokeObjectURL(url));
        emoteIdObjectUrls.delete(id);
      }
      return !reset;
    })
  );

  const newStyleProps: { [key: string]: StyleDeclaration } = {};
  bundle.emotes.forEach(({ id, name, images }) => {
    const urls = images.map(({ data, fileType }) =>
      URL.createObjectURL(new Blob([data], { type: fileTypeToImageType(fileType) }))
    );
    const imageSet = images.map(({ scale }, i) => `url(${urls[i]}) ${scaleToImageScale(scale)}x`);
    const sample = images[0];
    const sampleScale = scaleToImageScale(sample.scale);
    const height = sample.height / sampleScale;
    const width = sample.width / sampleScale;
    newStyleProps[`e${name}`] = {
      backgroundImage: `image-set(${imageSet.join(", ")})`,
      backgroundRepeat: "no-repeat",
      width: `${width}px`,
      height: `${height}px`,
      marginTop: `calc(0.5em - ${height / 2}px)`,
      marginBottom: `calc(0.5em - ${height / 2}px)`,
    };

    emoteIdNames.set(id, name);
    emoteIdObjectUrls.set(id, urls);
  });
  const newStyles = Object.fromEntries(
    Object.entries(StyleSheet.create(newStyleProps)).map(([name, style]) => [
      name.substr(1),
      css(style),
    ])
  );

  return {
    ...state,
    bundleMeta: {
      emoteIdObjectUrls,
      emoteIdNames,
    },
    styles: {
      ...styles,
      ...newStyles,
    },
    emotes: Array.from(emoteIdNames.values()).sort((a, b) =>
      a.toLowerCase().localeCompare(b.toLowerCase())
    ),
    modifiers: bundle.room.modifiers,
    tags: bundle.room.tags,
  };
};

const unloadAssets = (state: State) => {
  Object.values(state.bundleMeta.emoteIdObjectUrls).forEach((url) => URL.revokeObjectURL(url));
  Object.values(state.styles).forEach((name) => resetInjectedStyle(name));
};

type Actions = {
  sendMessage: (body: string) => void;
  mergeUIConfig: (config: Partial<IUIConfig>) => void;
};

export const useChat = (): [State, Actions] => {
  const [state, dispatch] = React.useContext(ChatContext);
  const client = useClient();

  const sendMessage = (body: string) =>
    client.chat.clientSendMessage({
      clientId: state.clientId,
      body,
    });

  const mergeUIConfig = (values: Partial<IUIConfig>) => {
    const uiConfig = new UIConfig({
      ...state.uiConfig,
      ...values,
    });
    dispatch({ type: "SET_UI_CONFIG", uiConfig });
    void client.chat.setUIConfig({ uiConfig });
  };

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
  const [state, dispatch] = React.useReducer(chatReducer, initialState);
  const client = useClient();

  useEffect(() => {
    void client.chat
      .getUIConfig()
      .then(({ uiConfig }) => uiConfig && dispatch({ type: "SET_UI_CONFIG", uiConfig }));

    const events = client.chat.openClient({ networkKey, serverKey });
    events.on("data", (data) => dispatch({ type: "CLIENT_DATA", data }));
    events.on("error", (error) => dispatch({ type: "CLIENT_ERROR", error }));

    const handleClose = () => dispatch({ type: "CLIENT_CLOSE" });
    events.on("close", handleClose);
    return () => {
      events.off("close", handleClose);
      events.destroy();
      unloadAssets(state);
    };
  }, []);

  return (
    <ChatContext.Provider value={[state, dispatch]}>
      <div className={clsx("chat")}>{children}</div>
    </ChatContext.Provider>
  );
};

Provider.displayName = "Chat.Provider";

import {
  StyleDeclaration,
  StyleDeclarationValue,
  StyleSheet,
  css,
  resetInjectedStyle,
} from "aphrodite/no-important";
import clsx from "clsx";
import React from "react";
import { useEffect } from "react";

import {
  AssetBundle,
  EmoteFileType,
  EmoteScale,
  Message,
  OpenClientResponse,
} from "../apis/strims/chat/v1/chat";
import { useClient } from "./FrontendApi";

type Action =
  | {
      type: "MESSAGE_SCROLLBACK";
      messages: Message[];
    }
  | {
      type: "MESSAGE";
      message: Message;
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
  messages: Message[];
  bundleMeta: BundleMeta;
  styles: {
    [key: string]: StyleDeclarationValue;
  };
  emotes: string[];
  modifiers: string[];
  nicks: string[];
  tags: string[];
  errors: Error[];
  state: "new" | "open" | "closed";
}

const initialState: State = {
  messages: [],
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
    case "MESSAGE_SCROLLBACK":
      return {
        ...state,
        messages: action.messages,
      };
    case "MESSAGE":
      return {
        ...state,
        messages: [...state.messages, action.message],
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
      return {
        ...state,
        messages: [...state.messages, event.body.message],
      };
    case OpenClientResponse.BodyCase.ASSET_BUNDLE:
      return assetBundleReducer(state, event.body.assetBundle);
    default:
      return state;
  }
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
        resetInjectedStyle(css(styles[name]));
        delete styles[name];
        emoteIdObjectUrls.get(id)?.forEach((url) => URL.revokeObjectURL(url));
        emoteIdObjectUrls.delete(id);
      }
      return reset;
    })
  );

  const newStyles: { [key: string]: StyleDeclaration } = {};
  bundle.emotes.forEach(({ id, name, images }) => {
    const urls = images.map(({ data, fileType }) =>
      URL.createObjectURL(new Blob([data], { type: fileTypeToImageType(fileType) }))
    );
    const imageSet = images.map(({ scale }, i) => `url(${urls[i]}) ${scaleToImageScale(scale)}x`);
    const sample = images[0];
    const sampleScale = scaleToImageScale(sample.scale);
    const height = sample.height / sampleScale;
    const width = sample.width / sampleScale;
    newStyles[`e${name}`] = {
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

  return {
    ...state,
    bundleMeta: {
      emoteIdObjectUrls,
      emoteIdNames,
    },
    styles: {
      ...styles,
      ...Object.fromEntries(
        Object.entries(StyleSheet.create(newStyles)).map(([name, style]) => [name.substr(1), style])
      ),
    },
    emotes: Array.from(emoteIdNames.values()).sort((a, b) =>
      a.toLowerCase().localeCompare(b.toLowerCase())
    ),
    modifiers: bundle.room.modifiers,
    tags: bundle.room.tags,
  };
};

type Actions = {
  sendMessage: (body: string) => void;
};

export const useChat = (): [State, Actions] => {
  const [state] = React.useContext(ChatContext);
  const client = useClient();

  const sendMessage = (body: string) =>
    client.chat.clientSendMessage({
      clientId: state.clientId,
      body,
    });

  // TODO: open chat client (start swarm lazily?)
  // TODO: load backlog
  // TODO: load server config
  // TDOO: load local ui config
  // TODO: transform stream events

  const actions = {
    sendMessage,
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

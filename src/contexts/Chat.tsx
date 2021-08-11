import { StyleDeclarationValue, StyleSheet, reset } from "aphrodite/no-important";
import clsx from "clsx";
import React from "react";
import { useEffect } from "react";

import { ClientEvent } from "../apis/strims/chat/v1/chat";
import { Emote, emotes } from "../components/Chat/test-emotes";
import stream, { messages } from "../components/Chat/test-history";
import { useClient } from "./FrontendApi";

type Action =
  | {
      type: "MESSAGE_SCROLLBACK";
      messages: ClientEvent.Message[];
    }
  | {
      type: "LOAD_EMOTES";
      emotes: Emote[];
    }
  | {
      type: "MESSAGE";
      message: ClientEvent.Message;
    }
  | {
      type: "CLIENT_DATA";
      data: ClientEvent;
    }
  | {
      type: "CLIENT_ERROR";
      error: Error;
    }
  | {
      type: "CLIENT_CLOSE";
    };

interface State {
  messages: ClientEvent.Message[];
  styles: {
    [key: string]: StyleDeclarationValue;
  };
  clientId?: bigint;
  errors: Error[];
  state: "new" | "open" | "closed";
}

const initialState: State = {
  messages: [],
  styles: {},
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
    case "LOAD_EMOTES":
      reset();
      return {
        ...state,
        styles: createEmoteStyles(action.emotes),
      };
    default:
      return state;
  }
};

const chatClientDataReducer = (state: State, event: ClientEvent): State => {
  switch (event.body.case) {
    case ClientEvent.BodyCase.OPEN:
      return {
        ...state,
        clientId: event.body.open.clientId,
        state: "open",
      };
    case ClientEvent.BodyCase.MESSAGE:
      return state;
    default:
      return state;
  }
};

const createEmoteStyles = (emotes: Emote[]) => {
  const styles = {};

  emotes.forEach((emote) => {
    const imageSet = emote.versions.map(({ url, size }) => `url(${url}) ${size}`);
    const { dimensions } = emote.versions.find(({ size }) => size === "1x");
    styles[emote.name] = {
      backgroundImage: `image-set(${imageSet.join(", ")})`,
      backgroundRepeat: "no-repeat",
      width: `${dimensions.width}px`,
      height: `${dimensions.height}px`,
      marginTop: `calc(0.5em - ${dimensions.height / 2}px)`,
      marginBottom: `calc(0.5em - ${dimensions.height / 2}px)`,
    };
  });

  console.log({ styles });

  return StyleSheet.create(styles);
};

export const useChat = () => {
  const [state, dispatch] = React.useContext(ChatContext);
  const client = useClient();

  const sendMessage = (body: string) =>
    client.chat.callClient({
      clientId: state.clientId,
      body: {
        message: {
          time: BigInt(Date.now()),
          body,
        },
      },
    });

  // TODO: open chat client (start swarm lazily?)
  // TODO: load backlog
  // TODO: load server config
  // TDOO: load local ui config
  // TODO: transform stream events

  const actions = {
    sendMessage,
  };
  return [state, actions] as [State, typeof actions];
};

interface ProviderProps {
  networkKey: Uint8Array;
  serverKey: Uint8Array;
}

export const Provider: React.FC<ProviderProps> = ({ networkKey, serverKey, children }) => {
  const [state, dispatch] = React.useReducer(chatReducer, initialState);
  const client = useClient();

  useEffect(() => {
    console.log({ networkKey, serverKey });
    const chatClient = client.chat.openClient({ networkKey, serverKey });
    chatClient.on("data", (data) => dispatch({ type: "CLIENT_DATA", data }));
    chatClient.on("error", (error) => dispatch({ type: "CLIENT_ERROR", error }));
    chatClient.on("close", () => dispatch({ type: "CLIENT_CLOSE" }));
    return () => client.chat.callClient({ clientId: state.clientId, body: { close: {} } });
  }, []);

  useEffect(() => {
    dispatch({ type: "LOAD_EMOTES", emotes });
    // dispatch({ type: "MESSAGE_SCROLLBACK", messages });
  }, [emotes]);

  // useEffect(() => {
  //   const handleData = (message) => dispatch({ type: "MESSAGE", message });
  //   stream.on("data", handleData);
  //   return () => stream.off("data", handleData);
  // }, []);

  return (
    <ChatContext.Provider value={[state, dispatch]}>
      <div className={clsx("chat")}>{children}</div>
    </ChatContext.Provider>
  );
};

Provider.displayName = "Chat.Provider";

import { StyleDeclarationValue, StyleSheet, reset } from "aphrodite/no-important";
import clsx from "clsx";
import * as React from "react";
import { useEffect } from "react";

import { Emote, emotes } from "../components/Chat/test-emotes";
import stream, { messages } from "../components/Chat/test-history";
import { ChatClientEvent } from "../lib/pb";

interface State {
  messages: ChatClientEvent.IMessage[];
  styles: {
    [key: string]: StyleDeclarationValue;
  };
}

type Action =
  | {
      type: "MESSAGE_SCROLLBACK";
      messages: ChatClientEvent.IMessage[];
    }
  | {
      type: "LOAD_EMOTES";
      emotes: Emote[];
    }
  | {
      type: "MESSAGE";
      message: ChatClientEvent.IMessage;
    };

const initialState: State = {
  messages: [],
  styles: {},
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

const createEmoteStyles = (emotes: Emote[]) => {
  const styles = {};

  emotes.forEach((emote) => {
    const image = emote.versions.find(({ size }) => size === "1x");
    styles[emote.name] = {
      background: `url(${image.url})`,
      width: `${image.dimensions.width}px`,
      height: `${image.dimensions.height}px`,
      // marginTop: `-${image.dimensions.height}px`,
      marginTop: `calc(0.5em - ${image.dimensions.height / 2}px)`,
      marginBottom: `calc(0.5em - ${image.dimensions.height / 2}px)`,
    };
  });

  console.log({ styles });

  return StyleSheet.create(styles);
};

export const useChat = () => {
  const [state, dispatch] = React.useContext(ChatContext);
  const sendMessage = (body: string) => console.log({ body });

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

export const Provider = ({ children }: any) => {
  const [state, dispatch] = React.useReducer(chatReducer, initialState);

  useEffect(() => {
    dispatch({ type: "LOAD_EMOTES", emotes });
    dispatch({ type: "MESSAGE_SCROLLBACK", messages });
  }, [emotes]);

  useEffect(() => {
    const handleData = (message) => dispatch({ type: "MESSAGE", message });
    stream.on("data", handleData);
    return () => stream.off("data", handleData);
  }, []);

  return (
    <ChatContext.Provider value={[state, dispatch]}>
      <div className={clsx("chat")}>{children}</div>
    </ChatContext.Provider>
  );
};

Provider.displayName = "Chat.Provider";

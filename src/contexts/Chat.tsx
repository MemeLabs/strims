import { StyleDeclarationValue, StyleSheet, reset } from "aphrodite/no-important";
import clsx from "clsx";
import * as React from "react";
import { useEffect } from "react";

import { Emote, emotes } from "../components/Chat/test-emotes";
import { ChatClientEvent } from "../lib/pb";

interface State {
  messages: ChatClientEvent.Message[];
  styles: {
    [key: string]: StyleDeclarationValue;
  };
}

type Action =
  | {
      type: "MESSAGE_SCROLLBACK";
      messages: ChatClientEvent.Message[];
    }
  | {
      type: "LOAD_EMOTES";
      emotes: Emote[];
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
  }, [emotes]);

  return (
    <ChatContext.Provider value={[state, dispatch]}>
      <div className={clsx("chat")}>{children}</div>
    </ChatContext.Provider>
  );
};

Provider.displayName = "Chat.Provider";

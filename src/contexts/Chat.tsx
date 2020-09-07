import clsx from "clsx";
import * as React from "react";

import { ChatClientEvent } from "../lib/pb";

interface State {
  messages: ChatClientEvent.Message[];
}

type Action =
  | {
      type: "MESSAGE_SCROLLBACK";
      messages: ChatClientEvent.Message[];
    }
  | {
      type: "MEME";
    };

const initialState: State = {
  messages: [],
};

const ChatContext = React.createContext<[State, (action: Action) => void]>(null);

const chatReducer = (state: State, action: Action): State => {
  switch (action.type) {
    case "MESSAGE_SCROLLBACK":
      return {
        ...state,
        messages: action.messages,
      };
    default:
      return state;
  }
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

  return (
    <ChatContext.Provider value={[state, dispatch]}>
      <div className={clsx("chat")}>{children}</div>
    </ChatContext.Provider>
  );
};

Provider.displayName = "Chat.Provider";

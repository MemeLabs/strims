import clsx from "clsx";
import { Base64 } from "js-base64";
import React from "react";
import { BsArrowBarLeft, BsArrowBarRight } from "react-icons/bs";
import { useToggle } from "react-use";

import Composer from "../components/Chat/Composer";
import Message from "../components/Chat/Message";
import Scroller, { MessageProps } from "../components/Chat/Scroller";
import { Provider, useChat } from "../contexts/Chat";

const ChatThing: React.FC = () => {
  const [state, { sendMessage }] = useChat();

  return (
    <>
      <div className="chat__messages">
        <Scroller
          renderMessage={({ index, style }: MessageProps) => (
            <Message message={state.messages[index]} style={style} />
          )}
          messageCount={state.messages.length}
        />
      </div>
      <div className="chat__footer">
        <Composer onMessage={sendMessage} />
      </div>
    </>
  );
};

const ChatPanel: React.FC = () => {
  const [closed, toggleClosed] = useToggle(false);

  const className = clsx({
    "home_page__right": true,
    "home_page__right--closed": closed,
  });

  return (
    <aside className={className}>
      <button className="home_page__right__toggle_on" onClick={toggleClosed}>
        <BsArrowBarLeft size={22} />
      </button>
      <div className="home_page__right__body">
        <header className="home_page__subheader">
          <button className="home_page__right__toggle_off" onClick={toggleClosed}>
            <BsArrowBarRight size={22} />
          </button>
        </header>
        <header className="home_page__chat__promo"></header>
        <div className="home_page__chat chat">
          <Provider
            networkKey={Base64.toUint8Array("HVmKdL3JUzXvjh3BQ8tFqFCvzPp7Wxe4ak2yWbjSj/c=")}
            serverKey={Base64.toUint8Array("laBoCbsGwcjSZk5y6qN1NEYpCxnFJZEHmNIgzV64Sc4=")}
          >
            <ChatThing />
          </Provider>
        </div>
      </div>
    </aside>
  );
};

export default ChatPanel;

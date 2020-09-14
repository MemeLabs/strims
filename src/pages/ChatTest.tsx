/* eslint-disable no-console */

import { Base64 } from "js-base64";
import * as React from "react";

import Composer from "../components/Chat/Composer";
import Message from "../components/Chat/Message";
import Scroller from "../components/Chat/Scroller";
import { MainLayout } from "../components/MainLayout";
import { Provider, useChat } from "../contexts/Chat";

const ChatThing = () => {
  const [state, { sendMessage }] = useChat();

  return (
    <>
      <div className="chat__messages">
        <Scroller
          renderMessage={({ index, style }) => (
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

const ChatTest = () => {
  return (
    <MainLayout>
      <main className="home_page__main">
        <header className="home_page__subheader"></header>
        <section className="home_page__main__video"></section>
      </main>
      <aside className="home_page__right">
        <header className="home_page__subheader"></header>
        <header className="home_page__chat__promo"></header>
        <div className="home_page__chat chat">
          <Provider
            networkKey={Base64.toUint8Array("HVmKdL3JUzXvjh3BQ8tFqFCvzPp7Wxe4ak2yWbjSj/c=")}
            serverKey={Base64.toUint8Array("laBoCbsGwcjSZk5y6qN1NEYpCxnFJZEHmNIgzV64Sc4=")}
          >
            <ChatThing />
          </Provider>
        </div>
      </aside>
    </MainLayout>
  );
};

export default ChatTest;

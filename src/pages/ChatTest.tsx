/* eslint-disable no-console */

import * as React from "react";

import Composer from "../components/Chat/Composer";
import Message from "../components/Chat/Message";
import Scroller from "../components/Chat/Scroller";
import { MainLayout } from "../components/MainLayout";
import { Provider, useChat } from "../contexts/Chat";

const ChatThing = () => {
  const [state] = useChat();

  return (
    <>
      <div className="chat__messages">
        <Scroller messages={state.messages} />
      </div>
      <div className="chat__footer">
        <Composer />
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
          <Provider>
            <ChatThing />
          </Provider>
        </div>
      </aside>
    </MainLayout>
  );
};

export default ChatTest;

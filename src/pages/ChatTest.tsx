/* eslint-disable no-console */

import * as React from "react";

import Message from "../components/Chat/Message";
import history from "../components/Chat/test-history";
import { MainLayout } from "../components/MainLayout";

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
        <div className="home_page__chat">
          {history.map((m, i) => (
            <Message message={m} key={i} />
          ))}
        </div>
      </aside>
    </MainLayout>
  );
};

export default ChatTest;

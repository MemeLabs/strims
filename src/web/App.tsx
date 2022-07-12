// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { BrowserRouter, MemoryRouter } from "react-router-dom";

import { APIDialer } from "../contexts/Session";
import { IS_PWA } from "../lib/userAgent";
import Provider from "../root/Provider";
import RootRouter from "../root/Router";

const Router = IS_PWA ? MemoryRouter : BrowserRouter;

interface AppProps {
  apiDialer: APIDialer;
}

const App: React.FC<AppProps> = ({ apiDialer }) => (
  <Router>
    <Provider apiDialer={apiDialer}>
      <RootRouter />
    </Provider>
  </Router>
);

export default App;

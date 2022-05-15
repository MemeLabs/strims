// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { HashRouter } from "react-router-dom";

import { APIDialer } from "../contexts/Session";
import Provider from "../root/Provider";
import RootRouter from "../root/Router";

interface AppProps {
  apiDialer: APIDialer;
}

const App: React.FC<AppProps> = ({ apiDialer }) => (
  <HashRouter>
    <Provider apiDialer={apiDialer}>
      <RootRouter />
    </Provider>
  </HashRouter>
);

export default App;

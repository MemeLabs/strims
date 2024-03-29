// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { useTitle } from "react-use";

import { DevToolsClient } from "../../apis/client";
import Provider from "./Provider";
import Router from "./Router";

interface AppProps {
  client: DevToolsClient;
}

const App: React.FC<AppProps> = ({ client }) => {
  useTitle("DevTools");

  return (
    <Provider client={client}>
      <Router />
    </Provider>
  );
};

export default App;

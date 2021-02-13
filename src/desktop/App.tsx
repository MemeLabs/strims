import React from "react";
import { HashRouter } from "react-router-dom";

import { FrontendClient } from "../apis/client";
import Provider from "../root/Provider";
import Router from "../root/Router";

interface AppProps {
  client: FrontendClient;
}

const App: React.FC<AppProps> = ({ client }) => (
  <HashRouter>
    <Provider client={client}>
      <Router />
    </Provider>
  </HashRouter>
);

export default App;

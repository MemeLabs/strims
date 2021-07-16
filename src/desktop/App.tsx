import React from "react";
import { HashRouter } from "react-router-dom";

import { FrontendClient } from "../apis/client";
import Provider from "../root/Provider";
import RootRouter from "../root/Router";

interface AppProps {
  client: FrontendClient;
}

const App: React.FC<AppProps> = ({ client }) => (
  <HashRouter>
    <Provider client={client}>
      <RootRouter />
    </Provider>
  </HashRouter>
);

export default App;

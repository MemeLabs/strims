import React from "react";
import { BrowserRouter } from "react-router-dom";

import { FrontendClient } from "../apis/client";
import Provider from "../root/Provider";
import Router from "../root/Router";

interface AppProps {
  client: FrontendClient;
}

const App: React.FC<AppProps> = ({ client }) => (
  <BrowserRouter>
    <Provider client={client}>
      <Router />
    </Provider>
  </BrowserRouter>
);

export default App;

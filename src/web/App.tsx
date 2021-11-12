import React, { ComponentType } from "react";
import { BrowserRouter, MemoryRouter } from "react-router-dom";

import { FrontendClient } from "../apis/client";
import { IS_PWA } from "../lib/userAgent";
import Provider from "../root/Provider";
import RootRouter from "../root/Router";

const Router: ComponentType = IS_PWA ? MemoryRouter : BrowserRouter;

interface AppProps {
  client: FrontendClient;
}

const App: React.FC<AppProps> = ({ client }) => (
  <Router>
    <Provider client={client}>
      <RootRouter />
    </Provider>
  </Router>
);

export default App;

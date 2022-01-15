import React, { ComponentType } from "react";
import { BrowserRouter, MemoryRouter } from "react-router-dom";

import { ConnFactoryThing } from "../contexts/Session";
// import { FrontendClient } from "../apis/client";
import { IS_PWA } from "../lib/userAgent";
import Provider from "../root/Provider";
import RootRouter from "../root/Router";

const Router: ComponentType = IS_PWA ? MemoryRouter : BrowserRouter;

interface AppProps {
  // client: FrontendClient;
  thing: ConnFactoryThing;
}

const App: React.FC<AppProps> = ({ thing }) => (
  <Router>
    <Provider thing={thing}>
      <RootRouter />
    </Provider>
  </Router>
);

export default App;

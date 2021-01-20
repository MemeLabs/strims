import * as React from "react";
import { HashRouter } from "react-router-dom";

import { FrontendClient } from "../apis/client";
import Provider from "../root/Provider";
import Router from "../root/Router";

const App = ({ client }: { client: FrontendClient }) => (
  <HashRouter>
    <Provider client={client}>
      <Router />
    </Provider>
  </HashRouter>
);

export default App;

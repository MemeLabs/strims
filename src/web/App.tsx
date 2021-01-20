import * as React from "react";
import { BrowserRouter } from "react-router-dom";

import { FrontendClient } from "../apis/client";
import Provider from "../root/Provider";
import Router from "../root/Router";

const App = ({ client }: { client: FrontendClient }) => (
  <BrowserRouter>
    <Provider client={client}>
      <Router />
    </Provider>
  </BrowserRouter>
);

export default App;

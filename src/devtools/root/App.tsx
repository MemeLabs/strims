import React from "react";
import { useTitle } from "react-use";

import { DevToolsClient } from "../../apis/client";
import Provider from "./Provider";
import Router from "./Router";

const App = ({ client }: { client: DevToolsClient }) => {
  useTitle("DevTools");

  return (
    <Provider client={client}>
      <Router />
    </Provider>
  );
};

export default App;

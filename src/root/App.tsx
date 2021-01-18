import * as React from "react";
import { useTitle } from "react-use";

import { FrontendClient } from "../apis/client";
import Provider from "./Provider";
import Router from "./Router";

const App = ({ client }: { client: FrontendClient }) => {
  useTitle("Strims");

  return (
    <Provider client={client}>
      <Router />
    </Provider>
  );
};

export default App;

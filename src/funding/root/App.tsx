import * as React from "react";
import { useTitle } from "react-use";

import { FundingClient } from "../../lib/api";
import Provider from "./Provider";
import Router from "./Router";

const App = ({ client }: { client: FundingClient }) => {
  useTitle("Strims");

  return (
    <Provider client={client}>
      <Router />
    </Provider>
  );
};

export default App;

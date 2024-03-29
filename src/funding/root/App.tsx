// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { useTitle } from "react-use";

import { FundingClient } from "../../apis/client";
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

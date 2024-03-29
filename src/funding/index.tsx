// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./styles/main.scss";

import React from "react";
import ReactDOM from "react-dom";

import { FundingClient } from "../apis/client";
import { WSReadWriter } from "../lib/ws";
import App from "./root/App";

(() => {
  const ws: any = new WSReadWriter(`wss://${location.host}/api/funding`);
  const client = new FundingClient(ws, ws);

  const root = document.createElement("div");
  root.setAttribute("id", "root");
  document.body.appendChild(root);

  ReactDOM.render(<App client={client} />, root);
})();

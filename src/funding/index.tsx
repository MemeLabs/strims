import "./styles/main.scss";

import * as React from "react";
import * as ReactDOM from "react-dom";

import { FundingClient } from "../lib/api";
import { WSReadWriter } from "../lib/ws";
import App from "./root/App";

(() => {
  const ws: any = new WSReadWriter(`wss://${location.host}/api`);
  const client = new FundingClient(ws, ws);

  const root = document.createElement("div");
  root.setAttribute("id", "root");
  document.body.appendChild(root);

  ReactDOM.render(<App client={client} />, root);
})();

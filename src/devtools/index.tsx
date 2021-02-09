import "./styles/main.scss";

import * as React from "react";
import * as ReactDOM from "react-dom";

import { DevToolsClient } from "../apis/client";
import { WSReadWriter } from "../lib/ws";
import App from "./root/App";

(() => {
  const ws: any = new WSReadWriter(`wss://${location.host}/api`);
  const client = new DevToolsClient(ws, ws);

  const root = document.createElement("div");
  root.setAttribute("id", "root");
  document.body.appendChild(root);

  ReactDOM.render(<App client={client} />, root);
})();

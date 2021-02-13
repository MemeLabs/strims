import "../styles/main.scss";

import React from "react";
import ReactDOM from "react-dom";

import { FrontendClient } from "../apis/client";
import { WindowBridge } from "../lib/bridge";
import { WSReadWriter } from "../lib/ws";
import Worker from "./svc.worker";

void (async () => {
  const bridge = new WindowBridge(Worker);
  const client = await new Promise<FrontendClient>((resolve) => {
    bridge.once("busopen:default", (b: any) => resolve(new FrontendClient(b, b)));
  });

  // const ws: any = new WSReadWriter(`wss://${location.host}/manage`);
  // const client = new FrontendClient(ws, ws);

  const root = document.createElement("div");
  root.setAttribute("id", "root");
  document.body.appendChild(root);

  const { default: App } = await import(/* webpackPreload: true */ "./App");
  ReactDOM.render(<App client={client} />, root);
})();

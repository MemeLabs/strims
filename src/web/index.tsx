import "../styles/main.scss";

import * as React from "react";
import * as ReactDOM from "react-dom";
import Worker from "worker-loader!./svc.worker";

import Client from "../lib/api";
import { WindowBridge } from "../lib/bridge";
import { WSReadWriter } from "../lib/ws";
import App from "../root/App";

(async () => {
  const bridge = new WindowBridge(Worker as any);
  const client = await new Promise<Client>((resolve) => {
    bridge.once("busopen:default", (b: any) => resolve(new Client(b, b)));
  });

  // const ws: any = new WSReadWriter(`wss://${location.host}/manage`);
  // const client = new Client(ws, ws);

  const root = document.createElement("div");
  root.setAttribute("id", "root");
  document.body.appendChild(root);

  ReactDOM.render(<App client={client} />, root);
})();

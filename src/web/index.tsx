
import * as React from "react";
import * as ReactDOM from "react-dom";
import Worker from "worker-loader!./svc.worker";
import Client from "../lib/api/client";
import { WindowBridge } from "../lib/bridge";
import App from "../root/App";

import "../styles/main.scss";

class WSReadWriter {
  public ws: Promise<WebSocket>;

  constructor(uri: string) {
    const ws = new WebSocket(uri);
    ws.binaryType = "arraybuffer";

    this.ws = new Promise((resolve, reject) => {
      ws.onopen = () => resolve(ws);
      ws.onerror = reject;
    });
  }

  public on(method: string, handler: (...args: any[]) => any) {
    this.ws.then((ws) => {
      if (method === "data") {
        ws.addEventListener("message", (e) => handler(new Uint8Array(e.data)));
      } else {
        ws.addEventListener(method, handler);
      }
    });
  }

  public write(data: Uint8Array) {
    this.ws.then((ws) => ws.send(data));
  }
}

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

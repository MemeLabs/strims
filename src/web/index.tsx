import "../styles/main.scss";

import React from "react";
import ReactDOM from "react-dom";

import { FrontendClient } from "../apis/client";
import { WindowBridge } from "../lib/bridge";
import { WSReadWriter } from "../lib/ws";
import Worker from "./svc.worker";

class Runner {
  root: HTMLDivElement;
  bridge: WindowBridge;

  constructor() {
    this.root = document.createElement("div");
    this.root.setAttribute("id", "root");
    document.body.appendChild(this.root);
  }

  async start() {
    this.bridge = new WindowBridge(Worker);
    const client = await new Promise<FrontendClient>((resolve) => {
      this.bridge.once("busopen:default", (b: any) => resolve(new FrontendClient(b, b)));
    });

    // const ws: any = new WSReadWriter(`wss://${location.host}/manage`);
    // const client = new FrontendClient(ws, ws);

    const { default: App } = await import(/* webpackPreload: true */ "./App");
    ReactDOM.render(<App client={client} />, this.root);
  }

  stop() {
    ReactDOM.unmountComponentAtNode(this.root);
    this.bridge?.close();
    this.bridge = undefined;
  }

  restart() {
    this.stop();
    void this.start();
  }
}

const runner = new Runner();
void runner.start();

if (import.meta.webpackHot) {
  import.meta.webpackHot.accept(
    ["../apis/client", "../lib/bridge", "../lib/ws", "./svc.worker"],
    () => runner.restart()
  );
}

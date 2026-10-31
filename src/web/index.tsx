// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "../styles/main.scss";
import "../lib/i18n";

import { Duplex } from "stream";

import React from "react";
import { Root, createRoot } from "react-dom/client";
import registerServiceWorker from "service-worker-loader!./sw";

import { APIDialer, ClientConstructor } from "../contexts/Session";
import { WSReadWriter } from "../lib/ws";
import App from "./App";
import manifest from "./manifest";
import { WasmNode, startWasmNode } from "./wasm";

void (async () => {
  if (IS_PRODUCTION && !(await caches.has(`${GIT_HASH}_static`))) {
    const cache = await caches.open(`${GIT_HASH}_static`);
    await cache.addAll(manifest);
  }
})();

void registerServiceWorker({ scope: "/" });

class WorkerConn {
  node: WasmNode;

  constructor() {
    this.node = startWasmNode({
      logLevel: parseInt(window.localStorage.getItem("log_level")) >> 0,
      allowInsecureWs: false,
      iceServers: ["stun:stun.l.google.com:19302"],
    });
  }

  client<T>(C: ClientConstructor<T>): Promise<T> {
    const bus = this.node.bus as unknown as Duplex;
    return Promise.resolve(new C(bus, bus));
  }

  get feeds() {
    return this.node.feeds;
  }

  close() {
    this.node.close();
  }
}

class WSConn extends WSReadWriter {
  client<T>(C: ClientConstructor<T>): Promise<T> {
    const ws = this as unknown as Duplex;
    return Promise.resolve(new C(ws, ws));
  }
}

const apiDialer: APIDialer = {
  local: (): WorkerConn => new WorkerConn(),
  remote: (address: string): WSConn => new WSConn(address),
};

class Runner {
  rootElement: HTMLDivElement;
  root: Root;

  constructor() {
    this.rootElement = document.createElement("div");
    this.rootElement.setAttribute("id", "root");
    document.body.appendChild(this.rootElement);
  }

  start() {
    this.root = createRoot(this.rootElement);
    this.root.render(<App apiDialer={apiDialer} />);
  }

  stop() {
    this.root?.unmount();
    this.root = null;
  }

  restart() {
    this.stop();
    this.start();
  }
}

const runner = new Runner();
runner.start();

if (import.meta.webpackHot) {
  import.meta.webpackHot.accept(["../apis/client", "../lib/ws", "./wasm"], () => runner.restart());
}

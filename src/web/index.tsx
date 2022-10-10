// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "../styles/main.scss";
import "../lib/i18n";

import { Duplex } from "stream";

import React from "react";
import { Root, createRoot } from "react-dom/client";
import registerServiceWorker from "service-worker-loader!./sw";

import { APIDialer, ClientConstructor } from "../contexts/Session";
import { WindowBridge } from "../lib/bridge";
import { WSReadWriter } from "../lib/ws";
import App from "./App";
import manifest from "./manifest";
import Worker from "./svc.worker";

void (async () => {
  if (IS_PRODUCTION && !(await caches.has(`${GIT_HASH}_static`))) {
    const cache = await caches.open(`${GIT_HASH}_static`);
    await cache.addAll(manifest);
  }
})();

void registerServiceWorker({ scope: "/" });

class WorkerConn {
  bridge: WindowBridge;
  bus: Promise<Duplex>;

  constructor() {
    this.bridge = new WindowBridge(Worker);
    this.bus = new Promise<Duplex>((resolve) => {
      this.bridge.once("busopen:default", (b: Duplex) => resolve(b));
    });
    this.bridge.createWorker("default", {
      logLevel: parseInt(window.localStorage.getItem("log_level")) >> 0,
    });
  }

  async client<T>(C: ClientConstructor<T>): Promise<T> {
    const b = await this.bus;
    return new C(b, b);
  }

  close() {
    this.bridge.close();
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
  import.meta.webpackHot.accept(
    ["../apis/client", "../lib/bridge", "../lib/ws", "./svc.worker"],
    () => runner.restart()
  );
}

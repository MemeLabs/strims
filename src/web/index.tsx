// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "../styles/main.scss";
import "../lib/i18n";

import { Readable, Writable } from "stream";

import React from "react";
import { Root, createRoot } from "react-dom/client";
import registerServiceWorker from "service-worker-loader!./sw";

import { APIDialer, ClientConstructor } from "../contexts/Session";
import { WindowBridge } from "../lib/bridge";
import { WSReadWriter } from "../lib/ws";
import App from "./App";
import Worker from "./svc.worker";

void registerServiceWorker({ scope: "/" })
  .then(() => console.log("service worker registered"))
  .catch((e) => console.log("error registering service worker", e));

class WorkerConn {
  bridge: WindowBridge;
  bus: Promise<Readable & Writable>;

  constructor() {
    this.bridge = new WindowBridge(Worker);
    this.bus = new Promise<Readable & Writable>((resolve) => {
      this.bridge.once("busopen:default", (b: Readable & Writable) => resolve(b));
    });
    this.bridge.createWorker("default");
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
    const ws = this as unknown as Readable & Writable;
    return Promise.resolve(new C(ws, ws));
  }
}

const apiDialer: APIDialer = {
  local: (): WorkerConn => new WorkerConn(),
  remote: (address: string): WSConn => new WSConn(address),
};

class Runner {
  root: Root;

  constructor() {
    const root = document.createElement("div");
    root.setAttribute("id", "root");
    document.body.appendChild(root);
    this.root = createRoot(root);
  }

  start() {
    this.root.render(<App apiDialer={apiDialer} />);
  }

  stop() {
    this.root.unmount();
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

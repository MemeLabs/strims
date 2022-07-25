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
import { wasmPath } from "./svc.go";
import Worker from "./svc.worker";

void (async () => {
  // webpack chunk prefetching with mini-css-extract-plugin doesn't load css
  // chunks. even if this worked safari and firefox seem to ignore prefetch
  // headers. so the only way to make sure the app assets actually get
  // prefetched is to manually load them.

  // TODO: delay this until after the translations load?

  const preloadTiers: string[][] = [[], [wasmPath]];
  const collectPreloadFiles = (c: ChunkManifest, d: number = 0) => {
    preloadTiers[d] = (preloadTiers[d] ?? []).concat(...c.files);
    for (const ac of c.asyncChunks ?? []) {
      collectPreloadFiles(ac, d + 1);
    }
  };
  collectPreloadFiles(MANIFEST);

  for (let i = 1; i < preloadTiers.length; i++) {
    await Promise.allSettled(
      preloadTiers[i].map((f) => fetch(f, { priority: "low" } as RequestInit))
    );
  }
})();

void registerServiceWorker({ scope: "/" })
  .then(() => console.log("service worker registered"))
  .catch((e) => console.log("error registering service worker", e));

class WorkerConn {
  bridge: WindowBridge;
  bus: Promise<Duplex>;

  constructor() {
    this.bridge = new WindowBridge(Worker);
    this.bus = new Promise<Duplex>((resolve) => {
      this.bridge.once("busopen:default", (b: Duplex) => resolve(b));
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

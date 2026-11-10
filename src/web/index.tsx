// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "../styles/main.scss";
import "../lib/i18n";

import React from "react";
import { Root, createRoot } from "react-dom/client";
import registerServiceWorker from "service-worker-loader!./sw";

import App from "./App";
import { apiDialer } from "./dialer";
import manifest from "./manifest";

void (async () => {
  if (IS_PRODUCTION && !(await caches.has(`${GIT_HASH}_static`))) {
    const cache = await caches.open(`${GIT_HASH}_static`);
    await cache.addAll(manifest);
  }
})();

void registerServiceWorker({ scope: "/" });

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
  import.meta.webpackHot.accept(["../apis/client", "./dialer"], () => runner.restart());
}

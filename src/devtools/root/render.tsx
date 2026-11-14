// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { Readable, Writable } from "stream";

import React from "react";
import { createRoot } from "react-dom/client";

import { DevToolsClient } from "../../apis/client";
import { WSReadWriter } from "../../lib/ws";
import App from "./App";

// Mounts the devtools app on `root`, its API on the page's own host.
export const renderApp = (root: HTMLElement): void => {
  const ws = new WSReadWriter(`wss://${location.host}/api`) as unknown as Readable & Writable;
  const client = new DevToolsClient(ws, ws);
  createRoot(root).render(<App client={client} />);
};

// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The page for the browser link tests. It starts ?count= e2e nodes (one by
// default) and relays node i's frontend connection to the WebSocket at
// /control?i=<i> on the page's host, as binary messages both ways.
// ?logLevel= sets the nodes' zap level.

import { WSReadWriter } from "../lib/ws";
import { startWasmNode } from "./wasm";

const params = new URLSearchParams(location.search);
const count = Math.max(1, parseInt(params.get("count") ?? "1") >> 0);
const logLevel = parseInt(params.get("logLevel") ?? "0") >> 0;
const scheme = location.protocol === "https:" ? "wss:" : "ws:";

for (let i = 0; i < count; i++) {
  const node = startWasmNode({ logLevel, allowInsecureWs: true, iceServers: [], e2e: true });
  const ws = new WSReadWriter(`${scheme}//${location.host}/control?i=${i}`);

  node.bus.on("data", (d: Uint8Array) => ws.write(d));
  node.bus.on("close", () => ws.close());
  ws.on("data", (d: Uint8Array) => node.bus.write(d));
  ws.on("close", () => node.close());
}

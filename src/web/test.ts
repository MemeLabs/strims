// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Relays one node's frontend connection to the WebSocket named by
// ?manage=, which defaults to wss://<host>/manage. ?allowInsecureWs=true
// lets the node dial ws:// URIs.

import { WSReadWriter } from "../lib/ws";
import { startWasmNode } from "./wasm";

class Success {}

const params = new URLSearchParams(location.search);

const node = startWasmNode({
  logLevel: parseInt(params.get("logLevel") ?? window.localStorage.getItem("log_level")) >> 0,
  allowInsecureWs: params.get("allowInsecureWs") === "true",
  iceServers: [],
});

const ws = new WSReadWriter(params.get("manage") ?? `wss://${location.host}/manage`);

node.bus.on("data", (d: Uint8Array) => ws.write(d));
node.bus.on("close", () => ws.close());
ws.on("data", (d: Uint8Array) => node.bus.write(d));

ws.on("close", () => {
  node.close();
  throw new Success();
});

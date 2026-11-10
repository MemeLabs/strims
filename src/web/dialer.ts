// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Duplex } from "stream";

import { APIDialer, ClientConstructor } from "../contexts/Session";
import { WSReadWriter } from "../lib/ws";
import { WasmNode, startWasmNode } from "./wasm";

// A node in the page: strims-wasm in its workers, its store in the page's
// IndexedDB, which every page of this origin shares.
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

/** The node in the page, or a native node's frontend API at a WebSocket address. */
export const apiDialer: APIDialer = {
  local: (): WorkerConn => new WorkerConn(),
  remote: (address: string): WSConn => new WSConn(address),
};

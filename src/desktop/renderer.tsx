import "../styles/main.scss";

import { ChildProcessWithoutNullStreams, spawn } from "child_process";
import { Readable, Writable } from "stream";

import React from "react";
import ReactDOM from "react-dom";

import { APIDialer, ClientConstructor } from "../contexts/Session";
import { WSReadWriter } from "../lib/ws";
import App from "./App";

class IPCConn {
  svc: ChildProcessWithoutNullStreams;

  constructor() {
    this.svc = spawn("./dist/desktop/svc");
    window.addEventListener("beforeunload", () => this.svc.kill());
    this.svc.stderr.on("data", (d: Buffer) => console.log(d.toString()));
  }

  async client<T>(C: ClientConstructor<T>): Promise<T> {
    return Promise.resolve(new C(this.svc.stdin, this.svc.stdout));
  }

  close() {
    this.svc.kill();
  }
}

class WSConn extends WSReadWriter {
  client<T>(C: ClientConstructor<T>): Promise<T> {
    const ws = this as unknown as Readable & Writable;
    return Promise.resolve(new C(ws, ws));
  }
}

const apiDialer: APIDialer = {
  local: (): IPCConn => new IPCConn(),
  remote: (address: string): WSConn => new WSConn(address),
};

window.addEventListener("DOMContentLoaded", () => {
  const root = document.createElement("div");
  root.setAttribute("id", "root");
  document.body.appendChild(root);

  ReactDOM.render(<App apiDialer={apiDialer} />, root);
});

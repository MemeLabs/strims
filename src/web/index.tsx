import "../styles/main.scss";
import "../lib/i18n";

import { Readable, Writable } from "stream";

import React from "react";
import ReactDOM from "react-dom";

import { ClientConstructor, ConnFactoryThing } from "../contexts/Session";
import { WindowBridge } from "../lib/bridge";
import { WSReadWriter } from "../lib/ws";
import Worker from "./svc.worker";

class WorkerConn {
  bridge: WindowBridge;
  bus: Promise<Readable & Writable>;

  constructor() {
    this.bridge = new WindowBridge(Worker);
    this.bus = new Promise<Readable & Writable>((resolve) => {
      this.bridge.once("busopen:default", (b: Readable & Writable) => resolve(b));
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
    return Promise.resolve(new C(this, this));
  }
}

class FooFactoryThing implements ConnFactoryThing {
  static local(): WorkerConn {
    return new WorkerConn();
  }

  static remote(address: string): WSConn {
    return new WSConn(address);
  }
}

class Runner {
  root: HTMLDivElement;

  constructor() {
    this.root = document.createElement("div");
    this.root.setAttribute("id", "root");
    document.body.appendChild(this.root);
  }

  async start() {
    const { default: App } = await import(/* webpackPreload: true */ "./App");
    ReactDOM.render(<App thing={FooFactoryThing} />, this.root);
  }

  stop() {
    ReactDOM.unmountComponentAtNode(this.root);
  }

  restart() {
    this.stop();
    void this.start();
  }
}

const runner = new Runner();
void runner.start();

if (import.meta.webpackHot) {
  import.meta.webpackHot.accept(
    ["../apis/client", "../lib/bridge", "../lib/ws", "./svc.worker"],
    () => runner.restart()
  );
}

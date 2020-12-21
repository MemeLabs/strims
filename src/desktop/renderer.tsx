import { spawn } from "child_process";

import storage from "electron-json-storage";
import * as React from "react";
import * as ReactDOM from "react-dom";

import { FrontendClient } from "../lib/api";
import App from "../root/App";

const p2p = spawn("./dist/desktop/p2p");
window.addEventListener("beforeunload", () => p2p.kill());
p2p.stderr.on("data", (d: Buffer) => console.log(d.toString()));

const client = new FrontendClient(p2p.stdin, p2p.stdout);

window.addEventListener("DOMContentLoaded", () => {
  const root = document.createElement("div");
  root.setAttribute("id", "root");
  document.body.appendChild(root);

  ReactDOM.render(<App client={client} />, root);
});

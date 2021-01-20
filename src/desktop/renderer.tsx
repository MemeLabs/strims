import "../styles/main.scss";

import { spawn } from "child_process";

import * as React from "react";
import * as ReactDOM from "react-dom";

import { FrontendClient } from "../apis/client";
import App from "./App";

const svc = spawn("./dist/desktop/svc");
window.addEventListener("beforeunload", () => svc.kill());
svc.stderr.on("data", (d: Buffer) => console.log(d.toString()));

const client = new FrontendClient(svc.stdin, svc.stdout);

window.addEventListener("DOMContentLoaded", () => {
  const root = document.createElement("div");
  root.setAttribute("id", "root");
  document.body.appendChild(root);

  ReactDOM.render(<App client={client} />, root);
});

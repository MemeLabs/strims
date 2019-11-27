import { spawn } from "child_process";
import * as React from "react";
import * as ReactDOM from "react-dom";
import Provider from "../components/Provider";
import Client from "../service/client";

const p2p = spawn("./dist/app/p2p");
window.addEventListener("beforeunload", () => p2p.kill());
p2p.stderr.on("data", (d: Buffer) => console.log(d.toString()));

const client = new Client(p2p.stdin, p2p.stdout);

window.addEventListener("DOMContentLoaded", () => {
  const root = document.createElement("div");
  root.setAttribute("id", "root");
  document.body.appendChild(root);

  ReactDOM.render(<Provider client={client} />, root);
});

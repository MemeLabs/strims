import { spawn } from "child_process";
import * as React from "react";
import * as ReactDOM from "react-dom";
import Provider from "../components/Provider";
import Client from "../service/client";

const proc = spawn("./dist/app/p2p");
window.addEventListener("beforeunload", () => proc.kill());
proc.stderr.on("data", (d: Buffer) => console.log(d.toString()));

const client = new Client(proc.stdin, proc.stdout);

window.addEventListener("DOMContentLoaded", () => {
  const root = document.createElement("div");
  root.setAttribute("id", "root");
  document.body.appendChild(root);

  ReactDOM.render(<Provider client={client} />, root);
});

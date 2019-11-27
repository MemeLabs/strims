
import * as React from "react";
import * as ReactDOM from "react-dom";
import Provider from "../components/Provider";
import Client from "../service/client";
import Bus from "./wasmio_bus";
import * as wrtc from "./wrtc";

import p2p from "./p2p.go";

(async () => {
  const b: any = new Bus();
  await p2p.init(new wrtc.Bridge(), b);
  const client = new Client(b, b);

  const root = document.createElement("div");
  root.setAttribute("id", "root");
  document.body.appendChild(root);

  ReactDOM.render(<Provider client={client} />, root);
})();

import * as React from "react";
import * as ReactDOM from "react-dom";
import { BrowserRouter } from "react-router-dom";
// import "./wasm_exec";

import "./main.scss";

import App from "./components/App";

const { Suspense } = React;

const LoadingMessage = () => (<p className="loading_message">loading</p>);

const Provider = () => (
  <BrowserRouter>
    <Suspense fallback={<LoadingMessage />}>
      <App />
    </Suspense>
  </BrowserRouter>
);

const root = document.createElement("div");
root.setAttribute("id", "root");
document.body.appendChild(root);

ReactDOM.render(<Provider />, root);

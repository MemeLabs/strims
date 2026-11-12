// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "../styles/main.scss";
import "../components/Theme.scss";
import "./watch.scss";
import "../lib/i18n";

import React from "react";
import { createRoot } from "react-dom/client";

import { apiDialer } from "../web/dialer";
import App from "./App";

// The worker is emitted as /sw.js, at the root its scope needs.
if ("serviceWorker" in navigator) {
  void navigator.serviceWorker.register(
    /* webpackChunkName: "sw", webpackEntryOptions: { filename: "sw.js" } */
    new URL("../web/sw.ts", import.meta.url),
    { scope: "/" }
  );
}

const rootElement = document.createElement("div");
rootElement.setAttribute("id", "root");
document.body.appendChild(rootElement);
createRoot(rootElement).render(<App apiDialer={apiDialer} />);

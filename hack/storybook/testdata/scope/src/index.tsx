// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// A frame document loads the frame alone, any other the app, as
// src/devtools/index.tsx does.
import "./main.scss";

if (location.hash.startsWith("#/frame/")) {
  void import("./host/Frame").then(({ renderFrame }) => renderFrame(location.hash.substring(8)));
} else {
  void import("./app/App").then(({ renderApp }) => renderApp());
}

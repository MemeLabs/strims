// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The devtools app's stand-in, which no frame loads.
import React from "react";
import { createRoot } from "react-dom/client";

export const renderApp = (): void => {
  createRoot(document.body).render(<main>the app</main>);
};

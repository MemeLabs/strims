// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { createRoot } from "react-dom/client";

import { loadStory } from "./catalog";

// The build's identity, which the manifest hashes as placeholders.
export const build = { commit: GIT_HASH, time: BUILD_TIME };

export const renderFrame = (route: string): void => {
  void loadStory(route).then((Story) => createRoot(document.body).render(<Story />));
};

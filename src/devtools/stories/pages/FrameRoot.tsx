// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./pages.stories.scss";

import React, { ReactNode } from "react";

import { withLayoutContext } from "../../../contexts/Layout";

// A layout root of the story's own that fixed position content (the landing
// page, the invite modal portalled into the layout's root) fills: the
// storybook's frame isn't a containing block for it, so it would cover the
// storybook's window.
export const FrameRoot = withLayoutContext<{ children: ReactNode }>(({ rootRef, children }) => (
  <div ref={rootRef} className="pages_story_frame_root">
    {children}
  </div>
));

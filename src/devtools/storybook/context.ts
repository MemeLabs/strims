// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { createContext, useContext } from "react";

/** What the storybook page offers the story it mounts. */
export interface StoryHostValue {
  /** Shows a StoryRouter's current path in the toolbar (null clears it). */
  setRouterPath: (path: string | null) => void;
}

const noop = (): void => undefined;

export const StoryHostContext = createContext<StoryHostValue>({ setRouterPath: noop });

export const useStoryHost = (): StoryHostValue => useContext(StoryHostContext);

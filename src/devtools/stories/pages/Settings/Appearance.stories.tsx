// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import AppearancePage from "../../../../pages/Settings/Appearance";
import { Story } from "../../../storybook";
import { SettingsStory } from "./Layout.harness";
import { SettingsStory as SteppedSettingsStory, click } from "./Network/harness";

const stories: Story[] = [
  {
    name: "Default",
    component: () => <SettingsStory path="/settings/appearance" />,
  },
  // The toggle flipped as a tap does: the frame's layout swaps with it.
  {
    name: "Chat on the left",
    component: () => (
      <SteppedSettingsStory element={<AppearancePage />} steps={[click(".input_toggle")]} />
    ),
  },
];

export default stories;

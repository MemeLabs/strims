// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import { Catalog, Story } from "../../../../storybook";
import { SettingsStory } from "../Network/harness";

const stories: Story[] = [
  {
    // The profile section's index: links to pairing and devices.
    name: "Default",
    component: () => <SettingsStory path="/settings/profile" />,
  },
];

export default stories;

export const catalog: Catalog = {
  covers: ["pages/Settings/Profile/index.tsx"],
};

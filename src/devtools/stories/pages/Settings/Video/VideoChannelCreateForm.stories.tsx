// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import { Story } from "../../../../storybook";
import { SettingsStory } from "../Layout.harness";

const PATH = "/settings/video/channels/new";

const stories: Story[] = [
  {
    name: "Default",
    component: () => <SettingsStory path={PATH} />,
  },
  {
    // Submitted empty.
    name: "Invalid",
    component: () => <SettingsStory path={PATH} submit />,
  },
];

export default stories;

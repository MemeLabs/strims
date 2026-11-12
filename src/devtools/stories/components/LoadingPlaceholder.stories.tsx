// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import LoadingPlaceholder from "../../../components/LoadingPlaceholder";
import { Story } from "../../storybook";

// The placeholder fills the viewport (position: fixed): the story's frame.
const stories: Story[] = [
  {
    name: "Default",
    component: () => <LoadingPlaceholder />,
  },
];

export default stories;

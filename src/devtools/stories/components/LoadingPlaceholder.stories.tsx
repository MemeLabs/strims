// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./core.stories.scss";

import React from "react";

import LoadingPlaceholder from "../../../components/LoadingPlaceholder";
import { Story } from "../../storybook";

// The placeholder fills the viewport (position: fixed): the story's frame here.
const stories: Story[] = [
  {
    name: "Default",
    component: () => (
      <div className="core_story__viewport core_story__viewport--fill">
        <LoadingPlaceholder />
      </div>
    ),
  },
];

export default stories;

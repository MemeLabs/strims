// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Settings.stories.scss";

import React from "react";

import Search from "../../../../components/Settings/Search";
import { TableTitleBar } from "../../../../components/Settings/Table";
import { Story } from "../../../storybook";

const stories: Story[] = [
  {
    name: "Empty",
    component: () => (
      <div className="settings_story">
        <div className="settings_story__row">
          <Search />
        </div>
      </div>
    ),
  },
  {
    name: "With value",
    component: () => (
      <div className="settings_story">
        <div className="settings_story__row">
          <Search defaultValue="comfy" />
        </div>
      </div>
    ),
  },
  {
    name: "Disabled",
    component: () => (
      <div className="settings_story">
        <div className="settings_story__row">
          <Search defaultValue="comfy" disabled />
        </div>
      </div>
    ),
  },
  {
    name: "In a title bar",
    component: () => (
      <div className="settings_story">
        <TableTitleBar label="Emotes">
          <Search defaultValue="a query long enough to overflow the search box" />
        </TableTitleBar>
      </div>
    ),
  },
];

export default stories;

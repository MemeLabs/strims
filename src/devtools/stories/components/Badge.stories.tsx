// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./core.stories.scss";

import React from "react";

import Badge, { BadgeProps } from "../../../components/Badge";
import { Story } from "../../storybook";

// Badges on the corner of a gem, as NetworkGem and the chat's room tabs show them.
const Gems: React.FC<{ badges: BadgeProps[] }> = ({ badges }) => (
  <div className="core_story">
    <div className="core_story__row">
      {badges.map((props, i) => (
        <div key={i} className="core_story__gem">
          <Badge {...props} />
        </div>
      ))}
    </div>
  </div>
);

const stories: Story[] = [
  {
    name: "Default",
    component: () => <Gems badges={[{ count: 1 }, { count: 7 }, { count: 42 }]} />,
  },
  {
    name: "Over max",
    component: () => (
      <Gems
        badges={[
          { count: 101, max: 100 },
          { count: 1234, max: 500 },
        ]}
      />
    ),
  },
  {
    name: "Thousands",
    component: () => <Gems badges={[{ count: 1234 }, { count: 65536 }, { count: 1048576 }]} />,
  },
  {
    name: "Zero",
    component: () => <Gems badges={[{ count: 0 }]} />,
  },
  {
    name: "Hidden",
    component: () => <Gems badges={[{ count: 3, hidden: true }]} />,
  },
];

export default stories;

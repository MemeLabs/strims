// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./core.stories.scss";

import React, { useState } from "react";

import Stopwatch from "../../../components/Stopwatch";
import { Story } from "../../storybook";

// A stopwatch started `elapsed` seconds before the story mounted: it ticks
// on from there.
const Running: React.FC<{ elapsed: number }> = ({ elapsed }) => {
  const [startTime] = useState(() => Math.floor(Date.now() / 1000) - elapsed);
  return (
    <div className="core_story">
      <span className="core_story__stopwatch">
        <Stopwatch startTime={startTime} />
      </span>
    </div>
  );
};

const stories: Story[] = [
  {
    name: "Running",
    component: () => <Running elapsed={3723} />,
  },
  {
    name: "Just started",
    component: () => <Running elapsed={0} />,
  },
  {
    name: "Long",
    component: () => <Running elapsed={123 * 3600 + 45 * 60} />,
  },
  {
    // A start time ahead of the viewer's clock.
    name: "Clock skew",
    component: () => <Running elapsed={-90} />,
  },
];

export default stories;

// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./core.stories.scss";

import React from "react";

import SwipablePanel from "../../../components/SwipablePanel";
import { Story } from "../../storybook";

type Direction = "up" | "down" | "left" | "right";

interface PanelProps {
  direction: Direction;
  open: boolean;
  locked?: boolean;
}

// A panel in a viewport of its own (it is position: fixed); a closed one
// peeks in by --swipable-peek-size. filterDeviceTypes={null} gives the real
// panel on the desktop too, where the default leaves a plain div.
const Panel: React.FC<PanelProps> = ({ direction, open, locked }) => (
  <div>
    <p className="core_story__caption">
      {direction}, {open ? "open" : "closed"}
      {locked && ", locked"}
    </p>
    <div className="core_story__viewport">
      <SwipablePanel
        direction={direction}
        open={open}
        locked={locked}
        filterDeviceTypes={null}
        className={`core_story__panel core_story__panel--${direction}`}
      >
        Swipe {direction === "up" || direction === "down" ? "vertically" : "sideways"} to
        {open ? " close" : " open"}.
      </SwipablePanel>
    </div>
  </div>
);

const Both: React.FC<{ direction: Direction }> = ({ direction }) => (
  <div className="core_story">
    <div className="core_story__row">
      <Panel direction={direction} open />
      <Panel direction={direction} open={false} />
    </div>
  </div>
);

const stories: Story[] = [
  {
    name: "Up",
    component: () => <Both direction="up" />,
  },
  {
    name: "Down",
    component: () => <Both direction="down" />,
  },
  {
    name: "Left",
    component: () => <Both direction="left" />,
  },
  {
    name: "Right",
    component: () => <Both direction="right" />,
  },
  {
    name: "Locked",
    component: () => (
      <div className="core_story">
        <div className="core_story__row">
          <Panel direction="up" open locked />
          <Panel direction="left" open locked />
        </div>
      </div>
    ),
  },
  {
    name: "Desktop",
    component: () => (
      <div className="core_story">
        <p className="core_story__caption">
          With the default filterDeviceTypes the panel is a plain div off portable devices.
        </p>
        <SwipablePanel direction="up" className="core_story__panel">
          A panel on the desktop.
        </SwipablePanel>
      </div>
    ),
  },
];

export default stories;

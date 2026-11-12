// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "../Layout.stories.scss";

import React, { useEffect, useRef } from "react";

import NetworkGem from "../../../../../components/Layout/NetworkNav/NetworkGem";
import Tooltip from "../../../../../components/Layout/NetworkNav/Tooltip";
import { Story, fixtures } from "../../../../storybook";

interface TooltipStoryProps {
  label: string;
  visible?: boolean;
}

// A tooltip around a network's gem, the pointer over it from mount; it
// shows to the gem's right, over the frame (the layout's root).
const TooltipStory: React.FC<TooltipStoryProps> = ({ label, visible }) => {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    ref.current
      .querySelector(".network_nav__tooltip")
      .dispatchEvent(new MouseEvent("mouseover", { bubbles: true, relatedTarget: null }));
  }, []);
  return (
    <div className="layout_story__tooltip" ref={ref}>
      <Tooltip label={label} visible={visible}>
        <div className="network_nav__link">
          <NetworkGem network={fixtures.network(2)} peerCount={12} />
        </div>
      </Tooltip>
    </div>
  );
};

const stories: Story[] = [
  {
    name: "Shown",
    component: () => <TooltipStory label="Strims" />,
  },
  {
    name: "Long label",
    component: () => (
      <TooltipStory label="Speedrunners Anonymous: Any% Glitchless Division (and friends)" />
    ),
  },
  {
    name: "Not visible",
    component: () => <TooltipStory label="Strims" visible={false} />,
  },
];

export default stories;

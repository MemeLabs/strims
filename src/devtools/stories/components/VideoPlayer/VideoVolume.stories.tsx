// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "../../../../components/VideoPlayer/VideoControls.scss";

import React, { useState } from "react";

import VideoVolume from "../../../../components/VideoPlayer/VideoVolume";
import { Catalog, Story } from "../../../storybook";
import { PlayerFrame } from "./video";

// The slider beside the controls' volume button, shown as under the
// pointer; dragging or wheeling it changes the value.
const VolumeStory: React.FC<{ value: number }> = ({ value: initial }) => {
  const [value, setValue] = useState(initial);
  return (
    <PlayerFrame hover>
      <div className="video_player__controls visible visible100 visible500">
        <div className="controls_group left">
          <div className="volume button-wrap">
            <VideoVolume
              value={value}
              onUpdate={(v) => setValue(Math.max(0, Math.min(1, v)))}
              onSlideStart={() => undefined}
              onSlideEnd={() => undefined}
            />
          </div>
        </div>
      </div>
    </PlayerFrame>
  );
};

const stories: Story[] = [
  {
    name: "Muted",
    component: () => <VolumeStory value={0} />,
  },
  {
    name: "Low",
    component: () => <VolumeStory value={0.25} />,
  },
  {
    name: "High",
    component: () => <VolumeStory value={0.8} />,
  },
  {
    name: "Full",
    component: () => <VolumeStory value={1} />,
  },
];

export default stories;

// The volume and progress bars' slider.
export const catalog: Catalog = { covers: ["components/VideoPlayer/Slider.tsx"] };

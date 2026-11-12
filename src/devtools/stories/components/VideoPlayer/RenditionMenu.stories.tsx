// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "../../../../components/VideoPlayer/VideoControls.scss";

import React, { useEffect, useRef, useState } from "react";

import RenditionMenu from "../../../../components/VideoPlayer/RenditionMenu";
import { Story } from "../../../storybook";
import { PlayerFrame } from "./video";

const RENDITIONS = [
  { id: "2160", label: "2160p60 AV1" },
  { id: "1080hevc", label: "1080p60 HEVC" },
  { id: "1080", label: "1080p H.264" },
  { id: "720", label: "720p H.264" },
  { id: "480", label: "480p H.264" },
];

interface MenuStoryProps {
  /** The rendition playing. */
  playing: string;
  picked?: string;
  /** Opens the menu on mount, as its button would. */
  open?: boolean;
  options?: typeof RENDITIONS;
}

// The menu in the controls' right group, where the swarm player puts it.
const MenuStory: React.FC<MenuStoryProps> = ({
  playing: playingProp,
  picked: pickedProp = null,
  open = true,
  options = RENDITIONS,
}) => {
  const [picked, setPicked] = useState(pickedProp);
  const playing = picked ?? playingProp;
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (open) {
      ref.current.querySelector<HTMLButtonElement>(".renditions button").click();
    }
  }, []);
  return (
    <PlayerFrame>
      <div className="video_player__controls visible visible100 visible500">
        <div className="progress_bar" />
        <div className="controls_group right" ref={ref}>
          <RenditionMenu options={options} playing={playing} picked={picked} pick={setPicked} />
        </div>
      </div>
    </PlayerFrame>
  );
};

const stories: Story[] = [
  {
    name: "Auto",
    component: () => <MenuStory playing="1080" />,
  },
  {
    name: "Picked",
    component: () => <MenuStory playing="1080" picked="720" />,
  },
  {
    name: "Two renditions",
    component: () => <MenuStory playing="1080" options={RENDITIONS.slice(2, 4)} />,
  },
  {
    name: "Closed",
    component: () => <MenuStory playing="1080" open={false} />,
  },
];

export default stories;

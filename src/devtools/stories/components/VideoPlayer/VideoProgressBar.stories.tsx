// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "../../../../components/VideoPlayer/VideoControls.scss";

import React from "react";

import VideoProgressBar from "../../../../components/VideoPlayer/VideoProgressBar";
import type { VideoState } from "../../../../hooks/useVideo";
import { Story } from "../../../storybook";
import { PlayerFrame, useFakeVideo } from "./video";

// The bar in the controls' middle, as VideoControls places it with
// showProgressBar (the app's players leave it out), shown as under the
// pointer. Its domain is the buffered range.
const ProgressStory: React.FC<{ state: Partial<VideoState> }> = ({ state }) => {
  const [videoState, videoControls] = useFakeVideo(state);
  return (
    <PlayerFrame hover>
      <div className="video_player__controls visible visible100 visible500">
        <div className="progress_bar">
          <VideoProgressBar videoState={videoState} videoControls={videoControls} />
        </div>
      </div>
    </PlayerFrame>
  );
};

const stories: Story[] = [
  {
    name: "Start",
    component: () => <ProgressStory state={{ currentTime: 0 }} />,
  },
  {
    name: "Middle",
    component: () => <ProgressStory state={{ currentTime: 30 }} />,
  },
  {
    name: "Live edge",
    component: () => <ProgressStory state={{ currentTime: 60 }} />,
  },
  {
    name: "Late buffer",
    component: () => (
      <ProgressStory state={{ bufferStart: 3600, bufferEnd: 3630, currentTime: 3610 }} />
    ),
  },
];

export default stories;

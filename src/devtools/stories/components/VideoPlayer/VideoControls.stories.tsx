// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "../../../../components/VideoPlayer/VideoPlayer.scss";

import React, { ComponentProps, useState } from "react";

import RenditionMenu from "../../../../components/VideoPlayer/RenditionMenu";
import VideoControls from "../../../../components/VideoPlayer/VideoControls";
import type { VideoState } from "../../../../hooks/useVideo";
import { Story } from "../../../storybook";
import { PlayerFrame, useFakeVideo } from "./video";

type Shown = Pick<
  ComponentProps<typeof VideoControls>,
  | "showPlayButton"
  | "showVolume"
  | "showProgressBar"
  | "showPiPButton"
  | "showTheaterButton"
  | "showFullscreenButton"
  | "showCloseButton"
  | "remoteButtons"
>;

// The buttons the swarm player shows on a desktop in the app's layout.
const DESKTOP: Shown = {
  showPlayButton: true,
  showVolume: true,
  showPiPButton: true,
  showTheaterButton: true,
  showFullscreenButton: true,
};

// On a phone, or on the watch page (no layout: a close button, no theater mode).
const PORTABLE: Shown = {
  showPlayButton: true,
  showVolume: true,
  showFullscreenButton: true,
  showCloseButton: true,
};

const RENDITIONS = [
  { id: "a", label: "1080p60 HEVC" },
  { id: "b", label: "1080p H.264" },
  { id: "c", label: "720p H.264" },
  { id: "d", label: "480p H.264" },
];

const Renditions: React.FC = () => {
  const [picked, pick] = useState<string>(null);
  return <RenditionMenu options={RENDITIONS} playing={picked ?? "b"} picked={picked} pick={pick} />;
};

const ALL: Shown = {
  ...DESKTOP,
  showProgressBar: true,
  showCloseButton: true,
  remoteButtons: <Renditions />,
};

interface ControlsStoryProps {
  shown?: Shown;
  state?: Partial<VideoState>;
  fullscreen?: boolean;
  theaterMode?: boolean;
  hover?: boolean;
}

const ControlsStory: React.FC<ControlsStoryProps> = ({
  shown = DESKTOP,
  state,
  fullscreen: fullscreenProp = false,
  theaterMode: theaterModeProp = false,
  hover,
}) => {
  const [videoState, videoControls] = useFakeVideo(state);
  const [fullscreen, setFullscreen] = useState(fullscreenProp);
  const [theaterMode, setTheaterMode] = useState(theaterModeProp);
  return (
    <PlayerFrame hover={hover}>
      <VideoControls
        visible
        videoState={videoState}
        videoControls={videoControls}
        fullscreen={fullscreen}
        toggleFullscreen={() => setFullscreen((f) => !f)}
        theaterMode={theaterMode}
        toggleTheaterMode={setTheaterMode}
        close={() => undefined}
        {...shown}
      />
    </PlayerFrame>
  );
};

const stories: Story[] = [
  {
    name: "Playing",
    component: () => <ControlsStory />,
  },
  {
    name: "Paused",
    component: () => <ControlsStory state={{ playing: false, paused: true }} />,
  },
  {
    name: "Muted",
    component: () => <ControlsStory state={{ muted: true, volume: 0 }} />,
  },
  {
    name: "Low volume",
    component: () => <ControlsStory state={{ volume: 0.2 }} hover />,
  },
  {
    name: "Buffering",
    component: () => (
      <ControlsStory
        state={{ playing: false, paused: false, waiting: true, bufferEnd: 42.5 }}
        shown={{ ...DESKTOP, showProgressBar: true }}
        hover
      />
    ),
  },
  {
    name: "Captions",
    component: () => (
      <ControlsStory state={{ cueKinds: ["subtitles", "captions"], captions: "subtitles" }} />
    ),
  },
  {
    name: "Captions off",
    component: () => <ControlsStory state={{ cueKinds: ["captions"], captions: null }} />,
  },
  {
    name: "No PiP",
    component: () => <ControlsStory state={{ supportPiP: false }} />,
  },
  {
    name: "Theater mode",
    component: () => <ControlsStory theaterMode />,
  },
  {
    name: "Fullscreen",
    component: () => <ControlsStory fullscreen />,
  },
  {
    name: "Portable",
    component: () => <ControlsStory shown={PORTABLE} />,
  },
  {
    name: "All buttons",
    component: () => (
      <ControlsStory shown={ALL} state={{ cueKinds: ["subtitles"], captions: "subtitles" }} hover />
    ),
  },
];

export default stories;

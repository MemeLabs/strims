// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import clsx from "clsx";
import React, { useEffect, useState } from "react";

import { VideoControls, VideoState } from "../../hooks/useVideo";
import Slider from "./Slider";

interface VideoProgressBarProps {
  videoState: VideoState;
  videoControls: VideoControls;
}

const VideoProgressBar: React.FC<VideoProgressBarProps> = ({ videoState, videoControls }) => {
  const { playing, bufferStart, bufferEnd, currentTime } = videoState;

  const { pause, play, setCurrentTime } = videoControls;

  const [dragging, setDragging] = useState(false);
  const [wasPlaying, setWasPlaying] = useState(false);
  const [value, setValue] = useState(0);
  const [domainStart, setDomainStart] = useState(0);
  const [domainEnd, setDomainEnd] = useState(1);

  useEffect(() => {
    if (!dragging) {
      setValue(currentTime);
    }
  }, [dragging, currentTime]);

  // TODO: domain end from bitrate and last announced chunk?
  useEffect(() => {
    setDomainStart(bufferStart);
    setDomainEnd(bufferEnd);
  }, [bufferStart, bufferEnd]);

  const sliderClassNames = clsx({
    video_progress_bar__slider: true,
    dragging,
  });

  const clampValue = (value: number) => Math.min(bufferEnd, value);

  const handleUpdate = (newValue: number) => {
    const clampedValue = clampValue(newValue);
    if (dragging && clampedValue !== value) {
      setCurrentTime(clampedValue);
      setValue(clampedValue);
    }
  };

  const handleSlideStart = () => {
    setDragging(true);
    setWasPlaying(playing);
    pause();
  };

  const handleSlideEnd = () => {
    setDragging(false);

    if (wasPlaying) {
      play();
    }
  };

  const domainWidth = domainEnd - domainStart;
  const bufferRailStart = ((bufferStart - domainStart) / domainWidth) * 100;
  const bufferRailWidth = ((bufferEnd - bufferStart) / domainWidth) * 100;
  const bufferStyle = {
    left: `${bufferRailStart}%`,
    width: `${bufferRailWidth}%`,
  };

  return (
    <Slider
      block="video_progress_bar"
      className={sliderClassNames}
      domain={[domainStart, domainEnd]}
      step={0.01}
      value={value}
      onUpdate={handleUpdate}
      onSlideStart={handleSlideStart}
      onSlideEnd={handleSlideEnd}
    >
      <div className="video_progress_bar__rail__buffer" style={bufferStyle} />
    </Slider>
  );
};

export default VideoProgressBar;

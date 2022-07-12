// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import clsx from "clsx";
import React, { useEffect, useState } from "react";
import {
  GetHandleProps,
  GetTrackProps,
  Handles,
  Rail,
  Slider,
  SliderItem,
  Tracks,
} from "react-compound-slider";

import { VideoControls, VideoState } from "../../hooks/useVideo";

interface HandleProps {
  domain: [number, number];
  handle: SliderItem;
  getHandleProps: GetHandleProps;
}

export const Handle: React.FC<HandleProps> = ({
  domain: [min, max],
  handle: { id, value, percent },
  getHandleProps,
}) => (
  <div
    role="slider"
    aria-valuemin={min}
    aria-valuemax={max}
    aria-valuenow={value}
    style={{ left: `${percent}%` }}
    className="video_progress_bar__handle"
    {...getHandleProps(id)}
  />
);

interface TrackProps {
  source: SliderItem;
  target: SliderItem;
  getTrackProps: GetTrackProps;
}

export const Track: React.FC<TrackProps> = ({ source, target, getTrackProps }) => (
  <div
    className="video_progress_bar__track"
    style={{
      left: `${source.percent}%`,
      width: `${target.percent - source.percent}%`,
    }}
    {...getTrackProps()}
  />
);

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

  const handleUpdate = ([newValue]: number[]) => {
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
      mode={1}
      step={0.01}
      className={sliderClassNames}
      domain={[domainStart, domainEnd]}
      onUpdate={handleUpdate}
      onSlideStart={handleSlideStart}
      onSlideEnd={handleSlideEnd}
      values={[value]}
    >
      <Rail>
        {({ getRailProps }) => (
          <div className="video_progress_bar__rail" {...getRailProps()}>
            <div className="video_progress_bar__rail__buffer" style={bufferStyle} />
          </div>
        )}
      </Rail>
      <Handles>
        {({ handles, getHandleProps }) => (
          <div>
            {handles.map((handle) => (
              <Handle
                key={handle.id}
                handle={handle}
                domain={[0, 1]}
                getHandleProps={getHandleProps}
              />
            ))}
          </div>
        )}
      </Handles>
      <Tracks right={false}>
        {({ tracks, getTrackProps }) => (
          <div>
            {tracks.map(({ id, source, target }) => (
              <Track key={id} source={source} target={target} getTrackProps={getTrackProps} />
            ))}
          </div>
        )}
      </Tracks>
    </Slider>
  );
};

export default VideoProgressBar;

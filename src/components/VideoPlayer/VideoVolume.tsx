// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import clsx from "clsx";
import React, { useCallback, useRef, useState } from "react";
import {
  GetHandleProps,
  GetTrackProps,
  Handles,
  Rail,
  Slider,
  SliderItem,
  Tracks,
} from "react-compound-slider";
import { useUpdateEffect } from "react-use";

import useIdleTimeout from "../../hooks/useIdleTimeout";
import useReady from "../../hooks/useReady";

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
    className="video_volume__handle"
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
    className="video_volume__track"
    style={{
      left: `${source.percent}%`,
      width: `${target.percent - source.percent}%`,
    }}
    {...getTrackProps()}
  />
);

interface VolumeProps {
  value: number;
  onUpdate: (value: number) => void;
  onSlideStart: () => void;
  onSlideEnd: () => void;
  stepSize?: number;
}

const VideoVolume: React.FC<VolumeProps> = ({
  value,
  onUpdate,
  onSlideStart,
  onSlideEnd,
  stepSize = 0.1,
}) => {
  const [dragging, setDragging] = useState(false);
  const [idle, renewIdleTimeout] = useIdleTimeout();

  useUpdateEffect(renewIdleTimeout, [value]);

  const sliderClassNames = clsx({
    video_volume__slider: true,
    dragging,
    active: !idle,
  });

  const handleUpdate = useCallback(
    (values: number[]) => {
      if (dragging) {
        onUpdate(values[0]);
      }
    },
    [dragging]
  );

  const handleSlideStart = useCallback(() => {
    onSlideStart();
    setDragging(true);
  }, []);

  const handleSlideEnd = useCallback(() => {
    onSlideEnd();
    setDragging(false);
  }, []);

  // state dispatchers invoked with a callback from outside react event handler
  // call stacks throw errors so we have to smuggle the current value into the
  // event handler in a reference.
  const valueRef = useRef<number>();
  valueRef.current = value;

  const ref = useRef<HTMLDivElement>();
  useReady(() => {
    const handleWheel = (e: WheelEvent) => {
      e.preventDefault();
      e.stopPropagation();

      const direction = e.deltaY < 0 ? 1 : -1;
      onUpdate(valueRef.current + direction * stepSize);
    };
    ref.current.addEventListener("wheel", handleWheel, { capture: true, passive: false });
  }, [ref.current]);

  return (
    <div ref={ref}>
      <Slider
        mode={1}
        step={0.01}
        className={sliderClassNames}
        domain={[0, 1]}
        onUpdate={handleUpdate}
        onSlideStart={handleSlideStart}
        onSlideEnd={handleSlideEnd}
        values={[value]}
      >
        <Rail>
          {({ getRailProps }) => <div className="video_volume__rail" {...getRailProps()} />}
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
    </div>
  );
};

export default VideoVolume;

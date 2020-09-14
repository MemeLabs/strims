import clsx from "clsx";
import React, { FunctionComponent, useState } from "react";
import {
  GetHandleProps,
  GetTrackProps,
  Handles,
  Rail,
  Slider,
  SliderItem,
  Tracks,
} from "react-compound-slider";

import useIdleTimeout from "../../hooks/useIdleTimeout";
import useUpdates from "../../hooks/useUpdates";

interface HandleProps {
  domain: [number, number];
  handle: SliderItem;
  getHandleProps: GetHandleProps;
}

export const Handle: FunctionComponent<HandleProps> = ({
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

export const Track: FunctionComponent<TrackProps> = ({ source, target, getTrackProps }) => (
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
}

const VideoVolume: FunctionComponent<VolumeProps> = ({
  value,
  onUpdate,
  onSlideStart,
  onSlideEnd,
}) => {
  const [dragging, setDragging] = useState(false);
  const [idle, renewIdleTimeout] = useIdleTimeout();

  useUpdates(renewIdleTimeout, [value]);

  const sliderClassNames = clsx({
    video_volume__slider: true,
    dragging,
    active: !idle,
  });

  const handleUpdate = React.useCallback(
    (values: number[]) => {
      if (dragging) {
        onUpdate(values[0]);
      }
    },
    [dragging]
  );

  const handleSlideStart = React.useCallback(() => {
    onSlideStart();
    setDragging(true);
  }, []);

  const handleSlideEnd = React.useCallback(() => {
    onSlideEnd();
    setDragging(false);
  }, []);

  return (
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
  );
};

export default VideoVolume;

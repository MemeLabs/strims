// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import clsx from "clsx";
import React, { useCallback, useRef, useState } from "react";
import { useUpdateEffect } from "react-use";

import useIdleTimeout from "../../hooks/useIdleTimeout";
import useReady from "../../hooks/useReady";
import Slider from "./Slider";

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
        block="video_volume"
        className={sliderClassNames}
        domain={[0, 1]}
        step={0.01}
        value={value}
        onUpdate={onUpdate}
        onSlideStart={handleSlideStart}
        onSlideEnd={handleSlideEnd}
      />
    </div>
  );
};

export default VideoVolume;

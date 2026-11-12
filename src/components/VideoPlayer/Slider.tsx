// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { ReactNode, useEffect, useRef } from "react";

interface SliderProps {
  /** The block of the rail's, track's and handle's classes: BLOCK__rail and so on. */
  block: string;
  className?: string;
  domain: [number, number];
  step: number;
  value: number;
  onUpdate: (value: number) => void;
  onSlideStart?: () => void;
  onSlideEnd?: () => void;
  /** Drawn inside the rail. */
  children?: ReactNode;
}

// The value on the slider's steps, within its domain.
const onStep = (value: number, [min, max]: [number, number], step: number) => {
  const p = (Math.min(Math.max(value, min), max) - min) / (max - min);
  return Math.min(Math.max(step * Math.round((p * (max - min)) / step) + min, min), max);
};

// A rail, a track from its start to the value and a handle at the value,
// under a transparent range input that takes the pointer, touch and keys,
// its thumb where the handle is drawn.
const Slider: React.FC<SliderProps> = ({
  block,
  className,
  domain,
  step,
  value,
  onUpdate,
  onSlideStart,
  onSlideEnd,
  children,
}) => {
  const [min, max] = domain;
  const percent = 100 * ((onStep(value, domain, step) - min) / (max - min));

  // A slide ends wherever the pointer is let go.
  const unlisten = useRef<() => void>(null);
  useEffect(() => () => unlisten.current?.(), []);
  const handlePointerDown = () => {
    if (unlisten.current) {
      return;
    }
    onSlideStart?.();
    const end = () => {
      unlisten.current();
      onSlideEnd?.();
    };
    unlisten.current = () => {
      window.removeEventListener("pointerup", end);
      window.removeEventListener("pointercancel", end);
      unlisten.current = null;
    };
    window.addEventListener("pointerup", end);
    window.addEventListener("pointercancel", end);
  };

  return (
    <div className={className}>
      <div className={`${block}__rail`}>{children}</div>
      <div className={`${block}__track`} style={{ left: "0%", width: `${percent}%` }} />
      <div className={`${block}__handle`} style={{ left: `${percent}%` }} />
      <input
        type="range"
        className="video_slider__input"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onUpdate(e.target.valueAsNumber)}
        onPointerDown={handlePointerDown}
      />
    </div>
  );
};

export default Slider;

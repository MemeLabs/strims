// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./UserPresenceIndicator.scss";

import clsx from "clsx";
import React from "react";

import { Message, UIConfig } from "../../apis/strims/chat/v1/chat";
import { getDirectoryRefColor } from "../../lib/chat";
import { createListingRng } from "../../lib/directory";

const imageClassName = (style: string) =>
  clsx("chat__viewer_state_indicator", `chat__viewer_state_indicator--${style}`);

export interface UserPresenceIndicatorProps {
  style: UIConfig.UserPresenceIndicator;
  directoryRef: Message.DirectoryRef;
}

export const UserPresenceIndicator: React.FC<UserPresenceIndicatorProps> = ({
  style,
  directoryRef,
}) => {
  switch (style) {
    case UIConfig.UserPresenceIndicator.USER_PRESENCE_INDICATOR_DISABLED:
      return null;
    case UIConfig.UserPresenceIndicator.USER_PRESENCE_INDICATOR_BAR:
      return <BarImage directoryRef={directoryRef} />;
    case UIConfig.UserPresenceIndicator.USER_PRESENCE_INDICATOR_DOT:
      return <DotImage directoryRef={directoryRef} />;
    case UIConfig.UserPresenceIndicator.USER_PRESENCE_INDICATOR_ARRAY:
      return <ArrayImage directoryRef={directoryRef} />;
  }
};

interface ImageProps {
  directoryRef: Message.DirectoryRef;
}

const BarImage: React.FC<ImageProps> = ({ directoryRef }) => (
  <svg className={imageClassName("bar")} viewBox="0 0 15 100" xmlns="http://www.w3.org/2000/svg">
    <rect width="15" height="100" style={{ "--fill-color": getDirectoryRefColor(directoryRef) }} />
  </svg>
);

const DotImage: React.FC<ImageProps> = ({ directoryRef }) => (
  <svg className={imageClassName("dot")} viewBox="0 0 85 100" xmlns="http://www.w3.org/2000/svg">
    <circle cx="50" cy="50" r="30" style={{ "--fill-color": getDirectoryRefColor(directoryRef) }} />
  </svg>
);

interface ArrayImageProps extends ImageProps {
  cols?: number;
  rows?: number;
}

const ArrayImage: React.FC<ArrayImageProps> = ({ directoryRef, cols = 3, rows = 3 }) => {
  const color = getDirectoryRefColor(directoryRef);
  const rng = createListingRng(directoryRef?.listing);

  const size = 100;
  const steps = Math.max(cols, rows);
  const stepSize = size / steps;

  const circles = [];
  for (let p = 0.5, done = false; !done; p *= 1.1) {
    for (let x = 0; x < cols; x++) {
      for (let y = 0; y < rows; y++) {
        const fill = rng() < p;
        done = done || fill;

        circles.push(
          <circle
            key={`${x}:${y}`}
            cx={x * stepSize + stepSize / 2}
            cy={y * stepSize + stepSize / 2}
            r={stepSize * 0.35}
            style={{ "--fill-color": fill ? color : "" }}
          />
        );
      }
    }
  }

  return (
    <svg
      className={imageClassName("array")}
      viewBox="0 0 100 100"
      xmlns="http://www.w3.org/2000/svg"
    >
      {circles}
    </svg>
  );
};

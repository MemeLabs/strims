// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./ViewerStateIndicator.scss";

import clsx from "clsx";
import React, { useMemo } from "react";

import { UIConfig } from "../../apis/strims/chat/v1/chat";
import { Listing } from "../../apis/strims/network/v1/directory/directory";
import { DirectoryListing, useDirectoryUser } from "../../contexts/Directory";

const fnv1a = (input: string) => {
  let hash = 2166136261;
  for (let i = 0, len = input.length; i < len; i++) {
    hash ^= input.charCodeAt(i);
    hash += (hash << 1) + (hash << 4) + (hash << 7) + (hash << 8) + (hash << 24);
  }
  return hash >>> 0;
};

const createRng = (seed: string) => {
  let n = 0;
  return () => fnv1a(`${n++}${seed}`) / 0xffffffff;
};

const generateColor = (rng: () => number) => {
  const h = Math.round(rng() * 360);
  const s = Math.round(rng() * 80 + 20);
  const l = Math.round(rng() * 50 + 20);
  return `hsl(${h}, ${s}%, ${l}%)`;
};

const imageClassName = (style: string) =>
  clsx("chat__viewer_state_indicator", `chat__viewer_state_indicator--${style}`);

const createListingRng = (listing: Listing) => {
  switch (listing?.content.case) {
    case Listing.ContentCase.MEDIA: {
      let seed: string;
      try {
        const url = new URL(listing.content.media.swarmUri);
        seed = url.searchParams.get("xt").split(":").pop();
      } catch {
        seed = listing.content.media.swarmUri;
      }
      return createRng(seed);
    }
    case Listing.ContentCase.EMBED: {
      const { id, service } = listing.content.embed;
      return createRng(`${id}:${service}`);
    }
    default:
      return createRng("default");
  }
};

const getListingColor = (listing: DirectoryListing) =>
  listing
    ? listing.snippet.themeColor
      ? `#${listing.snippet.themeColor.toString(16)}`
      : generateColor(createListingRng(listing.listing))
    : "";

export interface ViewerStateIndicatorProps {
  style: UIConfig.ViewerStateIndicator;
  peerKey: Uint8Array;
}

export const ViewerStateIndicator: React.FC<ViewerStateIndicatorProps> = ({ style, peerKey }) => {
  const user = useDirectoryUser(peerKey);
  const listing = useMemo(() => {
    for (const { listings } of user) {
      for (const listing of listings) {
        switch (listing.listing.content.case) {
          case Listing.ContentCase.EMBED:
          case Listing.ContentCase.MEDIA:
            return listing;
        }
      }
    }
  }, [user]);

  switch (style) {
    case UIConfig.ViewerStateIndicator.VIEWER_STATE_INDICATOR_DISABLED:
      return null;
    case UIConfig.ViewerStateIndicator.VIEWER_STATE_INDICATOR_BAR:
      return <BarImage listing={listing} />;
    case UIConfig.ViewerStateIndicator.VIEWER_STATE_INDICATOR_DOT:
      return <DotImage listing={listing} />;
    case UIConfig.ViewerStateIndicator.VIEWER_STATE_INDICATOR_ARRAY:
      return <ArrayImage listing={listing} />;
  }
};

interface ImageProps {
  listing: DirectoryListing;
}

const BarImage: React.FC<ImageProps> = ({ listing }) => (
  <svg className={imageClassName("bar")} viewBox="0 0 15 100" xmlns="http://www.w3.org/2000/svg">
    <rect width="15" height="100" style={{ "--fill-color": getListingColor(listing) }} />
  </svg>
);

const DotImage: React.FC<ImageProps> = ({ listing }) => (
  <svg className={imageClassName("dot")} viewBox="0 0 85 100" xmlns="http://www.w3.org/2000/svg">
    <circle cx="50" cy="50" r="30" style={{ "--fill-color": getListingColor(listing) }} />
  </svg>
);

interface ArrayImageProps extends ImageProps {
  cols?: number;
  rows?: number;
}

const ArrayImage: React.FC<ArrayImageProps> = ({ listing, cols = 3, rows = 3 }) => {
  const color = getListingColor(listing);
  const rng = createListingRng(listing?.listing);

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

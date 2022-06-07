// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Grid.scss";

import clsx from "clsx";
import React, { useCallback, useEffect, useState } from "react";

import monkey from "../../../assets/directory/monkey.png";
import { Listing, ListingSnippet } from "../../apis/strims/network/v1/directory/directory";
import SnippetImage from "../../components/Directory/SnippetImage";
import { DirectoryListing } from "../../contexts/Directory";
import { useLayout } from "../../contexts/Layout";
import { useOpenListing } from "../../hooks/directory";
import { formatNumber } from "../../lib/number";

interface DirectoryGridItemProps extends DirectoryListing {
  networkKey: string;
}

const EMPTY_SNIPPET = new ListingSnippet();

const DirectoryGridItem: React.FC<DirectoryGridItemProps> = ({
  listing,
  snippet,
  viewerCount,
  networkKey,
}) => {
  const layout = useLayout();

  // on mobile while the directory grid is obstructed by the content panel we
  // don't need to apply snippet changes. this prevents loading thumbnail and
  // channel images but preserves the scroll position.
  const willHide =
    (layout.overlayState.open && !layout.overlayState.transitioning) || layout.modalOpen;
  const [hide, setHide] = useState(willHide);
  useEffect(() => {
    const tid = setTimeout(() => setHide(willHide), 200);
    return () => clearTimeout(tid);
  }, [willHide]);

  if (willHide && hide) {
    snippet = EMPTY_SNIPPET;
  }

  const openListing = useOpenListing();
  const handleClick = useCallback(() => openListing(networkKey, listing), [networkKey, listing]);

  const title = snippet.title.trim();

  return (
    <div
      className={clsx({
        "directory_grid__item": true,
      })}
    >
      <button className="directory_grid__item__link" onClick={handleClick}>
        <SnippetImage
          className="directory_grid__item__thumbnail"
          fallback={monkey}
          source={snippet.thumbnail}
        />
        <span className="directory_grid__item__viewer_count">
          {formatNumber(Number(snippet.viewerCount))}{" "}
          {snippet.viewerCount === BigInt(1) ? "viewer" : "viewers"}
        </span>
      </button>
      <div className="directory_grid__item__channel">
        <SnippetImage
          className="directory_grid__item__channel__logo"
          fallback={monkey}
          source={snippet.channelLogo}
        />
        <div className="directory_grid__item__channel__label">
          {title && (
            <span className="directory_grid__item__channel__title" title={title}>
              {title}
            </span>
          )}
          {snippet.channelName && (
            <span className="directory_grid__item__channel__name">{snippet.channelName}</span>
          )}
        </div>
      </div>
    </div>
  );
};

export interface DirectoryGridProps {
  listings: DirectoryListing[];
  networkKey: string;
}

export const DirectoryGrid: React.FC<DirectoryGridProps> = ({ listings, networkKey }) => (
  <div className="directory_grid">
    {listings
      .filter(
        ({ listing }) =>
          listing?.content?.case === Listing.ContentCase.EMBED ||
          listing?.content?.case === Listing.ContentCase.MEDIA
      )
      .map((listing) => (
        <DirectoryGridItem key={listing.id.toString()} networkKey={networkKey} {...listing} />
      ))}
  </div>
);

export default DirectoryGrid;

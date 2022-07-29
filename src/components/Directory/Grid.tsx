// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Grid.scss";

import clsx from "clsx";
import { Base64 } from "js-base64";
import React, { useCallback, useEffect, useState } from "react";
import { HiOutlineUser } from "react-icons/hi";

import monkey from "../../../assets/directory/monkey.png";
import { Listing, ListingSnippet, Network } from "../../apis/strims/network/v1/directory/directory";
import SnippetImage from "../../components/Directory/SnippetImage";
import { useLayout } from "../../contexts/Layout";
import { useOpenListing } from "../../hooks/directory";
import { formatNumber } from "../../lib/number";

type DirectoryGridItemProps = DirectoryListing;

const EMPTY_SNIPPET = new ListingSnippet();

const DirectoryGridItem: React.FC<DirectoryGridItemProps> = ({
  listing,
  snippet,
  userCount,
  recentUserCount,
  network,
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
  const handleClick = useCallback(
    () => openListing(Base64.fromUint8Array(network.key, true), listing),
    [network, listing]
  );

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
        {snippet.userCount > 0 && (
          <span className="directory_grid__item__viewer_count">
            {formatNumber(Number(snippet.userCount))}{" "}
            {snippet.userCount === BigInt(1) ? "viewer" : "viewers"}
          </span>
        )}
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
        <span className="directory_grid__item__channel__user_count">
          {formatNumber(Number(userCount))}
          <HiOutlineUser />
        </span>
      </div>
    </div>
  );
};

export interface DirectoryListing {
  id: bigint;
  listing: Listing;
  snippet: ListingSnippet;
  userCount: number;
  recentUserCount: number;
  network: Network;
}

export interface DirectoryGridProps {
  listings: DirectoryListing[];
}

export const DirectoryGrid: React.FC<DirectoryGridProps> = ({ listings }) => (
  <div className="directory_grid">
    {listings
      .filter(
        ({ listing }) =>
          listing?.content?.case === Listing.ContentCase.EMBED ||
          listing?.content?.case === Listing.ContentCase.MEDIA
      )
      .map((listing) => (
        <DirectoryGridItem key={listing.id.toString()} {...listing} />
      ))}
  </div>
);

export default DirectoryGrid;

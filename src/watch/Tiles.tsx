// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import qs from "qs";
import React, { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { useTitle } from "react-use";

import { Listing, ListingSnippet } from "../apis/strims/network/v1/directory/directory";
import { DirectoryListing, DirectoryTile } from "../components/Directory/Grid";
import { NetworkListingsMap } from "../lib/listings";
import Status from "./Status";
import { useBootstrapListings } from "./listings";

const EMPTY_SNIPPET = new ListingSnippet();

/** The watch page's path to a media listing's player. */
export const playerPath = (networkKey: Uint8Array, media: Listing.Media): string =>
  `/${Base64.fromUint8Array(networkKey, true)}?${qs.stringify({
    mimeType: media.mimeType,
    swarmUri: media.swarmUri,
  })}`;

/** The media listings, most watched first. */
export const mediaTiles = (networkListings: NetworkListingsMap): DirectoryListing[] => {
  const tiles: DirectoryListing[] = [];
  for (const { network, listings } of networkListings.values()) {
    for (const l of listings.values()) {
      if (l.listing?.content?.case !== Listing.ContentCase.MEDIA) continue;
      tiles.push({
        id: l.id,
        listing: l.listing,
        snippet: l.snippet ?? EMPTY_SNIPPET,
        userCount: l.userCount,
        recentUserCount: l.recentUserCount,
        network,
      });
    }
  }
  return tiles.sort((a, b) => b.userCount - a.userCount || Number(a.id - b.id));
};

const Tiles: React.FC = () => {
  const { t } = useTranslation();
  useTitle(t("watch.title"));
  const navigate = useNavigate();
  const { networkListings, connected, error } = useBootstrapListings();
  const tiles = useMemo(() => mediaTiles(networkListings), [networkListings]);

  if (tiles.length === 0) {
    if (connected) return <Status message={t("watch.noStreams")} />;
    if (error) return <Status message={t("watch.directoryUnreachable")} />;
    return <Status />;
  }

  return (
    <div className="watch_tiles">
      <div className="directory_grid">
        {tiles.map((tile) => (
          <DirectoryTile
            key={`${tile.network.id}:${tile.id}`}
            {...tile}
            onOpen={() =>
              tile.listing.content.case === Listing.ContentCase.MEDIA &&
              navigate(playerPath(tile.network.key, tile.listing.content.media))
            }
          />
        ))}
      </div>
    </div>
  );
};

export default Tiles;

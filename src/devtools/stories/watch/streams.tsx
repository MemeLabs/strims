// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The watch page's own stylesheet, which its entry (watch/index.tsx) loads.
import "../../../watch/watch.scss";

import React, { ReactNode } from "react";

import { Listing, NetworkListingsItem } from "../../../apis/strims/network/v1/directory/directory";
import { NetworkListingsMap } from "../../../lib/listings";
import { fixtures } from "../../storybook";

const hex = (b: Uint8Array): string =>
  Array.from(b, (x) => x.toString(16).padStart(2, "0")).join("");

// Swarm stream `i`'s query, as a media listing names it.
export const swarmStream = (i: number): { swarmUri: string; mimeType: string } => ({
  swarmUri: `magnet:?xt=urn:ppspp:${hex(fixtures.bytes(`swarm:${i}`, 20))}&x.cs=4096`,
  mimeType: 'video/x.strims-units;codecs="avc1.640028,mp4a.40.2"',
});

// The directory mock's embeds (thumbnails, titles, channels) as media
// listings, the first `count`, spread over networks 1..networks in turn.
export const mediaListings = (count: number, networks = 2): NetworkListingsMap => {
  const map: NetworkListingsMap = new Map();
  for (let n = 1; n <= networks; n++) {
    const network = fixtures.directoryNetwork(fixtures.network(n));
    map.set(network.id, { network, listings: new Map() });
  }
  fixtures.listingItems
    .filter((e) => e.listing?.content?.case === Listing.ContentCase.EMBED)
    .slice(0, count)
    .forEach((e, i) => {
      map.get(BigInt((i % networks) + 1)).listings.set(
        e.id,
        new NetworkListingsItem({
          ...e,
          listing: { content: { media: swarmStream(i) } },
          userCount: (i * 7) % 50,
          recentUserCount: i % 3,
        })
      );
    });
  return map;
};

// The watch page's root, as watch/App renders it: dark whatever the
// storybook's theme, as the page always is.
export const WatchRoot: React.FC<{ children: ReactNode }> = ({ children }) => (
  <div className="watch theme theme--dark">{children}</div>
);

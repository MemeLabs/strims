// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import {
  INetworkListingsItem,
  Listing,
  ListingSnippet,
  Network as DirectoryNetwork,
} from "../../../apis/strims/network/v1/directory/directory";
import type { DirectoryListing } from "../../../components/Directory/Grid";
import events from "../../mocks/directory/events";
import { directoryNetwork, network } from "./networks";

/** The directory mock's listings (chat rooms, embeds and streams with thumbnails), as INetworkListingsItem. */
export const listingItems: INetworkListingsItem[] = events;

// Network 1 as directory messages name it: the network the listings are in.
export const listingsNetwork = (): DirectoryNetwork => directoryNetwork(network(1));

// The listings as the directory Grid takes them.
export const directoryListings = (n: DirectoryNetwork = listingsNetwork()): DirectoryListing[] =>
  events.map((e, i) => ({
    id: e.id,
    listing: new Listing(e.listing),
    snippet: new ListingSnippet(e.snippet),
    userCount: Number(e.snippet?.userCount ?? 0),
    recentUserCount: i % 3,
    network: n,
  }));

// The listing with `id`, or undefined.
export const listingItem = (id: bigint): INetworkListingsItem | undefined =>
  events.find((e) => e.id === id);

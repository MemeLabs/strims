// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import * as directoryv1 from "../apis/strims/network/v1/directory/directory";

/** A directory watch's listings by network id, then listing id. */
export type NetworkListingsMap = Map<
  bigint,
  {
    listings: Map<bigint, directoryv1.NetworkListingsItem>;
    network: directoryv1.Network;
  }
>;

const { EventCase } = directoryv1.FrontendWatchListingsResponse.Event;

/**
 * The listings after a watch's `events`, in order: a change adds or
 * replaces its network's listings, an unpublish drops one, and a user count
 * change updates a listing's count. `prev` is left as it was.
 */
export const foldListingEvents = (
  prev: NetworkListingsMap,
  events: directoryv1.FrontendWatchListingsResponse.Event[]
): NetworkListingsMap => {
  const networkListings = new Map(prev);
  for (const { event } of events) {
    switch (event.case) {
      case EventCase.CHANGE: {
        const prev = networkListings.get(event.change.listings.network.id);
        const next = {
          listings: new Map(prev?.listings),
          network: prev?.network ?? event.change.listings.network,
        };
        for (const listing of event.change.listings.listings) {
          next.listings.set(listing.id, listing);
        }
        networkListings.set(next.network.id, next);
        break;
      }

      case EventCase.UNPUBLISH: {
        const prev = networkListings.get(event.unpublish.networkId);
        if (prev) {
          const listings = new Map(prev.listings);
          listings.delete(event.unpublish.listingId);
          networkListings.set(event.unpublish.networkId, { ...prev, listings });
        }
        break;
      }

      case EventCase.USER_COUNT_CHANGE: {
        const prevNetworkListing = networkListings.get(event.userCountChange.networkId);
        const prevListing = prevNetworkListing?.listings.get(event.userCountChange.listingId);
        if (prevListing) {
          networkListings.set(event.userCountChange.networkId, {
            ...prevNetworkListing,
            listings: new Map(prevNetworkListing.listings).set(event.userCountChange.listingId, {
              ...prevListing,
              userCount: event.userCountChange.userCount,
            }),
          });
        }
        break;
      }
    }
  }
  return networkListings;
};

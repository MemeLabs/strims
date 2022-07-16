// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useCallback, useContext, useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router";

import * as directoryv1 from "../apis/strims/network/v1/directory/directory";
import { useClient } from "../contexts/FrontendApi";
import { useLayout } from "../contexts/Layout";
import { PlayerContext, PlayerMode } from "../contexts/Player";
import { formatUri, getListingPlayerSource } from "../lib/directory";
import { DEVICE_TYPE, DeviceType } from "../lib/userAgent";

export const useOpenListing = () => {
  const layout = useLayout();
  const player = useContext(PlayerContext);
  const navigate = useNavigate();

  return useCallback((networkKey: string, listing: directoryv1.Listing) => {
    layout.toggleOverlayOpen(true);
    layout.toggleShowVideo(true);
    player.setMode(PlayerMode.LARGE);
    player.setSource(getListingPlayerSource(networkKey, listing));

    if (DEVICE_TYPE !== DeviceType.Portable) {
      const path = formatUri(networkKey, listing);
      player.setPath(path);
      navigate(path);
    }
  }, []);
};

export type NetworkListingsMap = Map<
  bigint,
  {
    listings: Map<bigint, directoryv1.NetworkListingsItem>;
    network: directoryv1.Network;
  }
>;

export interface ListingValues {
  open: boolean;
  error: Error;
  networkListings: NetworkListingsMap;
}

const defaultListingValues: ListingValues = {
  open: false,
  error: null,
  networkListings: new Map(),
};

export const useListings = (args: directoryv1.IFrontendWatchListingsRequest) => {
  const client = useClient();

  const [values, setValues] = useState<ListingValues>(defaultListingValues);

  useEffect(() => {
    const events = client.directory.watchListings(args);
    events.on("data", ({ events }) =>
      setValues((prev) => {
        const networkListings = new Map(prev.networkListings);
        for (const { event } of events) {
          switch (event.case) {
            case directoryv1.FrontendWatchListingsResponse.Event.EventCase.CHANGE: {
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

            case directoryv1.FrontendWatchListingsResponse.Event.EventCase.UNPUBLISH: {
              const prev = networkListings.get(event.unpublish.networkId);
              if (prev) {
                const listings = new Map(prev.listings);
                listings.delete(event.unpublish.listingId);
                networkListings.set(event.unpublish.networkId, { ...prev, listings });
              }
              break;
            }

            case directoryv1.FrontendWatchListingsResponse.Event.EventCase.USER_COUNT_CHANGE: {
              const prevNetworkListing = networkListings.get(event.userCountChange.networkId);
              const prevListing = prevNetworkListing?.listings.get(event.userCountChange.listingId);
              if (prevListing) {
                networkListings.set(event.userCountChange.networkId, {
                  ...prevNetworkListing,
                  listings: new Map(prevNetworkListing.listings).set(
                    event.userCountChange.listingId,
                    {
                      ...prevListing,
                      userCount: event.userCountChange.userCount,
                    }
                  ),
                });
              }
              break;
            }
          }
        }
        return { ...prev, open: true, networkListings };
      })
    );
    events.on("error", (error) => setValues((prev) => ({ ...prev, error })));
    events.on("close", () => setValues((prev) => ({ ...prev, open: false })));
    return () => events.destroy();
  }, [args]);

  return values;
};

export const useListing = (networkKey: Uint8Array, listingId: bigint) => {
  const req = useMemo(() => {
    return {
      networkKeys: [networkKey],
      listingId,
    };
  }, [networkKey, listingId]);
  const { networkListings, ...res } = useListings(req);

  for (const { listings } of networkListings.values()) {
    for (const listing of listings.values()) {
      return { listing, ...res };
    }
  }
  return { listing: null, ...res };
};

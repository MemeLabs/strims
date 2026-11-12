// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useCallback, useContext, useMemo, useState } from "react";
import { useNavigate } from "react-router";

import * as directoryv1 from "../apis/strims/network/v1/directory/directory";
import { useClient } from "../contexts/FrontendApi";
import { useLayout } from "../contexts/Layout";
import { PlayerContext, PlayerMode } from "../contexts/Player";
import { formatUri, getListingPlayerSource } from "../lib/directory";
import { NetworkListingsMap, foldListingEvents } from "../lib/listings";
import { DEVICE_TYPE, DeviceType } from "../lib/userAgent";
import useReady from "./useReady";

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
      void navigate(path);
    }
  }, []);
};

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

  useReady(() => {
    setValues(defaultListingValues);
    const events = client.directory.watchListings(args);
    events.on("data", ({ events }) =>
      setValues((prev) => ({
        ...prev,
        open: true,
        networkListings: foldListingEvents(prev.networkListings, events),
      }))
    );
    events.on("error", (error) => setValues((prev) => ({ ...prev, error })));
    events.on("close", () => setValues((prev) => ({ ...prev, open: false })));
    return () => events.destroy();
  }, [args]);

  return values;
};

export const useListing = (networkKey: Uint8Array, listingId: bigint) => {
  const req = useMemo(() => {
    if (networkKey && listingId) {
      return {
        networkKeys: [networkKey],
        listingId,
      };
    }
  }, [networkKey, listingId]);
  const { networkListings, ...res } = useListings(req);

  for (const { listings } of networkListings.values()) {
    for (const listing of listings.values()) {
      return { listing, ...res };
    }
  }
  return { listing: null, ...res };
};

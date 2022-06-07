// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useCallback, useContext } from "react";
import { useNavigate } from "react-router";

import { Listing } from "../apis/strims/network/v1/directory/directory";
import { useLayout } from "../contexts/Layout";
import { PlayerContext, PlayerMode } from "../contexts/Player";
import { formatUri, getListingPlayerSource } from "../lib/directory";
import { DEVICE_TYPE, DeviceType } from "../lib/userAgent";

export const useOpenListing = () => {
  const layout = useLayout();
  const player = useContext(PlayerContext);
  const navigate = useNavigate();

  return useCallback((networkKey: string, listing: Listing) => {
    layout.toggleOverlayOpen(true);
    layout.toggleShowVideo(true);
    player.setMode(PlayerMode.FULL);
    player.setSource(getListingPlayerSource(networkKey, listing));

    if (DEVICE_TYPE !== DeviceType.Portable) {
      const path = formatUri(networkKey, listing);
      player.setPath(path);
      navigate(path);
    }
  }, []);
};

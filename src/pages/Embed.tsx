// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useContext, useEffect } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { useLocation, useParams } from "react-router-dom";

import { useLayout } from "../contexts/Layout";
import { PlayerContext, PlayerMode } from "../contexts/Player";
import useQuery from "../hooks/useQuery";
import { ServiceSlug } from "../lib/directory";

interface EmbedQueryParams {
  k: string;
  [key: string]: string;
  npm;
}

// TODO: merge with player page
const Embed: React.FC = () => {
  const routeParams = useParams<"service" | "id">();
  const location = useLocation();
  const { k: networkKey, ...queryParams } = useQuery<EmbedQueryParams>(location.search);
  const { toggleShowVideo, toggleOverlayOpen, toggleTheaterMode } = useLayout();

  useHotkeys("alt+t", () => toggleTheaterMode(), { enableOnContentEditable: true });

  const { setMode, setSource, setPath } = useContext(PlayerContext);
  useEffect(() => {
    toggleOverlayOpen(true);
    toggleShowVideo(true);
    setMode(PlayerMode.LARGE);
    setSource({
      type: "embed",
      service: routeParams.service as ServiceSlug,
      id: routeParams.id,
      queryParams: new Map(Object.entries(queryParams)),
      networkKey,
    });
    setPath(location.pathname + location.search);
    return () => {
      toggleTheaterMode(false);
      toggleOverlayOpen(false);
      setMode(PlayerMode.PIP);
    };
  }, [routeParams.service, routeParams.id, networkKey]);

  return null;
};

export default Embed;

// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import { isEqual } from "lodash";
import React, { createContext, useCallback, useEffect, useMemo, useState } from "react";

import { ServiceSlug, slugToService } from "../lib/directory";
import { useClient } from "./FrontendApi";

export const enum PlayerMode {
  FULL,
  LARGE,
  PIP,
  CLOSED,
}

export interface EmbedSource {
  type: "embed";
  service: ServiceSlug;
  id: string;
  queryParams?: Map<string, string>;
  networkKey?: string;
}

export interface SwarmSource {
  type: "swarm";
  swarmUri: string;
  networkKey: string;
  mimeType: string;
  listingId: bigint;
}

export type PlayerSource = EmbedSource | SwarmSource;

interface PlayerValue {
  path: string;
  setPath: (path: string) => void;
  source: PlayerSource;
  setSource: (source: PlayerSource) => void;
  mode: PlayerMode;
  setMode: (mode: PlayerMode) => void;
}

export const PlayerContext = createContext<PlayerValue>(null);

export const Provider: React.FC = ({ children }) => {
  const client = useClient();
  const [path, setPath] = useState<string>("");
  const [source, setSourceState] = useState<PlayerSource>(null);
  const [mode, setMode] = useState<PlayerMode>(PlayerMode.PIP);
  const [listingCleanup, setListingCleanup] = useState<() => void>();

  const publishEmbedListing = ({
    networkKey: networkKeyString,
    service,
    id,
    queryParams,
  }: EmbedSource) => {
    const networkKey = Base64.toUint8Array(networkKeyString);
    const res = client.directory.publish({
      networkKey,
      listing: {
        content: {
          embed: {
            service: slugToService(service),
            id,
            queryParams,
          },
        },
      },
    });
    return () => void res.then(({ id }) => client.directory.unpublish({ networkKey, id }));
  };

  const joinSwarmListing = ({ networkKey: networkKeyString, listingId: id }: SwarmSource) => {
    const networkKey = Base64.toUint8Array(networkKeyString);
    const res = client.directory.join({ networkKey, id });
    return () => void res.then(() => client.directory.part({ networkKey, id }));
  };

  const updateListing = (source: PlayerSource) => {
    setListingCleanup(() => {
      if (!source?.networkKey) {
        return () => null;
      }
      switch (source.type) {
        case "embed":
          return publishEmbedListing(source);
        case "swarm":
          return joinSwarmListing(source);
      }
    });
  };
  useEffect(() => () => listingCleanup?.(), [listingCleanup]);

  const setSource = (next: PlayerSource) => {
    setSourceState((prev) => {
      if (isEqual(next, prev)) {
        return prev;
      }

      updateListing(next);
      return next;
    });
  };

  const value = useMemo<PlayerValue>(
    () => ({
      path,
      setPath,
      source,
      setSource,
      mode,
      setMode,
    }),
    [source, mode]
  );

  return <PlayerContext.Provider value={value}>{children}</PlayerContext.Provider>;
};

Provider.displayName = "Player.Provider";

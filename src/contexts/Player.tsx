import { Base64 } from "js-base64";
import { isEqual } from "lodash";
import React, { createContext, useCallback, useMemo, useState } from "react";

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
  const [, setListingCleanup] = useState<() => void>();

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

  const setSource = useCallback(
    (next: PlayerSource) => {
      if (isEqual(next, source)) {
        return;
      }

      setSourceState(next);
      setListingCleanup((prev) => {
        prev?.();

        if (!next?.networkKey) {
          return () => null;
        }
        switch (next.type) {
          case "embed":
            return publishEmbedListing(next);
          case "swarm":
            return joinSwarmListing(next);
        }
      });
    },
    [source]
  );

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

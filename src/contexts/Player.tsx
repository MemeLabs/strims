// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import { isEqual } from "lodash";
import React, { ReactNode, createContext, useEffect, useMemo, useState } from "react";

import { useStableCallbacks } from "../hooks/useStableCallback";
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
}

export type PlayerSource = EmbedSource | SwarmSource;

interface State {
  path: string;
  source: PlayerSource;
  mode: PlayerMode;
  networkKey: Uint8Array;
  listingId: bigint;
}

const initialState: State = {
  path: "",
  source: null,
  mode: PlayerMode.PIP,
  networkKey: null,
  listingId: null,
};

interface PlayerValue extends State {
  setPath: (path: string) => void;
  setSource: (source: PlayerSource) => void;
  setMode: (mode: PlayerMode) => void;
}

export const PlayerContext = createContext<PlayerValue>(null);

interface ProviderProps {
  children: ReactNode;
}

export const Provider: React.FC<ProviderProps> = ({ children }) => {
  const client = useClient();
  const [state, setState] = useState<State>(initialState);
  const [listingCleanup, setListingCleanup] = useState<() => void>();

  const setDirectoryState = (networkKey?: Uint8Array, directoryId?: bigint) =>
    setState((state) => ({ ...state, networkKey, listingId: directoryId }));

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

    void res.then(({ id }) => setDirectoryState(networkKey, id));

    return () =>
      void res.then(({ id }) => {
        setDirectoryState();
        void client.directory.unpublish({ networkKey, id });
      });
  };

  const joinSwarmListing = ({ networkKey: networkKeyString, swarmUri, mimeType }: SwarmSource) => {
    const networkKey = Base64.toUint8Array(networkKeyString);
    const res = client.directory.join({
      networkKey,
      query: { query: { listing: { content: { media: { swarmUri, mimeType } } } } },
    });

    void res.then(({ id }) => setDirectoryState(networkKey, id));

    return () => {
      setDirectoryState();
      void res.then(({ id }) => client.directory.part({ networkKey, id }));
    };
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

  const actions = useMemo(() => {
    const setPath = (path: string) => setState((state) => ({ ...state, path }));

    const setSource = (source: PlayerSource) =>
      setState((state) => {
        if (isEqual(source, state.source)) {
          return state;
        }

        updateListing(source);
        return { ...state, source };
      });

    const setMode = (mode: PlayerMode) => setState((state) => ({ ...state, mode }));

    return { setPath, setSource, setMode };
  }, []);

  const value = useMemo<PlayerValue>(() => ({ ...state, ...actions }), [state]);

  return <PlayerContext.Provider value={value}>{children}</PlayerContext.Provider>;
};

Provider.displayName = "Player.Provider";

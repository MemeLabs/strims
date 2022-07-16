// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import { isEqual } from "lodash";
import React, { ReactNode, createContext, useEffect, useMemo, useState } from "react";

import { FrontendClient } from "../apis/client";
import promiseWithCancel from "../lib/promiseWithCancel";
import curryDispatchActions from "../lib/curryDispatchActions";
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

interface PublicState {
  path: string;
  source: PlayerSource;
  mode: PlayerMode;
  networkKey: Uint8Array;
  listingId: bigint;
}

interface PrivateState {
  closeListing: () => void;
}

type State = PublicState & PrivateState;

const initialState: State = {
  path: "",
  source: null,
  mode: PlayerMode.PIP,
  networkKey: null,
  listingId: null,
  closeListing: null,
};

interface PlayerValue extends PublicState {
  setPath: (path: string) => void;
  setSource: (source: PlayerSource) => void;
  setMode: (mode: PlayerMode) => void;
}

export const PlayerContext = createContext<PlayerValue>(null);

const createActions = (
  client: FrontendClient,
  setState: React.Dispatch<React.SetStateAction<State>>
) => {
  const openListing = (source: PlayerSource) => {
    const networkKey = Base64.toUint8Array(source.networkKey);
    const res = (() => {
      switch (source.type) {
        case "embed": {
          const { service, id, queryParams } = source;
          return client.directory.publish({
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
        }
        case "swarm": {
          const { swarmUri, mimeType } = source;
          return client.directory.join({
            networkKey,
            query: { query: { listing: { content: { media: { swarmUri, mimeType } } } } },
          });
        }
      }
    })();

    const [cancelled, cancel] = promiseWithCancel();
    void Promise.race([res, cancelled]).then((res) => {
      setState((state) => ({ ...state, networkKey, listingId: res.id }));
    });

    return () => {
      cancel();
      setState((state) => ({ ...state, networkKey: null, listingId: null }));
      void res.then(({ id }) => client.directory.unpublish({ networkKey, id }));
    };
  };

  const setPath = (state: State, path: string) => ({ ...state, path });

  const setSource = (state: State, source: PlayerSource) => {
    if (isEqual(source, state.source)) {
      return state;
    }

    const closeListing = source?.networkKey ? openListing(source) : null;

    return {
      ...state,
      source,
      networkKey: null,
      listingId: null,
      closeListing,
    };
  };

  const setMode = (state: State, mode: PlayerMode) => ({ ...state, mode });

  return { setPath, setSource, setMode };
};

interface ProviderProps {
  children: ReactNode;
}

export const Provider: React.FC<ProviderProps> = ({ children }) => {
  const client = useClient();
  const [state, setState] = useState<State>(initialState);

  useEffect(() => () => state.closeListing?.(), [state.closeListing]);

  const actions = useMemo(
    () => curryDispatchActions(setState, createActions(client, setState)),
    [client]
  );

  const value = useMemo<PlayerValue>(() => ({ ...state, ...actions }), [state]);

  return <PlayerContext.Provider value={value}>{children}</PlayerContext.Provider>;
};

Provider.displayName = "Player.Provider";

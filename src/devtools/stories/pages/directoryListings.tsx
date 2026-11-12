// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The app's form styles, which the directory's test button uses and doesn't load.
import "../../../components/Form/Form.scss";
import "../../../components/Form/Button.scss";

import { PassThrough } from "stream";

import { isEqual } from "lodash";
import React, { ReactNode } from "react";
import { Route } from "react-router-dom";

import {
  Network as DirectoryNetwork,
  FrontendWatchListingsRequest,
  FrontendWatchListingsResponse,
  INetworkListingsItem,
} from "../../../apis/strims/network/v1/directory/directory";
import DirectoryService from "../../mocks/directory/service";
import { MockApi, StaticPlayer, StoryRouter, fixtures } from "../../storybook";

export interface NetworkListings {
  network: DirectoryNetwork;
  listings: INetworkListingsItem[];
}

// The directory mock's listings spread over networks 1..count, in turn.
export const listingsByNetwork = (count: number): NetworkListings[] =>
  Array.from({ length: count }, (_, i) => ({
    network: fixtures.directoryNetwork(fixtures.network(i + 1)),
    listings: fixtures.listingItems.filter((_, j) => j % count === i),
  }));

// A directory whose watch answers `byNetwork`'s listings of the networks the
// request names (all of them when it names none).
class ListingsService extends DirectoryService {
  constructor(private readonly byNetwork: NetworkListings[]) {
    super();
  }

  watchListings(req: FrontendWatchListingsRequest) {
    const ch = new PassThrough({ objectMode: true });
    const named = this.byNetwork.filter(
      ({ network }) =>
        !req.networkKeys.length || req.networkKeys.some((key) => isEqual(key, network.key))
    );
    ch.push(
      new FrontendWatchListingsResponse({
        events: named.map(({ network, listings }) => ({
          event: {
            change: {
              listings: {
                network,
                listings: listings.map((item, i) => ({
                  ...item,
                  moderation: {},
                  userCount: (i * 7) % 50,
                  recentUserCount: i % 3,
                })),
              },
            },
          },
        })),
      })
    );
    return ch;
  }
}

export interface DirectoryContextProps {
  byNetwork: NetworkListings[];
  /** Where the story's router starts, and the route it renders `children` at. */
  initialPath: string;
  path: string;
  children: ReactNode;
}

// A directory page's providers: opening a listing sets the static player's
// source (a no-op) and navigates in the story.
export const DirectoryContext: React.FC<DirectoryContextProps> = ({
  byNetwork,
  initialPath,
  path,
  children,
}) => (
  <MockApi services={() => ({ directory: new ListingsService(byNetwork) })}>
    <StaticPlayer>
      <StoryRouter initialPath={initialPath}>
        <Route path={path} element={children} />
      </StoryRouter>
    </StaticPlayer>
  </MockApi>
);

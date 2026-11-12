// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { Route } from "react-router";

import { NetworkListingsItem } from "../../../apis/strims/network/v1/directory/directory";
import { NetworkListingsMap } from "../../../lib/listings";
import Tiles from "../../../watch/Tiles";
import { BootstrapListings, BootstrapListingsContext } from "../../../watch/listings";
import { Catalog, Story, StoryRouter } from "../../storybook";
import { WatchRoot, mediaListings } from "./streams";

// The tiles at the watch page's index over `listings`, in place of the
// node's stream; opening one navigates in the story.
const TilesStory: React.FC<Partial<BootstrapListings>> = (listings) => (
  <BootstrapListingsContext.Provider
    value={{ networkListings: new Map(), connected: true, error: null, ...listings }}
  >
    <WatchRoot>
      <StoryRouter>
        <Route path="/" element={<Tiles />} />
      </StoryRouter>
    </WatchRoot>
  </BootstrapListingsContext.Provider>
);

// Media listings without snippets: the tiles fall back to placeholder images.
const withoutSnippets = (listings: NetworkListingsMap): NetworkListingsMap =>
  new Map(
    Array.from(listings, ([id, { network, listings }]) => [
      id,
      {
        network,
        listings: new Map(
          Array.from(listings, ([id, l]) => [
            id,
            new NetworkListingsItem({ ...l, snippet: undefined }),
          ])
        ),
      },
    ])
  );

const stories: Story[] = [
  {
    // Twelve streams in two networks, most watched first.
    name: "Default",
    component: () => <TilesStory networkListings={mediaListings(12)} />,
  },
  {
    name: "Many",
    component: () => <TilesStory networkListings={mediaListings(Infinity, 3)} />,
  },
  {
    name: "No snippets",
    component: () => <TilesStory networkListings={withoutSnippets(mediaListings(4))} />,
  },
  {
    // No stream open yet: a spinner.
    name: "Connecting",
    component: () => <TilesStory connected={false} />,
  },
  {
    name: "Empty",
    component: () => <TilesStory />,
  },
  {
    // The stream failed before it gave any listings.
    name: "Error",
    component: () => (
      <TilesStory connected={false} error={new Error("directory events answered 502")} />
    ),
  },
];

export default stories;

export const catalog: Catalog = {
  covers: ["watch/listings.tsx"],
  skips: {
    "watch/App.tsx":
      "the watch page's root: a BrowserRouter of its own (basename /watch) and the real Session " +
      "provider, which dials the node in the page; its routes are storied as watch/Tiles and watch/Player",
  },
};

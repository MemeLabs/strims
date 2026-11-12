// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Directory.stories.scss";

import React, { ReactNode } from "react";
import { Route } from "react-router";

import {
  IListingSnippet,
  Listing,
  ListingSnippet,
} from "../../../../apis/strims/network/v1/directory/directory";
import DirectoryGrid, {
  DirectoryListing,
  DirectoryTile,
} from "../../../../components/Directory/Grid";
import { StaticPlayer, Story, StoryRouter, fixtures } from "../../../storybook";

// Opening a listing sets the player's source and navigates to it: both stay
// in the story.
const Context: React.FC<{ children: ReactNode }> = ({ children }) => (
  <StaticPlayer>
    <StoryRouter>
      <Route path="/" element={children} />
    </StoryRouter>
  </StaticPlayer>
);

// The listings the grid shows (embeds and media; not chat rooms).
const embedListings = (): DirectoryListing[] =>
  fixtures
    .directoryListings()
    .filter(({ listing }) => listing.content.case !== Listing.ContentCase.CHAT);

const withSnippet = (
  listings: DirectoryListing[],
  snippet: (s: ListingSnippet, i: number) => IListingSnippet
): DirectoryListing[] =>
  listings.map((l, i) => ({
    ...l,
    snippet: new ListingSnippet({ ...l.snippet, ...snippet(l.snippet, i) }),
  }));

const LONG_TITLE =
  "🔴 24 HOUR CHARITY MARATHON — every donation goal unlocks another round of the worst speedrun categories ever devised, now with extra commentary and a very long title that should truncate";
const LONG_NAME = "a_channel_name_that_goes_on_and_on_without_any_spaces_at_all";

const Grid: React.FC<{ listings: DirectoryListing[] }> = ({ listings }) => (
  <Context>
    <DirectoryGrid listings={listings} />
  </Context>
);

const stories: Story[] = [
  {
    name: "Many",
    component: () => <Grid listings={fixtures.directoryListings()} />,
  },
  {
    name: "Few",
    component: () => <Grid listings={embedListings().slice(1, 4)} />,
  },
  {
    name: "Empty",
    component: () => <Grid listings={[]} />,
  },
  {
    name: "Long text",
    component: () => (
      <Grid
        listings={withSnippet(embedListings().slice(0, 6), (_, i) => ({
          title: i % 2 ? LONG_TITLE : "",
          channelName: i % 3 ? LONG_NAME : "short",
          userCount: BigInt(Math.pow(10, i + 3)),
        }))}
      />
    ),
  },
  {
    name: "Missing images",
    component: () => (
      <Grid
        listings={withSnippet(embedListings().slice(0, 6), (_, i) => ({
          thumbnail: i % 2 ? undefined : { sourceOneof: { url: "" } },
          channelLogo: i % 3 ? undefined : { sourceOneof: { url: "" } },
        }))}
      />
    ),
  },
  {
    name: "Viewer counts",
    component: () => (
      <Grid
        listings={withSnippet(embedListings().slice(1, 7), (_, i) => ({
          userCount: BigInt([0, 1, 2, 999, 12_345, 4_200_000][i]),
        })).map((l, i) => ({ ...l, userCount: [0, 1, 3, 950, 12_000, 4_100_000][i] }))}
      />
    ),
  },
  {
    name: "Tile",
    component: () => {
      const [listing] = embedListings().slice(2, 3);
      return (
        <div className="directory_story__tile">
          <DirectoryTile {...listing} onOpen={() => undefined} />
        </div>
      );
    },
  },
];

export default stories;

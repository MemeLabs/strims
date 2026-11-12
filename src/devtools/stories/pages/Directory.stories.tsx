// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import Directory from "../../../pages/Directory";
import { Story, fixtures } from "../../storybook";
import { DirectoryContext, NetworkListings, listingsByNetwork } from "./directoryListings";

const networkPath = (i: number) => `/directory/${fixtures.keyB64(fixtures.networkKey(i))}`;

interface DirectoryStoryProps {
  byNetwork: NetworkListings[];
  /** The network the route names. */
  network?: number;
}

// The directory at /directory/:networkKey, the mock holding `byNetwork`'s listings.
const DirectoryStory: React.FC<DirectoryStoryProps> = ({ byNetwork, network = 1 }) => (
  <DirectoryContext
    byNetwork={byNetwork}
    initialPath={networkPath(network)}
    path="/directory/:networkKey"
  >
    <Directory />
  </DirectoryContext>
);

const stories: Story[] = [
  {
    // Network 1's listings alone, of the three networks' the directory holds.
    name: "Default",
    component: () => <DirectoryStory byNetwork={listingsByNetwork(3)} />,
  },
  {
    // Every listing of the directory mock, in network 1.
    name: "Many listings",
    component: () => <DirectoryStory byNetwork={listingsByNetwork(1)} />,
  },
  {
    // A network the directory holds no listings of.
    name: "Empty",
    component: () => <DirectoryStory byNetwork={listingsByNetwork(3)} network={7} />,
  },
];

export default stories;

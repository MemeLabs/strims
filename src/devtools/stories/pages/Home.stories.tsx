// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import Home from "../../../pages/Home";
import { Story } from "../../storybook";
import { DirectoryContext, NetworkListings, listingsByNetwork } from "./directoryListings";

// The home page at /: every network's listings.
const HomeStory: React.FC<{ byNetwork: NetworkListings[] }> = ({ byNetwork }) => (
  <DirectoryContext byNetwork={byNetwork} initialPath="/" path="/">
    <Home />
  </DirectoryContext>
);

const stories: Story[] = [
  {
    // Listings in three networks.
    name: "Default",
    component: () => <HomeStory byNetwork={listingsByNetwork(3)} />,
  },
  {
    name: "Empty",
    component: () => <HomeStory byNetwork={[]} />,
  },
];

export default stories;

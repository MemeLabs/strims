// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import qs from "qs";
import React from "react";
import { Route } from "react-router";

import Player from "../../../pages/Player";
import { Story, StoryRouter, fixtures } from "../../storybook";
import { swarmStream } from "../watch/streams";
import { PlayerReadout } from "./PlayerReadout";

// The page at /player/:networkKey?swarmUri=...&mimeType=..., as a media
// listing's link opens it: it asks the player for the swarm, large, and
// opens the overlay. Nothing plays here: the console build has no wasm.
const PlayerStory: React.FC<{ query: Record<string, string> }> = ({ query }) => (
  <PlayerReadout>
    <StoryRouter
      initialPath={`/player/${fixtures.keyB64(fixtures.testNetworkKey)}?${qs.stringify(query)}`}
    >
      <Route path="/player/:networkKey" element={<Player />} />
    </StoryRouter>
  </PlayerReadout>
);

const stories: Story[] = [
  {
    name: "Default",
    component: () => <PlayerStory query={{ ...swarmStream(1) }} />,
  },
  {
    // A link without the stream's query: the source names no swarm.
    name: "No stream",
    component: () => <PlayerStory query={{}} />,
  },
];

export default stories;

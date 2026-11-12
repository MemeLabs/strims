// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import qs from "qs";
import React from "react";
import { Route } from "react-router";

import { ServiceSlug } from "../../../lib/directory";
import Embed from "../../../pages/Embed";
import { Story, StoryRouter, fixtures } from "../../storybook";
import { PlayerReadout } from "./PlayerReadout";

interface EmbedStoryProps {
  service: ServiceSlug;
  id: string;
  /** The embed's own query parameters, beside the network's key (k). */
  query?: Record<string, string>;
}

// The page at /embed/:service/:id?k=..., as a listing's link opens it: it
// asks the player for the embed, large, and opens the overlay.
const EmbedStory: React.FC<EmbedStoryProps> = ({ service, id, query }) => (
  <PlayerReadout>
    <StoryRouter
      initialPath={`/embed/${service}/${id}?${qs.stringify({
        k: fixtures.keyB64(fixtures.testNetworkKey),
        ...query,
      })}`}
    >
      <Route path="/embed/:service/:id" element={<Embed />} />
    </StoryRouter>
  </PlayerReadout>
);

const stories: Story[] = [
  {
    name: "AngelThump",
    component: () => <EmbedStory service="angelthump" id="strims_test" />,
  },
  {
    name: "Twitch",
    component: () => <EmbedStory service="twitch" id="strims_test" />,
  },
  {
    name: "Twitch VOD",
    component: () => <EmbedStory service="twitch-vod" id="2071234567" query={{ t: "1h2m3s" }} />,
  },
  {
    name: "YouTube",
    component: () => <EmbedStory service="youtube" id="aqz-KE-bpKQ" query={{ t: "90" }} />,
  },
];

export default stories;

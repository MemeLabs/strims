// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Layout.stories.scss";

import { Base64 } from "js-base64";
import React from "react";
import { Route } from "react-router";

import Player from "../../../../components/Layout/Player";
import VideoMeta from "../../../../components/VideoMeta";
import { PlayerMode, PlayerSource } from "../../../../contexts/Player";
import {
  MockApi,
  SessionProvider,
  StaticPlayer,
  Story,
  StoryRouter,
  fixtures,
} from "../../../storybook";
import { EgressService, HLSEgressService, LayoutState, ListingDirectoryService } from "./shell";

const NETWORK_KEY = Base64.fromUint8Array(fixtures.testNetworkKey, true);

const EMBED: PlayerSource = {
  type: "embed",
  service: "youtube",
  id: "oEgZeTr3Vs4",
  networkKey: NETWORK_KEY,
};

const SWARM: PlayerSource = {
  type: "swarm",
  networkKey: NETWORK_KEY,
  swarmUri: "magnet:?xt=urn:ppspp:Ef8K5Wl5wl6nkCLc7KiUOgcmT0UdJIttU4NVg7iRTgw",
  mimeType: "video/x.strims-units",
};

interface PlayerStoryProps {
  source: PlayerSource | null;
  mode?: PlayerMode;
  /** The listing's meta in the meta slot (the fixtures' listing 36). */
  meta?: boolean;
  theaterMode?: boolean;
}

// The player as the layout's video column holds it. Its PiP expand link
// navigates in the story.
const PlayerStory: React.FC<PlayerStoryProps> = ({
  source,
  mode = PlayerMode.LARGE,
  meta,
  theaterMode,
}) => (
  <MockApi
    services={() => ({
      directory: new ListingDirectoryService(),
      hlsEgress: new HLSEgressService(),
      videoEgress: new EgressService(),
    })}
  >
    <SessionProvider>
      <StaticPlayer
        value={{
          source,
          mode,
          path: "/embed/youtube/oEgZeTr3Vs4",
          networkKey: fixtures.testNetworkKey,
          listingId: BigInt(36),
        }}
      >
        <LayoutState showVideo theaterMode={theaterMode} />
        <StoryRouter>
          <Route
            path="/"
            element={
              <div className="layout_story__player">
                <Player>{meta && <VideoMeta />}</Player>
              </div>
            }
          />
        </StoryRouter>
      </StaticPlayer>
    </SessionProvider>
  </MockApi>
);

const stories: Story[] = [
  {
    name: "Embed",
    component: () => <PlayerStory source={EMBED} meta />,
  },
  {
    name: "Embed full",
    component: () => <PlayerStory source={EMBED} mode={PlayerMode.FULL} meta />,
  },
  {
    name: "Embed PiP",
    component: () => <PlayerStory source={EMBED} mode={PlayerMode.PIP} />,
  },
  {
    name: "Swarm",
    component: () => <PlayerStory source={SWARM} meta />,
  },
  {
    name: "Theater mode",
    component: () => <PlayerStory source={SWARM} meta theaterMode />,
  },
  {
    name: "No source",
    component: () => <PlayerStory source={null} />,
  },
];

export default stories;

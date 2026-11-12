// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "../../../components/Layout/Player.scss";
import "./Layout/Layout.stories.scss";

import React from "react";

import { IListingSnippet } from "../../../apis/strims/network/v1/directory/directory";
import VideoMeta from "../../../components/VideoMeta";
import { PlayerMode } from "../../../contexts/Player";
import { Catalog, MockApi, StaticPlayer, Story, fixtures } from "../../storybook";
import { ListingDirectoryService } from "./Layout/shell";

interface MetaStoryProps {
  snippet?: IListingSnippet;
  userCount?: number;
  mode?: PlayerMode;
}

// The meta on the player's background, for the fixtures' listing 36 (a
// YouTube embed) playing. (The player's meta slot hides it on narrow
// layouts, phones in portrait among them: the Layout/Player stories show
// that.)
const MetaStory: React.FC<MetaStoryProps> = ({ snippet, userCount, mode = PlayerMode.LARGE }) => (
  <MockApi services={() => ({ directory: new ListingDirectoryService(snippet, userCount) })}>
    <StaticPlayer value={{ networkKey: fixtures.testNetworkKey, listingId: BigInt(36), mode }}>
      <div className="player_embed layout_story__meta">
        <VideoMeta />
      </div>
    </StaticPlayer>
  </MockApi>
);

const stories: Story[] = [
  {
    name: "Default",
    component: () => <MetaStory />,
  },
  {
    name: "Full height",
    component: () => <MetaStory mode={PlayerMode.FULL} />,
  },
  {
    name: "Long text",
    component: () => (
      <MetaStory
        snippet={{
          title:
            "🔴 24 HOUR CHARITY MARATHON — every donation goal unlocks another round of the worst speedrun categories ever devised, now with commentary",
          channelName: "a_channel_name_that_goes_on_and_on_without_any_spaces_at_all",
        }}
        userCount={4_100_000}
      />
    ),
  },
  {
    name: "No title or logo",
    component: () => <MetaStory snippet={{ title: "", channelLogo: null }} userCount={0} />,
  },
];

export default stories;

export const catalog: Catalog = {
  skips: {
    "components/VideoTitle.tsx":
      "renders nothing: it sets document.title from the playing listing's title and channel name",
  },
};

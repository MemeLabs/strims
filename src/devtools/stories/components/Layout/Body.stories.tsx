// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "../../../../components/Layout/Layout.scss";
import "./Layout.stories.scss";

import React from "react";
import { Route } from "react-router-dom";

import DirectoryGrid from "../../../../components/Directory/Grid";
import LayoutBody from "../../../../components/Layout/Body";
import { PlayerMode, PlayerSource } from "../../../../contexts/Player";
import { Story, StoryRouter, fixtures } from "../../../storybook";
import { LayoutState, ShellProviders } from "./shell";

const EMBED: PlayerSource = {
  type: "embed",
  service: "youtube",
  id: "oEgZeTr3Vs4",
  networkKey: "cgqhekoCTcy7OOkRdbNbYG3J4svZorYlH3KKaT660BE",
};

interface BodyStoryProps {
  room?: boolean;
  showChat?: boolean;
  source?: PlayerSource;
  mode?: PlayerMode;
  expandNav?: boolean;
}

// The layout's body (in the frame's layout: no header) with the directory
// grid in its outlet, as at /: the network nav, the content panel, the chat
// and its popout bar, and the player once something plays.
const BodyStory: React.FC<BodyStoryProps> = ({ room, showChat, source, mode, expandNav }) => (
  <ShellProviders room={room} source={source} mode={mode}>
    <LayoutState showChat={showChat} showVideo={!!source} expandNav={expandNav} />
    <StoryRouter>
      <Route
        path="/"
        element={
          <div className="layout_story__body">
            <div className="layout__body">
              <LayoutBody />
            </div>
          </div>
        }
      >
        <Route index element={<DirectoryGrid listings={fixtures.directoryListings()} />} />
      </Route>
    </StoryRouter>
  </ShellProviders>
);

const stories: Story[] = [
  {
    name: "Home",
    component: () => <BodyStory />,
  },
  {
    name: "Chat room",
    component: () => <BodyStory room />,
  },
  {
    name: "Chat closed",
    component: () => <BodyStory showChat={false} expandNav />,
  },
  {
    name: "Playing",
    component: () => <BodyStory room source={EMBED} />,
  },
];

export default stories;

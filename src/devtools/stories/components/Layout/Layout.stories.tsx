// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Layout.stories.scss";

import React from "react";
import { Route } from "react-router-dom";

import DirectoryGrid from "../../../../components/Directory/Grid";
import Layout from "../../../../components/Layout";
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

interface ShellStoryProps {
  room?: boolean;
  showChat?: boolean;
  source?: PlayerSource;
  mode?: PlayerMode;
  expandNav?: boolean;
  /** The player opened over the content, as opening a listing does. */
  open?: boolean;
}

// The whole shell as the app mounts it (MainProvider's providers, the
// layout around MainRouter's body route) at /, the directory grid in the
// body's outlet. Links navigate in the story. The layout keeps a layout
// context of its own, and the frame around it carries its context's
// layout--* classes, which the layout's rules match too: the flags are
// set in both.
const ShellStory: React.FC<ShellStoryProps> = ({
  room,
  showChat,
  source,
  mode,
  expandNav,
  open,
}) => {
  const flags = (
    <LayoutState
      showChat={showChat}
      showVideo={!!source}
      expandNav={expandNav}
      overlayOpen={open}
    />
  );
  return (
    <ShellProviders room={room} source={source} mode={mode}>
      {flags}
      <StoryRouter>
        <Route
          path="/"
          element={
            <Layout>
              {flags}
              <LayoutBody />
            </Layout>
          }
        >
          <Route index element={<DirectoryGrid listings={fixtures.directoryListings()} />} />
        </Route>
      </StoryRouter>
    </ShellProviders>
  );
};

const stories: Story[] = [
  {
    name: "Home",
    component: () => <ShellStory />,
  },
  {
    name: "Chat room",
    component: () => <ShellStory room />,
  },
  {
    name: "Chat closed",
    component: () => <ShellStory room showChat={false} expandNav />,
  },
  {
    name: "Playing",
    component: () => <ShellStory room source={EMBED} />,
  },
  {
    name: "Playing large",
    component: () => <ShellStory room source={EMBED} mode={PlayerMode.LARGE} open />,
  },
];

export default stories;

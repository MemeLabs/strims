// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { ReactNode } from "react";
import { Route } from "react-router-dom";

import DirectoryGrid from "../../components/Directory/Grid";
import Search from "../../components/Directory/Search";
import DirectoryService from "../mocks/directory/service";
import { MockApi, StaticPlayer, Story, StoryRouter, fixtures } from "../storybook";

// Opening a listing sets the player's source and navigates to it: both stay
// in the story.
const Context: React.FC<{ children: ReactNode }> = ({ children }) => (
  <MockApi services={() => ({ directory: new DirectoryService() })}>
    <StaticPlayer>
      <StoryRouter>
        <Route path="/" element={children} />
      </StoryRouter>
    </StaticPlayer>
  </MockApi>
);

const SearchStory: React.FC = () => (
  <Context>
    <div className="directory_mockup">
      <div className="directory_mockup__header">
        <Search maxResults={10} />
      </div>
    </div>
  </Context>
);

const GridStory: React.FC = () => (
  <Context>
    <DirectoryGrid listings={fixtures.directoryListings()} />
  </Context>
);

const stories: Story[] = [
  {
    name: "Search",
    component: () => <SearchStory />,
  },
  {
    name: "Grid",
    component: () => <GridStory />,
  },
];

export default stories;

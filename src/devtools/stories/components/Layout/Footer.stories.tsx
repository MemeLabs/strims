// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Layout.stories.scss";

import React from "react";
import { Route } from "react-router-dom";

import Footer from "../../../../components/Layout/Footer";
import DirectoryService from "../../../mocks/directory/service";
import { MockApi, StaticPlayer, Story, StoryRouter } from "../../../storybook";

// The footer the layout shows phones in place of the header, at the
// frame's foot: its directory link, search and settings buttons. Its search
// modal (a swipable sheet on phones, below the layout on a desktop, as
// here) is mounted closed; links and the settings modal's navigation stay
// in the story.
const FooterStory: React.FC = () => (
  <MockApi services={() => ({ directory: new DirectoryService() })}>
    <StaticPlayer>
      <StoryRouter>
        <Route
          path="/"
          element={
            <div className="layout_story__footer">
              <div className="layout_story__footer_bar">
                <Footer />
              </div>
            </div>
          }
        />
      </StoryRouter>
    </StaticPlayer>
  </MockApi>
);

const stories: Story[] = [
  {
    name: "Default",
    component: () => <FooterStory />,
  },
];

export default stories;

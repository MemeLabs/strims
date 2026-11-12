// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./core.stories.scss";

import React, { ReactNode } from "react";
import { Route } from "react-router-dom";

import InternalLink from "../../../components/InternalLink";
import { Story, StoryRouter } from "../../storybook";

const Page: React.FC<{ children: ReactNode }> = ({ children }) => (
  <StoryRouter initialPath="/login">
    <Route path="/login" element={<div className="core_story">{children}</div>} />
  </StoryRouter>
);

const stories: Story[] = [
  {
    name: "Default",
    component: () => (
      <Page>
        <div>
          <InternalLink to="/signup">Create a profile</InternalLink>
        </div>
      </Page>
    ),
  },
  {
    name: "In text",
    component: () => (
      <Page>
        <p className="core_story__text">
          No profile on this device yet? <InternalLink to="/signup">Create one</InternalLink> or{" "}
          <InternalLink to="/import">import a profile</InternalLink> from another device.
        </p>
      </Page>
    ),
  },
];

export default stories;

// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { ReactNode } from "react";
import { Route } from "react-router-dom";

import ForwardLink from "../../../../pages/Settings/ForwardLink";
import { Story, StoryRouter } from "../../../storybook";
import { SettingsContent, longDescription, longTitle } from "./Layout.harness";

// Links in a settings form's list; following one shows where it went.
const Context: React.FC<{ children: ReactNode }> = ({ children }) => (
  <StoryRouter initialPath="/settings/video/ingress">
    <Route
      path="/settings/video/ingress"
      element={<SettingsContent form>{children}</SettingsContent>}
    />
  </StoryRouter>
);

const stories: Story[] = [
  {
    name: "Default",
    component: () => (
      <Context>
        <ForwardLink to="/settings/video/channels" title="Channels" />
      </Context>
    ),
  },
  {
    name: "Description",
    component: () => (
      <Context>
        <ForwardLink
          to="/settings/autoseed/rules"
          title="Autoseed rules"
          description="Swarms this node seeds whenever they're live."
        />
      </Context>
    ),
  },
  {
    // Stacked as a form's list stacks them, the first and last rounded.
    name: "List",
    component: () => (
      <Context>
        <ForwardLink to="/settings/video/channels" title="Channels" />
        <ForwardLink to="/settings/autoseed/rules" title="Autoseed rules" />
        <ForwardLink
          to="/settings/debug/mock-stream"
          title="Mock streams"
          description="Synthetic streams for testing delivery."
        />
      </Context>
    ),
  },
  {
    name: "Long text",
    component: () => (
      <Context>
        <ForwardLink
          to="/settings/video/channels"
          title={longTitle}
          description={longDescription}
        />
      </Context>
    ),
  },
];

export default stories;

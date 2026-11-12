// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { ReactNode } from "react";
import { Route } from "react-router";

import BackLink from "../../../../pages/Settings/BackLink";
import { Story, StoryRouter } from "../../../storybook";
import { SettingsContent, longDescription, longTitle } from "./Layout.harness";

// A link in a settings form's list (no page uses one yet); following it
// shows where it went.
const Context: React.FC<{ children: ReactNode }> = ({ children }) => (
  <StoryRouter initialPath="/settings/video/channels">
    <Route
      path="/settings/video/channels"
      element={<SettingsContent form>{children}</SettingsContent>}
    />
  </StoryRouter>
);

const stories: Story[] = [
  {
    name: "Default",
    component: () => (
      <Context>
        <BackLink
          to="/settings/video/ingress"
          title="Video Ingress"
          description="The RTMP server's settings"
        />
      </Context>
    ),
  },
  {
    name: "Long text",
    component: () => (
      <Context>
        <BackLink to="/settings/video/ingress" title={longTitle} description={longDescription} />
      </Context>
    ),
  },
];

export default stories;

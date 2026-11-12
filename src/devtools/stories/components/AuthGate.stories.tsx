// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { Route } from "react-router-dom";

import { LinkedProfile } from "../../../apis/strims/auth/v1/auth";
import AuthGate from "../../../components/AuthGate";
import { Story, StoryRouter } from "../../storybook";
import { ProfileSession, ProfileSessionProps } from "./Layout/shell";

// The gate around a page at /settings/networks?tab=peers: a signed-in
// session sees the page, any other goes to sign in (or up) and back after.
const GateStory: React.FC<Omit<ProfileSessionProps, "children">> = (session) => (
  <ProfileSession {...session}>
    <StoryRouter initialPath="/settings/networks?tab=peers">
      <Route
        path="/settings/*"
        element={
          <AuthGate>
            <p className="storybook__message">the page behind the gate</p>
          </AuthGate>
        }
      />
    </StoryRouter>
  </ProfileSession>
);

const stories: Story[] = [
  {
    name: "Signed in",
    component: () => <GateStory />,
  },
  {
    name: "Signed out",
    component: () => (
      <GateStory
        signedIn={false}
        linkedProfiles={[new LinkedProfile({ id: BigInt(1), name: "test_user" })]}
      />
    ),
  },
  {
    name: "No profiles",
    component: () => <GateStory signedIn={false} />,
  },
];

export default stories;

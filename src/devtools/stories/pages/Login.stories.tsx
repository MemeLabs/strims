// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { Route } from "react-router";

import { LinkedProfile } from "../../../apis/strims/auth/v1/auth";
import { State } from "../../../contexts/Session";
import Login from "../../../pages/Login";
import { Story, StoryRouter, fixtures } from "../../storybook";
import { StorySession } from "./StorySession";
import { SubmitOnMount } from "./SubmitOnMount";

// Profiles this browser has signed in with: a key and a token one sign in
// on a click, a password one opens the form with its name and server.
const linkedProfiles = [
  new LinkedProfile({
    id: BigInt(1),
    name: "test_user",
    credentials: { key: { profileId: BigInt(1), profileKey: fixtures.profileKey } },
  }),
  new LinkedProfile({
    id: BigInt(2),
    name: "jbpratt",
    serverAddress: "wss://strims.gg/manage",
    credentials: {
      token: {
        profileId: BigInt(2),
        token: fixtures.bytes("token:2"),
        eol: fixtures.fixtureTime(30),
      },
    },
  }),
  new LinkedProfile({
    id: BigInt(3),
    name: "a_very_long_name_20c",
    serverAddress: "wss://next.strims.gg/api",
    credentials: { password: { totpRequired: false } },
  }),
];

interface LoginStoryProps {
  state?: Partial<State>;
  /** What a sign-in ends in; default signed in, which navigates to `next` or /. */
  failWith?: Error;
  initialPath?: string;
  newLogin?: boolean;
}

const LoginStory: React.FC<LoginStoryProps> = ({
  state,
  failWith,
  initialPath = "/login",
  newLogin,
}) => (
  <StorySession state={state} failWith={failWith}>
    <StoryRouter initialPath={initialPath}>
      <Route path="/login" element={<Login newLogin={newLogin} />} />
    </StoryRouter>
  </StorySession>
);

const signInError = new Error("invalid name or password");

const stories: Story[] = [
  {
    // No linked profiles: the sign-in form.
    name: "Default",
    component: () => <LoginStory />,
  },
  {
    name: "Linked profiles",
    component: () => <LoginStory state={{ linkedProfiles }} />,
  },
  {
    // /login/new: the form, though the browser has linked profiles.
    name: "New login",
    component: () => <LoginStory state={{ linkedProfiles }} newLogin />,
  },
  {
    name: "Loading",
    component: () => <LoginStory state={{ loading: true }} />,
  },
  {
    name: "Error",
    component: () => <LoginStory state={{ error: signInError }} failWith={signInError} />,
  },
  {
    // The form submitted empty.
    name: "Validation",
    component: () => (
      <SubmitOnMount>
        <LoginStory />
      </SubmitOnMount>
    ),
  },
  {
    // Signed in already: on to the page that sent it here.
    name: "Signed in",
    component: () => (
      <LoginStory
        state={{ profile: fixtures.profile() }}
        initialPath="/login?next=/settings/networks"
      />
    ),
  },
];

export default stories;

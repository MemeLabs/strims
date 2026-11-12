// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { Route } from "react-router-dom";

import { State } from "../../../contexts/Session";
import SignUp from "../../../pages/SignUp";
import { Story, StoryRouter, fixtures } from "../../storybook";
import { StorySession } from "./StorySession";
import { SubmitOnMount } from "./SubmitOnMount";

interface SignUpStoryProps {
  state?: Partial<State>;
  /** What a sign-up ends in; default signed in, which navigates to `next` or /. */
  failWith?: Error;
  initialPath?: string;
}

const SignUpStory: React.FC<SignUpStoryProps> = ({ state, failWith, initialPath = "/signup" }) => (
  <StorySession state={state} failWith={failWith}>
    <StoryRouter initialPath={initialPath}>
      <Route path="/signup" element={<SignUp />} />
    </StoryRouter>
  </StorySession>
);

const signUpError = new Error("a profile named test_user exists");

const stories: Story[] = [
  {
    name: "Default",
    component: () => <SignUpStory />,
  },
  {
    name: "Loading",
    component: () => <SignUpStory state={{ loading: true }} />,
  },
  {
    name: "Error",
    component: () => <SignUpStory state={{ error: signUpError }} failWith={signUpError} />,
  },
  {
    // The form submitted empty.
    name: "Validation",
    component: () => (
      <SubmitOnMount>
        <SignUpStory />
      </SubmitOnMount>
    ),
  },
  {
    // Signed in already: on to the page that sent it here.
    name: "Signed in",
    component: () => (
      <SignUpStory state={{ profile: fixtures.profile() }} initialPath="/signup?next=/broadcast" />
    ),
  },
];

export default stories;

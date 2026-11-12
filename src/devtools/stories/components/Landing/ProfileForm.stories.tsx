// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { ComponentProps, ReactNode, useEffect, useRef } from "react";
import { Route } from "react-router-dom";

import ProfileForm from "../../../../components/Landing/ProfileForm";
import LandingPageLayout from "../../../../components/LandingPageLayout";
import { Story, StoryRouter } from "../../../storybook";

type ProfileFormProps = ComponentProps<typeof ProfileForm>;

const noop = (): void => undefined;

// As pages/Login.tsx and pages/SignUp.tsx have it: in the landing layout,
// whose stylesheet holds the form's accordion. Its secondary link is a
// router link.
const Page: React.FC<{ children: ReactNode }> = ({ children }) => (
  <StoryRouter initialPath="/login">
    <Route path="/login" element={<LandingPageLayout>{children}</LandingPageLayout>} />
  </StoryRouter>
);

const signIn: ProfileFormProps = {
  onSubmit: noop,
  secondaryUri: "/signup",
  secondaryLabel: "Create profile",
  submitLabel: "Log in",
  defaultValues: { name: "test_user" },
  enablePairing: true,
};

const signUp: ProfileFormProps = {
  onSubmit: noop,
  secondaryUri: "/login",
  secondaryLabel: "Log in",
  submitLabel: "Create Profile",
  defaultValues: { persistLogin: true },
};

// Submits the form once mounted, as its button would: its rules' errors show.
const Submitted: React.FC<{ children: ReactNode }> = ({ children }) => {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => ref.current.querySelector("form").requestSubmit(), []);
  return <div ref={ref}>{children}</div>;
};

const stories: Story[] = [
  {
    name: "Sign in",
    component: () => (
      <Page>
        <ProfileForm {...signIn} />
      </Page>
    ),
  },
  {
    name: "Sign up",
    component: () => (
      <Page>
        <ProfileForm {...signUp} />
      </Page>
    ),
  },
  {
    name: "Error",
    component: () => (
      <Page>
        <ProfileForm {...signIn} error="invalid profile name or password" />
      </Page>
    ),
  },
  {
    name: "Validation errors",
    component: () => (
      <Page>
        <Submitted>
          <ProfileForm
            {...signUp}
            defaultValues={{
              name: "not valid!",
              advanced: true,
              serverAddress: "not a url",
            }}
          />
        </Submitted>
      </Page>
    ),
  },
  {
    name: "Advanced with pairing",
    component: () => (
      <Page>
        <ProfileForm
          {...signIn}
          defaultValues={{
            name: "test_user",
            advanced: true,
            serverAddress: "wss://next.strims.gg/api",
            persistLogin: true,
          }}
        />
      </Page>
    ),
  },
  {
    name: "No secondary",
    component: () => (
      <Page>
        <ProfileForm {...signUp} secondaryVisible={false} />
      </Page>
    ),
  },
];

export default stories;

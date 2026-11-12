// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./core.stories.scss";

import React from "react";
import { FiUser } from "react-icons/fi";

import LandingPageLayout from "../../../components/LandingPageLayout";
import { Story } from "../../storybook";

// The layout fills the viewport (position: fixed): the story's frame here.
const Viewport: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div className="core_story__viewport core_story__viewport--fill">
    <LandingPageLayout>{children}</LandingPageLayout>
  </div>
);

// Login's saved profiles, whose styles LandingPageLayout.scss holds.
const ProfileList: React.FC<{ names: string[] }> = ({ names }) => (
  <div>
    {names.map((name, i) => (
      <div key={name} className="login_profile_list__item">
        <FiUser className="login_profile_list__icon" />
        <span className="login_profile_list__text">
          {name}
          {i % 2 === 1 && <span className="login_profile_list__subtext">next.strims.gg</span>}
        </span>
      </div>
    ))}
  </div>
);

const stories: Story[] = [
  {
    name: "Default",
    component: () => (
      <Viewport>
        <ProfileList names={["test_user", "second_profile"]} />
      </Viewport>
    ),
  },
  {
    name: "Text",
    component: () => (
      <Viewport>
        <p>Pick a profile to sign in with, or create one.</p>
      </Viewport>
    ),
  },
  {
    name: "Long content",
    component: () => (
      <Viewport>
        <ProfileList names={Array.from({ length: 12 }, (_, i) => `profile_${i + 1}`)} />
      </Viewport>
    ),
  },
];

export default stories;

// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { FiUser } from "react-icons/fi";

import LandingPageLayout from "../../../components/LandingPageLayout";
import { Story } from "../../storybook";

// The layout fills the viewport (position: fixed), the story's frame.

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
      <LandingPageLayout>
        <ProfileList names={["test_user", "second_profile"]} />
      </LandingPageLayout>
    ),
  },
  {
    name: "Text",
    component: () => (
      <LandingPageLayout>
        <p>Pick a profile to sign in with, or create one.</p>
      </LandingPageLayout>
    ),
  },
  {
    name: "Long content",
    component: () => (
      <LandingPageLayout>
        <ProfileList names={Array.from({ length: 12 }, (_, i) => `profile_${i + 1}`)} />
      </LandingPageLayout>
    ),
  },
];

export default stories;

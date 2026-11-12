// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import NetworkService from "../../../../mocks/network/service";
import { Story } from "../../../../storybook";
import { SettingsStory, fill, submit, waitFor } from "./harness";

class AliasTakenNetworkService extends NetworkService {
  updateAlias(): never {
    throw new Error("alias jbpratt is in use");
  }
}

const stories: Story[] = [
  {
    // Network 1 is the profile's own: its links to the server's settings.
    name: "Owner",
    component: () => <SettingsStory path="/settings/networks/1" />,
  },
  {
    name: "Member",
    component: () => <SettingsStory path="/settings/networks/2" />,
  },
  {
    // No network 9.
    name: "Error",
    component: () => <SettingsStory path="/settings/networks/9" />,
  },
  {
    name: "Invalid",
    component: () => (
      <SettingsStory path="/settings/networks/2" steps={[fill("alias", "x"), submit()]} />
    ),
  },
  {
    name: "Update error",
    component: () => (
      <SettingsStory
        path="/settings/networks/2"
        services={() => ({ network: new AliasTakenNetworkService({ count: 5 }) })}
        steps={[fill("alias", "jbpratt"), submit()]}
      />
    ),
  },
  {
    // Saving goes back to the list.
    name: "Saved",
    component: () => (
      <SettingsStory
        path="/settings/networks/2"
        steps={[fill("alias", "new_alias"), submit(), waitFor(".thing_table")]}
      />
    ),
  },
];

export default stories;

// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import NetworkService from "../../../../mocks/network/service";
import { Story } from "../../../../storybook";
import { SettingsStory, fill, submit, waitFor } from "./harness";

const PATH = "/settings/networks/new";

class FailingNetworkService extends NetworkService {
  createServer(): never {
    throw new Error("a network named comfy_corner already runs here");
  }
}

const stories: Story[] = [
  {
    name: "Default",
    component: () => <SettingsStory path={PATH} />,
  },
  {
    // With no networks yet there's no list to go back to.
    name: "First network",
    component: () => (
      <SettingsStory path={PATH} services={() => ({ network: new NetworkService({ count: 0 }) })} />
    ),
  },
  {
    name: "Required",
    component: () => <SettingsStory path={PATH} steps={[submit()]} />,
  },
  {
    name: "Invalid",
    component: () => (
      <SettingsStory
        path={PATH}
        steps={[fill("name", "my network!"), fill("alias", "a"), submit()]}
      />
    ),
  },
  {
    name: "Create error",
    component: () => (
      <SettingsStory
        path={PATH}
        services={() => ({ network: new FailingNetworkService({ count: 5 }) })}
        steps={[fill("name", "comfy_corner"), submit()]}
      />
    ),
  },
  {
    // Creating opens the new network's page.
    name: "Created",
    component: () => (
      <SettingsStory
        path={PATH}
        steps={[
          fill("name", "comfy_corner"),
          fill("alias", "test_user"),
          submit(),
          waitFor(".thing_table__title_bar__title", "Edit Network"),
        ]}
      />
    ),
  },
];

export default stories;

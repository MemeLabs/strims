// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import BootstrapService from "../../../../mocks/bootstrap/service";
import { Story } from "../../../../storybook";
import { SettingsStory, click, fill, submit, waitFor } from "../Network/harness";

const PATH = "/settings/bootstrap/clients/new";

class FailingBootstrapService extends BootstrapService {
  createClient(): never {
    throw new Error("a client for wss://strims.gg/_bootstrap exists");
  }
}

const stories: Story[] = [
  {
    // With clients to go back to.
    name: "Default",
    component: () => <SettingsStory path={PATH} />,
  },
  {
    name: "No clients",
    component: () => (
      <SettingsStory
        path={PATH}
        services={() => ({ bootstrap: new BootstrapService({ clients: [] }) })}
      />
    ),
  },
  {
    name: "Required",
    component: () => <SettingsStory path={PATH} steps={[submit()]} />,
  },
  {
    name: "Invalid",
    component: () => <SettingsStory path={PATH} steps={[fill("url", "strims dot gg"), submit()]} />,
  },
  {
    name: "Create error",
    component: () => (
      <SettingsStory
        path={PATH}
        services={() => ({ bootstrap: new FailingBootstrapService() })}
        steps={[fill("url", "wss://strims.gg/_bootstrap"), submit()]}
      />
    ),
  },
  {
    // Creating goes to the list, the new client last.
    name: "Created",
    component: () => (
      <SettingsStory
        path={PATH}
        steps={[
          fill("url", "wss://new.example.org/_bootstrap"),
          click("input[name=insecureSkipVerifyTls]"),
          submit(),
          waitFor(".thing_table tbody tr:last-child a", "wss://new.example.org/_bootstrap"),
        ]}
      />
    ),
  },
];

export default stories;

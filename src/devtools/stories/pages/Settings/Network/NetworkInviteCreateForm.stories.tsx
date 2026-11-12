// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import BootstrapService from "../../../../mocks/bootstrap/service";
import NetworkService from "../../../../mocks/network/service";
import { Story } from "../../../../storybook";
import { SettingsStory, choose, openSelect, submit, waitFor } from "./harness";

class FailingNetworkService extends NetworkService {
  createInvitation(): never {
    throw new Error("only the network's admins can invite");
  }
}

const PATH = "/settings/networks/1/invite";

const stories: Story[] = [
  {
    name: "Default",
    component: () => <SettingsStory path={PATH} />,
  },
  {
    // The bootstraps the invitation may name.
    name: "Bootstraps",
    component: () => <SettingsStory path={PATH} steps={[openSelect()]} />,
  },
  {
    // No bootstraps to name: the select's options are empty.
    name: "No bootstraps",
    component: () => (
      <SettingsStory
        path={PATH}
        services={() => ({ bootstrap: new BootstrapService({ clients: [] }) })}
        steps={[openSelect()]}
      />
    ),
  },
  {
    // Creating one copies its code, says so and goes back to the list.
    name: "Created",
    component: () => (
      <SettingsStory
        path={PATH}
        steps={[
          openSelect(),
          choose("wss://strims.gg/_bootstrap"),
          submit(),
          waitFor(".notification_toast__item"),
        ]}
      />
    ),
  },
  {
    name: "Create error",
    component: () => (
      <SettingsStory
        path={PATH}
        services={() => ({ network: new FailingNetworkService({ count: 5 }) })}
        steps={[submit()]}
      />
    ),
  },
];

export default stories;

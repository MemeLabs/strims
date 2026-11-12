// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import { BootstrapClient } from "../../../../../apis/strims/network/v1/bootstrap/bootstrap";
import BootstrapService, { bootstrapClients } from "../../../../mocks/bootstrap/service";
import { Story } from "../../../../storybook";
import { SettingsStory, fill, submit, waitFor } from "../Network/harness";

class FailingBootstrapService extends BootstrapService {
  updateClient(): never {
    throw new Error("the profile's store is read-only");
  }
}

const stories: Story[] = [
  {
    name: "Default",
    component: () => <SettingsStory path="/settings/bootstrap/clients/1" />,
  },
  {
    // Client 3 skips TLS verification.
    name: "Insecure",
    component: () => <SettingsStory path="/settings/bootstrap/clients/3" />,
  },
  {
    // A client without websocket options has no form: the page is blank.
    name: "Other client",
    component: () => (
      <SettingsStory
        path="/settings/bootstrap/clients/4"
        services={() => ({
          bootstrap: new BootstrapService({
            clients: [...bootstrapClients(3), new BootstrapClient({ id: BigInt(4) })],
          }),
        })}
      />
    ),
  },
  {
    // No client 9.
    name: "Error",
    component: () => <SettingsStory path="/settings/bootstrap/clients/9" />,
  },
  {
    name: "Invalid",
    component: () => (
      <SettingsStory path="/settings/bootstrap/clients/1" steps={[fill("url", ""), submit()]} />
    ),
  },
  {
    name: "Update error",
    component: () => (
      <SettingsStory
        path="/settings/bootstrap/clients/1"
        services={() => ({ bootstrap: new FailingBootstrapService() })}
        steps={[submit()]}
      />
    ),
  },
  {
    // Saving goes back to the list.
    name: "Saved",
    component: () => (
      <SettingsStory
        path="/settings/bootstrap/clients/1"
        steps={[
          fill("url", "wss://strims.gg/_bootstrap/v2"),
          submit(),
          waitFor(".thing_table tbody tr:first-child a", "wss://strims.gg/_bootstrap/v2"),
        ]}
      />
    ),
  },
];

export default stories;

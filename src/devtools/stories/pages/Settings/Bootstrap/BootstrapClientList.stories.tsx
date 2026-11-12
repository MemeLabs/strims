// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import { BootstrapClient } from "../../../../../apis/strims/network/v1/bootstrap/bootstrap";
import BootstrapService, {
  bootstrapClients,
  websocketClient,
} from "../../../../mocks/bootstrap/service";
import { Story } from "../../../../storybook";
import { SettingsStory, click, waitFor } from "../Network/harness";

const PATH = "/settings/bootstrap/clients";

const withClients = (clients: BootstrapClient[]) => () => ({
  bootstrap: new BootstrapService({ clients }),
});

class FailingBootstrapService extends BootstrapService {
  listClients(): never {
    throw new Error("the node isn't answering");
  }
}

const firstRowMenu = ".thing_table tbody tr:first-child .thing_table__row_menu";

const stories: Story[] = [
  {
    name: "Default",
    component: () => <SettingsStory path={PATH} />,
  },
  {
    name: "Many",
    component: () => <SettingsStory path={PATH} services={withClients(bootstrapClients(20))} />,
  },
  {
    name: "Long URLs",
    component: () => (
      <SettingsStory
        path={PATH}
        services={withClients([
          websocketClient(
            1,
            "wss://a-very-long-bootstrap-host-name.eu-west-1.relay.example.net:8082/_bootstrap/with/a/long/path?token=0123456789abcdef0123456789abcdef"
          ),
          websocketClient(2, "wss://strims.gg/_bootstrap"),
        ])}
      />
    ),
  },
  {
    // Only websocket clients are listed: client 2 has no options.
    name: "Other clients",
    component: () => (
      <SettingsStory
        path={PATH}
        services={withClients([
          websocketClient(1, "wss://strims.gg/_bootstrap"),
          new BootstrapClient({ id: BigInt(2) }),
          websocketClient(3, "ws://192.168.1.20:8082/_bootstrap"),
        ])}
      />
    ),
  },
  {
    // With no clients the page goes to the create form.
    name: "Empty",
    component: () => <SettingsStory path={PATH} services={withClients([])} />,
  },
  {
    // A failed list goes to the create form too.
    name: "Error",
    component: () => (
      <SettingsStory path={PATH} services={() => ({ bootstrap: new FailingBootstrapService() })} />
    ),
  },
  {
    name: "Create menu",
    component: () => <SettingsStory path={PATH} steps={[click(".thing_table_dropdown__anchor")]} />,
  },
  {
    name: "Row menu",
    component: () => (
      <SettingsStory path={PATH} steps={[click(`${firstRowMenu} .dropdown__anchor`)]} />
    ),
  },
  {
    // Deleting the first client: the list reloads one shorter.
    name: "Deleted",
    component: () => (
      <SettingsStory
        path={PATH}
        steps={[
          click(`${firstRowMenu} .dropdown__anchor`),
          click(`${firstRowMenu} button`, "Delete"),
          waitFor(
            ".thing_table tbody tr:first-child a",
            "wss://bootstrap.example.org:8082/_bootstrap"
          ),
        ]}
      />
    ),
  },
];

export default stories;

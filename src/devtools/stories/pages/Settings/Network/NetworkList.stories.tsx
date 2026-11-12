// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import NetworkService from "../../../../mocks/network/service";
import { Catalog, Story, fixtures } from "../../../../storybook";
import { SettingsStory, click, waitFor } from "./harness";

const PATH = "/settings/networks";

class FailingNetworkService extends NetworkService {
  list(): never {
    throw new Error("the node isn't answering");
  }
}

const longName =
  "The Extremely Long Network Name Of The Late Night Speedrun And Film Appreciation Society";

const firstRowMenu = ".thing_table tbody tr:first-child .thing_table__row_menu";

const stories: Story[] = [
  {
    name: "Default",
    component: () => <SettingsStory path={PATH} />,
  },
  {
    name: "Many",
    component: () => (
      <SettingsStory
        path={PATH}
        services={() => ({ network: new NetworkService({ count: 30 }) })}
      />
    ),
  },
  {
    name: "Long names",
    component: () => (
      <SettingsStory
        path={PATH}
        services={() => ({
          network: new NetworkService({
            networks: [
              fixtures.network(1, { name: longName }),
              fixtures.network(2, { name: `${longName} II`, alias: "a_very_long_alternate_name" }),
              fixtures.network(3),
            ],
          }),
        })}
      />
    ),
  },
  {
    name: "Empty",
    component: () => (
      <SettingsStory path={PATH} services={() => ({ network: new NetworkService({ count: 0 }) })} />
    ),
  },
  {
    name: "Error",
    component: () => (
      <SettingsStory path={PATH} services={() => ({ network: new FailingNetworkService() })} />
    ),
  },
  {
    name: "Add menu",
    component: () => <SettingsStory path={PATH} steps={[click(".thing_table_dropdown__anchor")]} />,
  },
  {
    name: "Row menu",
    component: () => (
      <SettingsStory path={PATH} steps={[click(`${firstRowMenu} .dropdown__anchor`)]} />
    ),
  },
  {
    // The first network's Delete: the list reloads one shorter.
    name: "Deleted",
    component: () => (
      <SettingsStory
        path={PATH}
        steps={[
          click(`${firstRowMenu} .dropdown__anchor`),
          click(`${firstRowMenu} button`, "Delete"),
          waitFor(".thing_table tbody tr:first-child a", fixtures.networkName(2)),
        ]}
      />
    ),
  },
  {
    // The first network's Publish opens PublishNetworkModal over the list.
    name: "Publish",
    component: () => (
      <SettingsStory
        path={PATH}
        steps={[
          click(`${firstRowMenu} .dropdown__anchor`),
          click(`${firstRowMenu} button`, "Publish"),
        ]}
      />
    ),
  },
];

export default stories;

export const catalog: Catalog = {
  covers: ["pages/Settings/Network/index.tsx"],
};

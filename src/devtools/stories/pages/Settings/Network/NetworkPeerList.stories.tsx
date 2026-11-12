// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import { Peer } from "../../../../../apis/strims/network/v1/network";
import NetworkService from "../../../../mocks/network/service";
import { Story, fixtures } from "../../../../storybook";
import { SettingsStory, click, waitFor } from "./harness";

const PATH = "/settings/networks/1/peers";

// Network 1 with `peers` in place of the fixture's six.
const withPeers = (peers: Peer[]) => () => {
  const network = new NetworkService({ count: 5 });
  network.peers.set(BigInt(1), peers);
  return { network };
};

// Aliases at the 20 character limit, and older ones from before it.
const longAliases = [
  "twenty_characters_ok",
  "ABCDEFGHIJKLMNOPQRST",
  "a_name_from_before_the_twenty_character_limit_was_enforced",
  "short",
];

const firstRowMenu = ".thing_table tbody tr:first-child .thing_table__row_menu";

const stories: Story[] = [
  {
    // An admin (jbpratt) and a banned peer (KartoffelKopf).
    name: "Default",
    component: () => <SettingsStory path={PATH} />,
  },
  {
    name: "Many",
    component: () => (
      <SettingsStory path={PATH} services={withPeers(fixtures.peers(BigInt(1), 40))} />
    ),
  },
  {
    // Every third peer banned.
    name: "Banned",
    component: () => (
      <SettingsStory
        path={PATH}
        services={withPeers(
          fixtures.peers(BigInt(1), 12).map((p, i) => new Peer({ ...p, isBanned: i % 3 === 0 }))
        )}
      />
    ),
  },
  {
    name: "Long aliases",
    component: () => (
      <SettingsStory
        path={PATH}
        services={withPeers(
          fixtures.peers(BigInt(1), 4).map((p, i) => new Peer({ ...p, alias: longAliases[i] }))
        )}
      />
    ),
  },
  {
    name: "Empty",
    component: () => <SettingsStory path={PATH} services={withPeers([])} />,
  },
  {
    // No network 9.
    name: "Error",
    component: () => <SettingsStory path="/settings/networks/9/peers" />,
  },
  {
    name: "Row menu",
    component: () => (
      <SettingsStory path={PATH} steps={[click(`${firstRowMenu} .dropdown__anchor`)]} />
    ),
  },
  {
    // Banning the first peer (biscophan): the list reloads with it banned.
    name: "Ban",
    component: () => (
      <SettingsStory
        path={PATH}
        steps={[
          click(`${firstRowMenu} .dropdown__anchor`),
          click(`${firstRowMenu} button`, "Ban"),
          waitFor(`${firstRowMenu} button`, "Unban"),
        ]}
      />
    ),
  },
];

export default stories;

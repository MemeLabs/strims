// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import { AliasReservation } from "../../../../../apis/strims/network/v1/network";
import NetworkService from "../../../../mocks/network/service";
import { Story, fixtures } from "../../../../storybook";
import { SettingsStory, click, waitFor } from "./harness";

const PATH = "/settings/networks/1/alias-reservations";

// Network 1 with `reservations` in place of the fixture's three.
const withReservations = (reservations: AliasReservation[]) => () => {
  const network = new NetworkService({ count: 5 });
  network.reservations.set(BigInt(1), reservations);
  return { network };
};

// The page reads the time to tell a reservation on cooldown from a free one:
// these end long after and long before any day the stories are looked at.
const FAR_FUTURE = BigInt(4102444800); // 2100-01-01
const LONG_AGO = fixtures.fixtureTime(-365);

// Held by a peer, released and cooling down, and released long ago (the
// table sorts by alias: the second row cools down).
const states = (): AliasReservation[] => {
  const [held, cooling, free, ...rest] = fixtures.aliasReservations(BigInt(1), 5);
  return [
    new AliasReservation({ ...held, alias: "biscophan" }),
    new AliasReservation({
      ...cooling,
      alias: "jbpratt",
      peerKey: new Uint8Array(),
      reservedUntil: FAR_FUTURE,
    }),
    new AliasReservation({
      ...free,
      alias: "soadsoap",
      peerKey: new Uint8Array(),
      reservedUntil: LONG_AGO,
    }),
    ...rest.map((r, i) => new AliasReservation({ ...r, alias: ["tahley", "widdlez"][i] })),
  ];
};

const cooldownRowMenu = ".thing_table tbody tr:nth-child(2) .thing_table__row_menu";

const stories: Story[] = [
  {
    // Every reservation held by a peer.
    name: "Default",
    component: () => <SettingsStory path={PATH} />,
  },
  {
    // Active, on cooldown and free.
    name: "States",
    component: () => <SettingsStory path={PATH} services={withReservations(states())} />,
  },
  {
    name: "Long aliases",
    component: () => (
      <SettingsStory
        path={PATH}
        services={withReservations(
          fixtures.aliasReservations(BigInt(1), 2).map(
            (r, i) =>
              new AliasReservation({
                ...r,
                alias: ["twenty_characters_ok", "a_name_from_before_the_twenty_character_limit"][i],
              })
          )
        )}
      />
    ),
  },
  {
    name: "Empty",
    component: () => <SettingsStory path={PATH} services={withReservations([])} />,
  },
  {
    // No network 9.
    name: "Error",
    component: () => <SettingsStory path="/settings/networks/9/alias-reservations" />,
  },
  {
    // A released alias's menu: its cooldown can be reset.
    name: "Row menu",
    component: () => (
      <SettingsStory
        path={PATH}
        services={withReservations(states())}
        steps={[click(`${cooldownRowMenu} .dropdown__anchor`)]}
      />
    ),
  },
  {
    // Resetting jbpratt's cooldown frees it.
    name: "Reset",
    component: () => (
      <SettingsStory
        path={PATH}
        services={withReservations(states())}
        steps={[
          click(`${cooldownRowMenu} .dropdown__anchor`),
          click(`${cooldownRowMenu} button`, "Reset cooldown"),
          waitFor(".thing_table tbody tr:nth-child(2) td:nth-child(2)", "Free"),
        ]}
      />
    ),
  },
];

export default stories;

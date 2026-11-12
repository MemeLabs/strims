// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import * as networkv1 from "../../../../../apis/strims/network/v1/network";
import NetworkService from "../../../../mocks/network/service";
import { Story, fixtures } from "../../../../storybook";
import { SettingsStory, submit, waitFor } from "./harness";

// Networks 1..5, network 1's YouTube embeds with their key, so its form saves as it loads.
const networks = (): networkv1.Network[] => {
  const [own, ...rest] = fixtures.networks(5);
  own.serverConfig.directory.integrations.youtube.publicApiKey = "AIzaSyD-fixture-public-api-key";
  return [own, ...rest];
};

// Fails the story when an update drops the directory settings the form doesn't show.
class CheckingNetworkService extends NetworkService {
  updateServerConfig(
    req: networkv1.UpdateServerConfigRequest
  ): networkv1.UpdateServerConfigResponse {
    const want = networks()[0].serverConfig.directory;
    const got = req.serverConfig.directory;
    for (const field of [
      "publishQuota",
      "joinQuota",
      "broadcastInterval",
      "sessionTimeout",
    ] as const) {
      if (got[field] !== want[field]) {
        // eslint-disable-next-line no-console
        console.error(
          `storybook: the update's directory ${field} is ${got[field]}, not ${want[field]}`
        );
      }
    }
    return super.updateServerConfig(req);
  }
}

class FailingUpdateNetworkService extends NetworkService {
  updateServerConfig(): never {
    throw new Error("the network's server isn't running");
  }
}

const stories: Story[] = [
  {
    // Network 1 is the profile's own; its YouTube embeds lack their key.
    name: "Owner",
    component: () => <SettingsStory path="/settings/networks/1/directory" />,
  },
  {
    // A network the profile is a member of has no server config: the page is blank.
    name: "Member",
    component: () => <SettingsStory path="/settings/networks/2/directory" />,
  },
  {
    // No network 9.
    name: "Error",
    component: () => <SettingsStory path="/settings/networks/9/directory" />,
  },
  {
    // Saving without YouTube's key.
    name: "Invalid",
    component: () => <SettingsStory path="/settings/networks/1/directory" steps={[submit()]} />,
  },
  {
    name: "Update error",
    component: () => (
      <SettingsStory
        path="/settings/networks/1/directory"
        services={() => ({ network: new FailingUpdateNetworkService({ networks: networks() }) })}
        steps={[submit()]}
      />
    ),
  },
  {
    // Saving unchanged keeps the quotas and intervals, then goes back to the network.
    name: "Saved",
    component: () => (
      <SettingsStory
        path="/settings/networks/1/directory"
        services={() => ({ network: new CheckingNetworkService({ networks: networks() }) })}
        steps={[submit(), waitFor("input[name=alias]")]}
      />
    ),
  },
];

export default stories;

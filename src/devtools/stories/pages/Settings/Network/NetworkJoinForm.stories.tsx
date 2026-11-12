// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import React from "react";

import { Invitation } from "../../../../../apis/strims/network/v1/network";
import NetworkService from "../../../../mocks/network/service";
import { Story, fixtures } from "../../../../storybook";
import { SettingsStory, fill, submit, waitFor } from "./harness";

const PATH = "/settings/networks/join";

// An invitation to network 6, as the invite page copies it.
const code = Base64.fromUint8Array(
  Invitation.encode(fixtures.invitation(fixtures.network(6, { owner: false }))).finish()
).replace(/={0,2}$/, "");

class ExpiredNetworkService extends NetworkService {
  createNetworkFromInvitation(): never {
    throw new Error("the invitation's certificate has expired");
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
    name: "Filled",
    component: () => (
      <SettingsStory
        path={PATH}
        steps={[fill("invitationB64", code), fill("alias", "test_user")]}
      />
    ),
  },
  {
    name: "Invalid",
    component: () => (
      <SettingsStory
        path={PATH}
        steps={[
          fill("invitationB64", "not an invitation!"),
          fill("alias", "no spaces please"),
          submit(),
        ]}
      />
    ),
  },
  {
    name: "Required",
    component: () => <SettingsStory path={PATH} steps={[submit()]} />,
  },
  {
    name: "Join error",
    component: () => (
      <SettingsStory
        path={PATH}
        services={() => ({ network: new ExpiredNetworkService({ count: 5 }) })}
        steps={[fill("invitationB64", code), submit()]}
      />
    ),
  },
  {
    // Joining goes back to the list, the new network last.
    name: "Joined",
    component: () => (
      <SettingsStory
        path={PATH}
        steps={[
          fill("invitationB64", code),
          fill("alias", "test_user"),
          submit(),
          waitFor(".thing_table tbody tr:last-child a", fixtures.networkName(6)),
        ]}
      />
    ),
  },
];

export default stories;

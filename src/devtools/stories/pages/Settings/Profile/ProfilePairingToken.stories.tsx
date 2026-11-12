// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import ReplicationService from "../../../../mocks/replication/service";
import { Story } from "../../../../storybook";
import { SettingsStory, click, slow, waitFor } from "../Network/harness";

const PATH = "/settings/profile/pairing-token";

const stories: Story[] = [
  {
    // The token as a QR code and as text.
    name: "Default",
    component: () => <SettingsStory path={PATH} />,
  },
  {
    name: "Loading",
    component: () => (
      <SettingsStory
        path={PATH}
        services={() => ({ replication: slow(new ReplicationService(), ["createPairingToken"]) })}
      />
    ),
  },
  {
    // A profile in no network can't pair.
    name: "Error",
    component: () => (
      <SettingsStory
        path={PATH}
        services={() => ({ replication: new ReplicationService({ networks: [] }) })}
      />
    ),
  },
  {
    // Clicking the text copies it.
    name: "Copied",
    component: () => (
      <SettingsStory
        path={PATH}
        steps={[click(".thing_form code"), waitFor(".notification_toast__item")]}
      />
    ),
  },
];

export default stories;

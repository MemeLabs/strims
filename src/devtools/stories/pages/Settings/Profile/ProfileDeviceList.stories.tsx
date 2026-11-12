// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import { Device } from "../../../../../apis/strims/profile/v1/profile";
import ProfileService, { devices } from "../../../../mocks/profile/service";
import ReplicationService from "../../../../mocks/replication/service";
import { Story } from "../../../../storybook";
import { SettingsStory, click, waitFor } from "../Network/harness";

const PATH = "/settings/profile/devices";

const withDevices = (d: Device[]) => () => ({
  profile: new ProfileService({ devices: d }),
  replication: new ReplicationService({ devices: d }),
});

class FailingReplicationService extends ReplicationService {
  listCheckpoints(): never {
    throw new Error("the profile's store is locked");
  }
}

class FailingDeleteService extends ProfileService {
  deleteDevice(): never {
    throw new Error("the device is still signed in");
  }
}

// The second row: the first device after the current one.
const secondRowMenu = ".thing_table tbody tr:nth-child(2) .thing_table__row_menu";

const stories: Story[] = [
  {
    // This device first, without a menu, then the profile's others.
    name: "Default",
    component: () => <SettingsStory path={PATH} />,
  },
  {
    name: "Current only",
    component: () => <SettingsStory path={PATH} services={withDevices(devices(1))} />,
  },
  {
    name: "Many",
    component: () => <SettingsStory path={PATH} services={withDevices(devices(14))} />,
  },
  {
    name: "Error",
    component: () => (
      <SettingsStory
        path={PATH}
        services={() => ({ replication: new FailingReplicationService() })}
      />
    ),
  },
  {
    name: "Row menu",
    component: () => (
      <SettingsStory path={PATH} steps={[click(`${secondRowMenu} .dropdown__anchor`)]} />
    ),
  },
  {
    // Deleting the first other device: the list reloads one shorter.
    name: "Deleted",
    component: () => (
      <SettingsStory
        path={PATH}
        steps={[
          click(`${secondRowMenu} .dropdown__anchor`),
          click(`${secondRowMenu} button`, "Delete"),
          waitFor(".thing_table tbody tr:nth-child(2) td:nth-child(2)", "web"),
        ]}
      />
    ),
  },
  {
    // Deleting the first other device fails: the error shows above the list.
    name: "Delete error",
    component: () => (
      <SettingsStory
        path={PATH}
        services={() => ({ profile: new FailingDeleteService() })}
        steps={[
          click(`${secondRowMenu} .dropdown__anchor`),
          click(`${secondRowMenu} button`, "Delete"),
          waitFor(".thing_form .input_error"),
        ]}
      />
    ),
  },
];

export default stories;

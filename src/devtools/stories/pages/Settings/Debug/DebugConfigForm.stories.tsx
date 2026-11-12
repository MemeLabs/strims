// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import DebugService from "../../../../mocks/debug/service";
import { Catalog, Story, fixtures } from "../../../../storybook";
import { SettingsStory } from "../Layout.harness";

const PATH = "/settings/debug";

class FailingDebugService extends DebugService {
  setConfig(): never {
    throw new Error("store is read only");
  }
}

const stories: Story[] = [
  {
    // Mock streams on, in fixture network 2.
    name: "Enabled",
    component: () => (
      <SettingsStory
        path={PATH}
        services={() => ({
          debug: new DebugService({
            config: { enableMockStreams: true, mockStreamNetworkKey: fixtures.networkKey(2) },
          }),
        })}
      />
    ),
  },
  {
    name: "Disabled",
    component: () => (
      <SettingsStory
        path={PATH}
        services={() => ({ debug: new DebugService({ config: { enableMockStreams: false } }) })}
      />
    ),
  },
  {
    // Submitted, and setConfig fails.
    name: "Save error",
    component: () => (
      <SettingsStory path={PATH} submit services={() => ({ debug: new FailingDebugService() })} />
    ),
  },
];

export default stories;

// Every Debug story mounts the settings route table at its page's path.
export const catalog: Catalog = { covers: ["pages/Settings/Debug/index.tsx"] };

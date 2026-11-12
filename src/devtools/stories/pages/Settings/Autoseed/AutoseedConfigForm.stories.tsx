// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import AutoseedService from "../../../../mocks/autoseed/service";
import { Catalog, Story } from "../../../../storybook";
import { SettingsStory } from "../Layout.harness";

const PATH = "/settings/autoseed/config";

class FailingAutoseedService extends AutoseedService {
  setConfig(): never {
    throw new Error("store is read only");
  }
}

const stories: Story[] = [
  {
    name: "Enabled",
    component: () => <SettingsStory path={PATH} />,
  },
  {
    name: "Disabled",
    component: () => (
      <SettingsStory
        path={PATH}
        services={() => ({ autoseed: new AutoseedService({ config: { enable: false } }) })}
      />
    ),
  },
  {
    // Submitted, and setConfig fails.
    name: "Save error",
    component: () => (
      <SettingsStory
        path={PATH}
        submit
        services={() => ({ autoseed: new FailingAutoseedService() })}
      />
    ),
  },
];

export default stories;

// Every Autoseed story mounts the settings route table at its page's path.
export const catalog: Catalog = { covers: ["pages/Settings/Autoseed/index.tsx"] };

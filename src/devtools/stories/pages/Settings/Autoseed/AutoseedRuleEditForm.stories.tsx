// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import AutoseedService from "../../../../mocks/autoseed/service";
import { Story } from "../../../../storybook";
import { SettingsStory } from "../Layout.harness";

// Rule 2 has a salt.
const PATH = "/settings/autoseed/rules/2";

class MissingAutoseedService extends AutoseedService {
  getRule(): never {
    throw new Error("rule 2 not found");
  }
}

class FailingAutoseedService extends AutoseedService {
  updateRule(): never {
    throw new Error("store is read only");
  }
}

const stories: Story[] = [
  {
    name: "Default",
    component: () => <SettingsStory path={PATH} />,
  },
  {
    // getRule fails.
    name: "Error",
    component: () => (
      <SettingsStory path={PATH} services={() => ({ autoseed: new MissingAutoseedService() })} />
    ),
  },
  {
    // Submitted, and updateRule fails.
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

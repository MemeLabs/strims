// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import AutoseedService, { rule, rules } from "../../../../mocks/autoseed/service";
import { Story, fixtures } from "../../../../storybook";
import { SettingsStory } from "../Layout.harness";

const PATH = "/settings/autoseed/rules";

const stories: Story[] = [
  {
    name: "Default",
    component: () => <SettingsStory path={PATH} />,
  },
  {
    // A long label, a 32 byte swarm id and more rules than fit.
    name: "Long text",
    component: () => (
      <SettingsStory
        path={PATH}
        services={() => ({
          autoseed: new AutoseedService({
            rules: [
              rule(1, {
                label:
                  "A rule whose label goes on and on, far past what the table's first column can show in one line",
                swarmId: fixtures.bytes("swarm:long", 32),
              }),
              ...rules(12).slice(1),
            ],
          }),
        })}
      />
    ),
  },
  {
    // No rules: the list sends its visitor to the create form.
    name: "Empty",
    component: () => (
      <SettingsStory
        path={PATH}
        services={() => ({ autoseed: new AutoseedService({ rules: [] }) })}
      />
    ),
  },
];

export default stories;

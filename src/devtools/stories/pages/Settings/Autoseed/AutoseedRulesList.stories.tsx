// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import AutoseedRulesList from "../../../../../pages/Settings/Autoseed/AutoseedRulesList";
import AutoseedService, { rule, rules } from "../../../../mocks/autoseed/service";
import { Story, fixtures } from "../../../../storybook";
import { SettingsStory } from "../Layout.harness";
import { SettingsStory as SteppedStory, click, waitFor } from "../Network/harness";

const PATH = "/settings/autoseed/rules";

class FailingDeleteService extends AutoseedService {
  deleteRule(): never {
    throw new Error("the node isn't answering");
  }
}

const firstRowMenu = ".thing_table tbody tr:first-child .thing_table__row_menu";

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
  {
    // The first rule's Delete fails: the error shows above the list (the
    // page alone, under a harness whose steps can open its row menu).
    name: "Delete error",
    component: () => (
      <SteppedStory
        element={<AutoseedRulesList />}
        services={() => ({ autoseed: new FailingDeleteService() })}
        steps={[
          click(`${firstRowMenu} .dropdown__anchor`),
          click(`${firstRowMenu} button`, "Delete"),
          waitFor(".thing_form .input_error"),
        ]}
      />
    ),
  },
];

export default stories;

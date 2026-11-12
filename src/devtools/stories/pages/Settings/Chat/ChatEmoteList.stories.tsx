// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import ChatServerService from "../../../../mocks/chatserver/service";
import { Story } from "../../../../storybook";
import { ChatSettings, click, typeInto, waitFor } from "./harness";

const path = "/settings/chat-servers/1/emotes";

const stories: Story[] = [
  {
    // Every fixture emote: a dot marks the enabled ones, labels show from md widths.
    name: "Default",
    component: () => <ChatSettings path={path} />,
  },
  {
    // Searching matches names and labels.
    name: "Search",
    component: () => (
      <ChatSettings
        path={path}
        act={async (root) =>
          typeInto(await waitFor<HTMLInputElement>(root, ".settings_search__input"), "comfy")
        }
      />
    ),
  },
  {
    name: "No-matches",
    component: () => (
      <ChatSettings
        path={path}
        act={async (root) =>
          typeInto(await waitFor<HTMLInputElement>(root, ".settings_search__input"), "zzz")
        }
      />
    ),
  },
  {
    // Some rows checked, and the menu that enables or disables them open.
    name: "Selected",
    component: () => (
      <ChatSettings
        path={path}
        act={async (root) => {
          const boxes = "tbody .thing_table_checkbox_cell input";
          await waitFor(root, boxes);
          const inputs = root.querySelectorAll<HTMLInputElement>(boxes);
          for (const i of [1, 2, 4]) {
            inputs[i].click();
          }
          await click(root, ".thing_table_dropdown");
        }}
      />
    ),
  },
  {
    name: "Few",
    component: () => (
      <ChatSettings path={path} service={() => new ChatServerService({ emotes: 6 })} />
    ),
  },
  {
    // No emotes: the list sends the page to the create form.
    name: "Empty",
    component: () => (
      <ChatSettings path={path} service={() => new ChatServerService({ emotes: 0 })} />
    ),
  },
];

export default stories;

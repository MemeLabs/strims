// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import ChatServerService from "../../../../mocks/chatserver/service";
import { Story } from "../../../../storybook";
import { ChatSettings, click, failFirstDelete, typeInto, waitFor } from "./harness";

const path = "/settings/chat-servers/1/emotes";

class FailingService extends ChatServerService {
  deleteEmote(): never {
    throw new Error("the chat server isn't answering");
  }

  updateEmotes(): never {
    throw new Error("the chat server isn't answering");
  }
}

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
  {
    // The first emote's Delete fails: the error shows above the table.
    name: "Delete error",
    component: () => (
      <ChatSettings path={path} service={() => new FailingService()} act={failFirstDelete} />
    ),
  },
  {
    // Enabling two checked emotes fails: the error shows above the table.
    name: "Enable error",
    component: () => (
      <ChatSettings
        path={path}
        service={() => new FailingService()}
        act={async (root) => {
          const boxes = "tbody .thing_table_checkbox_cell input";
          await waitFor(root, boxes);
          const inputs = root.querySelectorAll<HTMLInputElement>(boxes);
          for (const i of [1, 2]) {
            inputs[i].click();
          }
          await click(root, ".thing_table_dropdown");
          await click(
            root,
            ".thing_table_dropdown .thing_table_item_dropdown__button",
            (b) => b.textContent === "Enable selected"
          );
          await waitFor(root, ".thing_form .input_error");
        }}
      />
    ),
  },
];

export default stories;

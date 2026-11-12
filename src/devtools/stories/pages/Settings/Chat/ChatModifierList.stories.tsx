// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import ChatServerService from "../../../../mocks/chatserver/service";
import { Story } from "../../../../storybook";
import { ChatSettings, click, failFirstDelete } from "./harness";

const path = "/settings/chat-servers/1/modifiers";

class FailingService extends ChatServerService {
  deleteModifier(): never {
    throw new Error("the chat server isn't answering");
  }
}

const stories: Story[] = [
  {
    name: "Default",
    component: () => <ChatSettings path={path} />,
  },
  {
    name: "Row-menu",
    component: () => (
      <ChatSettings path={path} act={(root) => click(root, ".thing_table_item_dropdown")} />
    ),
  },
  {
    // No modifiers: the list sends the page to the create form.
    name: "Empty",
    component: () => (
      <ChatSettings path={path} service={() => new ChatServerService({ modifiers: 0 })} />
    ),
  },
  {
    // The first modifier's Delete fails: the error shows above the table.
    name: "Delete error",
    component: () => (
      <ChatSettings path={path} service={() => new FailingService()} act={failFirstDelete} />
    ),
  },
];

export default stories;

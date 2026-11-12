// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import ChatServerService from "../../../../mocks/chatserver/service";
import { Story } from "../../../../storybook";
import { ChatSettings, click } from "./harness";

const path = "/settings/chat-servers/1/tags";

const stories: Story[] = [
  {
    name: "Default",
    component: () => <ChatSettings path={path} />,
  },
  {
    name: "Create-menu",
    component: () => (
      <ChatSettings path={path} act={(root) => click(root, ".thing_table_dropdown")} />
    ),
  },
  {
    // No tags: the list sends the page to the create form.
    name: "Empty",
    component: () => (
      <ChatSettings path={path} service={() => new ChatServerService({ tags: 0 })} />
    ),
  },
];

export default stories;

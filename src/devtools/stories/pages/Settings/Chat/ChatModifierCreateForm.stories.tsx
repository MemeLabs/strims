// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import ChatServerService from "../../../../mocks/chatserver/service";
import { Story } from "../../../../storybook";
import { ChatSettings, submit } from "./harness";

const path = "/settings/chat-servers/1/modifiers/new";

const stories: Story[] = [
  {
    // Back to the server's modifiers.
    name: "Default",
    component: () => <ChatSettings path={path} />,
  },
  {
    // The server's first modifier: back to the server.
    name: "First",
    component: () => (
      <ChatSettings path={path} service={() => new ChatServerService({ modifiers: 0 })} />
    ),
  },
  {
    name: "Invalid",
    component: () => <ChatSettings path={path} act={submit} />,
  },
];

export default stories;

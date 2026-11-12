// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import ChatServerService from "../../../../mocks/chatserver/service";
import { Story } from "../../../../storybook";
import { ChatSettings } from "./harness";

class FailingService extends ChatServerService {
  getServerIcon(): never {
    throw new Error("server icon unavailable");
  }
}

const stories: Story[] = [
  {
    name: "Default",
    component: () => <ChatSettings path="/settings/chat-servers/1/icon" />,
  },
  {
    // Server 2 has none yet.
    name: "No-icon",
    component: () => <ChatSettings path="/settings/chat-servers/2/icon" />,
  },
  {
    // Reading the icon fails.
    name: "Error",
    component: () => (
      <ChatSettings path="/settings/chat-servers/1/icon" service={() => new FailingService()} />
    ),
  },
];

export default stories;

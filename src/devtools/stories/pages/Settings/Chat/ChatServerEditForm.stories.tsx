// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import * as chatv1 from "../../../../../apis/strims/chat/v1/chat";
import ChatServerService from "../../../../mocks/chatserver/service";
import { Story } from "../../../../storybook";
import { ChatSettings } from "./harness";

class FailingService extends ChatServerService {
  getServer(): chatv1.GetServerResponse {
    throw new Error("chat server not found");
  }
}

const stories: Story[] = [
  {
    name: "Default",
    component: () => <ChatSettings path="/settings/chat-servers/1" />,
  },
  {
    // Reading the server fails.
    name: "Error",
    component: () => (
      <ChatSettings path="/settings/chat-servers/1" service={() => new FailingService()} />
    ),
  },
];

export default stories;

// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import { firstModifierId } from "../../../../mocks/chatserver/fixtures";
import ChatServerService from "../../../../mocks/chatserver/service";
import { Story } from "../../../../storybook";
import { ChatSettings } from "./harness";

// Fixture modifier `i` of server 1: 0 mirror, 12 banned, 24 virus.
const path = (i: number) => `/settings/chat-servers/1/modifiers/${firstModifierId(1) + BigInt(i)}`;

class FailingService extends ChatServerService {
  getModifier(): never {
    throw new Error("modifier not found");
  }
}

const stories: Story[] = [
  {
    name: "Default",
    component: () => <ChatSettings path={path(0)} />,
  },
  {
    // A style sheet with an image asset.
    name: "Asset",
    component: () => <ChatSettings path={path(12)} />,
  },
  {
    // Internal: its proc chance enabled.
    name: "Internal",
    component: () => <ChatSettings path={path(24)} />,
  },
  {
    // Reading the modifier fails.
    name: "Error",
    component: () => <ChatSettings path={path(0)} service={() => new FailingService()} />,
  },
];

export default stories;

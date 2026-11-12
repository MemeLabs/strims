// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import * as chatv1 from "../../../../../apis/strims/chat/v1/chat";
import { firstTagId } from "../../../../mocks/chatserver/fixtures";
import ChatServerService from "../../../../mocks/chatserver/service";
import { Story } from "../../../../storybook";
import { ChatSettings } from "./harness";

// Fixture tag `i` of server 1: 0 nsfw, 2 loud.
const path = (i: number) => `/settings/chat-servers/1/tags/${firstTagId(1) + BigInt(i)}`;

class FailingService extends ChatServerService {
  getTag(): chatv1.GetTagResponse {
    throw new Error("tag not found");
  }
}

const stories: Story[] = [
  {
    // A sensitive tag.
    name: "Default",
    component: () => <ChatSettings path={path(0)} />,
  },
  {
    name: "Not-sensitive",
    component: () => <ChatSettings path={path(2)} />,
  },
  {
    // Reading the tag fails: the form shows the error, its fields empty.
    name: "Error",
    component: () => <ChatSettings path={path(0)} service={() => new FailingService()} />,
  },
];

export default stories;

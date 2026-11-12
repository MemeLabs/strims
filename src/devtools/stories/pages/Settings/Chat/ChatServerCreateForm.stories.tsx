// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import * as chatv1 from "../../../../../apis/strims/chat/v1/chat";
import ChatServerService from "../../../../mocks/chatserver/service";
import { Story } from "../../../../storybook";
import { Act, ChatSettings, fill, select, submit } from "./harness";

const path = "/settings/chat-servers/new";

const fillForm: Act = async (root) => {
  await fill(root, "Name", "Speedrun Club Chat");
  await select(root, "Network", "Speedrun Club");
};

class FailingService extends ChatServerService {
  createServer(): chatv1.CreateServerResponse {
    throw new Error("a chat server for this network exists already");
  }
}

const stories: Story[] = [
  {
    // With servers to go back to.
    name: "Default",
    component: () => <ChatSettings path={path} />,
  },
  {
    // The first server: nothing to go back to.
    name: "First",
    component: () => (
      <ChatSettings path={path} service={() => new ChatServerService({ servers: 0 })} />
    ),
  },
  {
    name: "Filled",
    component: () => <ChatSettings path={path} act={fillForm} />,
  },
  {
    name: "Invalid",
    component: () => <ChatSettings path={path} act={submit} />,
  },
  {
    name: "Error",
    component: () => (
      <ChatSettings
        path={path}
        service={() => new FailingService()}
        act={async (root) => {
          await fillForm(root);
          await submit(root);
        }}
      />
    ),
    allowErrors: [/a chat server for this network exists already/],
  },
];

export default stories;

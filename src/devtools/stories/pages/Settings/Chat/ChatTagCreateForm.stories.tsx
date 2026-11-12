// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import * as chatv1 from "../../../../../apis/strims/chat/v1/chat";
import ChatServerService from "../../../../mocks/chatserver/service";
import { Story } from "../../../../storybook";
import { Act, ChatSettings, fill, submit } from "./harness";

const path = "/settings/chat-servers/1/tags/new";

class FailingService extends ChatServerService {
  createTag(): chatv1.CreateTagResponse {
    throw new Error("tag name in use");
  }
}

const fillForm: Act = async (root) => {
  await fill(root, "Name", "nsfw");
  await fill(root, "Color", "#e53935");
};

const stories: Story[] = [
  {
    // Back to the server's tags.
    name: "Default",
    component: () => <ChatSettings path={path} />,
  },
  {
    // The server's first tag: back to the server.
    name: "First",
    component: () => (
      <ChatSettings path={path} service={() => new ChatServerService({ tags: 0 })} />
    ),
  },
  {
    name: "Invalid",
    component: () => <ChatSettings path={path} act={submit} />,
  },
  {
    // Creating the tag fails.
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
  },
];

export default stories;

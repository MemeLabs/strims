// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import { server } from "../../../../mocks/chatserver/fixtures";
import ChatServerService from "../../../../mocks/chatserver/service";
import { Catalog, Story } from "../../../../storybook";
import { ChatSettings, click, failFirstDelete } from "./harness";

const path = "/settings/chat-servers";

class FailingService extends ChatServerService {
  deleteServer(): never {
    throw new Error("the chat server is still running");
  }
}

const stories: Story[] = [
  {
    name: "Default",
    component: () => <ChatSettings path={path} />,
  },
  {
    name: "Many",
    component: () => (
      <ChatSettings
        path={path}
        service={() =>
          new ChatServerService({
            servers: [
              ...Array.from({ length: 5 }, (_, i) => server(i + 1)),
              server(6, ""),
              server(
                7,
                "A chat server whose name goes on far longer than any table cell would like it to"
              ),
              ...Array.from({ length: 5 }, (_, i) => server(i + 8)),
            ],
          })
        }
      />
    ),
  },
  {
    name: "Row-menu",
    component: () => (
      <ChatSettings path={path} act={(root) => click(root, ".thing_table_item_dropdown")} />
    ),
  },
  {
    name: "Create-menu",
    component: () => (
      <ChatSettings path={path} act={(root) => click(root, ".thing_table_dropdown")} />
    ),
  },
  {
    // No servers: the list sends the page to the create form.
    name: "Empty",
    component: () => (
      <ChatSettings path={path} service={() => new ChatServerService({ servers: 0 })} />
    ),
  },
  {
    // The first server's Delete fails: the error shows above the table.
    name: "Delete error",
    component: () => (
      <ChatSettings path={path} service={() => new FailingService()} act={failFirstDelete} />
    ),
  },
];

export default stories;

// Every story here mounts the app's settings routes, the chat servers' table among them.
export const catalog: Catalog = { covers: ["pages/Settings/Chat/index.tsx"] };

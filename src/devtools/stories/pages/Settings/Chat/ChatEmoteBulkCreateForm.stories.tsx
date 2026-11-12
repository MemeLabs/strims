// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import * as chatv1 from "../../../../../apis/strims/chat/v1/chat";
import { emoteFixture } from "../../../../mocks/chatserver/fixtures";
import ChatServerService from "../../../../mocks/chatserver/service";
import { Story } from "../../../../storybook";
import { Act, ChatSettings, dropFiles, select, submit, typeInto, waitFor } from "./harness";

const path = "/settings/chat-servers/1/emotes/new-bulk";

// Drops fixture emotes `names` on the form's image drop zone, which stages one
// name and image input pair per file.
const stage =
  (...names: string[]): Act =>
  async (root) => {
    const input = await waitFor<HTMLInputElement>(root, "form .input_image input[type=file]");
    const staged = () =>
      Array.from(root.querySelectorAll("button")).filter((b) => b.textContent === "Remove Emote")
        .length;
    await dropFiles(
      input,
      names.map((name) => ({ name, url: emoteFixture(name).url })),
      (count) => staged() === count
    );
  };

class FailingService extends ChatServerService {
  createEmote(): chatv1.CreateEmoteResponse {
    throw new Error("emote name in use");
  }
}

const stories: Story[] = [
  {
    name: "Default",
    component: () => <ChatSettings path={path} />,
  },
  {
    // Four images dropped, named by their files, with a label for them all.
    name: "Staged",
    component: () => (
      <ChatSettings
        path={path}
        act={async (root) => {
          await stage("PepeLaugh", "ComfyCat", "NODDERS", "AYAYA")(root);
          await select(root, "Labels", "comfy");
        }}
      />
    ),
  },
  {
    // A staged emote's name cleared.
    name: "Invalid",
    component: () => (
      <ChatSettings
        path={path}
        act={async (root) => {
          await stage("PepeLaugh", "ComfyCat")(root);
          typeInto(await waitFor<HTMLInputElement>(root, "input[name=emote_1_name]"), "");
          await submit(root);
        }}
      />
    ),
  },
  {
    // Creating the first emote fails.
    name: "Error",
    component: () => (
      <ChatSettings
        path={path}
        service={() => new FailingService()}
        act={async (root) => {
          await stage("PepeLaugh", "ComfyCat")(root);
          await submit(root);
        }}
      />
    ),
    allowErrors: [/emote name in use/],
  },
];

export default stories;

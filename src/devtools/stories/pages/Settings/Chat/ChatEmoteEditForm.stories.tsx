// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import { emoteId } from "../../../../mocks/chatserver/fixtures";
import ChatServerService from "../../../../mocks/chatserver/service";
import { Story } from "../../../../storybook";
import { ChatSettings, scrollTo } from "./harness";

const path = (name: string) => `/settings/chat-servers/1/emotes/${emoteId(1, name)}`;

class FailingService extends ChatServerService {
  getEmote(): never {
    throw new Error("emote not found");
  }
}

const stories: Story[] = [
  {
    name: "Default",
    component: () => <ChatSettings path={path("PepeLaugh")} />,
  },
  {
    // A sprite sheet: the animation fields enabled.
    name: "Animated",
    component: () => <ChatSettings path={path("catJAM")} />,
  },
  {
    // Animated with a style sheet of its own and a default modifier (scrolled to it).
    name: "Styled",
    component: () => (
      <ChatSettings path={path("REE")} act={(root) => scrollTo(root, "Default modifiers")} />
    ),
  },
  {
    // A disabled emote (scrolled to its toggle).
    name: "Disabled",
    component: () => (
      <ChatSettings path={path("FeelsBadMan")} act={(root) => scrollTo(root, "enable")} />
    ),
  },
  {
    // Reading the emote fails.
    name: "Error",
    component: () => <ChatSettings path={path("PepeLaugh")} service={() => new FailingService()} />,
  },
];

export default stories;

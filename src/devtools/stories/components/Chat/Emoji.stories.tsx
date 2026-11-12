// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Chat and message markup around it, styled as Shell and Message style it.
import "../../../../components/Chat/Message.scss";
import "../../../../components/Chat/Shell.scss";
import "./chat.stories.scss";

import React from "react";

import Emoji from "../../../../components/Chat/Emoji";
import { Story } from "../../../storybook";

interface GridProps {
  items: [label: string, emoji: string][];
}

const Grid: React.FC<GridProps> = ({ items }) => (
  <div className="chat chat_story_grid">
    {items.map(([label, emoji]) => (
      <div key={label} className="chat_story_grid__cell">
        <Emoji title={label}>{emoji}</Emoji>
        <span className="chat_story_grid__label">{label}</span>
      </div>
    ))}
  </div>
);

const wave = "\u{1F44B}";

const stories: Story[] = [
  {
    name: "Default",
    component: () => (
      <Grid
        items={[
          ["grinning face", "😀"],
          ["party popper", "🎉"],
          ["gorilla", "🦍"],
          ["pizza", "🍕"],
          ["red heart", "❤️"],
          ["thumbs up", "👍"],
          ["fire", "🔥"],
          ["eyes", "👀"],
        ]}
      />
    ),
  },
  {
    name: "Skin tones",
    component: () => (
      <Grid
        items={[
          ["default", wave],
          ["light", `${wave}\u{1F3FB}`],
          ["medium-light", `${wave}\u{1F3FC}`],
          ["medium", `${wave}\u{1F3FD}`],
          ["medium-dark", `${wave}\u{1F3FE}`],
          ["dark", `${wave}\u{1F3FF}`],
        ]}
      />
    ),
  },
  {
    name: "Sequences",
    component: () => (
      <Grid
        items={[
          ["flag: United States", "🇺🇸"],
          ["rainbow flag", "🏳️‍🌈"],
          ["pirate flag", "🏴‍☠️"],
          ["man superhero", "🦸‍♂️"],
          ["astronaut", "🧑‍🚀"],
          ["family", "👨‍👩‍👧"],
        ]}
      />
    ),
  },
  {
    name: "Inline",
    component: () => (
      <div className="chat chat_story chat_story--messages">
        <div className="chat__message">
          <span className="chat__message__body">
            text around <Emoji>😀</Emoji> an emoji, and two at the end <Emoji>🎉</Emoji>
            <Emoji>🍕</Emoji>
          </span>
        </div>
      </div>
    ),
  },
];

export default stories;

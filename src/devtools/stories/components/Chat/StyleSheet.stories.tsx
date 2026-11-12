// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./chat.stories.scss";

import React, { useEffect } from "react";

import { useRoom } from "../../../../contexts/Chat";
import { MessageList, MockChat, RoomProvider, RoomStyles } from "../../../contexts/Chat";
import { peerKey, replay } from "../../../mocks/chat/MessageEmitter";
import { chatMessage } from "../../../mocks/chat/messages";
import ChatService, { ChatServiceOptions } from "../../../mocks/chat/service";
import { Story } from "../../../storybook";

// StyleSheet is storied by what it does to the messages in its room's scope
// (RoomStyles renders it as Shell does): emote images, tag colours, and the
// UI config's highlights, tags, ignores, selected peers and hidden tags.

const conversation = [
  chatMessage("soadsoap", "Run boy run PeepoRun they're trying to catch you", (e) => ({
    emotes: [e.emote("PeepoRun")],
  })),
  chatMessage("tahley", "soadsoap a lot of animals are mayors", (e) => ({
    nicks: [e.nick("soadsoap")],
  })),
  chatMessage("widdlez", "bezos is gonna run NODDERS", (e) => ({
    emotes: [e.emote("NODDERS")],
  })),
  chatMessage("biscophan", "widdlez he would be better for Texas", (e) => ({
    nicks: [e.nick("widdlez")],
  })),
  chatMessage("soadsoap", "nsfw strims.gg/weeb", (e) => ({
    tags: [e.tag("nsfw")],
    links: [e.link("strims.gg/weeb")],
  })),
  chatMessage("widdlez", "nsfl don't open strims.gg", (e) => ({
    tags: [e.tag("nsfl")],
    links: [e.link("strims.gg")],
  })),
  chatMessage("tahley", "loud strims.gg/loud", (e) => ({
    tags: [e.tag("loud")],
    links: [e.link("strims.gg/loud")],
  })),
  chatMessage("KartoffelKopf", "maybe we'll see a president FerretLOL in our life time", (e) => ({
    emotes: [e.emote("FerretLOL")],
  })),
];

// Selects peers (as clicking their names does) once the room opens.
const SelectPeers: React.FC<{ aliases: string[] }> = ({ aliases }) => {
  const [, { toggleSelectedPeer }] = useRoom();
  useEffect(() => aliases.forEach((a) => toggleSelectedPeer(peerKey(a), true)), []);
  return null;
};

interface StyledProps {
  options?: ChatServiceOptions;
  selected?: string[];
  caption: string;
}

const Styled: React.FC<StyledProps> = ({ options, selected, caption }) => (
  <MockChat chat={() => new ChatService(replay([]), options)}>
    <RoomProvider>
      <RoomStyles className="chat_story chat_story--messages">
        <p className="chat_story__caption">{caption}</p>
        {selected && <SelectPeers aliases={selected} />}
        <MessageList messages={conversation} />
      </RoomStyles>
    </RoomProvider>
  </MockChat>
);

const stories: Story[] = [
  {
    name: "Emotes",
    component: () => <Styled caption="emote images and tag colours from the room's assets" />,
  },
  {
    name: "Highlights",
    component: () => (
      <Styled options={{ highlights: ["soadsoap"] }} caption="soadsoap is highlighted" />
    ),
  },
  {
    name: "Tags",
    component: () => (
      <Styled
        options={{ tags: { soadsoap: "#e53935", tahley: "#43a047" } }}
        caption="soadsoap tagged red, tahley green"
      />
    ),
  },
  {
    name: "Tagged visibility",
    component: () => (
      <Styled
        options={{
          tags: { soadsoap: "#e53935", tahley: "#43a047" },
          uiConfig: { taggedVisibility: true },
        }}
        caption="tagged users stand out"
      />
    ),
  },
  {
    name: "Ignores",
    component: () => (
      <Styled
        options={{ ignores: ["widdlez"] }}
        caption="widdlez is ignored: their messages are gone"
      />
    ),
  },
  {
    name: "Ignore mentions",
    component: () => (
      <Styled
        options={{ ignores: ["widdlez"], uiConfig: { ignoreMentions: true } }}
        caption="widdlez is ignored, and messages mentioning them go too"
      />
    ),
  },
  {
    name: "Selected peer",
    component: () => (
      <Styled selected={["soadsoap"]} caption="soadsoap selected: the others fade" />
    ),
  },
  {
    name: "Focus mentioned",
    component: () => (
      <Styled
        selected={["soadsoap"]}
        options={{ uiConfig: { focusMentioned: true } }}
        caption="soadsoap selected, mentions of them included"
      />
    ),
  },
  {
    name: "Hide NSFW",
    component: () => (
      <Styled
        options={{ uiConfig: { hideNsfw: true } }}
        caption="messages tagged nsfw or nsfl are hidden"
      />
    ),
  },
];

export default stories;

// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./chat.stories.scss";

import React from "react";

import { MessageState } from "../../../../apis/strims/chat/v1/chat";
import {
  MessageList,
  MessageListProps,
  MockChat,
  RoomProvider,
  RoomStyles,
} from "../../../contexts/Chat";
import { replay } from "../../../mocks/chat/MessageEmitter";
import { chatMessage, directoryRef } from "../../../mocks/chat/messages";
import ChatService, { ChatServiceOptions } from "../../../mocks/chat/service";
import { Story, fixtures } from "../../../storybook";

interface MessageStoryProps extends MessageListProps {
  options?: ChatServiceOptions;
}

// `messages` in the stories' room, with its emotes, tags and UI config.
const MessageStory: React.FC<MessageStoryProps> = ({ options, ...props }) => (
  <MockChat chat={() => new ChatService(replay([]), options)}>
    <RoomProvider>
      <RoomStyles className="chat_story chat_story--messages">
        <MessageList {...props} />
      </RoomStyles>
    </RoomProvider>
  </MockChat>
);

const me = { peerKey: fixtures.profileKey };

const standard = [
  chatMessage("soadsoap", "Idk about the others"),
  chatMessage("abesus", "soadsoap its a classic", (e) => ({ nicks: [e.nick("soadsoap")] })),
  chatMessage("KartoffelKopf", "was there anyone like that before trump?"),
  chatMessage("tahley", "so it's not unheard of"),
];

const continued = [
  chatMessage("tahley", "the first female elected official in the US was put on the ballot"),
  chatMessage("tahley", "as a joke"),
  chatMessage("tahley", "and she did well"),
  chatMessage("Gehirnchirurg", "tahley seems like the joke was on the haters", (e) => ({
    nicks: [e.nick("tahley")],
  })),
  chatMessage("tahley", "many such examples"),
];

const self = [
  chatMessage("widdlez", "bezos is gonna run"),
  chatMessage("soadsoap", "/me runs for president", (e) => ({
    selfMessage: e.span("runs for president"),
  })),
  chatMessage("KartoffelKopf", "/me waves at everyone PeepoRun", (e) => ({
    selfMessage: e.span("waves at everyone PeepoRun"),
    emotes: [e.emote("PeepoRun")],
  })),
  chatMessage("widdlez", "FerretLOL", (e) => ({ emotes: [e.emote("FerretLOL")] })),
];

const greentext = [
  chatMessage("biscophan", ">implying greentext doesn't work", (e) => ({
    greenText: e.span(">implying greentext doesn't work"),
  })),
  chatMessage("soadsoap", ">be me >post an emote PepeLaugh", (e) => ({
    greenText: e.span(">be me >post an emote PepeLaugh"),
    emotes: [e.emote("PepeLaugh")],
  })),
  chatMessage("tahley", "not greentext > when it's not first"),
];

const spoilers = [
  chatMessage("Of_Odin", "spoiler ||dumbledore was gay all along||", (e) => ({
    spoilers: [e.spoiler("||dumbledore was gay all along||")],
  })),
  chatMessage("soadsoap", "revealed: ||snape kills dumbledore PepeLaugh||", (e) => ({
    spoilers: [e.spoiler("||snape kills dumbledore PepeLaugh||")],
    emotes: [e.emote("PepeLaugh")],
  })),
];

const code = [
  chatMessage("jbpratt", "`hacker mode activated`", (e) => ({
    codeBlocks: [e.code("`hacker mode activated`")],
  })),
  chatMessage("widdlez", "run `cargo nextest run -p strims-chat` before you push", (e) => ({
    codeBlocks: [e.code("`cargo nextest run -p strims-chat`")],
  })),
];

const longUrl =
  "https://example.com/a/very/long/path/that/goes/on/and/on/and/on/for/the/sake/of/the/shortener?with=query&and=more";
const links = [
  chatMessage("soadsoap", "strims.gg", (e) => ({ links: [e.link("strims.gg")] })),
  chatMessage("tahley", `long one ${longUrl} shortened`, (e) => ({
    links: [e.link(longUrl, longUrl)],
  })),
  chatMessage("biscophan", "watch https://www.twitch.tv/northernlion with us", (e) => ({
    links: [e.link("https://www.twitch.tv/northernlion", "https://www.twitch.tv/northernlion")],
  })),
  chatMessage("Of_Odin", "https://www.youtube.com/watch?v=dQw4w9WgXcQ", (e) => ({
    links: [
      e.link(
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
      ),
    ],
  })),
];

const emotes = [
  chatMessage("widdlez", "PeepoRun", (e) => ({ emotes: [e.emote("PeepoRun")] })),
  chatMessage("soadsoap", "Run boy run PeepoRun they're trying to catch you PeepoRun", (e) => ({
    emotes: [e.emote("PeepoRun"), e.emote("PeepoRun", [], { nth: 1 })],
  })),
  chatMessage("KartoffelKopf", "LUL at the start, at the end LUL", (e) => ({
    emotes: [e.emote("LUL"), e.emote("LUL", [], { nth: 1 })],
  })),
  chatMessage(
    "biscophan",
    "NODDERS catJAM NODDERS catJAM NODDERS catJAM NODDERS catJAM NODDERS catJAM",
    (e) => ({
      emotes: [0, 1, 2, 3, 4].flatMap((nth) => [
        e.emote("NODDERS", [], { nth }),
        e.emote("catJAM", [], { nth }),
      ]),
    })
  ),
];

// LUL, hidden by the "Emotes" story's config, stays text.
const hidden = chatMessage("tahley", "a hidden emote: LUL stays text", (e) => ({
  emotes: [e.emote("LUL")],
}));

const modifierNames = [
  ["wide"],
  ["mirror"],
  ["flip"],
  ["smol"],
  ["spin"],
  ["rustle"],
  ["banned"],
  ["gold"],
  ["pride"],
  ["love"],
  ["jam"],
  ["wide", "flip"],
];
const modifiers = modifierNames.map((m) => {
  const code = ["FeelsGoodMan", ...m].join(":");
  return chatMessage("soadsoap", `${m.join(" + ")}: ${code}`, (e) => ({
    emotes: [e.emote("FeelsGoodMan", m)],
  }));
});

const emoji = [
  chatMessage("tahley", "nice 😀 very nice", (e) => ({ emojis: [e.emoji("😀")] })),
  chatMessage("soadsoap", "🎉🦍🍕 party", (e) => ({
    emojis: [e.emoji("🎉"), e.emoji("🦍"), e.emoji("🍕")],
  })),
];

const mentions = [
  chatMessage("soadsoap", "KartoffelKopf widdlez I wonder how many more", (e) => ({
    nicks: [e.nick("KartoffelKopf"), e.nick("widdlez")],
  })),
  chatMessage("jbpratt", "test_user you're up next", (e) => ({
    nicks: [{ ...e.nick("test_user"), peerKey: fixtures.profileKey }],
  })),
  chatMessage("tahley", "anyone else on strims tonight?"),
  chatMessage("widdlez", "nobody mentioned anything here"),
];

const tags = [
  chatMessage("Of_Odin", "nsfw check this out google.com", (e) => ({
    tags: [e.tag("nsfw")],
    links: [e.link("google.com")],
  })),
  chatMessage("soadsoap", "loud weeb strims.gg/weeb", (e) => ({
    tags: [e.tag("loud"), e.tag("weeb")],
    links: [e.link("strims.gg/weeb")],
  })),
  chatMessage("biscophan", "untagged strims.gg", (e) => ({ links: [e.link("strims.gg")] })),
];

const combo = (count: number, name = "FeelsGoodMan") =>
  chatMessage("soadsoap", name, (e) => ({ emotes: [e.emote(name, [], { combo: count })] }));

const noon = 20 * 60;
const timestamps = [
  chatMessage("soadsoap", "Idk about the others", undefined, { minute: noon }),
  chatMessage("abesus", "its a classic", undefined, { minute: noon + 1 }),
  chatMessage("abesus", "one of the funniest things", undefined, { minute: noon + 1 }),
  chatMessage("KartoffelKopf", "more of what?", undefined, { minute: noon + 14 }),
];

const sent = [
  chatMessage("soadsoap", "Idk about the others"),
  chatMessage("test_user", "my own message", undefined, me),
  chatMessage("test_user", "and another one", undefined, me),
  chatMessage("tahley", "so it's not unheard of"),
];

const longText = [
  chatMessage("soadsoap", "a".repeat(240)),
  chatMessage(
    "KartoffelKopf",
    "Supercalifragilisticexpialidocious-and-then-some-more-words-joined-by-dashes-without-any-spaces-at-all done"
  ),
  chatMessage(
    "biscophan",
    "soadsoap i think he could have a chance and convince people in texas that he's suited for the comptroller position. gov or lt gov position not likely. he's like a new age ross perot person, and the longer this goes the more it wraps and wraps and wraps over many lines of the chat",
    (e) => ({ nicks: [e.nick("soadsoap")] })
  ),
];

const viewed = [
  chatMessage("soadsoap", "watching eastmancolor", undefined, { viewedListing: directoryRef(37) }),
  chatMessage("tahley", "watching TastelessTV", undefined, { viewedListing: directoryRef(31) }),
  chatMessage("widdlez", "theme coloured", undefined, {
    viewedListing: directoryRef(28, 0x22aa55),
  }),
  chatMessage("biscophan", "not watching anything"),
];

const stories: Story[] = [
  { name: "Standard", component: () => <MessageStory messages={standard} /> },
  { name: "Continued", component: () => <MessageStory messages={continued} /> },
  { name: "Self", component: () => <MessageStory messages={self} /> },
  { name: "Greentext", component: () => <MessageStory messages={greentext} /> },
  {
    name: "Spoiler",
    component: () => (
      <MessageStory
        messages={spoilers}
        onMount={(el) =>
          el.querySelectorAll<HTMLElement>(".chat__message__spoiler").item(1)?.click()
        }
      />
    ),
  },
  {
    name: "Spoilers disabled",
    component: () => (
      <MessageStory messages={spoilers} options={{ uiConfig: { disableSpoilers: true } }} />
    ),
  },
  { name: "Code", component: () => <MessageStory messages={code} /> },
  { name: "Links", component: () => <MessageStory messages={links} /> },
  {
    name: "Links unshortened",
    component: () => (
      <MessageStory messages={links} options={{ uiConfig: { shortenLinks: false } }} />
    ),
  },
  {
    name: "Emotes",
    component: () => (
      <MessageStory
        messages={[...emotes, hidden]}
        options={{ uiConfig: { hiddenEmotes: ["LUL"] } }}
      />
    ),
  },
  { name: "Emote modifiers", component: () => <MessageStory messages={modifiers} /> },
  {
    name: "Modifiers off",
    component: () => (
      <MessageStory messages={modifiers} options={{ uiConfig: { emoteModifiers: false } }} />
    ),
  },
  {
    name: "Compact emotes",
    component: () => (
      <MessageStory messages={emotes} options={{ uiConfig: { compactEmoteSpacing: true } }} />
    ),
  },
  {
    name: "Emotes as text",
    component: () => (
      <MessageStory messages={emotes} options={{ uiConfig: { formatterEmote: false } }} />
    ),
  },
  { name: "Emoji", component: () => <MessageStory messages={emoji} /> },
  {
    name: "Mentions",
    component: () => (
      <MessageStory messages={mentions} options={{ uiConfig: { customHighlight: ["strims"] } }} />
    ),
  },
  { name: "Tags", component: () => <MessageStory messages={tags} /> },
  {
    name: "Combo",
    component: () => (
      <MessageStory
        messages={[
          combo(2),
          chatMessage("tahley", "between combos"),
          combo(5, "PeepoRun"),
          chatMessage("tahley", "between combos"),
          combo(20, "NODDERS"),
          chatMessage("tahley", "between combos"),
          combo(50),
        ]}
        live
      />
    ),
  },
  {
    name: "Timestamps",
    component: () => (
      <MessageStory messages={timestamps} options={{ uiConfig: { showTime: true } }} />
    ),
  },
  {
    name: "Timestamp format",
    component: () => (
      <MessageStory
        messages={timestamps}
        options={{ uiConfig: { showTime: true, timestampFormat: "YYYY-MM-DD h:mm A" } }}
      />
    ),
  },
  { name: "Sent", component: () => <MessageStory messages={sent} /> },
  {
    name: "Failed",
    component: () => (
      <MessageStory
        messages={sent}
        states={[
          MessageState.MESSAGE_STATE_DELIVERED,
          MessageState.MESSAGE_STATE_FAILED,
          MessageState.MESSAGE_STATE_ENQUEUED,
          MessageState.MESSAGE_STATE_DELIVERED,
        ]}
      />
    ),
  },
  {
    // A combo that failed to send is marked as a failed message is; the
    // delivered one after it isn't.
    name: "Combo failed",
    component: () => (
      <MessageStory
        messages={[
          combo(5, "PeepoRun"),
          chatMessage("tahley", "between combos"),
          combo(20, "NODDERS"),
        ]}
        states={[
          MessageState.MESSAGE_STATE_FAILED,
          MessageState.MESSAGE_STATE_DELIVERED,
          MessageState.MESSAGE_STATE_DELIVERED,
        ]}
        live
      />
    ),
  },
  { name: "Long text", component: () => <MessageStory messages={longText} /> },
  { name: "Viewed listings", component: () => <MessageStory messages={viewed} /> },
];

export default stories;

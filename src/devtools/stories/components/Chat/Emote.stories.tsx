// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./chat.stories.scss";

import { Base64 } from "js-base64";
import { isEqual } from "lodash";
import React, { useEffect, useState } from "react";
import Scrollbars from "react-custom-scrollbars-2";

import { AssetBundle, EmoteScale, MessageState } from "../../../../apis/strims/chat/v1/chat";
import Emote, { EmoteProps } from "../../../../components/Chat/Emote";
import ChatMessage from "../../../../components/Chat/Message";
import ChatScroller, { MessageProps } from "../../../../components/Chat/Scroller";
import { useChat, useRoom } from "../../../../contexts/Chat";
import ChatEmoteForm, { ChatEmoteFormData } from "../../../../pages/Settings/Chat/ChatEmoteForm";
import { toEmoteProps } from "../../../../pages/Settings/Chat/utils";
import { MockChat, RoomProvider, RoomStyles } from "../../../contexts/Chat";
import { replay } from "../../../mocks/chat/MessageEmitter";
import { emoteNames, modifierNames } from "../../../mocks/chat/assetBundle";
import imgBrick from "../../../mocks/chat/emotes/static/Brick.png";
import { chatMessage } from "../../../mocks/chat/messages";
import ChatService from "../../../mocks/chat/service";
import { Story, useMockService } from "../../../storybook";

const animatedNames = [
  "BONK",
  "Cinnabunny",
  "ComfyMoobers",
  "DuckJAM",
  "NODDERS",
  "NOPERS",
  "PeepoRun",
  "REE",
  "RIDIN",
  "TANTIES",
  "VroomVroom",
  "WAG",
  "WAYTOODANK",
  "catJAM",
];

interface Cell {
  label: string;
  props: EmoteProps;
}

// Emotes in labelled cells, in the stories' room once its emotes arrive.
const EmoteGrid: React.FC<{ cells: Cell[]; wide?: boolean }> = ({ cells, wide }) => (
  <MockChat chat={() => new ChatService(replay([]))}>
    <RoomProvider>
      <RoomStyles className={wide ? "chat_story_grid chat_story_grid--wide" : "chat_story_grid"}>
        {cells.map(({ label, props }) => (
          <div key={label} className="chat_story_grid__cell">
            <Emote {...props}>{props.name}</Emote>
            <span className="chat_story_grid__label">{label}</span>
          </div>
        ))}
      </RoomStyles>
    </RoomProvider>
  </MockChat>
);

const modifierCells = (name: string): Cell[] => [
  { label: name, props: { name, shouldAnimateForever: true } },
  ...modifierNames.map((m) => ({
    label: `${name}:${m}`,
    props: { name, modifiers: [m], shouldAnimateForever: true },
  })),
  {
    label: `${name}:wide:flip:spin`,
    props: { name, modifiers: ["wide", "flip", "spin"], shouldAnimateForever: true },
  },
];

// A line of text with emotes in it, as a message body lays them out.
const Line: React.FC<{ compactSpacing?: boolean }> = ({ compactSpacing }) => (
  <MockChat chat={() => new ChatService(replay([]))}>
    <RoomProvider>
      <RoomStyles className="chat_story chat_story--messages">
        <div className="chat__message">
          <span className="chat__message__body">
            text above, and an emote <Emote name="PEPE" compactSpacing={compactSpacing} /> in the
            middle of a line long enough to wrap{" "}
            <Emote name="Abathur" compactSpacing={compactSpacing} />
            over lines <Emote name="NODDERS" compactSpacing={compactSpacing} shouldAnimateForever />
            and on.
          </span>
        </div>
      </RoomStyles>
    </RoomProvider>
  </MockChat>
);

const emptyFormData: Omit<ChatEmoteFormData, "image"> = {
  name: "test",
  scale: { value: EmoteScale.EMOTE_SCALE_4X, label: "4x" },
  contributor: "",
  contributorLink: "",
  assetCount: 0,
  scss: "",
  extraWrapCount: 0,
  wrapAdjacent: false,
  animated: false,
  animationFrameCount: 0,
  animationDuration: 0,
  animationIterationCount: 0,
  animationEndOnFrame: 0,
  animationLoopForever: false,
  animationAlternateDirection: false,
  defaultModifiers: [],
  labels: [],
  enable: true,
};

const testerMessages = (() => {
  const messages = [
    "standard variants",
    "test",
    "emote in the test middle of a messages",
    "emote at the end of a lot of text emote at the end of a lot of text emote at the end of a lot of text test",
    `as a wall of emotes ${Array(40).fill("test").join(" ")}`,
    "test",
  ].map((body) =>
    chatMessage("rustler", body, (e) => ({
      emotes: body
        .split(" ")
        .flatMap((w, i, ws) =>
          w === "test"
            ? [e.emote("test", [], { nth: ws.slice(0, i).filter((v) => v === "test").length })]
            : []
        ),
    }))
  );
  messages.push(
    chatMessage("rustler", "test", (e) => ({ emotes: [e.emote("test", [], { combo: 25 })] }))
  );
  return messages;
})();

// The emote "test" made from ChatEmoteForm's values, shown in messages.
const TesterMessages: React.FC<{ formData: ChatEmoteFormData }> = ({ formData }) => {
  const [{ uiConfig }] = useChat();
  const [room] = useRoom();
  const service = useMockService<ChatService>("chat");

  useEffect(() => {
    if (formData) {
      void toEmoteProps(formData).then((emote) =>
        service.emitAssetBundle(
          new AssetBundle({ isDelta: true, emotes: [{ ...emote, id: BigInt(9999), name: "test" }] })
        )
      );
    }
  }, [formData]);

  return (
    <ChatScroller
      renderMessage={({ index, style, ref }: MessageProps) => (
        <ChatMessage
          ref={ref}
          uiConfig={uiConfig}
          message={room.messages[index]}
          messageState={MessageState.MESSAGE_STATE_DELIVERED}
          style={style}
          isMostRecent={false}
        />
      )}
      messageCount={room.messages.length}
      messageSizeCache={room.messageSizeCache}
    />
  );
};

// An emote editor beside messages using what it makes.
const Tester: React.FC = () => {
  const [formData, setFormData] = useState<ChatEmoteFormData>(null);

  useEffect(() => {
    void fetch(imgBrick)
      .then((res) => res.arrayBuffer())
      .then((data) =>
        setFormData({
          ...emptyFormData,
          image: {
            data: Base64.fromUint8Array(new Uint8Array(data)),
            type: "image/png",
            height: 128,
            width: 168,
          },
        })
      );
  }, []);

  const handleChange = (next: ChatEmoteFormData) =>
    setFormData((prev) => (isEqual(prev, next) ? prev : next));

  return (
    <MockChat chat={() => new ChatService(replay(testerMessages), { assetBundle: null })}>
      <RoomProvider>
        <div className="chat_story chat_story--pair">
          <RoomStyles className="chat_story__panel">
            <div className="chat__messages">
              <TesterMessages formData={formData} />
            </div>
          </RoomStyles>
          <Scrollbars>
            {formData && (
              <ChatEmoteForm values={formData} onChange={handleChange} submitLabel="Update Emote" />
            )}
          </Scrollbars>
        </div>
      </RoomProvider>
    </MockChat>
  );
};

const stories: Story[] = [
  {
    name: "All",
    component: () => (
      <EmoteGrid
        cells={emoteNames.map((name) => ({
          label: name,
          props: { name, shouldAnimateForever: true },
        }))}
      />
    ),
  },
  {
    name: "Modifiers",
    component: () => <EmoteGrid cells={modifierCells("FeelsGoodMan")} />,
  },
  {
    name: "Animated modifiers",
    component: () => <EmoteGrid cells={modifierCells("PeepoRun")} />,
  },
  {
    name: "Modifiers hidden",
    component: () => (
      <EmoteGrid
        cells={["wide", "flip", "spin", "banned"].map((m) => ({
          label: `FeelsGoodMan:${m}, not shown`,
          props: { name: "FeelsGoodMan", modifiers: [m], shouldShowModifiers: false },
        }))}
      />
    ),
  },
  {
    name: "Animated",
    component: () => (
      <EmoteGrid
        wide
        cells={animatedNames.flatMap((name) => [
          { label: `${name} looping`, props: { name, shouldAnimateForever: true } },
          { label: `${name} once`, props: { name } },
        ])}
      />
    ),
  },
  { name: "In text", component: () => <Line /> },
  { name: "Compact spacing", component: () => <Line compactSpacing /> },
  {
    name: "Unknown name",
    component: () => (
      <EmoteGrid
        cells={[
          { label: "PEPE", props: { name: "PEPE" } },
          { label: "NotAnEmote (renders nothing)", props: { name: "NotAnEmote" } },
        ]}
      />
    ),
  },
  { name: "Tester", component: () => <Tester /> },
];

export default stories;

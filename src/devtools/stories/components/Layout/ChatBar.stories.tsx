// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./layout_chat.stories.scss";

import { Base64 } from "js-base64";
import React, { useEffect, useRef } from "react";

import ChatBar from "../../../../components/Layout/ChatBar";
import { useChat } from "../../../../contexts/Chat";
import {
  MockChat,
  OpenTopics,
  RoomTopic,
  WhisperTopic,
  roomServerKey,
} from "../../../contexts/Chat";
import { messages, replay } from "../../../mocks/chat/MessageEmitter";
import { chatMessage } from "../../../mocks/chat/messages";
import ChatService, { MockWhisperThread } from "../../../mocks/chat/service";
import { DesktopOnly, Story, fixtures } from "../../../storybook";

const room: RoomTopic = { room: roomServerKey };
const otherRoom: RoomTopic = { room: fixtures.key("room:2") };
const whisper: WhisperTopic = { whisper: "soadsoap" };

interface BarProps {
  /** Runs once the popouts are open, with the bar's frame. */
  onOpen?: (el: HTMLElement) => void;
  caption: string;
}

// The bar along the bottom of the app's body, under the page.
const Bar: React.FC<BarProps> = ({ onOpen, caption }) => {
  const [{ popoutTopics }] = useChat();
  const ref = useRef<HTMLDivElement>(null);
  const open = popoutTopics.length > 0;
  useEffect(() => {
    if (open) {
      onOpen?.(ref.current);
    }
  }, [open]);

  return (
    <div className="layout_chat_story" ref={ref}>
      <div className="layout_chat_story__content">
        <p>{caption}</p>
      </div>
      <div className="layout_chat_story__bar">
        <ChatBar />
      </div>
    </div>
  );
};

interface BarStoryProps extends BarProps {
  topics: (RoomTopic | WhisperTopic)[];
  popouts: number[];
  /**
   * The whisper thread's update from the node once the popouts are open
   * (its unread count), as one sent before the popout's read can come.
   */
  threadUpdate?: Partial<MockWhisperThread>;
}

// Opens the topics, then pops some out once ChatBar has measured its room.
const BarView: React.FC<BarStoryProps> = ({ topics, popouts, threadUpdate, onOpen, ...props }) => {
  const service = useRef<ChatService>(null);
  const handleOpen = (el: HTMLElement) => {
    onOpen?.(el);
    if (threadUpdate) {
      window.setTimeout(() => service.current.emitWhisperThread(0, threadUpdate), 100);
    }
  };
  return (
    <MockChat
      chat={() =>
        (service.current = new ChatService(replay(messages.slice(0, 40)), {
          roomBundles: {
            [Base64.fromUint8Array(otherRoom.room, true)]: { room: { name: "Comfy Corner" } },
          },
          whispers: [
            {
              alias: "soadsoap",
              unreadCount: 1,
              records: [
                { message: chatMessage("soadsoap", "hey, you around tonight?") },
                {
                  message: chatMessage("test_user", "yeah, after 9", undefined, {
                    peerKey: fixtures.profileKey,
                  }),
                },
              ],
            },
          ],
        }))
      }
    >
      <OpenTopics topics={topics} popouts={popouts} />
      <Bar {...props} onOpen={handleOpen} />
    </MockChat>
  );
};

// ChatBar renders nothing on portable devices.
const BarStory: React.FC<BarStoryProps> = (props) => (
  <DesktopOnly note="The chat bar is desktop-only: on phones ChatBar renders nothing, and chats don't pop out.">
    <BarView {...props} />
  </DesktopOnly>
);

const stories: Story[] = [
  {
    name: "Popouts",
    component: () => (
      <BarStory
        topics={[room, otherRoom, whisper]}
        popouts={[0, 2]}
        caption="a room and a whisper thread popped out (as many as fit)"
      />
    ),
  },
  // The whisper thread's update, unread, comes after its popout opened and
  // read it: the open popout keeps it read.
  {
    name: "Popouts, thread updated once open",
    component: () => (
      <BarStory
        topics={[room, otherRoom, whisper]}
        popouts={[0, 2]}
        threadUpdate={{ unreadCount: 1 }}
        caption="the whisper thread's update, one unread, comes once its popout is open"
      />
    ),
  },
  {
    name: "Minimized",
    component: () => (
      <BarStory
        topics={[room, otherRoom]}
        popouts={[0, 1]}
        caption="two rooms popped out (as many as fit), the last minimized"
        onOpen={(el) =>
          window.setTimeout(
            () => el.querySelectorAll<HTMLElement>(".chat_popout__header").item(0)?.click(),
            100
          )
        }
      />
    ),
  },
  {
    name: "Empty",
    component: () => <BarStory topics={[room]} popouts={[]} caption="nothing popped out" />,
  },
];

export default stories;

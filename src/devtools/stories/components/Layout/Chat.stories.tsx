// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./layout_chat.stories.scss";

import { Base64 } from "js-base64";
import React, { useEffect, useRef } from "react";

import LayoutChat from "../../../../components/Layout/Chat";
import { useChat } from "../../../../contexts/Chat";
import { useLayout } from "../../../../contexts/Layout";
import { DEVICE_TYPE, DeviceType } from "../../../../lib/userAgent";
import {
  MockChat,
  OpenTopics,
  OpenTopicsProps,
  RoomTopic,
  WhisperTopic,
  roomServerKey,
} from "../../../contexts/Chat";
import { messages, replay } from "../../../mocks/chat/MessageEmitter";
import { chatMessage } from "../../../mocks/chat/messages";
import ChatService from "../../../mocks/chat/service";
import { StaticNetwork, Story, fixtures } from "../../../storybook";

interface PanelProps {
  /** Runs once the panel mounts, with its column. */
  onMount?: (el: HTMLElement) => void;
  /** Whether the chat is hidden (Layout's showChat), as its toggle does. */
  closed?: boolean;
  /**
   * On a phone or a tablet, the drawer left closed: its Shell mounted, the
   * chat off screen but for the peek.
   */
  drawerClosed?: boolean;
}

// The chat panel once a topic is active (or at once with none), as the app
// shows it on the right of its body; on a phone or a tablet, where the chat
// is the drawer's, as the open drawer shows it.
const Panel: React.FC<PanelProps & { wait: boolean }> = ({
  onMount,
  closed,
  drawerClosed,
  wait,
}) => {
  const [{ mainActiveTopic }] = useChat();
  const { toggleShowChat, toggleOverlayOpen } = useLayout();
  const ref = useRef<HTMLDivElement>(null);
  const ready = !wait || !!mainActiveTopic;

  useEffect(() => {
    if (closed) {
      toggleShowChat(false);
      // The layout is the storybook's: leave it as it was.
      return () => toggleShowChat(true);
    }
  }, []);
  useEffect(() => {
    if (DEVICE_TYPE === DeviceType.Portable && !drawerClosed) {
      toggleOverlayOpen(true);
      return () => toggleOverlayOpen(false);
    }
  }, []);
  useEffect(() => {
    if (ready) {
      onMount?.(ref.current);
    }
  }, [ready]);

  return (
    <div className="layout_chat_story">
      <div className="layout_chat_story__content">
        <p>the page</p>
      </div>
      <div className="layout_chat_story__chat" ref={ref}>
        {ready && <LayoutChat />}
      </div>
    </div>
  );
};

const room: RoomTopic = { room: roomServerKey };
const otherRoom: RoomTopic = { room: fixtures.key("room:2") };
const whisper: WhisperTopic = { whisper: "soadsoap" };

interface ChatStoryProps extends PanelProps, Partial<OpenTopicsProps> {}

const ChatStory: React.FC<ChatStoryProps> = ({ topics = [], active, ...props }) => (
  <MockChat
    chat={() =>
      new ChatService(replay(messages.slice(0, 60)), {
        roomBundles: {
          [Base64.fromUint8Array(otherRoom.room, true)]: { room: { name: "Comfy Corner" } },
        },
        whispers: [
          {
            alias: "soadsoap",
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
      })
    }
  >
    <StaticNetwork>
      <OpenTopics topics={topics} active={active} />
      <Panel wait={topics.length > 0} {...props} />
    </StaticNetwork>
  </MockChat>
);

const openMenu = (el: HTMLElement) => {
  const toggles = el.querySelectorAll<HTMLElement>(".layout_chat__header .layout_chat__toggle");
  toggles.item(toggles.length - 1).click();
};

const stories: Story[] = [
  { name: "No rooms", component: () => <ChatStory /> },
  { name: "Room", component: () => <ChatStory topics={[room]} /> },
  {
    name: "Several topics",
    component: () => <ChatStory topics={[room, otherRoom, whisper]} active={0} />,
  },
  { name: "Whisper", component: () => <ChatStory topics={[room, whisper]} active={1} /> },
  { name: "Menu open", component: () => <ChatStory topics={[room]} onMount={openMenu} /> },
  { name: "Closed", component: () => <ChatStory topics={[room]} closed /> },
  // On a phone or a tablet, what the closed drawer holds: the room's
  // messages arrive while its chat is off screen and count as unread. On a
  // desktop the panel is shown, as in "Room".
  { name: "Room, drawer closed", component: () => <ChatStory topics={[room]} drawerClosed /> },
];

export default stories;

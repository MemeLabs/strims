// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./chat.stories.scss";

import { Base64 } from "js-base64";
import React, { useEffect, useRef } from "react";

import { IAssetBundle } from "../../../../apis/strims/chat/v1/chat";
import { ImageType } from "../../../../apis/strims/type/image";
import RoomCarousel from "../../../../components/Chat/RoomCarousel";
import { useChat } from "../../../../contexts/Chat";
import {
  MockChat,
  OpenTopics,
  OpenTopicsProps,
  RoomTopic,
  WhisperTopic,
} from "../../../contexts/Chat";
import MessageEmitter, { messages, replay } from "../../../mocks/chat/MessageEmitter";
import ChatService, { ChatServiceOptions } from "../../../mocks/chat/service";
import { Story, fixtures, useMockService } from "../../../storybook";

// Room i's server key and name (from 1).
const roomKey = (i: number) => fixtures.key(`room:${i}`);
const room = (i: number): RoomTopic => ({ room: roomKey(i) });
const whisper = (alias: string): WhisperTopic => ({ whisper: alias });

// Room i's name, and its icon (one of the fixture images) when `icon`.
const roomBundle = (i: number, icon: boolean) => async (): Promise<IAssetBundle> => ({
  room: { name: fixtures.networkName(i) },
  icon: icon
    ? {
        type: ImageType.IMAGE_TYPE_PNG,
        data: await fixtures.imageData(fixtures.iconUrls[i % fixtures.iconUrls.length]),
      }
    : undefined,
});

const roomBundles = (count: number, icons: boolean) =>
  Object.fromEntries(
    Array.from({ length: count }, (_, i) => [
      Base64.fromUint8Array(roomKey(i + 1), true),
      roomBundle(i + 1, icons),
    ])
  );

interface CarouselProps {
  /** Runs a moment after mount (the topics open, their labels in), with the carousel's frame. */
  onReady?: (el: HTMLElement) => void;
}

const Carousel: React.FC<CarouselProps> = ({ onReady }) => {
  const [, { setMainActiveTopic }] = useChat();
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (onReady) {
      const id = window.setTimeout(() => onReady(ref.current), 1000);
      return () => clearTimeout(id);
    }
  }, []);
  return (
    <div className="chat_story_header" ref={ref}>
      <RoomCarousel onChange={setMainActiveTopic} />
    </div>
  );
};

// Raises whisper thread 0's unread count once it's open.
const WhisperUnread: React.FC<{ count: number }> = ({ count }) => {
  const service = useMockService<ChatService>("chat");
  useEffect(() => {
    const id = window.setTimeout(() => service.emitWhisperThread(0, { unreadCount: count }), 500);
    return () => clearTimeout(id);
  }, []);
  return null;
};

interface CarouselStoryProps extends OpenTopicsProps, CarouselProps {
  rooms?: number;
  icons?: boolean;
  emitter?: () => MessageEmitter;
  options?: ChatServiceOptions;
  whisperUnread?: number;
}

const CarouselStory: React.FC<CarouselStoryProps> = ({
  rooms = 0,
  icons = true,
  emitter,
  options,
  whisperUnread,
  onReady,
  ...topics
}) => (
  <MockChat
    chat={() =>
      new ChatService(emitter ? emitter() : replay([]), {
        assetBundle: null,
        roomBundles: roomBundles(rooms, icons),
        whispers: [{ alias: "tahley" }, { alias: "KartoffelKopf" }],
        ...options,
      })
    }
  >
    <OpenTopics {...topics} />
    {whisperUnread !== undefined && <WhisperUnread count={whisperUnread} />}
    <div className="chat_story chat_story--messages">
      <Carousel onReady={onReady} />
      {topics.topics.length === 0 && <p className="chat_story__caption">no rooms open</p>}
    </div>
  </MockChat>
);

const mixed = [room(1), room(2), room(3), whisper("tahley"), room(4)];

const stories: Story[] = [
  { name: "Rooms", component: () => <CarouselStory rooms={4} topics={mixed} active={1} /> },
  {
    name: "Labels",
    component: () => <CarouselStory rooms={4} icons={false} topics={mixed} active={0} />,
  },
  {
    name: "Unread",
    component: () => (
      <CarouselStory
        rooms={4}
        topics={mixed}
        active={0}
        hidden={[1, 2, 4]}
        whisperUnread={3}
        // Arrive after the rooms are hidden: every hidden room counts them.
        emitter={() => new MessageEmitter({ ivl: 800, preload: 12, limit: 12, messages })}
      />
    ),
  },
  {
    name: "Unread over 100",
    component: () => (
      <CarouselStory
        rooms={2}
        topics={[room(1), room(2)]}
        active={0}
        hidden={[1]}
        emitter={() => new MessageEmitter({ ivl: 800, preload: 150, limit: 150, messages })}
      />
    ),
  },
  {
    name: "Context menu",
    component: () => (
      <CarouselStory
        rooms={4}
        topics={mixed}
        active={0}
        onReady={(el) => {
          const gem = el.querySelector(".room_carousel__gem");
          const r = gem.getBoundingClientRect();
          gem.dispatchEvent(
            new MouseEvent("contextmenu", {
              bubbles: true,
              cancelable: true,
              clientX: r.left + r.width / 2,
              clientY: r.bottom,
            })
          );
        }}
      />
    ),
  },
  {
    name: "Many",
    component: () => (
      <CarouselStory
        rooms={14}
        topics={Array.from({ length: 14 }, (_, i) => room(i + 1))}
        active={2}
      />
    ),
  },
  { name: "Empty", component: () => <CarouselStory topics={[]} /> },
];

export default stories;

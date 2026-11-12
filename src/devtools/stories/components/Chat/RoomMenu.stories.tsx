// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./chat.stories.scss";

import { PassThrough } from "stream";

import React, { useEffect, useRef, useState } from "react";

import * as directoryv1 from "../../../../apis/strims/network/v1/directory/directory";
import { RoomButtons } from "../../../../components/Chat/RoomMenu";
import { Topic } from "../../../../contexts/Chat";
import { ChatDirectoryService, MockChat, roomUsers } from "../../../contexts/Chat";
import { replay } from "../../../mocks/chat/MessageEmitter";
import ChatService, { MockWhisperThread } from "../../../mocks/chat/service";
import { StaticNetwork, Story, fixtures } from "../../../storybook";

const roomNames = [
  "general",
  "games",
  "music",
  "movie night",
  "off-topic",
  "a room with a long name",
];

// Chat rooms in `networks` networks, `perNetwork` each, with user counts.
class RoomsDirectoryService extends ChatDirectoryService {
  networks: number;
  perNetwork: number;

  constructor(networks: number, perNetwork: number, users: string[] = roomUsers) {
    super(users);
    this.networks = networks;
    this.perNetwork = perNetwork;
  }

  watchListings() {
    const ch = new PassThrough({ objectMode: true });
    const events = Array.from({ length: this.networks }, (_, n) => ({
      event: {
        change: {
          listings: {
            network: fixtures.directoryNetwork(fixtures.network(n + 1)),
            listings: Array.from({ length: this.perNetwork }, (_, i) => ({
              id: BigInt((n + 1) * 100 + i),
              listing: {
                content: {
                  chat: {
                    key: fixtures.key(`room:${n}:${i}`),
                    name: roomNames[i % roomNames.length],
                  },
                },
              },
              snippet: {},
              moderation: {},
              userCount: [3, 1250, 42, 0, 7, 98000][(n + i) % 6],
              recentUserCount: 0,
            })),
          },
        },
      },
    }));
    ch.push(new directoryv1.FrontendWatchListingsResponse({ events }));
    return ch;
  }
}

const threads: MockWhisperThread[] = [
  { alias: "soadsoap", unreadCount: 2, lastMessageTime: fixtures.fixtureTime(31.5) * BigInt(1000) },
  { alias: "tahley", lastMessageTime: fixtures.fixtureTime(30.5) * BigInt(1000) },
  { alias: "a_peer_who_left", lastMessageTime: fixtures.fixtureTime(-40) * BigInt(1000) },
];

interface MenuProps {
  /** The tab to open on mount (default rooms). */
  tab?: "whispers" | "settings";
}

const Menu: React.FC<MenuProps> = ({ tab }) => {
  const [last, setLast] = useState<string>(null);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const i = { whispers: 1, settings: 2 }[tab];
    if (i !== undefined) {
      ref.current.querySelectorAll<HTMLElement>(".room_menu__tab").item(i).click();
    }
  }, []);

  const handleChange = (topic: Topic) => setLast(`opened a ${topic.type.toLowerCase()}`);

  return (
    <div className="chat_story__panel" ref={ref}>
      <RoomButtons onChange={handleChange} onClose={() => setLast("closed")} />
      {last && <p className="chat_story__caption">{last}</p>}
    </div>
  );
};

interface MenuStoryProps extends MenuProps {
  directory?: () => ChatDirectoryService;
  whispers?: MockWhisperThread[];
  wide?: boolean;
}

const MenuStory: React.FC<MenuStoryProps> = ({ directory, whispers = threads, wide, ...props }) => (
  <MockChat chat={() => new ChatService(replay([]), { whispers })} directory={directory}>
    <StaticNetwork>
      <div className={wide ? "chat_story chat_story--wide" : "chat_story"}>
        <Menu {...props} />
      </div>
    </StaticNetwork>
  </MockChat>
);

const stories: Story[] = [
  { name: "Rooms", component: () => <MenuStory /> },
  {
    name: "Many rooms",
    component: () => <MenuStory directory={() => new RoomsDirectoryService(3, 6)} />,
  },
  {
    name: "No rooms",
    component: () => <MenuStory directory={() => new RoomsDirectoryService(0, 0)} />,
  },
  { name: "Whispers", component: () => <MenuStory tab="whispers" /> },
  {
    name: "No whispers",
    component: () => (
      <MenuStory
        tab="whispers"
        whispers={[]}
        directory={() => new RoomsDirectoryService(1, 2, [])}
      />
    ),
  },
  { name: "Settings", component: () => <MenuStory tab="settings" /> },
  { name: "Wide", component: () => <MenuStory wide /> },
];

export default stories;

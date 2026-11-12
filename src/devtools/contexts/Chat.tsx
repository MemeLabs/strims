// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// RoomStyles' .chat and .chat__messages are Shell's.
import "../../components/Chat/Shell.scss";

import { PassThrough } from "stream";

import clsx from "clsx";
import { Base64 } from "js-base64";
import { isEqual } from "lodash";
import React, { ReactNode, useEffect, useRef } from "react";
import { Route } from "react-router-dom";

import { Message, MessageState } from "../../apis/strims/chat/v1/chat";
import * as directoryv1 from "../../apis/strims/network/v1/directory/directory";
import ChatMessage from "../../components/Chat/Message";
import StyleSheet from "../../components/Chat/StyleSheet";
import {
  Provider as ChatProvider,
  ThreadProvider as ChatRoomProvider,
  Topic,
  useChat,
  useRoom,
} from "../../contexts/Chat";
import { Ops, SessionContext, State } from "../../contexts/Session";
import useSize from "../../hooks/useSize";
import { peerKey, replay } from "../mocks/chat/MessageEmitter";
import ChatService from "../mocks/chat/service";
import DirectoryService from "../mocks/directory/service";
import { MockApi, StaticPlayer, StoryRouter, fixtures } from "../storybook";

/** The network the chat mock's rooms are in (network 1's key). */
export const roomNetworkKey = fixtures.testNetworkKey;
/** The directory mock's "test server" chat listing's key, the stories' room. */
export const roomServerKey = Base64.toUint8Array("fHyr7+njRTRAShsdcDB1vOz9373dtPA476Phw+DYh0Q=");
/** The directory mock's other chat listing, "test chat server 2". */
export const otherServerKey = Base64.toUint8Array("oci1E5fBylBj8LQ2TVuV/yJQDmRjJL7+qOkZUy9M2+s=");

/** Aliases in the stories' rooms: the message history's authors among them. */
export const roomUsers = [
  "soadsoap",
  "KartoffelKopf",
  "widdlez",
  "jbpratt",
  "tahley",
  "biscophan",
  "Of_Odin",
  "Gehirnchirurg",
];

// The shared directory mock with users: watchListingUsers says `users`
// joined each room, getUsers lists them (online, for whispers) in network 1.
export class ChatDirectoryService extends DirectoryService {
  users: string[];

  constructor(users = roomUsers) {
    super();
    this.users = users;
  }

  watchListingUsers() {
    const ch = new PassThrough({ objectMode: true });
    ch.push(
      new directoryv1.FrontendWatchListingUsersResponse({
        type: directoryv1.FrontendWatchListingUsersResponse.UserEventType.USER_EVENT_TYPE_JOIN,
        users: this.users.map((alias, i) => ({
          id: BigInt(i + 1),
          alias,
          peerKey: peerKey(alias),
        })),
      })
    );
    return ch;
  }

  getUsers() {
    const network = fixtures.listingsNetwork();
    return new directoryv1.FrontendGetUsersResponse({
      users: this.users.map((alias) => ({
        aliases: [{ alias, networkIds: [network.id] }],
        peerKey: peerKey(alias),
      })),
      networks: new Map([[network.id, network]]),
    });
  }
}

const session: [State, Ops] = [
  { linkedProfiles: [], profile: fixtures.profile(), loading: false },
  {
    createProfile: () => Promise.resolve(null),
    signIn: () => Promise.resolve(true),
    loadLinkedProfiles: () => Promise.resolve([]),
  },
];

export interface MockChatProps {
  /** The chat mock, made once at mount; default an empty room's. */
  chat?: () => ChatService;
  /** The directory mock; default ChatDirectoryService's. */
  directory?: () => DirectoryService;
  children: ReactNode;
}

// The Chat provider over chat and directory mocks, signed in as the fixtures'
// profile, in a StoryRouter of its own (links and listings a message opens
// navigate inside the story) with a static player.
export const MockChat: React.FC<MockChatProps> = ({ chat, directory, children }) => (
  <MockApi
    services={() => {
      const service = chat ? chat() : new ChatService(replay([]));
      return {
        chat: service,
        chatServer: service,
        directory: directory ? directory() : new ChatDirectoryService(),
      };
    }}
  >
    <SessionContext.Provider value={session}>
      <StaticPlayer>
        <StoryRouter>
          <Route path="/" element={<ChatProvider>{children}</ChatProvider>} />
        </StoryRouter>
      </StaticPlayer>
    </SessionContext.Provider>
  </MockApi>
);

export interface RoomProviderProps {
  networkKey?: Uint8Array;
  serverKey?: Uint8Array;
  children: ReactNode;
}

// Opens the room (the stories' by default) and provides its thread once the
// Chat state holds it.
export const RoomProvider: React.FC<RoomProviderProps> = ({
  networkKey = roomNetworkKey,
  serverKey = roomServerKey,
  children,
}) => {
  const key = Base64.fromUint8Array(serverKey, true);

  const [{ rooms }, { openRoom }] = useChat();

  useEffect(() => openRoom(serverKey, networkKey), [Base64.fromUint8Array(networkKey, true), key]);

  if (!rooms.has(key)) {
    return null;
  }

  return (
    <ChatRoomProvider type="ROOM" topicKey={serverKey}>
      {children}
    </ChatRoomProvider>
  );
};

export interface WhisperProviderProps {
  /** The peer, whose key is peerKey(alias): the chat mock's whispers name threads so. */
  alias: string;
  children: ReactNode;
}

// Opens the whisper thread with `alias` and provides it once the Chat state
// holds it.
export const WhisperProvider: React.FC<WhisperProviderProps> = ({ alias, children }) => {
  const key = peerKey(alias);
  const [{ whispers }, { openWhispers }] = useChat();

  useEffect(() => openWhispers(key, [roomNetworkKey], alias), [alias]);

  if (!whispers.has(Base64.fromUint8Array(key, true))) {
    return null;
  }

  return (
    <ChatRoomProvider type="WHISPER" topicKey={key}>
      {children}
    </ChatRoomProvider>
  );
};

/** A room OpenTopics opens: its server key (the stories' network's). */
export type RoomTopic = { room: Uint8Array };
/** A whisper thread OpenTopics opens, with peerKey(alias). */
export type WhisperTopic = { whisper: string };

export interface OpenTopicsProps {
  topics: (RoomTopic | WhisperTopic)[];
  /** Which of them is the main active topic (default the last opened). */
  active?: number;
  /** Which of them are hidden, so messages that arrive count as unread. */
  hidden?: number[];
  /** Which of them are popped out, once ChatBar has room for them. */
  popouts?: number[];
}

const topicOf = (t: RoomTopic | WhisperTopic): Topic =>
  "room" in t
    ? { type: "ROOM", topicKey: t.room }
    : { type: "WHISPER", topicKey: peerKey(t.whisper) };

// Hides a thread as Shell does when it isn't shown.
const HideThread: React.FC = () => {
  const [room, { toggleVisible }] = useRoom();
  useEffect(() => toggleVisible(false), [room.id]);
  return null;
};

// Opens rooms and whisper threads on mount, as the room menu does, then
// makes one active, hides some and pops some out.
export const OpenTopics: React.FC<OpenTopicsProps> = ({
  topics,
  active,
  hidden = [],
  popouts = [],
}) => {
  const [{ rooms, whispers, popoutTopicCapacity }, actions] = useChat();

  useEffect(() => {
    for (const t of topics) {
      if ("room" in t) {
        actions.openRoom(t.room, roomNetworkKey);
      } else {
        actions.openWhispers(peerKey(t.whisper), [roomNetworkKey], t.whisper);
      }
    }
    if (active !== undefined) {
      actions.setMainActiveTopic(topicOf(topics[active]));
    }
  }, []);

  const opened = rooms.size + whispers.size >= topics.length;
  const popped = useRef(false);
  useEffect(() => {
    // Those past the capacity go back to the main topics, as they do in the app.
    if (opened && !popped.current && popouts.length && popoutTopicCapacity > 0) {
      popped.current = true;
      for (const i of popouts) {
        actions.openTopicPopout(topicOf(topics[i]));
      }
    }
  }, [opened, popoutTopicCapacity]);

  return (
    <>
      {opened &&
        hidden.map((i) => (
          <ChatRoomProvider key={i} {...topicOf(topics[i])}>
            <HideThread />
          </ChatRoomProvider>
        ))}
    </>
  );
};

export interface MessageListProps {
  messages: Message[];
  /** Each message's state (default delivered). */
  states?: MessageState[];
  /** Whether the last message is the room's most recent (a combo's is clickable). */
  live?: boolean;
  /** Runs on mount with the list's element (to reveal a spoiler, say). */
  onMount?: (el: HTMLElement) => void;
}

// Messages in the flow as Shell renders them (no scroller), a message
// continued when the next has its author.
export const MessageList: React.FC<MessageListProps> = ({
  messages,
  states = [],
  live,
  onMount,
}) => {
  const [{ uiConfig }] = useChat();
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => onMount?.(ref.current), []);

  return (
    <div ref={ref}>
      {messages.map((message, i) => (
        <ChatMessage
          key={i}
          uiConfig={uiConfig}
          message={message}
          messageState={states[i] ?? MessageState.MESSAGE_STATE_DELIVERED}
          isMostRecent={live && i === messages.length - 1}
          isContinued={isEqual(message.peerKey, messages[i + 1]?.peerKey)}
        />
      ))}
    </div>
  );
};

export interface RoomStylesProps {
  className?: string;
  /** Rendered when the room's emotes have arrived (default at once). */
  waitForEmotes?: boolean;
  children: ReactNode;
}

// The room's scope (#chat-ID) and StyleSheet, with the --chat-width and
// --chat-height Shell gives its content: emotes, tags, highlights and ignores
// take effect inside it, and menus size themselves by it.
export const RoomStyles: React.FC<RoomStylesProps> = ({ className, waitForEmotes, children }) => {
  const [{ uiConfig, uiConfigHighlights, uiConfigTags, uiConfigIgnores }] = useChat();
  const [room] = useRoom();
  const ready = !waitForEmotes || room.liveEmotes.length > 0;
  const ref = useRef<HTMLDivElement>(null);
  const size = useSize(ref);
  return (
    <div
      ref={ref}
      id={`chat-${room.id}`}
      className={clsx("chat", className)}
      style={{
        "--chat-width": size ? `${size.width}px` : "100%",
        "--chat-height": size ? `${size.height}px` : "100%",
      }}
    >
      <StyleSheet
        id={room.id}
        liveEmotes={room.liveEmotes}
        styles={room.styles}
        uiConfig={uiConfig}
        uiConfigHighlights={uiConfigHighlights}
        uiConfigTags={uiConfigTags}
        uiConfigIgnores={uiConfigIgnores}
      />
      {ready && children}
    </div>
  );
};

// Sets an input's value as typing does (through the prototype's setter,
// past React's value tracker), so React's onChange hears it.
export const typeInto = (input: HTMLInputElement, value: string): void => {
  Reflect.set(HTMLInputElement.prototype, "value", value, input);
  input.dispatchEvent(new Event("input", { bubbles: true }));
};

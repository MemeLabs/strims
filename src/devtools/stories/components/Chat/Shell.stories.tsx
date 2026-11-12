// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./chat.stories.scss";

import React from "react";

import { MessageState } from "../../../../apis/strims/chat/v1/chat";
import SettingsDrawer from "../../../../components/Chat/SettingsDrawer";
import Shell from "../../../../components/Chat/Shell";
import { MockChat, RoomProvider, WhisperProvider } from "../../../contexts/Chat";
import { messages, replay } from "../../../mocks/chat/MessageEmitter";
import { chatMessage } from "../../../mocks/chat/messages";
import ChatService, { ChatServiceOptions } from "../../../mocks/chat/service";
import { Story, fixtures } from "../../../storybook";

interface ShellStoryProps {
  /** The room's history (default the mock's ~235 lines). */
  history?: ReturnType<typeof chatMessage>[];
  options?: ChatServiceOptions;
  /** Settings beside the chat, to try them on it. */
  settings?: boolean;
  /** A whisper thread with this alias in place of the room. */
  whisper?: string;
}

const ShellStory: React.FC<ShellStoryProps> = ({
  history = messages,
  options,
  settings,
  whisper,
}) => {
  const shell = <Shell className="chat_story__panel" />;
  return (
    <MockChat chat={() => new ChatService(replay(history), options)}>
      <div className={settings ? "chat_story chat_story--pair" : "chat_story"}>
        {whisper ? (
          <WhisperProvider alias={whisper}>{shell}</WhisperProvider>
        ) : (
          <RoomProvider>{shell}</RoomProvider>
        )}
        {settings && (
          <div className="chat_story__panel">
            <SettingsDrawer />
          </div>
        )}
      </div>
    </MockChat>
  );
};

const me = { peerKey: fixtures.profileKey };
const whisperRecords = [
  { message: chatMessage("soadsoap", "hey, you around tonight?", undefined, { minute: 600 }) },
  {
    message: chatMessage("test_user", "yeah, after 9", undefined, { ...me, minute: 602 }),
  },
  {
    message: chatMessage("soadsoap", "cool, I'll set up the stream", undefined, { minute: 603 }),
  },
  {
    message: chatMessage("test_user", "this one didn't send", undefined, { ...me, minute: 604 }),
    state: MessageState.MESSAGE_STATE_FAILED,
  },
  {
    message: chatMessage("test_user", "and this one is on its way", undefined, {
      ...me,
      minute: 605,
    }),
    state: MessageState.MESSAGE_STATE_ENQUEUED,
  },
];

const stories: Story[] = [
  { name: "Default", component: () => <ShellStory /> },
  { name: "Empty", component: () => <ShellStory history={[]} /> },
  {
    name: "Few messages",
    component: () => <ShellStory history={messages.slice(0, 4)} />,
  },
  {
    name: "Timestamps",
    component: () => <ShellStory options={{ uiConfig: { showTime: true } }} />,
  },
  {
    name: "No emotes",
    component: () => <ShellStory options={{ assetBundle: null }} />,
  },
  {
    name: "Whisper",
    component: () => (
      <ShellStory
        history={[]}
        whisper="soadsoap"
        options={{
          uiConfig: { showTime: true },
          whispers: [{ alias: "soadsoap", records: whisperRecords }],
        }}
      />
    ),
  },
  { name: "With settings", component: () => <ShellStory settings /> },
];

export default stories;

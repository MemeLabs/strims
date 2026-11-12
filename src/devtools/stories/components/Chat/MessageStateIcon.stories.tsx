// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Chat and message markup around it, styled as Shell and Message style it.
import "../../../../components/Chat/Message.scss";
import "../../../../components/Chat/Shell.scss";
import "./chat.stories.scss";

import React from "react";

import { MessageState } from "../../../../apis/strims/chat/v1/chat";
import MessageStateIcon from "../../../../components/Chat/MessageStateIcon";
import { Story } from "../../../storybook";

const states: [string, MessageState][] = [
  ["failed", MessageState.MESSAGE_STATE_FAILED],
  ["unread (nothing)", MessageState.MESSAGE_STATE_UNREAD],
  ["enqueued (nothing)", MessageState.MESSAGE_STATE_ENQUEUED],
  ["delivered (nothing)", MessageState.MESSAGE_STATE_DELIVERED],
  ["read (nothing)", MessageState.MESSAGE_STATE_READ],
];

const stories: Story[] = [
  {
    name: "Failed",
    component: () => (
      <div className="chat chat_story chat_story--messages">
        <div className="chat__message chat__message--failed">
          <MessageStateIcon messageState={MessageState.MESSAGE_STATE_FAILED} />
          <span className="chat__message__body">a whisper that didn&apos;t send</span>
        </div>
      </div>
    ),
  },
  {
    name: "States",
    component: () => (
      <div className="chat chat_story_grid">
        {states.map(([label, state]) => (
          <div key={label} className="chat_story_grid__cell">
            <MessageStateIcon messageState={state} />
            <span className="chat_story_grid__label">{label}</span>
          </div>
        ))}
      </div>
    ),
  },
];

export default stories;

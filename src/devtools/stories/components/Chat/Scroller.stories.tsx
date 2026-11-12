// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./chat.stories.scss";

import React, { useCallback, useEffect, useRef } from "react";

import Message from "../../../../components/Chat/Message";
import Scroller, { MessageProps } from "../../../../components/Chat/Scroller";
import { useChat, useRoom } from "../../../../contexts/Chat";
import { MockChat, RoomProvider, RoomStyles } from "../../../contexts/Chat";
import { messages, replay } from "../../../mocks/chat/MessageEmitter";
import ChatService from "../../../mocks/chat/service";
import { Story } from "../../../storybook";

interface MessagesProps {
  /** Runs once the room's history has arrived, with the scroller's parent. */
  onReady?: (el: HTMLElement) => void;
}

// The room's messages in a Scroller, as Shell renders them.
const Messages: React.FC<MessagesProps> = ({ onReady }) => {
  const [{ uiConfig }] = useChat();
  const [room, roomActions] = useRoom();
  const ref = useRef<HTMLDivElement>(null);

  const renderMessage = useCallback(
    ({ index, style, ref }: MessageProps) => (
      <Message
        uiConfig={uiConfig}
        message={roomActions.getMessage(index)}
        messageState={roomActions.getMessageState(index)}
        style={style}
        isMostRecent={index === roomActions.getMessageCount() - 1}
        isContinued={roomActions.getMessageIsContinued(index)}
        ref={ref}
      />
    ),
    [uiConfig, room.styles]
  );

  const ready = room.messages.length > 0;
  useEffect(() => {
    if (ready && onReady) {
      // After the autoscroll settles, whose scroll events the scroller ignores.
      const id = window.setTimeout(() => onReady(ref.current), 500);
      return () => clearTimeout(id);
    }
  }, [ready]);

  return (
    <div className="chat__messages" ref={ref}>
      <Scroller
        renderMessage={renderMessage}
        messageCount={room.messages.length}
        messageSizeCache={room.messageSizeCache}
      />
    </div>
  );
};

interface ScrollerStoryProps extends MessagesProps {
  history: typeof messages;
}

const ScrollerStory: React.FC<ScrollerStoryProps> = ({ history, ...props }) => (
  <MockChat chat={() => new ChatService(replay(history))}>
    <RoomProvider>
      <RoomStyles className="chat_story">
        {history.length === 0 && <p className="chat_story__caption">no messages yet</p>}
        <Messages {...props} />
      </RoomStyles>
    </RoomProvider>
  </MockChat>
);

// Scrolls to the top, as a reader going back does: autoscroll stops and the
// resume button shows.
const scrollUp = (el: HTMLElement) => {
  // Scrollbars' container, then its view (ScrollerView's div).
  const view = el.firstElementChild.firstElementChild as HTMLElement;
  view.scrollTop = 0;
};

const stories: Story[] = [
  { name: "Default", component: () => <ScrollerStory history={messages} /> },
  { name: "Few messages", component: () => <ScrollerStory history={messages.slice(0, 5)} /> },
  { name: "Empty", component: () => <ScrollerStory history={[]} /> },
  {
    name: "Scrolled up",
    component: () => <ScrollerStory history={messages} onReady={scrollUp} />,
  },
];

export default stories;

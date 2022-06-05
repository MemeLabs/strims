// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./ChatBar.scss";

import clsx from "clsx";
import { Base64 } from "js-base64";
import React, { useCallback, useEffect, useRef } from "react";
import { MdClose } from "react-icons/md";
import { useToggle } from "react-use";

import { RoomProvider, Topic, useChat, useRoom } from "../../contexts/Chat";
import useSize from "../../hooks/useSize";
import { useStableCallback } from "../../hooks/useStableCallback";
import { DEVICE_TYPE, DeviceType } from "../../lib/userAgent";
import Composer from "../Chat/Composer";
import Message from "../Chat/Message";
import Scroller, { MessageProps } from "../Chat/Scroller";
import StyleSheet from "../Chat/StyleSheet";

interface ChatPopoutProps {
  topic: Topic;
}

const ChatPopout: React.FC<ChatPopoutProps> = ({ topic }) => {
  const [{ uiConfig }, { closeTopic, toggleTopicVisible }] = useChat();
  const [room, roomActions] = useRoom();
  const [minimized, toggleMinimized] = useToggle(false);

  useEffect(() => {
    toggleTopicVisible(room.topic, true);
    return () => toggleTopicVisible(room.topic, false);
  }, []);

  const renderMessage = useCallback(
    ({ index, style }: MessageProps) => (
      <Message
        uiConfig={uiConfig}
        message={roomActions.getMessage(index)}
        style={style}
        isMostRecent={index === roomActions.getMessageCount() - 1}
      />
    ),
    [uiConfig, room.styles]
  );

  const handleHeaderClick = useStableCallback(() => {
    toggleMinimized(!minimized);
    toggleTopicVisible(room.topic, minimized);
  });

  const handleCloseClick = useStableCallback(() => closeTopic(topic));

  const className = clsx("chat_popout", {
    "chat_popout--minimized": minimized,
  });

  return (
    <div className={className}>
      <div className="chat_popout__header" onClick={handleHeaderClick}>
        <div className="chat_popout__title">
          {room.label}
          {room.unreadCount > 0 ? ` (${room.unreadCount.toLocaleString()})` : ""}
        </div>
        <div className="chat_popout__controls">
          <button className="chat_popout__control" onClick={handleCloseClick}>
            <MdClose />
          </button>
        </div>
      </div>
      {!minimized && (
        <>
          <StyleSheet liveEmotes={room.liveEmotes} styles={room.styles} uiConfig={uiConfig} />
          <div className="chat_popout__messages">
            <Scroller
              uiConfig={uiConfig}
              renderMessage={renderMessage}
              messageCount={room.messages.length}
              messageSizeCache={room.messageSizeCache}
              onAutoScrollChange={roomActions.toggleMessageGC}
            />
          </div>
          <div className="chat_popout__footer">
            <Composer
              emotes={room.emotes}
              modifiers={room.modifiers}
              tags={room.tags}
              nicks={room.nicks}
              onMessage={roomActions.sendMessage}
            />
          </div>
        </>
      )}
    </div>
  );
};

const ChatBar: React.FC = () => {
  if (DEVICE_TYPE === DeviceType.Portable) {
    return null;
  }

  const [{ popoutTopics }, { setPopoutTopicCapacity }] = useChat();

  const ref = useRef<HTMLDivElement>();
  const size = useSize(ref.current);

  const capacity = Math.floor(size?.width / 330);
  useEffect(() => setPopoutTopicCapacity(capacity), [capacity]);

  const topics = popoutTopics.map((topic) => (
    <RoomProvider key={Base64.fromUint8Array(topic.topicKey, true)} {...topic}>
      <ChatPopout topic={topic} />
    </RoomProvider>
  ));

  return (
    <div ref={ref} className="layout_chat_bar">
      {topics}
    </div>
  );
};

export default ChatBar;

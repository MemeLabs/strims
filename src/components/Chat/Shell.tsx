// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Shell.scss";

import clsx from "clsx";
import React, { useCallback, useEffect, useRef } from "react";
import { Helmet } from "react-helmet";

import twemoji from "../../../assets/chat/TwemojiMozilla.ttf";
import { ThreadInitState, useChat, useRoom } from "../../contexts/Chat";
import useSize from "../../hooks/useSize";
import Composer from "./Composer";
import Message from "./Message";
import Scroller, { MessageProps } from "./Scroller";
import StyleSheet from "./StyleSheet";

interface ShellProps {
  className?: string;
}

const Shell: React.FC<ShellProps> = ({ className }) => {
  const [{ uiConfig }, chatActions] = useChat();
  const [room, roomActions] = useRoom();

  useEffect(() => {
    if (room.state === ThreadInitState.OPEN) {
      roomActions.toggleVisible(true);
      chatActions.resetTopicUnreadCount(room.topic);
      return () => roomActions.toggleVisible(false);
    }
  }, [room.id, room.state]);

  const ref = useRef<HTMLDivElement>(null);
  const size = useSize(ref);

  const renderMessage = useCallback(
    ({ index, style, ref }: MessageProps) => (
      <Message
        uiConfig={uiConfig}
        message={roomActions.getMessage(index)}
        style={style}
        isMostRecent={index === roomActions.getMessageCount() - 1}
        isContinued={roomActions.getMessageIsContinued(index)}
        ref={ref}
      />
    ),
    [uiConfig, room.styles]
  );

  return (
    <div
      ref={ref}
      className={clsx(className, "chat")}
      style={{
        "--chat-width": size ? `${size.width}px` : "100%",
        "--chat-height": size ? `${size.height}px` : "100%",
      }}
    >
      <Helmet link={[{ rel: "preload", as: "font", href: twemoji }]} />
      <StyleSheet liveEmotes={room.liveEmotes} styles={room.styles} uiConfig={uiConfig} />
      <div className="chat__messages">
        <Scroller
          renderMessage={renderMessage}
          messageCount={room.messages.length}
          messageSizeCache={room.messageSizeCache}
        />
      </div>
      <div className="chat__footer">
        <Composer
          emotes={room.emotes}
          modifiers={room.modifiers}
          tags={room.tags}
          nicks={room.nicks}
          commands={room.commands}
          onMessage={roomActions.sendMessage}
        />
      </div>
    </div>
  );
};

export default Shell;

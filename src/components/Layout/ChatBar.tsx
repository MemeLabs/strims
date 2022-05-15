// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./ChatBar.scss";

import clsx from "clsx";
import React, { ReactNode, useCallback, useMemo, useRef } from "react";
import { MdClose } from "react-icons/md";
import { useToggle } from "react-use";

import { RoomProvider, TabGroup, useChat, useRoom } from "../../contexts/Chat";
import useSize from "../../hooks/useSize";
import { DEVICE_TYPE, DeviceType } from "../../lib/userAgent";
import Composer from "../Chat/Composer";
import Message from "../Chat/Message";
import Scroller, { MessageProps } from "../Chat/Scroller";

const ChatWhisper: React.FC = () => {
  const [{ uiConfig }] = useChat();
  const [room, { getMessage, getMessageCount, toggleMessageGC, sendMessage }] = useRoom();
  const [minimized, toggleMinimized] = useToggle(false);

  const renderMessage = useCallback(
    ({ index, style }: MessageProps) => (
      <Message
        uiConfig={uiConfig}
        message={getMessage(index)}
        style={style}
        isMostRecent={index === getMessageCount() - 1}
      />
    ),
    [uiConfig, room.styles]
  );

  const className = clsx("chat_whisper", {
    "chat_whisper--minimized": minimized,
  });

  return (
    <div className={className}>
      <div className="chat_whisper__header" onClick={toggleMinimized}>
        <div className="chat_whisper__title">Test</div>
        <div className="chat_whisper__controls">
          <button className="chat_whisper__control">
            <MdClose />
          </button>
        </div>
      </div>
      {!minimized && (
        <>
          <div className="chat_whisper__messages">
            <Scroller
              uiConfig={uiConfig}
              renderMessage={renderMessage}
              messageCount={room.messages.length}
              messageSizeCache={room.messageSizeCache}
              onAutoScrollChange={toggleMessageGC}
            />
          </div>
          <div className="chat_whisper__footer">
            <Composer
              emotes={room.emotes}
              modifiers={room.modifiers}
              tags={room.tags}
              nicks={room.nicks}
              onMessage={sendMessage}
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

  const ref = useRef<HTMLDivElement>();
  const size = useSize(ref.current);

  // const chatRoom = useMemo(
  //   () => ({
  //     "networkKey": new Uint8Array([
  //       75, 240, 39, 32, 10, 69, 227, 236, 208, 66, 17, 161, 35, 51, 241, 158, 107, 44, 150, 179,
  //       185, 131, 132, 130, 9, 95, 62, 100, 96, 253, 219, 155,
  //     ]),
  //     "serverKey": new Uint8Array([
  //       232, 18, 127, 18, 130, 32, 20, 235, 147, 86, 21, 15, 43, 45, 46, 175, 140, 61, 224, 156, 69,
  //       253, 117, 62, 183, 158, 231, 128, 109, 64, 81, 92,
  //     ]),
  //   }),
  //   []
  // );

  // const count = Math.floor(size?.width / 330);
  // const whispers: ReactNode[] = [];
  // for (let i = 0; i < count; i++) {
  //   whispers.push(
  //     <RoomProvider key={i} {...chatRoom}>
  //       {/* <ChatWhisper /> */}
  //     </RoomProvider>
  //   );
  // }

  return (
    <div ref={ref} className="layout_chat_bar">
      {/* {whispers} */}
    </div>
  );
};

export default ChatBar;

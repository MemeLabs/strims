// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Chat.scss";

import clsx from "clsx";
import React, { useCallback, useRef, useState } from "react";
import { BsArrowBarLeft } from "react-icons/bs";
import { HiOutlineDotsVertical } from "react-icons/hi";
import { useToggle } from "react-use";

import { RoomButtons } from "../../components/Chat/RoomMenu";
import { RoomProvider, RoomProviderProps } from "../../contexts/Chat";
import { useLayout } from "../../contexts/Layout";
import useClickAway from "../../hooks/useClickAway";
import { DEVICE_TYPE, DeviceType } from "../../lib/userAgent";
import RoomCarousel from "../Chat/RoomCarousel";
import ChatShell from "../Chat/Shell";
import SwipablePanel from "../SwipablePanel";

interface HeaderProps {
  onToggleClick: () => void;
  onMenuToggleClick: () => void;
  onChange: (topic: RoomProviderProps) => void;
  selected: RoomProviderProps;
}

const Header: React.FC<HeaderProps> = ({
  onToggleClick,
  onMenuToggleClick,
  onChange,
  selected: topic,
}) => (
  <div className="layout_chat__header">
    {DEVICE_TYPE !== DeviceType.Portable && (
      <button className="layout_chat__toggle layout_chat__toggle--off" onClick={onToggleClick}>
        <BsArrowBarLeft />
      </button>
    )}
    <RoomCarousel className="layout_chat__room_carousel" onChange={onChange} selected={topic} />
    <button className="layout_chat__toggle" onClick={onMenuToggleClick}>
      <HiOutlineDotsVertical />
    </button>
  </div>
);

const Chat: React.FC = () => {
  const { showChat, toggleShowChat } = useLayout();
  const onToggleClick = useCallback(() => toggleShowChat(), []);

  const [menuOpen, toggleMenuOpen] = useToggle(false);
  const onMenuToggleClick = useCallback(() => toggleMenuOpen(), []);

  const ref = useRef<HTMLDivElement>();
  useClickAway(ref, () => toggleMenuOpen(false));

  const [topic, setTopic] = useState<RoomProviderProps>(null);

  const handleRoomMenuChange = useCallback((topic: RoomProviderProps) => {
    toggleMenuOpen(false);
    setTopic(topic);
    console.log({ topic });
  }, []);

  return (
    <div
      className={clsx({
        "layout_chat": true,
        "layout_chat--closed": !showChat,
      })}
      ref={ref}
    >
      <button className="layout_chat__toggle layout_chat__toggle--on" onClick={onToggleClick}>
        <BsArrowBarLeft />
      </button>
      <div className="layout_chat__body">
        <SwipablePanel
          open={menuOpen}
          onToggle={toggleMenuOpen}
          className="layout_chat__foo"
          direction="left"
          filterDeviceTypes={null}
          preventScroll={true}
        >
          <RoomButtons onChange={handleRoomMenuChange} />
        </SwipablePanel>
        <Header
          onToggleClick={onToggleClick}
          onMenuToggleClick={onMenuToggleClick}
          onChange={setTopic}
          selected={topic}
        />
        {topic && (
          <RoomProvider {...topic}>
            <ChatShell className="home_page__chat" shouldHide={closed} />
          </RoomProvider>
        )}
      </div>
    </div>
  );
};

export default Chat;

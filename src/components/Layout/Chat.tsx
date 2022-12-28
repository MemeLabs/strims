// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Chat.scss";

import clsx from "clsx";
import React, { useCallback, useRef } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { BsArrowBarLeft } from "react-icons/bs";
import { HiOutlineDotsVertical } from "react-icons/hi";
import { useToggle } from "react-use";

import { RoomButtons } from "../../components/Chat/RoomMenu";
import { ThreadProvider, ThreadProviderProps, useChat } from "../../contexts/Chat";
import { useLayout } from "../../contexts/Layout";
import useClickAway from "../../hooks/useClickAway";
import { DEVICE_TYPE, DeviceType } from "../../lib/userAgent";
import RoomCarousel from "../Chat/RoomCarousel";
import ChatShell from "../Chat/Shell";
import SwipablePanel from "../SwipablePanel";

interface HeaderProps {
  onToggleClick: () => void;
  onMenuToggleClick: () => void;
  onChange: (topic: ThreadProviderProps) => void;
}

const Header: React.FC<HeaderProps> = ({ onToggleClick, onMenuToggleClick, onChange }) => (
  <div className="layout_chat__header">
    <button className="layout_chat__toggle layout_chat__toggle--off" onClick={onToggleClick}>
      <BsArrowBarLeft />
    </button>
    <RoomCarousel className="layout_chat__room_carousel" onChange={onChange} />
    <button className="layout_chat__toggle" onClick={onMenuToggleClick}>
      <HiOutlineDotsVertical />
    </button>
  </div>
);

const Chat: React.FC = () => {
  const { showChat, toggleShowChat, swapMainPanels, setOverlayState } = useLayout();
  const [{ mainActiveTopic }, { setMainActiveTopic }] = useChat();
  const [menuOpenToggled, toggleMenuOpen] = useToggle(!mainActiveTopic);

  const menuLocked = !mainActiveTopic;
  const menuOpen = menuOpenToggled || menuLocked;

  const ref = useRef<HTMLDivElement>();
  useClickAway(ref, () => toggleMenuOpen(menuLocked));

  useHotkeys("alt+r", () => toggleShowChat(), {
    enableOnContentEditable: true,
    enableOnFormTags: ["INPUT"],
  });

  const handleToggleClick = useCallback(() => {
    if (DEVICE_TYPE === DeviceType.Portable) {
      setOverlayState(({ open }) => ({
        open: !open,
        transitioning: false,
      }));
    } else {
      toggleShowChat();
    }
  }, []);

  const handleMenuToggleClick = useCallback(() => toggleMenuOpen(), []);

  const handleRoomMenuChange = useCallback((topic: ThreadProviderProps) => {
    toggleMenuOpen(false);
    setMainActiveTopic(topic);
  }, []);

  const handleRoomMenuClose = useCallback(() => {
    if (!menuLocked) {
      toggleMenuOpen(false);
    } else if (DEVICE_TYPE === DeviceType.Portable) {
      setOverlayState(({ open }) => ({
        open: !open,
        transitioning: false,
      }));
    } else {
      toggleShowChat(false);
    }
  }, [menuLocked]);

  return (
    <div
      className={clsx({
        "layout_chat": true,
        "layout_chat--closed": !showChat,
      })}
      ref={ref}
    >
      <button className="layout_chat__toggle layout_chat__toggle--on" onClick={handleToggleClick}>
        <BsArrowBarLeft />
      </button>
      <div className="layout_chat__body">
        <SwipablePanel
          open={menuOpen}
          locked={menuLocked}
          onToggle={toggleMenuOpen}
          className={clsx("layout_chat__menu", {
            "layout_chat__menu--locked": menuLocked,
          })}
          direction={swapMainPanels ? "right" : "left"}
          filterDeviceTypes={null}
          preventScroll={true}
        >
          {menuOpen && (
            <RoomButtons onChange={handleRoomMenuChange} onClose={handleRoomMenuClose} />
          )}
        </SwipablePanel>
        <Header
          onToggleClick={handleToggleClick}
          onMenuToggleClick={handleMenuToggleClick}
          onChange={setMainActiveTopic}
        />
        {showChat && mainActiveTopic && (
          <ThreadProvider {...mainActiveTopic}>
            <ChatShell />
          </ThreadProvider>
        )}
      </div>
    </div>
  );
};

export default Chat;

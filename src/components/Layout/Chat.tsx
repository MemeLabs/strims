import "./Chat.scss";

import clsx from "clsx";
import { Base64 } from "js-base64";
import React, { useCallback, useRef, useState } from "react";
import Scrollbars from "react-custom-scrollbars-2";
import { BsArrowBarLeft, BsArrowBarRight } from "react-icons/bs";
import { FiMenu } from "react-icons/fi";
import { useToggle } from "react-use";

import { RoomDropdown, RoomList, RoomMenuItem } from "../../components/Chat/RoomMenu";
import { RoomProvider } from "../../contexts/Chat";
import { useLayout } from "../../contexts/Layout";
import useClickAway from "../../hooks/useClickAway";
import { DEVICE_TYPE, DeviceType } from "../../lib/userAgent";
import ChatShell from "../Chat/Shell";

interface PortableHeaderProps {
  onRoomChange: (item: RoomMenuItem) => void;
}

const PortableHeader: React.FC<PortableHeaderProps> = ({ onRoomChange }) => {
  const [showRoomMenu, toggleShowRoomMenu] = useToggle(true);

  const handleRoomListChange = useCallback((item: RoomMenuItem) => {
    onRoomChange(item);
    toggleShowRoomMenu(false);
  }, []);

  const button = useRef<HTMLButtonElement>(null);
  const list = useRef<HTMLDivElement>(null);
  useClickAway([button, list], () => toggleShowRoomMenu(false));

  return (
    <header
      className={clsx({
        "layout_chat__header": true,
        "layout_chat__header--open": showRoomMenu,
      })}
    >
      <button
        ref={button}
        className="layout_chat__header__room_list_button"
        onClick={() => toggleShowRoomMenu()}
      >
        <FiMenu size={22} />
      </button>
      <Scrollbars className="layout_chat__header__room_list" autoHide>
        <RoomList ref={list} onChange={handleRoomListChange} />
      </Scrollbars>
    </header>
  );
};

interface DesktopHeaderProps {
  onToggleClick: () => void;
  onRoomChange: (item: RoomMenuItem) => void;
  selectedRoom: RoomMenuItem;
}

const DesktopHeader: React.FC<DesktopHeaderProps> = ({
  onToggleClick,
  onRoomChange,
  selectedRoom,
}) => (
  <header className="layout_chat__header">
    <button className="layout_chat__toggle_off" onClick={onToggleClick}>
      <BsArrowBarRight size={22} />
    </button>
    <RoomDropdown onChange={onRoomChange} defaultSelection={selectedRoom} />
  </header>
);

const Chat: React.FC = () => {
  const { showChat, toggleShowChat } = useLayout();

  const [chatRoom, setChatRoom] = useState<RoomMenuItem>({
    networkKey: Base64.toUint8Array("cgqhekoCTcy7OOkRdbNbYG3J4svZorYlH3KKaT660BE="),
    serverKey: Base64.toUint8Array("fHyr7+njRTRAShsdcDB1vOz9373dtPA476Phw+DYh0Q="),
    name: "test",
  });

  return (
    <div
      className={clsx({
        "layout_chat": true,
        "layout_chat--closed": !showChat,
      })}
    >
      <button className="layout_chat__toggle_on" onClick={() => toggleShowChat()}>
        <BsArrowBarLeft size={22} />
      </button>
      <div className="layout_chat__body">
        {DEVICE_TYPE === DeviceType.Portable ? (
          <PortableHeader onRoomChange={setChatRoom} />
        ) : (
          <DesktopHeader
            onToggleClick={() => toggleShowChat()}
            onRoomChange={setChatRoom}
            selectedRoom={chatRoom}
          />
        )}
        <RoomProvider {...chatRoom}>
          <ChatShell className="home_page__chat" shouldHide={closed} />
        </RoomProvider>
      </div>
    </div>
  );
};

export default Chat;

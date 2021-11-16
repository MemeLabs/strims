import "./RoomMenu.scss";

import { Base64 } from "js-base64";
import React, { useContext, useMemo, useState } from "react";
import { MdArrowDropDown } from "react-icons/md";

import * as directoryv1 from "../../apis/strims/network/v1/directory/directory";
import { DirectoryContext } from "../../contexts/Directory";
import Dropdown from "../Dropdown";

export interface RoomMenuItem {
  key?: string;
  networkKey: Uint8Array;
  serverKey: Uint8Array;
  name: string;
}

export interface RoomMenuProps {
  onChange?: (item: RoomMenuItem) => void;
}

const RoomMenu: React.FC<RoomMenuProps> = ({ onChange }) => {
  const [directories] = useContext(DirectoryContext);
  const chats = useMemo(() => {
    const chats: RoomMenuItem[] = [];
    for (const { networkKey, listings } of Object.values(directories)) {
      for (const { listing } of listings) {
        if (listing.content.case === directoryv1.Listing.ContentCase.CHAT) {
          const { key, name } = listing.content.chat;
          chats.push({
            key: Base64.fromUint8Array(key),
            networkKey,
            serverKey: key,
            name,
          });
        }
      }
    }
    return chats.sort((a, b) => a.name.localeCompare(b.name));
  }, [directories]);

  const [selection, setSelection] = useState<RoomMenuItem>(null);
  const handleItemClick = (item) => {
    setSelection(item);
    onChange?.(item);
  };

  return (
    <Dropdown
      baseClassName="room_menu"
      anchor={
        <>
          <div className="room_menu__text">{selection?.name}</div>
          <MdArrowDropDown className="room_menu__icon" />
        </>
      }
      items={chats.map((chat) => (
        <button className="room_menu__item" onClick={() => handleItemClick(chat)} key={chat.key}>
          {chat.name}
        </button>
      ))}
    />
  );
};

export default RoomMenu;

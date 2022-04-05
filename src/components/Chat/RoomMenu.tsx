import "./RoomMenu.scss";

import { Base64 } from "js-base64";
import React, { forwardRef, useContext, useMemo, useState } from "react";
import { MdArrowDropDown } from "react-icons/md";

import * as directoryv1 from "../../apis/strims/network/v1/directory/directory";
import { DirectoryContext } from "../../contexts/Directory";
import Dropdown from "../Dropdown";

export interface RoomMenuItem {
  key?: string;
  directoryListingId?: bigint;
  networkKey: Uint8Array;
  serverKey: Uint8Array;
  name: string;
}

export interface RoomMenuProps {
  onChange?: (item: RoomMenuItem) => void;
}

export const RoomButtons: React.FC<RoomMenuProps> = ({ onChange }) => {
  const { directories } = useContext(DirectoryContext);
  const chats = useMemo(() => {
    const chats: RoomMenuItem[] = [];
    for (const { networkKey, listings } of Object.values(directories)) {
      for (const { id, listing } of listings.values()) {
        if (listing?.content?.case === directoryv1.Listing.ContentCase.CHAT) {
          const { key, name } = listing.content.chat;
          chats.push({
            key: Base64.fromUint8Array(key),
            directoryListingId: id,
            networkKey,
            serverKey: key,
            name,
          });
        }
      }
    }
    return chats.sort((a, b) => a.name.localeCompare(b.name));
  }, [directories]);

  return (
    <>
      {chats.map((chat) => (
        <button className="room_menu__item" onClick={() => onChange?.(chat)} key={chat.key}>
          {chat.name}
        </button>
      ))}
    </>
  );
};

export const RoomList = forwardRef<HTMLDivElement, RoomMenuProps>((props, ref) => (
  <div ref={ref} className="room_list">
    <RoomButtons {...props} />
  </div>
));

interface RoomDropdownPsop extends RoomMenuProps {
  defaultSelection: RoomMenuItem;
}

export const RoomDropdown: React.FC<RoomDropdownPsop> = ({ onChange, defaultSelection }) => {
  const [selection, setSelection] = useState<RoomMenuItem>(defaultSelection);
  const handleChange = (item: RoomMenuItem) => {
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
      items={<RoomButtons onChange={handleChange} />}
    />
  );
};

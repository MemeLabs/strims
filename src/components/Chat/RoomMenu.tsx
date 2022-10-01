// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./RoomMenu.scss";

import clsx from "clsx";
import date from "date-and-time";
import { Base64 } from "js-base64";
import React, { ReactNode, useCallback, useContext, useMemo, useRef, useState } from "react";
import Scrollbars from "react-custom-scrollbars-2";
import { BsArrowBarRight } from "react-icons/bs";
import { FiSettings } from "react-icons/fi";
import { HiOutlineChatAlt2, HiOutlineUser, HiOutlineUsers } from "react-icons/hi";

import { WhisperThread } from "../../apis/strims/chat/v1/chat";
import * as directoryv1 from "../../apis/strims/network/v1/directory/directory";
import { Topic, useChat } from "../../contexts/Chat";
import { useCall, useClient } from "../../contexts/FrontendApi";
import { useLayout } from "../../contexts/Layout";
import { NetworkContext } from "../../contexts/Network";
import { useListings } from "../../hooks/directory";
import useSize from "../../hooks/useSize";
import { useStableCallback } from "../../hooks/useStableCallback";
import { certificateRoot } from "../../lib/certificate";
import { MenuItem, useContextMenu } from "../ContextMenu";
import SettingsDrawer from "./SettingsDrawer";

enum Tab {
  Rooms,
  Whispers,
  Settings,
}

interface TabsProps<T> {
  onChange: (tab: T) => void;
  active: T;
  tabs: { key: T; label: ReactNode }[];
}

const Tabs = <T extends number>({ onChange, active, tabs }: TabsProps<T>) => (
  <div className="room_menu__tab_bar">
    {tabs.map(({ key, label }) => (
      <button
        key={key}
        className={clsx("room_menu__tab", {
          "room_menu__tab--active": key === active,
        })}
        onClick={() => onChange(key)}
      >
        {label}
      </button>
    ))}
  </div>
);

interface RoomMenuPropsBase {
  onChange?: (item: Topic) => void;
}

interface RoomMenuProps extends RoomMenuPropsBase {
  onClose: () => void;
}

export const RoomButtons: React.FC<RoomMenuProps> = ({ onChange, onClose }) => {
  const [activeTab, setActiveTab] = useState<Tab>(Tab.Rooms);
  const { toggleShowChat } = useLayout();
  const [{ mainActiveTopic }] = useChat();

  const tabs = useMemo(
    () => [
      {
        key: Tab.Rooms,
        label: (
          <>
            <HiOutlineChatAlt2 className="room_menu__tab__icon" title="rooms" />
            <span className="room_menu__tab__label">rooms</span>
          </>
        ),
      },
      {
        key: Tab.Whispers,
        label: (
          <>
            <HiOutlineUsers className="room_menu__tab__icon" title="whispers" />
            <span className="room_menu__tab__label">whispers</span>
          </>
        ),
      },
      {
        key: Tab.Settings,
        label: (
          <>
            <FiSettings className="room_menu__tab__icon" title="settings" />
            <span className="room_menu__tab__label">settings</span>
          </>
        ),
      },
    ],
    []
  );

  const handleToggleClick = useCallback(() => {
    toggleShowChat();
  }, []);
  const list = (() => {
    switch (activeTab) {
      case Tab.Rooms:
        return <RoomsList onChange={onChange} />;
      case Tab.Whispers:
        return <WhispersList onChange={onChange} />;
      case Tab.Settings:
        return <SettingsDrawer />;
    }
  })();

  const ref = useRef();
  const size = useSize(ref);

  return (
    <div
      className={clsx({
        "room_menu": true,
        "room_menu--wide": size?.width > 400,
      })}
      ref={ref}
    >
      <div className="room_menu__header">
        {!mainActiveTopic && (
          <button className="room_menu__toggle--off" onClick={handleToggleClick}>
            <BsArrowBarRight />
          </button>
        )}

        {/* <button className="room_menu__toggle--on" onClick={onClose}>
          <BsArrowBarRight />
        </button> */}
        <Tabs onChange={setActiveTab} active={activeTab} tabs={tabs} />
      </div>
      <div className="room_menu__content">{list}</div>
    </div>
  );
};

interface RoomsListItemProps extends RoomMenuPropsBase {
  networkKey: Uint8Array;
  serverKey: Uint8Array;
  userCount: number;
  name: string;
}

const RoomsListItem: React.FC<RoomsListItemProps> = ({
  onChange,
  networkKey,
  serverKey,
  userCount,
  name,
}) => {
  const [, { openRoom }] = useChat();

  const handleClick = useStableCallback(() => {
    openRoom(serverKey, networkKey);
    onChange({ type: "ROOM", topicKey: serverKey });
    console.log("rooooms prooopsss", name, openRoom);
  });

  return (
    <button className="rooms_list__network_rooms_item" onClick={handleClick}>
      <span className="rooms_list__network_rooms_item__name">{name}</span>
      <span className="rooms_list__network_rooms_item__dash">&mdash;</span>
      <span className="rooms_list__network_rooms_item__viewers">
        {userCount.toLocaleString()}
        <HiOutlineUser />
      </span>
    </button>
  );
};

const roomListingsReq = {
  contentTypes: [directoryv1.ListingContentType.LISTING_CONTENT_TYPE_CHAT],
};

const RoomsList: React.FC<RoomMenuPropsBase> = ({ onChange }) => {
  const { networkListings } = useListings(roomListingsReq);

  const content: ReactNode[] = [];
  for (const { network, listings } of networkListings.values()) {
    const rooms: ReactNode[] = [];
    for (const { id, listing, userCount } of listings.values()) {
      if (listing.content.case === directoryv1.Listing.ContentCase.CHAT) {
        rooms.push(
          <RoomsListItem
            key={id.toString()}
            networkKey={network.key}
            serverKey={listing.content.chat.key}
            name={listing.content.chat.name}
            userCount={userCount}
            onChange={onChange}
          />
        );
      }
    }

    if (rooms.length) {
      content.push(
        <div key={network.id.toString()} className="rooms_list__network">
          <div className="rooms_list__network_name">{network.name}</div>
          <div className="rooms_list__network_rooms">{rooms}</div>
        </div>
      );
    }
  }

  return (
    <Scrollbars autoHide={true} className="rooms_list">
      <div className="rooms_list__content">
        <h4 className="rooms_list__header">rooms</h4>
        {content}
      </div>
    </Scrollbars>
  );
};

const useMessageTimeFormatter = () => {
  // TODO: load formats from localization config
  return (time: Date) => {
    const now = new Date();
    const sameYear = now.getFullYear() === time.getFullYear();
    const sameMonth = sameYear && now.getMonth() === time.getMonth();
    const sameDay = sameMonth && now.getDate() === time.getDate();

    if (sameDay) {
      return date.format(time, "h:mm A");
    }
    if (sameYear) {
      return date.format(time, "MMM DD");
    }
    return date.format(time, "M/D/YY");
  };
};

interface WhispersListItemProps extends RoomMenuPropsBase {
  thread: WhisperThread;
  online: boolean;
}

const WhispersListItem: React.FC<WhispersListItemProps> = ({ onChange, thread, online }) => {
  const [, { openWhispers }] = useChat();
  const formatMessageTime = useMessageTimeFormatter();

  // TODO: get these from somewhere meaningful... the thread? directory?
  const { items } = useContext(NetworkContext);
  const networkKeys = useMemo(
    () => items.map((i) => certificateRoot(i.network.certificate).key),
    [items]
  );

  const handleClick = useStableCallback(() => {
    openWhispers(thread.peerKey, networkKeys);
    onChange({ type: "WHISPER", topicKey: thread.peerKey });
  });

  const { openMenu, closeMenu, Menu } = useContextMenu();
  const client = useClient();

  const handleContextMenu = useStableCallback((e: React.MouseEvent) => {
    e.preventDefault();
    openMenu(e);
  });

  const handleDeleteThreadClick = useStableCallback(() => {
    void client.chat.deleteWhisperThread({ threadId: thread.id });
    closeMenu();
  });

  return (
    <>
      <tr
        key={thread.id.toString()}
        className="whispers_list__row"
        onClick={handleClick}
        onContextMenu={handleContextMenu}
      >
        <td className="whispers_list__status">
          <span
            className={clsx({
              "whispers_list__status__icon": true,
              "whispers_list__status__icon--online": online,
            })}
            title={online ? "online" : "offline"}
          />
        </td>
        <td className="whispers_list__label">
          <span className="whispers_list__alias">{thread.alias}</span>
          {thread.unreadCount > 0 && (
            <span className="whispers_list__unread">({thread.unreadCount})</span>
          )}
        </td>
        <td className="whispers_list__time">
          {formatMessageTime(new Date(Number(thread.lastMessageTime)))}
        </td>
      </tr>
      <Menu>
        <MenuItem onClick={handleDeleteThreadClick}>delete thread</MenuItem>
      </Menu>
    </>
  );
};

interface UsersListItemProps extends RoomMenuPropsBase {
  user: directoryv1.FrontendGetUsersResponse.User;
  networks: Map<bigint, directoryv1.Network>;
}

const UsersListItem: React.FC<UsersListItemProps> = ({ onChange, user, networks }) => {
  const [, { openWhispers }] = useChat();

  const { alias } = user.aliases[0];

  const handleClick = useStableCallback(() => {
    const networkKeys = user.aliases
      .reduce((ids, { networkIds }) => ids.concat(networkIds), [] as bigint[])
      .map((id) => networks.get(id).key);
    openWhispers(user.peerKey, networkKeys, alias);
    onChange({ type: "WHISPER", topicKey: user.peerKey });
  });

  return (
    <button onClick={handleClick} className="whispers_list__chatter">
      {alias}
    </button>
  );
};

const WhispersList: React.FC<RoomMenuPropsBase> = ({ onChange }) => {
  const [{ whisperThreads }] = useChat();
  const [getUsersRes] = useCall("directory", "getUsers");

  const threads = useMemo(() => {
    return Array.from(whisperThreads.values()).sort((a, b) =>
      Number(b.lastMessageTime - a.lastMessageTime)
    );
  }, [whisperThreads]);

  const threadPeerKeys = useMemo(() => {
    return new Set(
      Array.from(whisperThreads.values()).map(({ peerKey }) => Base64.fromUint8Array(peerKey, true))
    );
  }, [whisperThreads]);

  const users = useMemo(() => {
    return getUsersRes.value?.users
      .filter(({ peerKey }) => !threadPeerKeys.has(Base64.fromUint8Array(peerKey, true)))
      .sort((a, b) => a.aliases[0].alias.localeCompare(b.aliases[0].alias));
  }, [getUsersRes, threadPeerKeys]);

  const onlinePeerKeys = useMemo(() => {
    return new Set(
      getUsersRes.value?.users.map(({ peerKey }) => Base64.fromUint8Array(peerKey, true))
    );
  }, [getUsersRes]);

  return (
    <Scrollbars autoHide={true} className="whispers_list">
      <div className="whispers_list__content">
        {threads.length > 0 && (
          <>
            <h4 className="whispers_list__header">whispers</h4>
            <table className="whispers_list__table">
              <tbody>
                {threads.map((thread) => (
                  <WhispersListItem
                    key={thread.id.toString()}
                    thread={thread}
                    onChange={onChange}
                    online={onlinePeerKeys.has(Base64.fromUint8Array(thread.peerKey, true))}
                  />
                ))}
              </tbody>
            </table>
          </>
        )}
        {users?.length > 0 && (
          <>
            <h4 className="whispers_list__header">chatters</h4>
            {users.map((user) => (
              <UsersListItem
                key={Base64.fromUint8Array(user.peerKey)}
                user={user}
                networks={getUsersRes.value.networks}
                onChange={onChange}
              />
            ))}
          </>
        )}
      </div>
    </Scrollbars>
  );
};

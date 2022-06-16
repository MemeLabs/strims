// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./RoomMenu.scss";

import clsx from "clsx";
import date from "date-and-time";
import { Base64 } from "js-base64";
import React, { forwardRef, useContext, useEffect, useMemo, useState } from "react";
import Scrollbars from "react-custom-scrollbars-2";
import { BsArrowBarRight } from "react-icons/bs";
import { HiOutlineUser } from "react-icons/hi";

import { WhisperThread } from "../../apis/strims/chat/v1/chat";
import * as directoryv1 from "../../apis/strims/network/v1/directory/directory";
import { ThreadProviderProps, useChat } from "../../contexts/Chat";
import { DirectoryContext, DirectoryUser } from "../../contexts/Directory";
import { useCall, useClient } from "../../contexts/FrontendApi";
import { NetworkContext } from "../../contexts/Network";
import { useStableCallback } from "../../hooks/useStableCallback";
import { certificateRoot } from "../../lib/certificate";

enum Tab {
  Rooms,
  Whispers,
  Users,
}

interface TabsProps<T> {
  onChange: (tab: T) => void;
  active: T;
  tabs: { key: T; label: string }[];
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
  onChange?: (item: ThreadProviderProps) => void;
}

interface RoomMenuProps extends RoomMenuPropsBase {
  onClose: () => void;
}

export const RoomButtons: React.FC<RoomMenuProps> = ({ onChange, onClose }) => {
  const [activeTab, setActiveTab] = useState<Tab>(Tab.Rooms);
  const tabs = useMemo(
    () => [
      { key: Tab.Rooms, label: "rooms" },
      { key: Tab.Whispers, label: "whispers" },
      { key: Tab.Users, label: "users" },
    ],
    []
  );

  const list = (() => {
    switch (activeTab) {
      case Tab.Rooms:
        return <RoomsList onChange={onChange} />;
      case Tab.Whispers:
        return <WhispersList onChange={onChange} />;
      case Tab.Users:
        return <UsersList onChange={onChange} />;
    }
  })();

  return (
    <div className="room_menu">
      <div className="room_menu__header">
        <button className="room_menu__toggle" onClick={onClose}>
          <BsArrowBarRight />
        </button>
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
  });

  return (
    <button className="rooms_list__network_rooms_item" onClick={handleClick}>
      <span className="rooms_list__network_rooms_item__name">{name}</span>
      <span className="rooms_list__network_rooms_item__viewers">
        {userCount.toLocaleString()}
        <HiOutlineUser />
      </span>
    </button>
  );
};

const RoomsList: React.FC<RoomMenuPropsBase> = ({ onChange }) => {
  const [result] = useCall("directory", "getListings", {
    args: [{ contentTypes: [directoryv1.ListingContentType.LISTING_CONTENT_TYPE_CHAT] }],
  });

  if (result.loading) {
    return null;
  }

  return (
    <Scrollbars autoHide={true} className="rooms_list">
      <div className="rooms_list__content">
        {result.value.listings.map(({ network, listings }) => (
          <div key={network.id.toString()} className="rooms_list__network">
            <div className="rooms_list__network_name">{network.name}</div>
            <div className="rooms_list__network_rooms">
              {listings.map(({ id, listing, userCount }) => {
                if (listing.content.case === directoryv1.Listing.ContentCase.CHAT) {
                  return (
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
              })}
            </div>
          </div>
        ))}
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
}

const WhispersListItem: React.FC<WhispersListItemProps> = ({ onChange, thread }) => {
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

  return (
    <tr key={thread.id.toString()} className="whispers_list__row" onClick={handleClick}>
      <td className="whispers_list__label">
        <span className="whispers_list__alias">{thread.alias}</span>
        <span className="whispers_list__unread">{thread.unreadCount}</span>
      </td>
      <td className="whispers_list__time">
        {formatMessageTime(new Date(Number(thread.lastMessageTime)))}
      </td>
    </tr>
  );
};

const WhispersList: React.FC<RoomMenuPropsBase> = ({ onChange }) => {
  const [{ whisperThreads }] = useChat();

  const sortedThreads = useMemo(
    () =>
      Array.from(whisperThreads.values()).sort((a, b) =>
        Number(b.lastMessageTime - a.lastMessageTime)
      ),
    [whisperThreads]
  );

  return (
    <Scrollbars autoHide={true} className="whispers_list">
      <table className="whispers_list__table">
        <tbody>
          {sortedThreads.map((thread, i) => (
            <WhispersListItem key={i} thread={thread} onChange={onChange} />
          ))}
        </tbody>
      </table>
    </Scrollbars>
  );
};

interface UsersListItemProps extends RoomMenuPropsBase {
  networks: Uint8Array[];
  user: DirectoryUser;
}

const UsersListItem: React.FC<UsersListItemProps> = ({ onChange, networks, user }) => {
  const [, { openWhispers }] = useChat();

  const handleClick = useStableCallback(() => {
    openWhispers(user.peerKey, networks);
    onChange({ type: "WHISPER", topicKey: user.peerKey });
  });

  return (
    <button onClick={handleClick} key={user.id.toString()} className="user_list__item">
      {user.alias}
    </button>
  );
};

interface RoomUserThing extends RoomMenuPropsBase {
  servers: directoryv1.Listing[];
  networks: Uint8Array[];
  user: DirectoryUser;
}

const UsersList: React.FC<RoomMenuPropsBase> = ({ onChange }) => {
  const { directories } = useContext(DirectoryContext);
  const users = useMemo(() => {
    const users = new Map<string, RoomUserThing>();
    for (const { networkKey, listings } of Object.values(directories)) {
      for (const { listing, viewers } of listings.values()) {
        if (listing?.content?.case === directoryv1.Listing.ContentCase.CHAT) {
          for (const viewer of viewers.values()) {
            const key = Base64.fromUint8Array(viewer.peerKey, true);
            let user = users.get(key);
            if (user === undefined) {
              user = {
                servers: [],
                networks: [],
                user: viewer,
              };
              users.set(key, user);
            }
            user.servers.push(listing);
            user.networks.push(networkKey);
          }
        }
      }
    }
    return Array.from(users.values()).sort((a, b) => a.user.alias.localeCompare(b.user.alias));
  }, [directories]);

  return (
    <Scrollbars autoHide={true} className="user_list">
      <div className="user_list__content">
        {users.map(({ user, networks }, i) => (
          <UsersListItem key={i} user={user} networks={networks} onChange={onChange} />
        ))}
      </div>
    </Scrollbars>
  );
};

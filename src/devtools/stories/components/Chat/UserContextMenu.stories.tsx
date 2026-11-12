// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./chat.stories.scss";

import React, { useEffect, useMemo, useRef } from "react";

import { Message } from "../../../../apis/strims/chat/v1/chat";
import { useUserContextMenu } from "../../../../components/Chat/UserContextMenu";
import { MockChat, RoomProvider, RoomStyles } from "../../../contexts/Chat";
import { peerKey, replay } from "../../../mocks/chat/MessageEmitter";
import { directoryRef } from "../../../mocks/chat/messages";
import ChatService, { ChatServiceOptions } from "../../../mocks/chat/service";
import { Story, fixtures } from "../../../storybook";

interface AuthorProps {
  nick: string;
  peerKey: Uint8Array;
  viewedListing?: Message.IDirectoryRef;
}

// An author's name, its menu opened beside it on mount as a right-click does.
const Author: React.FC<AuthorProps> = ({ nick, peerKey, viewedListing }) => {
  const ref = useRef<HTMLSpanElement>(null);
  const listing = useMemo(
    () => viewedListing && new Message.DirectoryRef(viewedListing),
    [viewedListing]
  );
  const { UserContextMenu, openUserContextMenu, disabled } = useUserContextMenu({
    nick,
    peerKey,
    viewedListing: listing,
  });

  useEffect(() => {
    const r = ref.current.getBoundingClientRect();
    openUserContextMenu({ pageX: r.right + 4, pageY: r.top } as React.MouseEvent);
  }, []);

  return (
    <div className="chat__message">
      <span
        className="chat__message__author"
        ref={ref}
        onContextMenu={(e) => {
          e.preventDefault();
          openUserContextMenu(e);
        }}
      >
        <span className="chat__message__author__text">{nick}</span>
      </span>
      <span className="chat__message__colon">{": "}</span>
      <span className="chat__message__body">
        {disabled ? "your own name opens no menu" : "right-click a name for its menu"}
      </span>
      <UserContextMenu />
    </div>
  );
};

interface MenuStoryProps extends AuthorProps {
  options?: ChatServiceOptions;
}

const MenuStory: React.FC<MenuStoryProps> = ({ options, ...props }) => (
  <MockChat chat={() => new ChatService(replay([]), options)}>
    <RoomProvider>
      <RoomStyles className="chat_story chat_story--messages">
        <Author {...props} />
      </RoomStyles>
    </RoomProvider>
  </MockChat>
);

const soadsoap = { nick: "soadsoap", peerKey: peerKey("soadsoap") };

const stories: Story[] = [
  { name: "Default", component: () => <MenuStory {...soadsoap} /> },
  {
    name: "Viewed listing",
    component: () => <MenuStory {...soadsoap} viewedListing={directoryRef(28)} />,
  },
  {
    name: "Highlighted and ignored",
    component: () => (
      <MenuStory {...soadsoap} options={{ highlights: ["soadsoap"], ignores: ["soadsoap"] }} />
    ),
  },
  {
    name: "Self",
    component: () => <MenuStory nick="test_user" peerKey={fixtures.profileKey} />,
  },
];

export default stories;

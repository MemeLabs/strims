// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./chat.stories.scss";

import React, { ReactNode, useEffect, useMemo, useRef } from "react";

import { Message } from "../../../../apis/strims/chat/v1/chat";
import { useUserContextMenu } from "../../../../components/Chat/UserContextMenu";
import Toast from "../../../../components/Notification/Toast";
import { MockChat, RoomProvider, RoomStyles } from "../../../contexts/Chat";
import { peerKey, replay } from "../../../mocks/chat/MessageEmitter";
import { directoryRef } from "../../../mocks/chat/messages";
import ChatService, { ChatServiceOptions } from "../../../mocks/chat/service";
import { MockNotifications, Story, fixtures } from "../../../storybook";

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
  /** The chat mock; default the plain one over `options`. */
  chat?: () => ChatService;
  /** Beside the author, under its notifications (toasts, a step). */
  children?: ReactNode;
}

const MenuStory: React.FC<MenuStoryProps> = ({ options, chat, children, ...props }) => (
  <MockChat chat={chat ?? (() => new ChatService(replay([]), options))}>
    <MockNotifications>
      <RoomProvider>
        <RoomStyles className="chat_story chat_story--messages">
          <Author {...props} />
          {children}
        </RoomStyles>
      </RoomProvider>
    </MockNotifications>
  </MockChat>
);

class FailingChatService extends ChatService {
  highlight(): never {
    throw new Error("the chat server isn't answering");
  }

  tag(): never {
    throw new Error("the chat server isn't answering");
  }
}

// Clicks the open menu's item matching `selector` (and `label`) once it shows.
const ClickMenuItem: React.FC<{ selector?: string; label?: string }> = ({
  selector = ".context_menu__item",
  label,
}) => {
  useEffect(() => {
    const timer = setInterval(() => {
      const item = Array.from(document.querySelectorAll<HTMLElement>(selector)).find(
        (el) => label === undefined || el.textContent.trim() === label
      );
      if (item) {
        clearInterval(timer);
        item.click();
      }
    }, 50);
    return () => clearInterval(timer);
  }, []);
  return null;
};

const TOAST_MS = 10 * 60 * 1000;

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
  {
    // Highlight fails: the menu closes and a toast says so.
    name: "Highlight error",
    component: () => (
      <MenuStory {...soadsoap} chat={() => new FailingChatService(replay([]))}>
        <ClickMenuItem label="Highlight" />
        <Toast timeoutMs={TOAST_MS} />
      </MenuStory>
    ),
  },
  {
    // Tagging red fails: the menu closes and a toast says so.
    name: "Tag error",
    component: () => (
      <MenuStory {...soadsoap} chat={() => new FailingChatService(replay([]))}>
        <ClickMenuItem selector=".user_context_menu__tag__item[data-color=red]" />
        <Toast timeoutMs={TOAST_MS} />
      </MenuStory>
    ),
  },
];

export default stories;

// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./UserContextMenu.scss";

import { Base64 } from "js-base64";
import { isEqual } from "lodash";
import React, { useCallback, useMemo } from "react";
import { HiOutlineUser } from "react-icons/hi";

import monkey from "../../../assets/directory/monkey.png";
import { Message } from "../../apis/strims/chat/v1/chat";
import { useChat, useRoom } from "../../contexts/Chat";
import { useCall, useClient } from "../../contexts/FrontendApi";
import { useSession } from "../../contexts/Session";
import { useOpenListing } from "../../hooks/directory";
import { useStableCallback } from "../../hooks/useStableCallback";
import { getDirectoryRefColor } from "../../lib/chat";
import { formatNumber } from "../../lib/number";
import { MenuItem, useContextMenu } from "../ContextMenu";
import SnippetImage from "../Directory/SnippetImage";

interface UserContextMenuItemsProps {
  nick: string;
  peerKey: Uint8Array;
  viewedListing: Message.DirectoryRef;
  onClose: () => void;
}

const UserContextMenuItems: React.FC<UserContextMenuItemsProps> = ({
  nick,
  peerKey,
  viewedListing,
  onClose,
}) => {
  const client = useClient();
  const [{ uiConfigHighlights }, { openWhispers }] = useChat();
  const [, { getNetworkKeys }] = useRoom();

  const handleWhisperClick = useStableCallback(() => {
    openWhispers(peerKey, getNetworkKeys(), nick);
    onClose();
  });

  const highlighted = uiConfigHighlights.has(Base64.fromUint8Array(peerKey, true));
  const [networkKey] = getNetworkKeys();

  const handleHighlightClick = useStableCallback(() => {
    if (highlighted) {
      void client.chat.unhighlight({ networkKey, alias: nick });
    } else {
      void client.chat.highlight({ networkKey, alias: nick });
    }
    onClose();
  });

  const handleIgnoreClick = useStableCallback(() => {
    void client.chat.ignore({ networkKey, alias: nick });
    onClose();
  });

  return (
    <>
      <ListingMenuItem viewedListing={viewedListing} onClick={onClose} />
      <MenuItem onClick={handleWhisperClick}>Whisper</MenuItem>
      <MenuItem onClick={handleHighlightClick}>
        {highlighted ? "Unhighlight" : "Highlight"}
      </MenuItem>
      <MenuItem onClick={handleIgnoreClick}>Ignore</MenuItem>
    </>
  );
};

interface ListingMenuItemProps {
  viewedListing: Message.DirectoryRef;
  onClick: () => void;
}

const ListingMenuItem: React.FC<ListingMenuItemProps> = ({ viewedListing, onClick }) => {
  if (!viewedListing) {
    return null;
  }

  const { directoryId, listing, networkKey } = viewedListing;

  const openListing = useOpenListing();
  const handleClick = useCallback(() => {
    openListing(Base64.fromUint8Array(networkKey, true), listing);
    onClick();
  }, [networkKey, listing]);

  const [res] = useCall("directory", "getListing", {
    args: [{ query: { query: { id: directoryId } }, networkKey }],
  });

  if (!res.value?.snippet) {
    return null;
  }

  const { snippet, userCount } = res.value;

  return (
    <MenuItem className="user_context_menu__listing" onClick={handleClick}>
      <div>
        <div
          className="user_context_menu__listing__image_frame"
          style={{ "--channel-color": getDirectoryRefColor(viewedListing) }}
        >
          <SnippetImage
            className="user_context_menu__listing__image"
            fallback={monkey}
            source={snippet.thumbnail}
          />
          <span
            className="user_context_menu__listing__viewers"
            title={`${formatNumber(Number(snippet.userCount))} ${
              snippet.live ? "viewers" : "views"
            }`}
          >
            <HiOutlineUser />
            {formatNumber(userCount)}
          </span>
        </div>
      </div>
      <div className="user_context_menu__listing__label">
        <div className="user_context_menu__listing__channel">
          <SnippetImage
            className="user_context_menu__listing__channel_logo"
            fallback={monkey}
            source={snippet.channelLogo}
          />
          <span className="user_context_menu__listing__channel_name">{snippet.channelName}</span>
        </div>
        <span className="user_context_menu__listing__title">{snippet.title}</span>
      </div>
    </MenuItem>
  );
};

interface UseUserContextMenuOptions {
  nick: string;
  peerKey: Uint8Array;
  viewedListing: Message.DirectoryRef;
}

export const useUserContextMenu = ({ nick, peerKey, viewedListing }: UseUserContextMenuOptions) => {
  const { Menu, isOpen, openMenu, closeMenu } = useContextMenu();

  const UserContextMenu = useCallback(() => {
    return (
      isOpen && (
        <Menu>
          <UserContextMenuItems
            nick={nick}
            peerKey={peerKey}
            viewedListing={viewedListing}
            onClose={closeMenu}
          />
        </Menu>
      )
    );
  }, [isOpen]);

  const [{ profile }] = useSession();
  const disabled = useMemo(() => isEqual(profile.key.public, peerKey), []);

  const openUserContextMenu = (e: React.MouseEvent) => {
    if (!disabled) {
      openMenu(e);
    }
  };

  return { UserContextMenu, openUserContextMenu, disabled };
};

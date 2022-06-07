// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Shell.scss";

import useResizeObserver from "@react-hook/resize-observer";
import clsx from "clsx";
import { isEqual } from "lodash";
import React, { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { BiSmile } from "react-icons/bi";
import { FiSettings } from "react-icons/fi";
import { IconType } from "react-icons/lib";

import { ThreadInitState, useChat, useRoom } from "../../contexts/Chat";
import Composer from "./Composer";
import ChatDrawer from "./Drawer";
import EmotesDrawer from "./EmotesDrawer";
import Message from "./Message";
import Scroller, { MessageProps } from "./Scroller";
import SettingsDrawer from "./SettingsDrawer";
import StyleSheet from "./StyleSheet";

enum ChatDrawerRole {
  None = "none",
  Emotes = "emotes",
  Settings = "settings",
}

type ChatDrawerButtonProps = {
  icon: IconType;
  onToggle: (state: boolean) => void;
  active: boolean;
};

const ChatDrawerButton: React.FC<ChatDrawerButtonProps> = ({ icon: Icon, onToggle, active }) => {
  const className = clsx({
    "chat__nav__icon": true,
    "chat__nav__icon--active": active,
  });

  // it isn't possible to predict whether react will re-render in the event
  // handler call stack or defer until afterward. if the click-away handler
  // in the drawer is bound before the event finishes bubbling it will fire
  // immediately and close the drawer.
  const handleClick = useCallback(() => setTimeout(() => onToggle(!active)), [onToggle, active]);

  return <Icon className={className} onClick={handleClick} />;
};

interface ShellProps {
  className?: string;
}

const Shell: React.FC<ShellProps> = ({ className }) => {
  const { t } = useTranslation();

  const [{ uiConfig }, { toggleTopicVisible, resetTopicUnreadCount }] = useChat();
  const [room, { getMessage, getMessageCount, sendMessage }] = useRoom();
  const [activePanel, setActivePanel] = useState(ChatDrawerRole.None);

  useEffect(() => {
    if (room.state === ThreadInitState.OPEN) {
      toggleTopicVisible(room.topic, true);
      resetTopicUnreadCount(room.topic);
      return () => toggleTopicVisible(room.topic, false);
    }
  }, [room.id, room.state]);

  const closePanel = useCallback(() => setActivePanel(ChatDrawerRole.None), []);

  const drawerToggler = (role: ChatDrawerRole) => (state: boolean) =>
    setActivePanel(state ? role : ChatDrawerRole.None);
  const toggleEmotes = useCallback(drawerToggler(ChatDrawerRole.Emotes), []);
  const toggleSettings = useCallback(drawerToggler(ChatDrawerRole.Settings), []);

  const ref = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState<DOMRectReadOnly>();
  useLayoutEffect(() => setSize(ref.current?.getBoundingClientRect()), [ref.current, room]);
  useResizeObserver(ref, (entry) => setSize(entry.contentRect));

  const renderMessage = useCallback(
    ({ index, style }: MessageProps) => (
      <Message
        uiConfig={uiConfig}
        message={getMessage(index)}
        style={style}
        isMostRecent={index === getMessageCount() - 1}
        isContinued={isEqual(getMessage(index).peerKey, getMessage(index + 1)?.peerKey)}
      />
    ),
    [uiConfig, room.styles]
  );

  return (
    <div
      ref={ref}
      className={clsx(className, "chat")}
      style={{
        "--chat-width": size ? `${size.width}px` : "100%",
        "--chat-height": size ? `${size.height}px` : "100%",
      }}
    >
      <StyleSheet liveEmotes={room.liveEmotes} styles={room.styles} uiConfig={uiConfig} />
      <div className="chat__messages">
        <ChatDrawer
          title={t("chat.drawers.Emotes")}
          side="left"
          role={ChatDrawerRole.Emotes}
          active={activePanel === ChatDrawerRole.Emotes}
          onClose={closePanel}
        >
          <EmotesDrawer />
        </ChatDrawer>
        <ChatDrawer
          title={t("chat.drawers.Settings")}
          side="right"
          role={ChatDrawerRole.Settings}
          active={activePanel === ChatDrawerRole.Settings}
          onClose={closePanel}
        >
          <SettingsDrawer />
        </ChatDrawer>
        <Scroller
          uiConfig={uiConfig}
          renderMessage={renderMessage}
          messageCount={room.messages.length}
          messageSizeCache={room.messageSizeCache}
        />
      </div>
      <div className="chat__footer">
        <Composer
          emotes={room.emotes}
          modifiers={room.modifiers}
          tags={room.tags}
          nicks={room.nicks}
          onMessage={sendMessage}
        />
      </div>
      <div className="chat__nav">
        <div className="chat__nav__left">
          <ChatDrawerButton
            icon={BiSmile}
            active={activePanel === ChatDrawerRole.Emotes}
            onToggle={toggleEmotes}
          />
        </div>
        <div className="chat__nav__right">
          <ChatDrawerButton
            icon={FiSettings}
            active={activePanel === ChatDrawerRole.Settings}
            onToggle={toggleSettings}
          />
        </div>
      </div>
    </div>
  );
};

export default Shell;

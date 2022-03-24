import "./Shell.scss";

import useResizeObserver from "@react-hook/resize-observer";
import clsx from "clsx";
import React, { useCallback, useRef, useState } from "react";
import Scrollbars from "react-custom-scrollbars-2";
import { useTranslation } from "react-i18next";
import { BiConversation, BiSmile } from "react-icons/bi";
import { FiSettings } from "react-icons/fi";
import { HiOutlineUserGroup } from "react-icons/hi";
import { IconType } from "react-icons/lib";

import { useChat, useRoom } from "../../contexts/Chat";
import Composer from "./Composer";
import ChatDrawer from "./Drawer";
import EmotesDrawer from "./EmotesDrawer";
import Message from "./Message";
import Scroller, { MessageProps } from "./Scroller";
import SettingsDrawer from "./SettingsDrawer";
import StyleSheet from "./StyleSheet";
import UsersDrawer from "./UsersDrawer";

enum ChatDrawerRole {
  None = "none",
  Emotes = "emotes",
  Whispers = "whispers",
  Settings = "settings",
  Users = "users",
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

const TestContent: React.FC = () => (
  <Scrollbars autoHide={true}>
    <div style={{ height: "1000px" }} />
  </Scrollbars>
);

interface ShellProps {
  shouldHide?: boolean;
  className?: string;
}

const Shell: React.FC<ShellProps> = ({ shouldHide = false, className }) => {
  const { t } = useTranslation();

  const [{ uiConfig }] = useChat();
  const [room, { getMessage, getMessageCount, toggleMessageGC, sendMessage }] = useRoom();
  const [activePanel, setActivePanel] = useState(ChatDrawerRole.None);

  const closePanel = useCallback(() => setActivePanel(ChatDrawerRole.None), []);

  const drawerToggler = (role: ChatDrawerRole) => (state: boolean) =>
    setActivePanel(state ? role : ChatDrawerRole.None);
  const toggleEmotes = useCallback(drawerToggler(ChatDrawerRole.Emotes), []);
  const toggleWhispers = useCallback(drawerToggler(ChatDrawerRole.Whispers), []);
  const toggleSettings = useCallback(drawerToggler(ChatDrawerRole.Settings), []);
  const toggleUsers = useCallback(drawerToggler(ChatDrawerRole.Users), []);

  const ref = useRef<HTMLDivElement>(null);
  const [size, setSize] = React.useState<DOMRectReadOnly>();
  React.useLayoutEffect(() => setSize(ref.current?.getBoundingClientRect()), [ref.current, room]);
  useResizeObserver(ref, (entry) => setSize(entry.contentRect));

  const renderMessage = useCallback(
    ({ index, style }: MessageProps) => (
      <Message
        uiConfig={uiConfig}
        message={getMessage(index)}
        style={style}
        isMostRecent={index === getMessageCount() - 1}
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
          title={t("chat.drawers.Whispers")}
          side="left"
          role={ChatDrawerRole.Whispers}
          active={activePanel === ChatDrawerRole.Whispers}
          onClose={closePanel}
        >
          <TestContent />
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
        <ChatDrawer
          title={t("chat.drawers.Users")}
          side="right"
          role={ChatDrawerRole.Users}
          active={activePanel === ChatDrawerRole.Users}
          onClose={closePanel}
        >
          <UsersDrawer />
        </ChatDrawer>
        {!shouldHide && (
          // TODO: scroller is super fucking sketchy... this should probably be
          // wrapped with an error boundary to keep it from taking down the app
          <Scroller
            uiConfig={uiConfig}
            renderMessage={renderMessage}
            messageCount={room.messages.length}
            messageSizeCache={room.messageSizeCache}
            onAutoScrollChange={toggleMessageGC}
          />
        )}
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
          <ChatDrawerButton
            icon={BiConversation}
            active={activePanel === ChatDrawerRole.Whispers}
            onToggle={toggleWhispers}
          />
        </div>
        <div className="chat__nav__right">
          <ChatDrawerButton
            icon={FiSettings}
            active={activePanel === ChatDrawerRole.Settings}
            onToggle={toggleSettings}
          />
          <ChatDrawerButton
            icon={HiOutlineUserGroup}
            active={activePanel === ChatDrawerRole.Users}
            onToggle={toggleUsers}
          />
        </div>
      </div>
    </div>
  );
};

export default Shell;

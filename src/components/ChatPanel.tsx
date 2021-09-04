import clsx from "clsx";
import { Base64 } from "js-base64";
import React, { useCallback, useRef, useState } from "react";
import Scrollbars from "react-custom-scrollbars-2";
import { BiConversation, BiSmile } from "react-icons/bi";
import { BsArrowBarLeft, BsArrowBarRight } from "react-icons/bs";
import { FiSettings } from "react-icons/fi";
import { HiOutlineUserGroup } from "react-icons/hi";
import { ImCircleLeft, ImCircleRight } from "react-icons/im";
import { IconType } from "react-icons/lib";
import { useToggle } from "react-use";

import Composer from "../components/Chat/Composer";
import Message from "../components/Chat/Message";
import Scroller, { MessageProps } from "../components/Chat/Scroller";
import { Provider, useChat } from "../contexts/Chat";
import useClickAway from "../hooks/useClickAway";
import EmotesDrawer from "./Chat/EmotesDrawer";
import SettingsDrawer from "./Chat/SettingsDrawer";

enum ChatDrawerRole {
  None = "none",
  Emotes = "emotes",
  Whispers = "whispers",
  Settings = "settings",
  Users = "users",
}

interface ChatDrawerBodyProps {
  onClose: () => void;
}

const ChatDrawerBody: React.FC<ChatDrawerBodyProps> = ({ onClose, children }) => {
  const ref = useRef<HTMLDivElement>(null);
  useClickAway(ref, onClose, ["click"]);

  return (
    <div className="chat__drawer__body" ref={ref}>
      {children}
    </div>
  );
};

interface ChatDrawerProps extends ChatDrawerBodyProps {
  side: "left" | "right";
  role: ChatDrawerRole;
  title: string;
  active: boolean;
}

const ChatDrawer: React.FC<ChatDrawerProps> = ({
  side,
  role,
  active,
  title,
  onClose,
  children,
}) => {
  const classNames = clsx({
    "chat__drawer": true,
    [`chat__drawer--${side}`]: true,
    [`chat__drawer--${role}`]: true,
    "chat__drawer--active": active,
  });

  const Icon = side === "left" ? ImCircleLeft : ImCircleRight;

  return (
    <div className={classNames}>
      <div className="chat__drawer__header">
        <span>{title}</span>
        <Icon className="chat__drawer__header__close_button" onClick={onClose} />
      </div>
      {active && <ChatDrawerBody onClose={onClose}>{children}</ChatDrawerBody>}
    </div>
  );
};

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

const ChatThing: React.FC = () => {
  const [state, { sendMessage }] = useChat();
  const [activePanel, setActivePanel] = useState(ChatDrawerRole.None);

  const closePanel = useCallback(() => setActivePanel(ChatDrawerRole.None), []);

  const drawerToggler = (role: ChatDrawerRole) => (state: boolean) =>
    setActivePanel(state ? role : ChatDrawerRole.None);
  const toggleEmotes = useCallback(drawerToggler(ChatDrawerRole.Emotes), []);
  const toggleWhispers = useCallback(drawerToggler(ChatDrawerRole.Whispers), []);
  const toggleSettings = useCallback(drawerToggler(ChatDrawerRole.Settings), []);
  const toggleUsers = useCallback(drawerToggler(ChatDrawerRole.Users), []);

  return (
    <>
      <div className="chat__messages">
        <ChatDrawer
          title="Emotes"
          side="left"
          role={ChatDrawerRole.Emotes}
          active={activePanel === ChatDrawerRole.Emotes}
          onClose={closePanel}
        >
          <EmotesDrawer />
        </ChatDrawer>
        <ChatDrawer
          title="Whispers"
          side="left"
          role={ChatDrawerRole.Whispers}
          active={activePanel === ChatDrawerRole.Whispers}
          onClose={closePanel}
        >
          <TestContent />
        </ChatDrawer>
        <ChatDrawer
          title="Settings"
          side="right"
          role={ChatDrawerRole.Settings}
          active={activePanel === ChatDrawerRole.Settings}
          onClose={closePanel}
        >
          <SettingsDrawer />
        </ChatDrawer>
        <ChatDrawer
          title="Users"
          side="right"
          role={ChatDrawerRole.Users}
          active={activePanel === ChatDrawerRole.Users}
          onClose={closePanel}
        >
          <TestContent />
        </ChatDrawer>
        <Scroller
          renderMessage={({ index, style }: MessageProps) => (
            <Message
              message={state.messages[index]}
              style={style}
              isMostRecent={index === state.messages.length - 1}
            />
          )}
          messageCount={state.messages.length}
          messageSizeCache={state.messageSizeCache}
        />
      </div>
      <div className="chat__footer">
        <Composer
          emotes={state.emotes}
          modifiers={state.modifiers}
          tags={state.tags}
          nicks={state.nicks}
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
    </>
  );
};

const ChatPanel: React.FC = () => {
  const [closed, toggleClosed] = useToggle(false);

  const className = clsx({
    "home_page__right": true,
    "home_page__right--closed": closed,
  });

  return (
    <aside className={className}>
      <button className="home_page__right__toggle_on" onClick={toggleClosed}>
        <BsArrowBarLeft size={22} />
      </button>
      <div className="home_page__right__body">
        <header className="home_page__subheader">
          <button className="home_page__right__toggle_off" onClick={toggleClosed}>
            <BsArrowBarRight size={22} />
          </button>
        </header>
        <header className="home_page__chat__promo"></header>
        <div className="home_page__chat chat">
          <Provider
            networkKey={Base64.toUint8Array("ewOeQgqCCXYwVmR+nZIcbLfDszuIgV8l0Xj0OVa5Vw4=")}
            serverKey={Base64.toUint8Array("fHyr7+njRTRAShsdcDB1vOz9373dtPA476Phw+DYh0Q=")}
          >
            <ChatThing />
          </Provider>
        </div>
      </div>
    </aside>
  );
};

export default ChatPanel;

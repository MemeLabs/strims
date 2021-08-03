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
import { useClickAway, useToggle } from "react-use";

import Composer from "../components/Chat/Composer";
import Message from "../components/Chat/Message";
import Scroller, { MessageProps } from "../components/Chat/Scroller";
import { Provider, useChat } from "../contexts/Chat";

enum ChatPanelRole {
  None = "none",
  Emotes = "emotes",
  Whispers = "whispers",
  Settings = "settings",
  Users = "users",
}

interface ChatPanelBodyProps {
  onClose: () => void;
}

const ChatPanelBody: React.FC<ChatPanelBodyProps> = ({ onClose, children }) => {
  const ref = useRef<HTMLDivElement>(null);
  useClickAway(ref, onClose, ["click"]);

  return (
    <div className="chat__panel__body" ref={ref}>
      {children}
    </div>
  );
};

interface ChatPanelProps extends ChatPanelBodyProps {
  side: "left" | "right";
  role: ChatPanelRole;
  title: string;
  active: boolean;
}

const ChatPanel: React.FC<ChatPanelProps> = ({ side, role, active, title, onClose, children }) => {
  const classNames = clsx({
    "chat__panel": true,
    [`chat__panel--${side}`]: true,
    [`chat__panel--${role}`]: true,
    "chat__panel--active": active,
  });

  const Icon = side === "left" ? ImCircleLeft : ImCircleRight;

  return (
    <div className={classNames}>
      <div className="chat__panel__header">
        <span>{title}</span>
        <Icon className="chat__panel__header__close_button" onClick={onClose} />
      </div>
      {active && <ChatPanelBody onClose={onClose}>{children}</ChatPanelBody>}
    </div>
  );
};

type ChatPanelButtonProps = {
  icon: IconType;
  onClick: () => void;
  active: boolean;
};

const ChatPanelButton: React.FC<ChatPanelButtonProps> = ({ icon: Icon, onClick, active }) => {
  const className = clsx({
    "chat__nav__icon": true,
    "chat__nav__icon--active": active,
  });

  const handleClick = (e: React.MouseEvent) => {
    if (!active) {
      e.stopPropagation();
      onClick();
    }
  };

  return <Icon className={className} onClickCapture={handleClick} />;
};

const TestContent: React.FC = () => (
  <Scrollbars autoHide={true}>
    <div style={{ height: "1000px" }} />
  </Scrollbars>
);

const ChatThing: React.FC = () => {
  const [state, { sendMessage }] = useChat();
  const [activePanel, setActivePanel] = useState(ChatPanelRole.None);

  const closePanel = useCallback(() => setActivePanel(ChatPanelRole.None), []);

  return (
    <>
      <div className="chat__messages">
        <ChatPanel
          title="Emotes"
          side="left"
          role={ChatPanelRole.Emotes}
          active={activePanel === ChatPanelRole.Emotes}
          onClose={closePanel}
        >
          <TestContent />
        </ChatPanel>
        <ChatPanel
          title="Whispers"
          side="left"
          role={ChatPanelRole.Whispers}
          active={activePanel === ChatPanelRole.Whispers}
          onClose={closePanel}
        >
          <TestContent />
        </ChatPanel>
        <ChatPanel
          title="Settings"
          side="right"
          role={ChatPanelRole.Settings}
          active={activePanel === ChatPanelRole.Settings}
          onClose={closePanel}
        >
          <TestContent />
        </ChatPanel>
        <ChatPanel
          title="Users"
          side="right"
          role={ChatPanelRole.Users}
          active={activePanel === ChatPanelRole.Users}
          onClose={closePanel}
        >
          <TestContent />
        </ChatPanel>
        <Scroller
          renderMessage={({ index, style }: MessageProps) => (
            <Message message={state.messages[index]} style={style} />
          )}
          messageCount={state.messages.length}
        />
      </div>
      <div className="chat__footer">
        <Composer onMessage={sendMessage} />
      </div>
      <div className="chat__nav">
        <div className="chat__nav__left">
          <ChatPanelButton
            icon={BiSmile}
            active={activePanel === ChatPanelRole.Emotes}
            onClick={() => setActivePanel(ChatPanelRole.Emotes)}
          />
          <ChatPanelButton
            icon={BiConversation}
            active={activePanel === ChatPanelRole.Whispers}
            onClick={() => setActivePanel(ChatPanelRole.Whispers)}
          />
        </div>
        <div className="chat__nav__right">
          <ChatPanelButton
            icon={FiSettings}
            active={activePanel === ChatPanelRole.Settings}
            onClick={() => setActivePanel(ChatPanelRole.Settings)}
          />
          <ChatPanelButton
            icon={HiOutlineUserGroup}
            active={activePanel === ChatPanelRole.Users}
            onClick={() => setActivePanel(ChatPanelRole.Users)}
          />
        </div>
      </div>
    </>
  );
};

const Chat: React.FC = () => {
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
            networkKey={Base64.toUint8Array("HVmKdL3JUzXvjh3BQ8tFqFCvzPp7Wxe4ak2yWbjSj/c=")}
            serverKey={Base64.toUint8Array("laBoCbsGwcjSZk5y6qN1NEYpCxnFJZEHmNIgzV64Sc4=")}
          >
            <ChatThing />
          </Provider>
        </div>
      </div>
    </aside>
  );
};

export default Chat;

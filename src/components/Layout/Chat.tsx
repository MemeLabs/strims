import "./Chat.scss";

import { Base64 } from "js-base64";
import React, { useState } from "react";
import { BsArrowBarLeft, BsArrowBarRight } from "react-icons/bs";

import ChatRoomMenu, { RoomMenuItem } from "../../components/Chat/RoomMenu";
import { RoomProvider } from "../../contexts/Chat";
import { useLayout } from "../../contexts/Layout";
import ChatThing from "../ChatPanel";

const Chat: React.FC = () => {
  const { toggleShowChat } = useLayout();

  const [chatRoom, setChatRoom] = useState<RoomMenuItem>({
    networkKey: Base64.toUint8Array("cgqhekoCTcy7OOkRdbNbYG3J4svZorYlH3KKaT660BE="),
    serverKey: Base64.toUint8Array("fHyr7+njRTRAShsdcDB1vOz9373dtPA476Phw+DYh0Q="),
    name: "test",
  });

  return (
    <div className="layout_chat">
      <button className="layout_chat__toggle_on" onClick={() => toggleShowChat()}>
        <BsArrowBarLeft size={22} />
      </button>
      <div className="layout_chat__body">
        <header className="layout_chat__header">
          <button className="layout_chat__toggle_off" onClick={() => toggleShowChat()}>
            <BsArrowBarRight size={22} />
          </button>
          <ChatRoomMenu onChange={setChatRoom} />
        </header>
        <RoomProvider {...chatRoom}>
          <ChatThing className="home_page__chat" shouldHide={closed} />
        </RoomProvider>
      </div>
    </div>
  );
};

export default Chat;

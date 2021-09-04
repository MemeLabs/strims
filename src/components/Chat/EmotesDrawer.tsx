import React from "react";
import Scrollbars from "react-custom-scrollbars-2";

import { useChat } from "../../contexts/Chat";
import Emote from "./Emote";

const EmotesDrawer: React.FC = () => {
  const [chat] = useChat();

  return (
    <Scrollbars autoHide={true}>
      <div className="chat__emote_grid">
        {chat.emotes.map((name) => (
          <div key={name} className="chat__emote_grid__emote">
            <Emote name={name} />
          </div>
        ))}
      </div>
    </Scrollbars>
  );
};

export default EmotesDrawer;

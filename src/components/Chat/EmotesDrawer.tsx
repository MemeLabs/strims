import React from "react";
import Scrollbars from "react-custom-scrollbars-2";

import { useRoom } from "../../contexts/Chat";
import Emote from "./Emote";

const EmotesDrawer: React.FC = () => {
  const [room] = useRoom();

  return (
    <Scrollbars autoHide={true}>
      <div className="chat__emote_grid">
        {room.emotes.map((name) => (
          <div key={name} className="chat__emote_grid__emote">
            <Emote name={name} shouldAnimateForever />
          </div>
        ))}
      </div>
    </Scrollbars>
  );
};

export default EmotesDrawer;

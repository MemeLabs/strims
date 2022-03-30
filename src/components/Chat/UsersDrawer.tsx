import React, { useContext } from "react";
import Scrollbars from "react-custom-scrollbars-2";

import { useRoom } from "../../contexts/Chat";
import { DirectoryContext } from "../../contexts/Directory";

const EmotesDrawer: React.FC = () => {
  const [room] = useRoom();
  console.log({ room });

  return (
    <Scrollbars autoHide={true}>
      <div className="chat__user_list">
        {/* {room.emotes.map((name) => (
          <div key={name} className="chat__user_list__user">
            <Emote name={name} shouldAnimateForever />
          </div>
        ))} */}
      </div>
    </Scrollbars>
  );
};

export default EmotesDrawer;

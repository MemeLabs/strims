import React, { useContext } from "react";
import Scrollbars from "react-custom-scrollbars-2";

import { DirectoryContext } from "../../contexts/Directory";

const EmotesDrawer: React.FC = () => {
  const [directory] = useContext(DirectoryContext);

  console.log(directory);

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

// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./UsersDrawer.scss";

import React from "react";
import Scrollbars from "react-custom-scrollbars-2";

import { useChat, useRoom } from "../../contexts/Chat";
import { ViewerStateIndicator } from "./ViewerStateIndicator";

const EmotesDrawer: React.FC = () => {
  const [{ uiConfig }] = useChat();
  const [room] = useRoom();

  return (
    <Scrollbars autoHide={true}>
      <ul className="chat__users_list">
        {Array.from(room.users.values()).map(({ alias, listing }) => (
          <li key={alias} className="chat__users_list__item">
            <ViewerStateIndicator style={uiConfig.viewerStateIndicator} listing={listing} />
            <span>{alias}</span>
          </li>
        ))}
      </ul>
    </Scrollbars>
  );
};

export default EmotesDrawer;

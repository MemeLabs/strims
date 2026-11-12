// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Chat and message markup around it, styled as Shell and Message style it.
import "../../../../components/Chat/Message.scss";
import "../../../../components/Chat/Shell.scss";
import "./chat.stories.scss";

import React from "react";

import { Message, UIConfig } from "../../../../apis/strims/chat/v1/chat";
import { UserPresenceIndicator } from "../../../../components/Chat/UserPresenceIndicator";
import { directoryRef } from "../../../mocks/chat/messages";
import { Story } from "../../../storybook";

const Style = UIConfig.UserPresenceIndicator;

const refs: [string, Message.DirectoryRef | undefined][] = [
  ["eastmancolor (AngelThump)", new Message.DirectoryRef(directoryRef(37))],
  ["TastelessTV (Twitch)", new Message.DirectoryRef(directoryRef(31))],
  ["Destiny (Twitch)", new Message.DirectoryRef(directoryRef(28))],
  ["a YouTube video", new Message.DirectoryRef(directoryRef(36))],
  ["theme colour #22aa55", new Message.DirectoryRef(directoryRef(28, 0x22aa55))],
  ["watching nothing", undefined],
];

// The indicator for each viewed listing beside an author's name, as a
// message's author shows it.
const Indicators: React.FC<{ style: UIConfig.UserPresenceIndicator }> = ({ style }) => (
  <div className="chat chat_story_grid chat_story_grid--wide">
    {refs.map(([label, ref]) => (
      <div key={label} className="chat_story_grid__cell">
        <div className="chat__message">
          <span className="chat__message__author">
            <UserPresenceIndicator style={style} directoryRef={ref} />
            <span className="chat__message__author__text">soadsoap</span>
          </span>
        </div>
        <span className="chat_story_grid__label">{label}</span>
      </div>
    ))}
  </div>
);

const stories: Story[] = [
  { name: "Bar", component: () => <Indicators style={Style.USER_PRESENCE_INDICATOR_BAR} /> },
  { name: "Dot", component: () => <Indicators style={Style.USER_PRESENCE_INDICATOR_DOT} /> },
  { name: "Array", component: () => <Indicators style={Style.USER_PRESENCE_INDICATOR_ARRAY} /> },
  {
    name: "Disabled",
    component: () => <Indicators style={Style.USER_PRESENCE_INDICATOR_DISABLED} />,
  },
];

export default stories;

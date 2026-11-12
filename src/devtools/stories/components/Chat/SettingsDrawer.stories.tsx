// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./chat.stories.scss";

import React from "react";

import { IUIConfig, UIConfig } from "../../../../apis/strims/chat/v1/chat";
import SettingsDrawer from "../../../../components/Chat/SettingsDrawer";
import { MockChat } from "../../../contexts/Chat";
import { replay } from "../../../mocks/chat/MessageEmitter";
import ChatService, { defaultUIConfig } from "../../../mocks/chat/service";
import { Story } from "../../../storybook";

// The settings over the chat mock's UI config; changes go to its setUIConfig
// and come back through watchUIConfig.
const Settings: React.FC<{ uiConfig?: Partial<IUIConfig> }> = ({ uiConfig }) => (
  <MockChat chat={() => new ChatService(replay([]), { uiConfig })}>
    <div className="chat_story">
      <SettingsDrawer />
    </div>
  </MockChat>
);

const allOff = Object.fromEntries(
  Object.entries(defaultUIConfig).map(([k, v]) => [k, typeof v === "boolean" ? false : v])
) as Partial<IUIConfig>;

const stories: Story[] = [
  // The provider's own defaults: the mock sends no config of its own.
  { name: "Default", component: () => <Settings /> },
  {
    name: "Customised",
    component: () => (
      <Settings
        uiConfig={{
          showTime: true,
          compactEmoteSpacing: true,
          maxLines: 250,
          userPresenceIndicator: UIConfig.UserPresenceIndicator.USER_PRESENCE_INDICATOR_DOT,
          showRemoved: UIConfig.ShowRemoved.SHOW_REMOVED_CENSOR,
          customHighlight: ["strims", "ppspp", "test_user"],
          taggedVisibility: true,
          hideNsfw: true,
        }}
      />
    ),
  },
  {
    name: "All off",
    component: () => (
      <Settings
        uiConfig={{
          ...allOff,
          userPresenceIndicator: UIConfig.UserPresenceIndicator.USER_PRESENCE_INDICATOR_DISABLED,
          showRemoved: UIConfig.ShowRemoved.SHOW_REMOVED_DO_NOTHING,
        }}
      />
    ),
  },
];

export default stories;

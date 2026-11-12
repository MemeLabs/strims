// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import clsx from "clsx";
import { Base64 } from "js-base64";
import React, { ReactNode } from "react";

import Composer from "../../../components/Chat/Composer";
import SettingsDrawer from "../../../components/Chat/SettingsDrawer";
import ChatPanel from "../../../components/Chat/Shell";
import StyleSheet from "../../../components/Chat/StyleSheet";
import { Provider as ChatProvider, useChat, useRoom } from "../../../contexts/Chat";
import { RoomProvider } from "../../contexts/Chat";
import Emitter from "../../mocks/chat/MessageEmitter";
import ChatService from "../../mocks/chat/service";
import DirectoryService from "../../mocks/directory/service";
import { MockApi, SessionProvider, Story } from "../../storybook";

interface ContextProps {
  emitter: () => Emitter;
  children: ReactNode;
}

// A room of the chat mock's, its messages from `emitter`.
const Context: React.FC<ContextProps> = ({ emitter, children }) => (
  <div className="chat_mockup chat_mockup--with_settings">
    <MockApi
      services={() => ({ chat: new ChatService(emitter()), directory: new DirectoryService() })}
    >
      <SessionProvider>
        <ChatProvider>
          <RoomProvider
            networkKey={Base64.toUint8Array("cgqhekoCTcy7OOkRdbNbYG3J4svZorYlH3KKaT660BE=")}
            serverKey={Base64.toUint8Array("fHyr7+njRTRAShsdcDB1vOz9373dtPA476Phw+DYh0Q=")}
          >
            {children}
          </RoomProvider>
        </ChatProvider>
      </SessionProvider>
    </MockApi>
  </div>
);

const Chat: React.FC = () => (
  <Context emitter={() => new Emitter({ ivl: 500, limit: 15000, preload: 10000 })}>
    <ChatPanel />
    <div className="chat_mockup__settings">
      <SettingsDrawer />
    </div>
  </Context>
);

const ComposerTestContent: React.FC = () => {
  const [{ uiConfig, uiConfigHighlights, uiConfigTags, uiConfigIgnores }] = useChat();
  const [room, roomActions] = useRoom();
  return (
    <div id={`chat-${room.id}`} className={clsx("chat_composer_mockup")}>
      <div className="chat_composer_mockup__filler" />
      <StyleSheet
        id={room.id}
        liveEmotes={room.liveEmotes}
        styles={room.styles}
        uiConfig={uiConfig}
        uiConfigHighlights={uiConfigHighlights}
        uiConfigTags={uiConfigTags}
        uiConfigIgnores={uiConfigIgnores}
      />
      <Composer
        emotes={room.emotes}
        modifiers={room.modifiers}
        tags={room.tags}
        nicks={room.nicks}
        commands={room.commands}
        onMessage={roomActions.sendMessage}
      />
    </div>
  );
};

const ComposerTest: React.FC = () => (
  <Context emitter={() => new Emitter({ limit: 0 })}>
    <ComposerTestContent />
  </Context>
);

const stories: Story[] = [
  {
    name: "Chat",
    component: () => <Chat />,
  },
  {
    name: "Composer",
    component: () => <ComposerTest />,
  },
];

export default stories;

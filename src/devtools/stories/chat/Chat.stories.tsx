// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Host from "@memelabs/protobuf/lib/rpc/host";
import ServiceRegistry from "@memelabs/protobuf/lib/rpc/service";
import clsx from "clsx";
import { Base64 } from "js-base64";
import React from "react";

import { FrontendClient } from "../../../apis/client";
import { registerChatFrontendService } from "../../../apis/strims/chat/v1/chat_rpc";
import { registerDirectoryFrontendService } from "../../../apis/strims/network/v1/directory/directory_rpc";
import Composer from "../../../components/Chat/Composer";
import SettingsDrawer from "../../../components/Chat/SettingsDrawer";
import ChatPanel from "../../../components/Chat/Shell";
import StyleSheet from "../../../components/Chat/StyleSheet";
import { Provider as ChatProvider, useChat, useRoom } from "../../../contexts/Chat";
import { Provider as ApiProvider } from "../../../contexts/FrontendApi";
import { AsyncPassThrough } from "../../../lib/stream";
import { RoomProvider } from "../../contexts/Chat";
import { SessionProvider } from "../../contexts/Session";
import Emitter from "../../mocks/chat/MessageEmitter";
import ChatService from "../../mocks/chat/service";
import DirectoryService from "../../mocks/directory/service";

const Chat: React.FC = () => {
  const [[chatService, client]] = React.useState((): [ChatService, FrontendClient] => {
    const svc = new ServiceRegistry();
    const chatService = new ChatService(new Emitter({ ivl: 500, limit: 15000, preload: 10000 }));
    const directoryService = new DirectoryService();
    registerChatFrontendService(svc, chatService);
    registerDirectoryFrontendService(svc, directoryService);

    const [a, b] = [new AsyncPassThrough(), new AsyncPassThrough()];
    new Host(a, b, svc);
    return [chatService, new FrontendClient(b, a)];
  });

  React.useEffect(() => () => chatService.destroy(), [chatService]);

  return (
    <div className="chat_mockup chat_mockup--with_settings">
      <ApiProvider value={client}>
        <SessionProvider>
          <ChatProvider>
            <RoomProvider
              networkKey={Base64.toUint8Array("cgqhekoCTcy7OOkRdbNbYG3J4svZorYlH3KKaT660BE=")}
              serverKey={Base64.toUint8Array("fHyr7+njRTRAShsdcDB1vOz9373dtPA476Phw+DYh0Q=")}
            >
              <ChatPanel />
              <div className="chat_mockup__settings">
                <SettingsDrawer />
              </div>
            </RoomProvider>
          </ChatProvider>
        </SessionProvider>
      </ApiProvider>
    </div>
  );
};

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

const ComposerTest: React.FC = () => {
  const [[chatService, client]] = React.useState((): [ChatService, FrontendClient] => {
    const svc = new ServiceRegistry();
    const chatService = new ChatService(new Emitter({ limit: 0 }));
    const directoryService = new DirectoryService();
    registerChatFrontendService(svc, chatService);
    registerDirectoryFrontendService(svc, directoryService);

    const [a, b] = [new AsyncPassThrough(), new AsyncPassThrough()];
    new Host(a, b, svc);
    return [chatService, new FrontendClient(b, a)];
  });

  React.useEffect(() => () => chatService.destroy(), [chatService]);

  return (
    <div className="chat_mockup chat_mockup--with_settings">
      <ApiProvider value={client}>
        <SessionProvider>
          <ChatProvider>
            <RoomProvider
              networkKey={Base64.toUint8Array("cgqhekoCTcy7OOkRdbNbYG3J4svZorYlH3KKaT660BE=")}
              serverKey={Base64.toUint8Array("fHyr7+njRTRAShsdcDB1vOz9373dtPA476Phw+DYh0Q=")}
            >
              <ComposerTestContent />
            </RoomProvider>
          </ChatProvider>
        </SessionProvider>
      </ApiProvider>
    </div>
  );
};

export default [
  {
    name: "Chat",
    component: () => <Chat />,
  },
  {
    name: "Composer",
    component: () => <ComposerTest />,
  },
];

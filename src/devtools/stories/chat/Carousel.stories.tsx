// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Host from "@memelabs/protobuf/lib/rpc/host";
import ServiceRegistry from "@memelabs/protobuf/lib/rpc/service";
import React, { useEffect, useState } from "react";

import { FrontendClient } from "../../../apis/client";
import { registerChatFrontendService } from "../../../apis/strims/chat/v1/chat_rpc";
import { registerDirectoryFrontendService } from "../../../apis/strims/network/v1/directory/directory_rpc";
import RoomCarousel from "../../../components/Chat/RoomCarousel";
import { Provider as ChatProvider, useChat } from "../../../contexts/Chat";
import { RoomProviderProps } from "../../../contexts/Chat";
import { Provider as DirectoryProvider } from "../../../contexts/Directory";
import { Provider as ApiProvider } from "../../../contexts/FrontendApi";
import { AsyncPassThrough } from "../../../lib/stream";
import ChatService from "../../mocks/chat/service";
import DirectoryService from "../../mocks/directory/service";

const Context: React.FC = ({ children }) => {
  const [[chatService, client]] = useState((): [ChatService, FrontendClient] => {
    const svc = new ServiceRegistry();
    const chatService = new ChatService();
    const directoryService = new DirectoryService();
    registerChatFrontendService(svc, chatService);
    registerDirectoryFrontendService(svc, directoryService);

    const [a, b] = [new AsyncPassThrough(), new AsyncPassThrough()];
    new Host(a, b, svc);
    return [chatService, new FrontendClient(b, a)];
  });

  useEffect(() => () => chatService.destroy(), [chatService]);

  return (
    <div className="chat_mockup">
      <ApiProvider value={client}>
        <DirectoryProvider>
          <ChatProvider>{children}</ChatProvider>
        </DirectoryProvider>
      </ApiProvider>
    </div>
  );
};

const Carousel: React.FC = () => {
  const [, { openWhispers }] = useChat();

  useEffect(() => {
    const key = new Uint8Array(32);
    for (let i = 0; i < 6; i++) {
      key[0]++;
      const networkKey = key.slice();
      key[0]++;
      const serverKey = key.slice();
      openWhispers(networkKey, [serverKey]);
    }
  }, []);

  const [selected, setSelected] = useState<RoomProviderProps>();

  return <RoomCarousel className="chat_carousel" onChange={setSelected} selected={selected} />;
};

export default [
  {
    name: "Carousel",
    component: () => (
      <Context>
        <Carousel />
      </Context>
    ),
  },
];

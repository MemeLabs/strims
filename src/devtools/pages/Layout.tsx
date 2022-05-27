// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Host from "@memelabs/protobuf/lib/rpc/host";
import ServiceRegistry from "@memelabs/protobuf/lib/rpc/service";
import React from "react";

import { FrontendClient } from "../../apis/client";
import { registerChatFrontendService } from "../../apis/strims/chat/v1/chat_rpc";
import { registerDirectoryFrontendService } from "../../apis/strims/network/v1/directory/directory_rpc";
import { registerNetworkFrontendService } from "../../apis/strims/network/v1/network_rpc";
import { registerNotificationFrontendService } from "../../apis/strims/notification/v1/notification_rpc";
import LayoutPage from "../../components/Layout";
import LayoutBody from "../../components/Layout/Body";
import { Provider as ChatProvider } from "../../contexts/Chat";
import { Provider as DirectoryProvider } from "../../contexts/Directory";
import { Provider as ApiProvider } from "../../contexts/FrontendApi";
import { Provider as NetworkProvider } from "../../contexts/Network";
import { Provider as NotificationProvider } from "../../contexts/Notification";
import { Provider as PlayerProvider } from "../../contexts/Player";
import { Provider as ThemeProvider } from "../../contexts/Theme";
import { AsyncPassThrough } from "../../lib/stream";
import Directory from "../../pages/Directory";
import LayoutControl from "../components/LayoutControl";
import ChatService from "../mocks/chat/service";
import DirectoryService from "../mocks/directory/service";
import NetworkService from "../mocks/network/service";
import NotificationService from "../mocks/notification/service";

const LayoutTest: React.FC = () => {
  const [[chatService, client]] = React.useState((): [ChatService, FrontendClient] => {
    const svc = new ServiceRegistry();
    const chatService = new ChatService();
    registerChatFrontendService(svc, chatService);
    registerNetworkFrontendService(svc, new NetworkService(8));
    registerDirectoryFrontendService(svc, new DirectoryService());
    registerNotificationFrontendService(svc, new NotificationService());

    const [a, b] = [new AsyncPassThrough(), new AsyncPassThrough()];
    new Host(a, b, svc);
    return [chatService, new FrontendClient(b, a)];
  });

  React.useEffect(() => () => chatService.destroy(), [chatService]);

  return (
    <ApiProvider value={client}>
      <ThemeProvider>
        <DirectoryProvider>
          <NetworkProvider>
            <NotificationProvider>
              <ChatProvider>
                <PlayerProvider>
                  <LayoutPage>
                    <LayoutControl />
                    <LayoutBody>
                      <Directory />
                    </LayoutBody>
                  </LayoutPage>
                </PlayerProvider>
              </ChatProvider>
            </NotificationProvider>
          </NetworkProvider>
        </DirectoryProvider>
      </ThemeProvider>
    </ApiProvider>
  );
};

export default LayoutTest;

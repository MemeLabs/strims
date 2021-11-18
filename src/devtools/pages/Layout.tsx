import { PassThrough } from "stream";

import Host from "@memelabs/protobuf/lib/rpc/host";
import ServiceRegistry from "@memelabs/protobuf/lib/rpc/service";
import React from "react";

import { FrontendClient } from "../../apis/client";
import { registerChatFrontendService } from "../../apis/strims/chat/v1/chat_rpc";
import { registerDirectoryFrontendService } from "../../apis/strims/network/v1/directory/directory_rpc";
import { registerNetworkServiceService } from "../../apis/strims/network/v1/network_rpc";
import { registerNotificationFrontendService } from "../../apis/strims/notification/v1/notification_rpc";
import LayoutPage from "../../components/Layout";
import LayoutBody from "../../components/Layout/Body";
import { Provider as ChatProvider } from "../../contexts/Chat";
import { Provider as DirectoryProvider } from "../../contexts/Directory";
import { Provider as ApiProvider } from "../../contexts/FrontendApi";
import { Provider as NetworkProvider } from "../../contexts/Network";
import { Provider as NotificationProvider } from "../../contexts/Notification";
import { Provider as ProfileProvider } from "../../contexts/Profile";
import { Provider as ThemeProvider } from "../../contexts/Theme";
import Directory from "../../pages/Directory";
import ChatService from "../mocks/chat/service";
import DirectoryService from "../mocks/directory/service";
import NetworkService from "../mocks/network/service";
import NotificationService from "../mocks/notification/service";

const LayoutTest: React.FC = () => {
  const [[chatService, client]] = React.useState((): [ChatService, FrontendClient] => {
    const svc = new ServiceRegistry();
    const chatService = new ChatService();
    registerChatFrontendService(svc, chatService);
    registerNetworkServiceService(svc, new NetworkService(8));
    registerDirectoryFrontendService(svc, new DirectoryService());
    registerNotificationFrontendService(svc, new NotificationService());

    const [a, b] = [new PassThrough(), new PassThrough()];
    new Host(a, b, svc);
    return [chatService, new FrontendClient(b, a)];
  });

  React.useEffect(() => () => chatService.destroy(), [chatService]);

  return (
    <ApiProvider value={client}>
      <ProfileProvider>
        <ThemeProvider>
          <DirectoryProvider>
            <NetworkProvider>
              <NotificationProvider>
                <ChatProvider>
                  <LayoutPage>
                    <LayoutBody>
                      <Directory />
                    </LayoutBody>
                  </LayoutPage>
                </ChatProvider>
              </NotificationProvider>
            </NetworkProvider>
          </DirectoryProvider>
        </ThemeProvider>
      </ProfileProvider>
    </ApiProvider>
  );
};

export default LayoutTest;

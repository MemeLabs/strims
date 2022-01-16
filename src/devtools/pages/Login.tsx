import { PassThrough } from "stream";

import Host from "@memelabs/protobuf/lib/rpc/host";
import ServiceRegistry from "@memelabs/protobuf/lib/rpc/service";
import React from "react";

import { FrontendClient } from "../../apis/client";
import { registerChatFrontendService } from "../../apis/strims/chat/v1/chat_rpc";
import { registerNetworkServiceService } from "../../apis/strims/network/v1/network_rpc";
import LandingPageLayout from "../../components/LandingPageLayout";
import { Provider as ApiProvider } from "../../contexts/FrontendApi";
import { Provider as ThemeProvider } from "../../contexts/Theme";
import ChatService from "../mocks/chat/service";
import NetworkService from "../mocks/network/service";

const LayoutTest: React.FC = () => {
  const [[chatService, client]] = React.useState((): [ChatService, FrontendClient] => {
    const svc = new ServiceRegistry();
    const chatService = new ChatService();
    registerChatFrontendService(svc, chatService);
    registerNetworkServiceService(svc, new NetworkService(8));

    const [a, b] = [new PassThrough(), new PassThrough()];
    new Host(a, b, svc);
    return [chatService, new FrontendClient(b, a)];
  });

  React.useEffect(() => () => chatService.destroy(), [chatService]);

  return (
    <ApiProvider value={client}>
      <ThemeProvider>
        <LandingPageLayout>
          <div className="login_profile_list">foo</div>
        </LandingPageLayout>
      </ThemeProvider>
    </ApiProvider>
  );
};

export default LayoutTest;

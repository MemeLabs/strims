import { PassThrough } from "stream";

import Host from "@memelabs/protobuf/lib/rpc/host";
import ServiceRegistry from "@memelabs/protobuf/lib/rpc/service";
import { Base64 } from "js-base64";
import React, { useState } from "react";

import { FrontendClient } from "../../apis/client";
import { registerChatFrontendService } from "../../apis/strims/chat/v1/chat_rpc";
import ChatThing from "../../components/ChatPanel";
import { Provider as ChatProvider, RoomProvider } from "../../contexts/Chat";
import { Provider as ApiProvider } from "../../contexts/FrontendApi";
import ChatService from "../mocks/chat/service";

const TestChat: React.FC = () => {
  const [[service, client]] = useState((): [ChatService, FrontendClient] => {
    const svc = new ServiceRegistry();
    const service = new ChatService();
    registerChatFrontendService(svc, service);

    const [a, b] = [new PassThrough(), new PassThrough()];
    new Host(a, b, svc);
    return [service, new FrontendClient(b, a)];
  });

  React.useEffect(() => () => service.destroy(), [service]);

  return (
    <ApiProvider value={client}>
      <ChatProvider>
        <RoomProvider
          networkKey={Base64.toUint8Array("cgqhekoCTcy7OOkRdbNbYG3J4svZorYlH3KKaT660BE=")}
          serverKey={Base64.toUint8Array("fHyr7+njRTRAShsdcDB1vOz9373dtPA476Phw+DYh0Q=")}
        >
          <ChatThing className="home_page__chat" shouldHide={closed} />
        </RoomProvider>
      </ChatProvider>
    </ApiProvider>
  );
};

export default TestChat;

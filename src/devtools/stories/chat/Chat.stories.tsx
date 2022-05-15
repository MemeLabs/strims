import Host from "@memelabs/protobuf/lib/rpc/host";
import ServiceRegistry from "@memelabs/protobuf/lib/rpc/service";
import { Base64 } from "js-base64";
import React from "react";

import { FrontendClient } from "../../../apis/client";
import { registerChatFrontendService } from "../../../apis/strims/chat/v1/chat_rpc";
import { registerDirectoryFrontendService } from "../../../apis/strims/network/v1/directory/directory_rpc";
import ChatPanel from "../../../components/Chat/Shell";
import { Provider as ChatProvider } from "../../../contexts/Chat";
import { Provider as DirectoryProvider } from "../../../contexts/Directory";
import { Provider as ApiProvider } from "../../../contexts/FrontendApi";
import { AsyncPassThrough } from "../../../lib/stream";
import { RoomProvider } from "../../contexts/Chat";
import ChatService from "../../mocks/chat/service";
import DirectoryService from "../../mocks/directory/service";

const Chat: React.FC = () => {
  const [[chatService, client]] = React.useState((): [ChatService, FrontendClient] => {
    const svc = new ServiceRegistry();
    const chatService = new ChatService();
    const directoryService = new DirectoryService();
    registerChatFrontendService(svc, chatService);
    registerDirectoryFrontendService(svc, directoryService);

    const [a, b] = [new AsyncPassThrough(), new AsyncPassThrough()];
    new Host(a, b, svc);
    return [chatService, new FrontendClient(b, a)];
  });

  React.useEffect(() => () => chatService.destroy(), [chatService]);

  return (
    <div className="chat_mockup">
      <ApiProvider value={client}>
        <DirectoryProvider>
          <ChatProvider>
            <RoomProvider
              networkKey={Base64.toUint8Array("cgqhekoCTcy7OOkRdbNbYG3J4svZorYlH3KKaT660BE=")}
              serverKey={Base64.toUint8Array("fHyr7+njRTRAShsdcDB1vOz9373dtPA476Phw+DYh0Q=")}
            >
              <ChatPanel />
            </RoomProvider>
          </ChatProvider>
        </DirectoryProvider>
      </ApiProvider>
    </div>
  );
};

export default [
  {
    name: "Chat",
    component: () => <Chat />,
  },
];

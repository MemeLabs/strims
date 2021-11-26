import { PassThrough } from "stream";

import Host from "@memelabs/protobuf/lib/rpc/host";
import ServiceRegistry from "@memelabs/protobuf/lib/rpc/service";
import { Base64 } from "js-base64";
import React from "react";

import { FrontendClient } from "../../../apis/client";
import { registerChatFrontendService } from "../../../apis/strims/chat/v1/chat_rpc";
import ChatPanel from "../../../components/Chat/Shell";
import { Provider as ChatProvider, RoomProvider } from "../../../contexts/Chat";
import { Provider as ApiProvider } from "../../../contexts/FrontendApi";
import ChatService from "../../mocks/chat/service";

interface ChatProps {
  theme: "dark" | "light";
}

const Chat: React.FC<ChatProps> = ({ theme }) => {
  const [[service, client]] = React.useState((): [ChatService, FrontendClient] => {
    const svc = new ServiceRegistry();
    const service = new ChatService();
    registerChatFrontendService(svc, service);

    const [a, b] = [new PassThrough(), new PassThrough()];
    new Host(a, b, svc);
    return [service, new FrontendClient(b, a)];
  });

  React.useEffect(() => () => service.destroy(), [service]);

  return (
    <div className="chat_mockup">
      <ApiProvider value={client}>
        <ChatProvider>
          <RoomProvider
            networkKey={Base64.toUint8Array("cgqhekoCTcy7OOkRdbNbYG3J4svZorYlH3KKaT660BE=")}
            serverKey={Base64.toUint8Array("fHyr7+njRTRAShsdcDB1vOz9373dtPA476Phw+DYh0Q=")}
          >
            <ChatPanel />
          </RoomProvider>
        </ChatProvider>
      </ApiProvider>
    </div>
  );
};

export default [
  {
    name: "Dark",
    component: () => <Chat theme="dark" />,
  },
  {
    name: "Light",
    component: () => <Chat theme="light" />,
  },
];

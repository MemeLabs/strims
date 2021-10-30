import { PassThrough } from "stream";

import Host from "@memelabs/protobuf/lib/rpc/host";
import ServiceRegistry from "@memelabs/protobuf/lib/rpc/service";
import { Base64 } from "js-base64";
import React, { useContext } from "react";
import { BsArrowBarLeft, BsArrowBarRight } from "react-icons/bs";

import { FrontendClient } from "../../apis/client";
import { registerChatFrontendService } from "../../apis/strims/chat/v1/chat_rpc";
import { ChatThing } from "../../components/ChatPanel";
import { Provider as ChatProvider } from "../../contexts/Chat";
import { Provider as ApiProvider } from "../../contexts/FrontendApi";
import { LayoutContext } from "../contexts/Layout";
import ChatService from "../mocks/chat/service";

const TestChat: React.FC = () => {
  const [[service, client]] = React.useState((): [ChatService, FrontendClient] => {
    const svc = new ServiceRegistry();
    const service = new ChatService();
    registerChatFrontendService(svc, service);

    const [a, b] = [new PassThrough(), new PassThrough()];
    new Host(a, b, svc);
    return [service, new FrontendClient(b, a)];
  });

  React.useEffect(() => () => service.destroy(), [service]);

  const { toggleShowChat } = useContext(LayoutContext);

  return (
    <ApiProvider value={client}>
      {/* <button className="home_page__right__toggle_on" onClick={() => toggleShowChat()}>
        <BsArrowBarLeft size={22} />
      </button>
      <header className="home_page__subheader">
        <button className="home_page__right__toggle_off" onClick={() => toggleShowChat()}>
          <BsArrowBarRight size={22} />
        </button>
      </header>
      <header className="home_page__chat__promo"></header> */}
      <ChatProvider
        networkKey={Base64.toUint8Array("cgqhekoCTcy7OOkRdbNbYG3J4svZorYlH3KKaT660BE=")}
        serverKey={Base64.toUint8Array("fHyr7+njRTRAShsdcDB1vOz9373dtPA476Phw+DYh0Q=")}
      >
        <ChatThing className="home_page__chat" shouldHide={closed} />
      </ChatProvider>
    </ApiProvider>
  );
};

export default TestChat;

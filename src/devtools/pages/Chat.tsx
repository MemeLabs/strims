import { PassThrough } from "stream";

import Host from "@memelabs/protobuf/lib/rpc/host";
import ServiceRegistry from "@memelabs/protobuf/lib/rpc/service";
import { Readable } from "@memelabs/protobuf/lib/rpc/stream";
import * as React from "react";

import { FrontendClient } from "../../apis/client";
import * as chatv1 from "../../apis/strims/chat/v1/chat";
import { registerChatFrontendService } from "../../apis/strims/chat/v1/chat_rpc";
import ChatPanel from "../../components/ChatPanel";
import { Provider as ApiProvider } from "../../contexts/FrontendApi";
import Nav from "../components/Nav";
import assetBundle from "../test/chat/assetBundle";
import MessageEmitter from "../test/chat/MessageEmitter";

class MockChatSvc {
  messages: Readable<chatv1.Message>;

  constructor() {
    this.messages = new MessageEmitter(0, 1);
  }

  destroy() {
    this.messages.destroy();
  }

  openClient(): Readable<chatv1.OpenClientResponse> {
    const ch = new PassThrough({ objectMode: true });

    window.setTimeout(() =>
      ch.push(
        new chatv1.OpenClientResponse({
          body: new chatv1.OpenClientResponse.Body({
            open: new chatv1.OpenClientResponse.Open({
              clientId: BigInt(1),
            }),
          }),
        })
      )
    );

    void assetBundle().then((assetBundle) =>
      ch.push(
        new chatv1.OpenClientResponse({
          body: new chatv1.OpenClientResponse.Body({ assetBundle }),
        })
      )
    );

    this.messages.on("data", (message) =>
      ch.push(
        new chatv1.OpenClientResponse({
          body: new chatv1.OpenClientResponse.Body({ message }),
        })
      )
    );

    return ch;
  }
  clientSendMessage(
    req: chatv1.ClientSendMessageRequest
  ): Promise<chatv1.ClientSendMessageResponse> {
    this.messages.push(
      new chatv1.Message({
        nick: "test_user",
        serverTime: BigInt(Date.now()),
        body: req.body,
        entities: new chatv1.Message.Entities(),
      })
    );
    return Promise.resolve(new chatv1.ClientSendMessageResponse());
  }

  setUIConfig(): Promise<chatv1.SetUIConfigResponse> {
    return Promise.resolve(new chatv1.SetUIConfigResponse());
  }

  getUIConfig(): Promise<chatv1.GetUIConfigResponse> {
    return Promise.resolve(new chatv1.GetUIConfigResponse());
  }
}

const ChatPage: React.FC = () => {
  const [[chatSvc, client]] = React.useState((): [MockChatSvc, FrontendClient] => {
    const svc = new ServiceRegistry();
    const chatSvc = new MockChatSvc();
    registerChatFrontendService(svc, chatSvc);

    const [a, b] = [new PassThrough(), new PassThrough()];
    new Host(a, b, svc);
    return [chatSvc, new FrontendClient(b, a)];
  });

  React.useEffect(() => () => chatSvc.destroy(), [chatSvc]);

  return (
    <>
      <Nav />
      <div className="chat_mockup">
        <ApiProvider value={client}>
          <div className="app app--dark">
            <div className="chat_mockup__content">
              <ChatPanel />
            </div>
          </div>
        </ApiProvider>
        {/* <ApiProvider value={client}>
          <div className="app app--light ">
            <div className="chat_mockup__content">
              <ChatPanel />
            </div>
          </div>
        </ApiProvider> */}
      </div>
    </>
  );
};

export default ChatPage;

import { PassThrough } from "stream";

import Host from "@memelabs/protobuf/lib/rpc/host";
import ServiceRegistry from "@memelabs/protobuf/lib/rpc/service";
import React from "react";

import { FrontendClient } from "../../../apis/client";
import { registerChatFrontendService } from "../../../apis/strims/chat/v1/chat_rpc";
import Emote from "../../../components/Chat/Emote";
import StyleSheet from "../../../components/Chat/StyleSheet";
import { Provider as ChatProvider, useChat } from "../../../contexts/Chat";
import { Provider as ApiProvider } from "../../../contexts/FrontendApi";
import { emotes, modifiers } from "../../mocks/chat/assetBundle";
import ChatService from "../../mocks/chat/service";
import { SelectInput } from "../../../components/Form";
import { useForm } from "react-hook-form";

const ChatStylesheet: React.FC = ({ children }) => {
  const [state] = useChat();
  return (
    <>
      <StyleSheet liveEmotes={state.liveEmotes} styles={state.styles} uiConfig={state.uiConfig} />
      {children}
    </>
  );
};

const Chat: React.FC = ({ children }) => {
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
    <ApiProvider value={client}>
      <ChatProvider networkKey={new Uint8Array()} serverKey={new Uint8Array()}>
        <ChatStylesheet>{children}</ChatStylesheet>
      </ChatProvider>
    </ApiProvider>
  );
};

const modifierOptions = modifiers.map(m => ({value: m, label: m}));

const Modifiers: React.FC =() => {
  const {control, watch} = useForm({
    defaultValues: {
      modifiers: [],
    }
  });

  const values = watch();

  console.log(values);

  return (
    <div>
      <div className="emote_grid">
        <Chat>
          {emotes.map((emote) => (
            <div className="emote_grid__cell">
              <Emote key={emote} name={emote} shouldAnimateForever modifiers={values.modifiers.map(({value}) => value)}>
                {emote}
              </Emote>
            </div>
          )}
        </Chat>
      </div>
      <SelectInput
        label="modifiers"
        name="modifiers"
        control={control}
        options={modifierOptions}
        isMulti
      />
    </div>
  )
}

export default [
  {
    name: "Modifiers",
    component: () => <Modifiers />,
  },
];

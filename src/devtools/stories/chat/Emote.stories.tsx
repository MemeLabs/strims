import { PassThrough } from "stream";

import Host from "@memelabs/protobuf/lib/rpc/host";
import ServiceRegistry from "@memelabs/protobuf/lib/rpc/service";
import React from "react";
import { useForm } from "react-hook-form";

import { FrontendClient } from "../../../apis/client";
import { registerChatFrontendService } from "../../../apis/strims/chat/v1/chat_rpc";
import Emote from "../../../components/Chat/Emote";
import StyleSheet from "../../../components/Chat/StyleSheet";
import { SelectInput, SelectOption } from "../../../components/Form";
import { Consumer as ChatConsumer, Provider as ChatProvider } from "../../../contexts/Chat";
import { Provider as ApiProvider } from "../../../contexts/FrontendApi";
import { emotes, modifiers } from "../../mocks/chat/assetBundle";
import ChatService from "../../mocks/chat/service";

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
        <ChatConsumer>
          {([{ liveEmotes, styles, uiConfig }]) => (
            <StyleSheet liveEmotes={liveEmotes} styles={styles} uiConfig={uiConfig} />
          )}
        </ChatConsumer>
        {children}
      </ChatProvider>
    </ApiProvider>
  );
};

const modifierOptions = modifiers.map((m) => ({ value: m, label: m }));

const Modifiers: React.FC = () => {
  const { control, watch } = useForm<{
    modifiers: SelectOption<string>[];
  }>({
    defaultValues: {
      modifiers: [],
    },
  });

  const values = watch();

  return (
    <div className="emotes app--dark">
      <div className="emotes__grid">
        <Chat>
          {emotes.map((emote) => (
            <div key={emote} className="emotes__grid__cell">
              <Emote
                name={emote}
                shouldAnimateForever
                modifiers={values.modifiers.map(({ value }) => value)}
              >
                {emote}
              </Emote>
            </div>
          ))}
        </Chat>
      </div>
      <div className="emotes__form">
        <SelectInput
          label="modifiers"
          name="modifiers"
          control={control}
          options={modifierOptions}
          menuPlacement="auto"
          isMulti
        />
      </div>
    </div>
  );
};

export default [
  {
    name: "Modifiers",
    component: () => <Modifiers />,
  },
];

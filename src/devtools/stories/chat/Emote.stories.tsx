import { PassThrough } from "stream";

import Host from "@memelabs/protobuf/lib/rpc/host";
import ServiceRegistry from "@memelabs/protobuf/lib/rpc/service";
import React, { useEffect, useMemo, useState } from "react";
import { useForm } from "react-hook-form";
import { useUpdateEffect } from "react-use";

import { FrontendClient } from "../../../apis/client";
import { Message } from "../../../apis/strims/chat/v1/chat";
import { registerChatFrontendService } from "../../../apis/strims/chat/v1/chat_rpc";
import Emote from "../../../components/Chat/Emote";
import ChatMessage from "../../../components/Chat/Message";
import ChatScroller, { MessageProps } from "../../../components/Chat/Scroller";
import StyleSheet from "../../../components/Chat/StyleSheet";
import { SelectInput, SelectOption } from "../../../components/Form";
import {
  Consumer as ChatConsumer,
  Provider as ChatProvider,
  useChat,
} from "../../../contexts/Chat";
import { Provider as ApiProvider } from "../../../contexts/FrontendApi";
import { emotes, modifiers } from "../../mocks/chat/assetBundle";
import MessageEmitter from "../../mocks/chat/MessageEmitter";
import ChatService from "../../mocks/chat/service";

const initChatState = (messages?: MessageEmitter): [ChatService, FrontendClient] => {
  const svc = new ServiceRegistry();
  const service = new ChatService(messages);
  registerChatFrontendService(svc, service);

  const [a, b] = [new PassThrough(), new PassThrough()];
  new Host(a, b, svc);
  return [service, new FrontendClient(b, a)];
};

interface ChatProps {
  messages?: MessageEmitter;
}

const Chat: React.FC<ChatProps> = ({ children, messages }) => {
  const [[service, client], setState] = useState(() => initChatState(messages));

  useUpdateEffect(() => setState(initChatState(messages)), [messages]);
  useEffect(() => () => service.destroy(), [service]);

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
    <div className="emotes app app--dark">
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

const initEmoteTesterMessages = (): MessageEmitter => {
  const messages = [
    new Message({
      nick: "rustler",
      serverTime: BigInt(Date.now()),
      body: "standard variants",
      entities: { emotes: [] },
    }),
    new Message({
      nick: "rustler",
      serverTime: BigInt(Date.now()),
      body: "test",
      entities: { emotes: [] },
    }),
    new Message({
      nick: "rustler",
      serverTime: BigInt(Date.now()),
      body: "emote in the test middle of a messages",
      entities: { emotes: [] },
    }),
    new Message({
      nick: "rustler",
      serverTime: BigInt(Date.now()),
      body: "emote at the end of a lot of text emote at the end of a lot of text emote at the end of a lot of text emote at the end of a lot of text test",
      entities: { emotes: [] },
    }),
    new Message({
      nick: "rustler",
      serverTime: BigInt(Date.now()),
      body: "as a wall of emotes test test test test test test test test test test test test test test test test test test test test test test test test test test test test test test test test test test test test test test test test test test test test test test test test",
      entities: { emotes: [] },
    }),
    new Message({
      nick: "rustler",
      serverTime: BigInt(Date.now()),
      body: "test",
      entities: { emotes: [] },
    }),
    new Message({
      nick: "rustler",
      serverTime: BigInt(Date.now()),
      body: "combo",
      entities: {
        emotes: [
          {
            name: "FeelsGoodMan",
            bounds: {
              start: 0,
              end: 5,
            },
            combo: 25,
          },
        ],
      },
    }),
  ];

  for (const message of messages) {
    for (let i = message.body.indexOf("test"); i !== -1; i = message.body.indexOf("test", i + 5)) {
      message.entities.emotes.push(
        new Message.Entities.Emote({
          name: "FeelsGoodMan",
          bounds: {
            start: i,
            end: i + 4,
          },
        })
      );
    }
  }

  return new MessageEmitter(0, messages.length, messages);
};

const EmoteTesterMessages: React.FC = () => {
  const [state, { getMessage, getMessageCount }] = useChat();

  return (
    <ChatScroller
      uiConfig={state.uiConfig}
      renderMessage={({ index, style }: MessageProps) => (
        <ChatMessage
          uiConfig={state.uiConfig}
          message={getMessage(index)}
          style={style}
          isMostRecent={index === getMessageCount() - 1}
        />
      )}
      messageCount={state.messages.length}
      messageSizeCache={state.messageSizeCache}
    />
  );
};

const EmoteTester: React.FC = () => {
  const messages = useMemo(() => initEmoteTesterMessages(), []);

  return (
    <div className="emote_tester app app--dark">
      <div className="emote_tester__messages">
        <Chat messages={messages}>
          <EmoteTesterMessages />
        </Chat>
      </div>
    </div>
  );
};

export default [
  {
    name: "Modifiers",
    component: Modifiers,
  },
  {
    name: "EmoteTester",
    component: EmoteTester,
  },
];

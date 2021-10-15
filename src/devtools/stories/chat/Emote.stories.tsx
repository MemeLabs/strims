import { PassThrough } from "stream";

import Host from "@memelabs/protobuf/lib/rpc/host";
import ServiceRegistry from "@memelabs/protobuf/lib/rpc/service";
import { Base64 } from "js-base64";
import React, { createContext, useContext, useEffect, useMemo, useState } from "react";
import { useForm } from "react-hook-form";
import { useUpdateEffect } from "react-use";

import { FrontendClient } from "../../../apis/client";
import { AssetBundle, IEmoteImage, Message } from "../../../apis/strims/chat/v1/chat";
import { registerChatFrontendService } from "../../../apis/strims/chat/v1/chat_rpc";
import Emote from "../../../components/Chat/Emote";
import ChatMessage from "../../../components/Chat/Message";
import ChatScroller, { MessageProps } from "../../../components/Chat/Scroller";
import StyleSheet, { ExtraEmoteRules } from "../../../components/Chat/StyleSheet";
import {
  ImageInput,
  ImageValue,
  InputLabel,
  SelectInput,
  SelectOption,
  TextInput,
} from "../../../components/Form";
import {
  Consumer as ChatConsumer,
  Provider as ChatProvider,
  useChat,
} from "../../../contexts/Chat";
import { Provider as ApiProvider } from "../../../contexts/FrontendApi";
import { emotes, modifiers } from "../../mocks/chat/assetBundle";
import imgBrick from "../../mocks/chat/emotes/static/Brick.png";
import MessageEmitter from "../../mocks/chat/MessageEmitter";
import ChatService from "../../mocks/chat/service";

const MockChatContext = createContext<ChatService>(null);

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
  shouldRenderStyleSheet?: boolean;
}

const Chat: React.FC<ChatProps> = ({ children, messages, shouldRenderStyleSheet = true }) => {
  const [[service, client], setState] = useState(() => initChatState(messages));

  useUpdateEffect(() => setState(initChatState(messages)), [messages]);
  useEffect(() => () => service.destroy(), [service]);

  return (
    <ApiProvider value={client}>
      <ChatProvider networkKey={new Uint8Array()} serverKey={new Uint8Array()}>
        {shouldRenderStyleSheet && (
          <ChatConsumer>
            {([{ liveEmotes, styles, uiConfig }]) => (
              <StyleSheet liveEmotes={liveEmotes} styles={styles} uiConfig={uiConfig} />
            )}
          </ChatConsumer>
        )}
        <MockChatContext.Provider value={service}>{children}</MockChatContext.Provider>
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
            name: "test",
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
          name: "test",
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

type EmoteSource =
  | {
      url: string;
      height: number;
      width: number;
    }
  | ImageValue;

interface EmoteTesterMessagesProps {
  emoteSource?: EmoteSource;
}

const EmoteTesterMessages: React.FC<EmoteTesterMessagesProps> = ({ emoteSource }) => {
  const [state] = useChat();
  const service = useContext(MockChatContext);

  useEffect(() => {
    const emitImage = (image: IEmoteImage) =>
      service.emitAssetBundle(
        new AssetBundle({
          isDelta: true,
          emotes: [
            {
              "id": BigInt(9999),
              "name": "test",
              "images": [image],
              "effects": [],
            },
          ],
        })
      );

    if (emoteSource && "data" in emoteSource) {
      emitImage({
        "data": Base64.toUint8Array(emoteSource.data),
        "fileType": 1,
        "height": emoteSource.height,
        "width": emoteSource.width,
        "scale": 2,
      });
    } else {
      void fetch(imgBrick)
        .then((res) => res.arrayBuffer())
        .then((data) =>
          emitImage({
            "data": new Uint8Array(data),
            "fileType": 1,
            "height": emoteSource?.height || 128,
            "width": emoteSource?.width || 168,
            "scale": 2,
          })
        );
    }
  }, [service, emoteSource]);

  const extraEmoteRules: ExtraEmoteRules = {};
  if (emoteSource && "url" in emoteSource) {
    extraEmoteRules.test = [
      ["background-image", `image-set(url(${emoteSource.url}) 4x)`],
      ["background-image", `-webkit-image-set(url(${emoteSource.url}) 4x)`],
    ];
  }

  return (
    <>
      <StyleSheet
        liveEmotes={state.liveEmotes}
        styles={state.styles}
        uiConfig={state.uiConfig}
        extraEmoteRules={extraEmoteRules}
      />
      <ChatScroller
        uiConfig={state.uiConfig}
        renderMessage={({ index, style }: MessageProps) => (
          <ChatMessage
            uiConfig={state.uiConfig}
            message={state.messages[index]}
            style={style}
            isMostRecent={false}
          />
        )}
        messageCount={state.messages.length}
        messageSizeCache={state.messageSizeCache}
      />
    </>
  );
};

interface EmoteTesterFormProps {
  url: string;
  image: ImageValue;
}

const EmoteTester: React.FC = () => {
  const messages = useMemo(() => initEmoteTesterMessages(), []);

  const { control, watch } = useForm<EmoteTesterFormProps>();
  const values = watch();

  const [emoteSource, setEmoteSource] = useState<EmoteSource>();

  useEffect(() => {
    if (values.url) {
      const img = new Image();
      img.onload = () =>
        setEmoteSource({
          url: values.url,
          height: img.height,
          width: img.width,
        });
      img.src = values.url;
    }
  }, [values.url]);

  useEffect(() => {
    if (values.image) {
      setEmoteSource(values.image);
    }
  }, [values.image]);

  return (
    <div className="emote_tester app app--dark">
      <div className="emote_tester__messages chat">
        <Chat messages={messages} shouldRenderStyleSheet={false}>
          <EmoteTesterMessages emoteSource={emoteSource} />
        </Chat>
      </div>
      <div className="emote_tester__form">
        <TextInput label="url" name="url" control={control} />
        <InputLabel component="div" text="image">
          <ImageInput name="image" control={control} />
        </InputLabel>
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

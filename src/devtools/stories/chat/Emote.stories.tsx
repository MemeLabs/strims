import Host from "@memelabs/protobuf/lib/rpc/host";
import ServiceRegistry from "@memelabs/protobuf/lib/rpc/service";
import { Base64 } from "js-base64";
import React, { createContext, useContext, useEffect, useMemo, useState } from "react";
import { useForm } from "react-hook-form";
import { useUpdateEffect } from "react-use";

import { FrontendClient } from "../../../apis/client";
import { AssetBundle, EmoteScale, Message } from "../../../apis/strims/chat/v1/chat";
import {
  registerChatFrontendService,
  registerChatServerFrontendService,
} from "../../../apis/strims/chat/v1/chat_rpc";
import { registerDirectoryFrontendService } from "../../../apis/strims/network/v1/directory/directory_rpc";
import Emote from "../../../components/Chat/Emote";
import ChatMessage from "../../../components/Chat/Message";
import ChatScroller, { MessageProps } from "../../../components/Chat/Scroller";
import StyleSheet from "../../../components/Chat/StyleSheet";
import { ImageValue, SelectInput, SelectOption, TextInput } from "../../../components/Form";
import {
  ChatConsumer,
  Provider as ChatProvider,
  RoomConsumer,
  RoomProvider,
  useChat,
  useRoom,
} from "../../../contexts/Chat";
import { Provider as DirectoryProvider } from "../../../contexts/Directory";
import { Provider as ApiProvider } from "../../../contexts/FrontendApi";
import { AsyncPassThrough } from "../../../lib/stream";
import ChatEmoteForm, { ChatEmoteFormData } from "../../../pages/Settings/Chat/ChatEmoteForm";
import { toEmoteProps } from "../../../pages/Settings/Chat/utils";
import { emoteNames, modifierNames } from "../../mocks/chat/assetBundle";
import imgBrick from "../../mocks/chat/emotes/static/Brick.png";
import MessageEmitter from "../../mocks/chat/MessageEmitter";
import ChatService from "../../mocks/chat/service";
import DirectoryService from "../../mocks/directory/service";

const MockChatContext = createContext<ChatService>(null);

const initChatState = (messages?: MessageEmitter): [ChatService, FrontendClient] => {
  const svc = new ServiceRegistry();
  const chatService = new ChatService(messages);
  const directoryService = new DirectoryService();
  registerChatFrontendService(svc, chatService);
  registerChatServerFrontendService(svc, chatService);
  registerDirectoryFrontendService(svc, directoryService);

  const [a, b] = [new AsyncPassThrough(), new AsyncPassThrough()];
  new Host(a, b, svc);
  return [chatService, new FrontendClient(b, a)];
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
      <DirectoryProvider>
        <ChatProvider>
          <RoomProvider networkKey={new Uint8Array()} serverKey={new Uint8Array()}>
            {shouldRenderStyleSheet && (
              <ChatConsumer>
                {([{ uiConfig }]) => (
                  <RoomConsumer>
                    {([room]) => (
                      <StyleSheet
                        liveEmotes={room.liveEmotes}
                        styles={room.styles}
                        uiConfig={uiConfig}
                      />
                    )}
                  </RoomConsumer>
                )}
              </ChatConsumer>
            )}
            <MockChatContext.Provider value={service}>{children}</MockChatContext.Provider>
          </RoomProvider>
        </ChatProvider>
      </DirectoryProvider>
    </ApiProvider>
  );
};

const modifierOptions = modifierNames.map((m) => ({ value: m, label: m }));

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
    <div className="emotes">
      <div className="emotes__grid">
        <Chat>
          {emoteNames.map((emote) => (
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
  formData: ChatEmoteFormData;
}

const EmoteTesterMessages: React.FC<EmoteTesterMessagesProps> = ({ formData }) => {
  const [{ uiConfig }, { mergeUIConfig }] = useChat();
  const [room] = useRoom();
  const service = useContext(MockChatContext);

  useEffect(() => {
    mergeUIConfig({
      animateForever: true,
    });
  }, []);

  useEffect(() => {
    if (!formData) {
      return;
    }

    service.emitAssetBundle(
      new AssetBundle({
        isDelta: true,
        emotes: [
          {
            ...toEmoteProps(formData),
            "id": BigInt(9999),
            "name": "test",
          },
        ],
      })
    );
  }, [service, formData]);

  return (
    <>
      <StyleSheet liveEmotes={room.liveEmotes} styles={room.styles} uiConfig={uiConfig} />
      <ChatScroller
        uiConfig={uiConfig}
        renderMessage={({ index, style }: MessageProps) => (
          <ChatMessage
            uiConfig={uiConfig}
            message={room.messages[index]}
            style={style}
            isMostRecent={false}
          />
        )}
        messageCount={room.messages.length}
        messageSizeCache={room.messageSizeCache}
      />
    </>
  );
};

interface EmoteTesterFormProps {
  url: string;
  image: ImageValue;
  legacyEmoteSpacing: boolean;
}

const EmoteTester: React.FC = () => {
  const messages = useMemo(() => initEmoteTesterMessages(), []);

  const [formData, setFormData] = useState<ChatEmoteFormData>(null);

  useEffect(() => {
    void fetch(imgBrick)
      .then((res) => res.arrayBuffer())
      .then((data) =>
        setFormData({
          name: "test",
          image: {
            data: Base64.fromUint8Array(new Uint8Array(data)),
            type: "image/png",
            height: 128,
            width: 168,
          },
          scale: {
            value: EmoteScale.EMOTE_SCALE_4X,
            label: "4x",
          },
          contributor: "",
          contributorLink: "",
          css: "",
          animated: false,
          animationFrameCount: 0,
          animationDuration: 0,
          animationIterationCount: 0,
          animationEndOnFrame: 0,
          animationLoopForever: false,
          animationAlternateDirection: false,
          defaultModifiers: [],
        })
      );
  }, []);

  return (
    <div className="emote_tester">
      <Chat messages={messages} shouldRenderStyleSheet={false}>
        <div className="emote_tester__messages chat">
          <EmoteTesterMessages formData={formData} />
        </div>
        <div className="emote_tester__form">
          {formData && (
            <ChatEmoteForm
              values={formData}
              onSubmit={(values) => setFormData(values)}
              submitLabel="Update Emote"
            />
          )}
        </div>
      </Chat>
    </div>
  );
};

interface ComboMessagesProps {
  emote?: string;
  count?: number;
  interval?: number;
}

const ComboMessages: React.FC<ComboMessagesProps> = ({
  emote = "FeelsGoodMan",
  count = 10,
  interval = 100,
}) => {
  const [{ uiConfig }] = useChat();
  const [room] = useRoom();
  const service = useContext(MockChatContext);
  const [done, setDone] = useState(true);

  useEffect(() => {
    setDone(false);

    let i = 0;
    const iid = setInterval(() => {
      service.emitMessage(
        new Message({
          nick: "rustler",
          serverTime: BigInt(Date.now()),
          body: "combo",
          entities: {
            emotes: [
              {
                name: emote,
                bounds: {
                  start: 0,
                  end: 5,
                },
                combo: i,
              },
            ],
          },
        })
      );

      if (i++ === count) {
        stop();
      }
    }, interval);

    const stop = () => {
      setDone(true);
      clearInterval(iid);
    };

    return stop;
  }, [emote, count, interval]);

  return (
    <ChatScroller
      uiConfig={uiConfig}
      renderMessage={({ index, style }: MessageProps) => (
        <ChatMessage
          uiConfig={uiConfig}
          message={room.messages[index]}
          style={style}
          isMostRecent={!done && index === room.messages.length - 1}
        />
      )}
      messageCount={room.messages.length}
      messageSizeCache={room.messageSizeCache}
    />
  );
};

const emoteOptions = emoteNames.map((m) => ({ value: m, label: m }));

interface ComboFormProps {
  emote: SelectOption<string>;
  count: number;
  interval: number;
}

const defaultComboFormValues: ComboFormProps = {
  emote: emoteOptions.find(({ value }) => value === "FeelsGoodMan"),
  count: 10,
  interval: 100,
};

const Combo: React.FC = () => {
  const messages = useMemo(() => new MessageEmitter(0, 0), []);

  const { control, watch } = useForm<ComboFormProps>({
    defaultValues: defaultComboFormValues,
  });
  const values = watch();

  return (
    <div className="combo">
      <div className="combo__messages chat">
        <Chat messages={messages}>
          <ComboMessages
            emote={values.emote.value}
            count={values.count}
            interval={values.interval}
          />
        </Chat>
      </div>
      <div className="combo__form">
        <SelectInput label="emote" options={emoteOptions} name="emote" control={control} />
        <TextInput label="count" type="number" name="count" control={control} />
        <TextInput label="interval" type="number" name="interval" control={control} />
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
  {
    name: "Combo",
    component: Combo,
  },
];

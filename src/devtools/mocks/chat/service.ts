// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { PassThrough } from "stream";

import { Readable } from "@memelabs/protobuf/lib/rpc/stream";

import * as chatv1 from "../../../apis/strims/chat/v1/chat";
import {
  ChatFrontendService,
  ChatServerFrontendService,
  UnimplementedChatFrontendService,
  UnimplementedChatServerFrontendService,
} from "../../../apis/strims/chat/v1/chat_rpc";
import assetBundle, { modifiers } from "../../mocks/chat/assetBundle";
import MessageEmitter from "../../mocks/chat/MessageEmitter";

class ChatService {
  messages: Readable<chatv1.Message[]>;
  assetBundles: Readable<chatv1.AssetBundle>;
  uiConfigs: Readable<chatv1.UIConfig>;

  constructor(messages = new MessageEmitter({ ivl: 1000 })) {
    this.messages = messages;
    this.assetBundles = new PassThrough({ objectMode: true });
    this.uiConfigs = new PassThrough({ objectMode: true });
  }

  destroy(): void {
    this.messages.destroy();
    this.assetBundles.destroy();
  }

  emitMessage(message: chatv1.Message): void {
    this.messages.push([message]);
  }

  emitAssetBundle(bundle: chatv1.AssetBundle): void {
    this.assetBundles.push(bundle);
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

    this.assetBundles.on("data", (assetBundle) =>
      ch.push(
        new chatv1.OpenClientResponse({
          body: new chatv1.OpenClientResponse.Body({ assetBundle }),
        })
      )
    );

    void assetBundle().then((assetBundle) => this.assetBundles.push(assetBundle));

    this.messages.on("data", (messages) =>
      ch.push(
        new chatv1.OpenClientResponse({
          body: {
            serverEvents: {
              events: messages.map((message) => ({
                body: { message },
              })),
            },
          },
        })
      )
    );

    return ch;
  }

  clientSendMessage(req: chatv1.ClientSendMessageRequest): chatv1.ClientSendMessageResponse {
    this.messages.push([
      new chatv1.Message({
        nick: "test_user",
        serverTime: BigInt(Date.now()),
        body: req.body,
        entities: new chatv1.Message.Entities(),
      }),
    ]);
    return new chatv1.ClientSendMessageResponse();
  }

  setUIConfig(req: chatv1.SetUIConfigRequest): chatv1.SetUIConfigResponse {
    this.uiConfigs.push(req.uiConfig);
    return new chatv1.SetUIConfigResponse();
  }

  watchUIConfig(): Readable<chatv1.WatchUIConfigResponse> {
    const ch = new PassThrough({ objectMode: true });

    this.uiConfigs.on("data", (uiConfig) =>
      ch.push(new chatv1.WatchUIConfigResponse({ config: { uiConfig } }))
    );

    return ch;
  }

  watchWhispers(): Readable<chatv1.WatchWhispersResponse> {
    return new PassThrough({ objectMode: true });
  }

  // ChatServerFrontendService

  listModifiers(): chatv1.ListModifiersResponse {
    return new chatv1.ListModifiersResponse({ modifiers });
  }
}

interface ChatService extends ChatFrontendService, ChatServerFrontendService {}

Object.defineProperties(ChatService.prototype, {
  ...Object.getOwnPropertyDescriptors(UnimplementedChatFrontendService.prototype),
  ...Object.getOwnPropertyDescriptors(UnimplementedChatServerFrontendService.prototype),
  ...Object.getOwnPropertyDescriptors(ChatService.prototype),
});

export default ChatService;

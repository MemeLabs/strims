import { Base64 } from "js-base64";
import React from "react";
import { useParams } from "react-router-dom";

import { EmoteEffect } from "../../../apis/strims/chat/v1/chat";
import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import ChatEmoteForm, { ChatEmoteFormData } from "./ChatEmoteForm";
import { fileTypeToMimeType, scaleToDOMScale, toEmoteProps } from "./utils";

const ChatEmoteEditFormPage: React.FC = () => {
  const { serverId, emoteId } = useParams<"serverId" | "emoteId">();
  const [{ value, ...getRes }] = useCall("chatServer", "getEmote", {
    args: [{ id: BigInt(emoteId) }],
  });

  const [updateRes, updateChatEmote] = useLazyCall("chatServer", "updateEmote");

  const onSubmit = (data: ChatEmoteFormData) =>
    updateChatEmote({
      serverId: BigInt(serverId),
      id: BigInt(emoteId),
      ...toEmoteProps(data),
    });

  if (getRes.loading) {
    return null;
  }

  const { emote } = value;
  const data: ChatEmoteFormData = {
    name: emote.name,
    image: {
      data: Base64.fromUint8Array(emote.images[0].data),
      type: fileTypeToMimeType(emote.images[0].fileType),
      height: emote.images[0].height,
      width: emote.images[0].width,
    },
    scale: {
      value: emote.images[0].scale,
      label: scaleToDOMScale(emote.images[0].scale),
    },
    contributor: emote.contributor?.name,
    contributorLink: emote.contributor?.link,
    css: "",
    animated: false,
    animationFrameCount: 0,
    animationDuration: 0,
    animationIterationCount: 0,
    animationEndOnFrame: 0,
    animationLoopForever: false,
    animationAlternateDirection: false,
    defaultModifiers: [],
  };

  emote.effects.forEach(({ effect }) => {
    switch (effect.case) {
      case EmoteEffect.EffectCase.CUSTOM_CSS:
        data.css = effect.customCss.css;
        break;
      case EmoteEffect.EffectCase.SPRITE_ANIMATION:
        data.animated = true;
        data.animationFrameCount = effect.spriteAnimation.frameCount;
        data.animationDuration = effect.spriteAnimation.durationMs;
        data.animationIterationCount = effect.spriteAnimation.iterationCount;
        data.animationEndOnFrame = effect.spriteAnimation.endOnFrame;
        data.animationLoopForever = effect.spriteAnimation.loopForever;
        data.animationAlternateDirection = effect.spriteAnimation.alternateDirection;
        break;
      case EmoteEffect.EffectCase.DEFAULT_MODIFIERS:
        data.defaultModifiers = effect.defaultModifiers.modifiers.map((m) => ({
          label: m,
          value: m,
        }));
        break;
    }
  });

  return (
    <>
      <TableTitleBar label="Edit Emote" backLink={`/settings/chat-servers/${serverId}/emotes`} />
      <ChatEmoteForm
        onSubmit={onSubmit}
        error={getRes.error || updateRes.error}
        loading={getRes.loading || updateRes.loading}
        values={data}
        serverId={BigInt(serverId)}
        submitLabel={"Update Emote"}
      />
    </>
  );
};

export default ChatEmoteEditFormPage;

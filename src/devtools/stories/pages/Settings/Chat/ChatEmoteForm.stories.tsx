// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Error as RPCError } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import { Base64 } from "js-base64";
import React from "react";

import ChatEmoteForm, {
  ChatEmoteFormData,
  ChatEmoteFormProps,
} from "../../../../../pages/Settings/Chat/ChatEmoteForm";
import { scaleOptions } from "../../../../../pages/Settings/Chat/utils";
import { emoteFixture, emoteImage } from "../../../../mocks/chatserver/fixtures";
import { Story } from "../../../../storybook";
import { Act, ChatSettings, scrollTo, submit, useLoaded } from "./harness";

const options = (values: string[]) => values.map((v) => ({ label: v, value: v }));

// Fixture emote `name` as the form takes it.
const formValues = async (name: string): Promise<ChatEmoteFormData> => {
  const f = emoteFixture(name);
  const image = await emoteImage(f.url);
  const data: ChatEmoteFormData = {
    name,
    image: {
      data: Base64.fromUint8Array(image.data),
      type: "image/png",
      height: image.height,
      width: image.width,
    },
    scale: scaleOptions[1],
    contributor: "jbpratt",
    contributorLink: "https://example.com/jbpratt",
    labels: options(f.labels),
    enable: true,
    assetCount: 0,
    scss: "",
    extraWrapCount: 0,
    wrapAdjacent: false,
    animated: false,
    animationFrameCount: 0,
    animationDuration: 0,
    animationIterationCount: 0,
    animationEndOnFrame: 0,
    animationLoopForever: false,
    animationAlternateDirection: false,
    defaultModifiers: [],
  };
  for (const { effect } of f.effects) {
    if ("spriteAnimation" in effect) {
      const s = effect.spriteAnimation;
      Object.assign(data, {
        animated: true,
        animationFrameCount: s.frameCount,
        animationDuration: s.durationMs,
        animationIterationCount: s.iterationCount,
        animationEndOnFrame: s.endOnFrame ?? 0,
        animationLoopForever: s.loopForever,
        animationAlternateDirection: s.alternateDirection,
      });
    } else if ("customCss" in effect) {
      data.scss = effect.customCss.styleSheet.scss;
      data.extraWrapCount = 1;
    } else if ("defaultModifiers" in effect) {
      data.defaultModifiers = options(effect.defaultModifiers.modifiers);
    }
  }
  return data;
};

const noop = (): void => undefined;

interface FormProps extends Partial<ChatEmoteFormProps> {
  /** Starts with this fixture emote's values. */
  emote?: string;
  act?: Act;
}

const Form: React.FC<FormProps> = ({ emote, act, ...props }) => {
  const values = useLoaded(async () => (emote ? formValues(emote) : null));
  if (values === undefined) {
    return null;
  }
  return (
    <ChatSettings
      path="/settings/chat-servers/1/emotes/new"
      act={act}
      element={
        <ChatEmoteForm
          onSubmit={noop}
          serverId={BigInt(1)}
          values={values ?? undefined}
          submitLabel={emote ? "Update Emote" : "Create Emote"}
          {...props}
        />
      }
    />
  );
};

const stories: Story[] = [
  {
    name: "New",
    component: () => <Form />,
  },
  {
    // A static image, labelled and credited.
    name: "Static",
    component: () => <Form emote="PepeLaugh" />,
  },
  {
    // A sprite sheet: the animation fields enabled.
    name: "Animated",
    component: () => <Form emote="catJAM" />,
  },
  {
    // Animated with a style sheet of its own and a default modifier (scrolled to it).
    name: "Styled",
    component: () => <Form emote="REE" act={(root) => scrollTo(root, "Default modifiers")} />,
  },
  {
    name: "Invalid",
    component: () => <Form act={submit} />,
  },
  {
    name: "Error",
    component: () => (
      <Form emote="PepeLaugh" error={new RPCError({ message: "emote name in use" })} />
    ),
  },
  {
    name: "Loading",
    component: () => <Form emote="PepeLaugh" loading />,
  },
];

export default stories;

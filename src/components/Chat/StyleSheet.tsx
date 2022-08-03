// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import React, { useEffect, useLayoutEffect, useRef, useState } from "react";

import {
  Emote,
  EmoteEffect,
  EmoteFileType,
  EmoteScale,
  UIConfig,
} from "../../apis/strims/chat/v1/chat";
import { ChatStyles } from "../../contexts/Chat";

const toMimeType = (fileType: EmoteFileType): string => {
  switch (fileType) {
    case EmoteFileType.FILE_TYPE_PNG:
      return "image/png";
    case EmoteFileType.FILE_TYPE_GIF:
      return "image/gif";
  }
};

const toCSSScale = (scale: EmoteScale): number => {
  switch (scale) {
    case EmoteScale.EMOTE_SCALE_1X:
      return 1;
    case EmoteScale.EMOTE_SCALE_2X:
      return 2;
    case EmoteScale.EMOTE_SCALE_4X:
      return 4;
  }
};

export const computeEmoteWidth = (e: Emote) => {
  const sample = e.images[0];
  const sampleScale = toCSSScale(sample.scale);
  let width = sample.width / sampleScale;

  for (const { effect } of e.effects) {
    switch (effect.case) {
      case EmoteEffect.EffectCase.SPRITE_ANIMATION:
        width = width / effect.spriteAnimation.frameCount;
    }
  }

  return width;
};

export type ExtraRules = { [key: string]: PropList };

export interface StyleSheetProps {
  liveEmotes: Emote[];
  styles: ChatStyles;
  uiConfig: UIConfig;
  extraEmoteRules?: ExtraRules;
  extraContainerRules?: ExtraRules;
}

interface EmoteState {
  uris: string[];
  name: string;
}

type Prop = [string, string];
type PropList = Prop[];

const upsertProps = (prev: PropList, ...vs: Prop[]): PropList => [
  ...(prev ?? []).filter(([pp]) => !vs.some(([vp]) => pp === vp)),
  ...vs,
];

const deleteMatchingCSSRules = (sheet: CSSStyleSheet, filter: (rule: CSSRule) => boolean): void => {
  for (let i = 0; i < sheet.cssRules.length; ) {
    if (filter(sheet.cssRules.item(i))) {
      sheet.deleteRule(i);
    } else {
      i++;
    }
  }
};

const StyleSheet: React.FC<StyleSheetProps> = ({
  liveEmotes,
  styles,
  uiConfig,
  extraEmoteRules = {},
  extraContainerRules = {},
}) => {
  const ref = useRef<HTMLStyleElement>(null);
  const [, setEmotes] = useState(new Map<Emote, EmoteState>());

  useLayoutEffect(() => {
    setEmotes((prev) => {
      const next = new Map(prev);
      const added = liveEmotes.filter((e) => !prev.has(e));
      const removed = Array.from(prev.entries()).filter(([e]) => !liveEmotes.includes(e));

      for (const [e, { name, uris }] of removed) {
        for (const uri of uris) {
          URL.revokeObjectURL(uri);
        }
        next.delete(e);
        deleteMatchingCSSRules(ref.current.sheet, (r) => r.cssText.includes(name));
      }
      for (const e of added) {
        const uris = e.images.map(({ data, fileType }) =>
          URL.createObjectURL(new Blob([data], { type: toMimeType(fileType) }))
        );
        const { name } = styles.emotes.get(e.name);
        const imageSet = e.images.map(({ scale }, i) => `url(${uris[i]}) ${toCSSScale(scale)}x`);
        const sample = e.images[0];
        const sampleScale = toCSSScale(sample.scale);
        const height = sample.height / sampleScale;
        const width = sample.width / sampleScale;

        let rules: PropList = [
          ["background-image", `image-set(${imageSet.join(", ")})`],
          ["background-image", `-webkit-image-set(${imageSet.join(", ")})`],
        ];
        let containerRules: PropList = [
          ["--width", `${width}px`],
          ["--height", `${height}px`],
          ["--background-image", `url(${uris[0]})`],
        ];

        for (const { effect } of e.effects) {
          switch (effect.case) {
            case EmoteEffect.EffectCase.CUSTOM_CSS:
              break;
            case EmoteEffect.EffectCase.SPRITE_ANIMATION:
              {
                const {
                  frameCount,
                  durationMs,
                  iterationCount,
                  endOnFrame,
                  alternateDirection,
                  loopForever,
                } = effect.spriteAnimation;
                const frameWidth = width / frameCount;
                const direction = alternateDirection ? "alternate" : "normal";
                const reverseDirection = alternateDirection ? "alternate-reverse" : "reverse";
                const maxIterations = loopForever
                  ? "infinite"
                  : iterationCount + endOnFrame / frameCount;

                containerRules = upsertProps(
                  containerRules,
                  ["--width", `${frameWidth}px`],
                  ["--animation-duration-ms", durationMs.toString()],
                  ["--animation-iterations", iterationCount.toString()],
                  ["--animation-max-iterations", maxIterations.toString()],
                  ["--animation-spritesheet-width", `${width}px`],
                  ["--animation-end-on-frame", endOnFrame.toString()],
                  ["--animation-frame-count", frameCount.toString()],
                  ["--animation-direction", direction],
                  ["--animation-reverse-direction", reverseDirection]
                );
              }
              break;
          }
        }

        if (e.name in extraEmoteRules) {
          rules = upsertProps(rules, ...extraEmoteRules[e.name]);
        }
        if (e.name in extraContainerRules) {
          containerRules = upsertProps(containerRules, ...extraContainerRules[e.name]);
        }

        ref.current.sheet.insertRule(
          `.${name} {${[...rules, ...containerRules].map((r) => r.join(":")).join(";")}}`
        );
        ref.current.sheet.insertRule(
          `.${name}_container {${containerRules.map((r) => r.join(":")).join(";")}}`
        );

        next.set(e, { name, uris });
      }

      return next;
    });
  }, [liveEmotes, styles]);

  useLayoutEffect(() => {
    deleteMatchingCSSRules(ref.current.sheet, (r) => r.cssText.includes("chat__message--tag_"));

    for (const { name, color, sensitive } of styles.tags) {
      let rules: PropList = [["--tag-color", color]];
      if (sensitive && uiConfig.hideNsfw) {
        rules = upsertProps(rules, ["display", "none"]);
      }

      ref.current.sheet.insertRule(
        `.chat__message--tag_${name} {${rules.map((r) => r.join(":")).join(";")}}`
      );
    }

    deleteMatchingCSSRules(ref.current.sheet, (r) => r.cssText.includes("chat__message--author_"));
    deleteMatchingCSSRules(ref.current.sheet, (r) => r.cssText.includes("chat__message--mention_"));

    const props = new Map<string, PropList>();

    for (const { peerKey, color } of uiConfig.tags) {
      const rules: PropList = [["--color-chat-author-tag", color]];
      if (uiConfig.taggedVisibility) {
        rules.push(["--color-background-chat-message", "var(--color-background-chat-tagged)"]);
      }
      const key = Base64.fromUint8Array(peerKey, true);
      props.set(key, upsertProps(props.get(key), ...rules));
    }

    for (const { peerKey } of uiConfig.highlights) {
      const rules: PropList = [
        ["--color-background-chat-message", "var(--color-background-chat-highlight)"],
      ];
      const key = Base64.fromUint8Array(peerKey, true);
      props.set(key, upsertProps(props.get(key), ...rules));
    }

    for (const { peerKey } of uiConfig.ignores) {
      const key = Base64.fromUint8Array(peerKey, true);
      props.set(key, upsertProps(props.get(key), ["display", "none"]));
    }

    for (const [key, rules] of props) {
      ref.current.sheet.insertRule(
        `.chat__message--author_${key} {${rules.map((r) => r.join(":")).join(";")}}`
      );
    }

    if (styles.selectedPeers.size !== 0) {
      const keys = [];
      for (const key of styles.selectedPeers) {
        keys.push(`.chat__message--author_${key}`);
        if (uiConfig.focusMentioned) {
          keys.push(`.chat__message--mention_${key}`);
        }
      }
      const selector = `.chat__message:not(${keys.join(", ")})`;
      ref.current.sheet.insertRule(
        `${selector} { --opacity-chat-message: var(--opacity-chat-unselected); }`
      );
    }

    if (uiConfig.ignoreMentions) {
      for (const { peerKey } of uiConfig.ignores) {
        ref.current.sheet.insertRule(
          `.chat__message--mention_${Base64.fromUint8Array(peerKey, true)} { display: none; }`
        );
      }
    }
  }, [styles, uiConfig]);

  useEffect(() => {
    return () =>
      setEmotes((prev) => {
        for (const { uris } of prev.values()) {
          for (const uri of uris) {
            URL.revokeObjectURL(uri);
          }
        }
        return prev;
      });
  }, []);

  return <style ref={ref} />;
};

export default StyleSheet;

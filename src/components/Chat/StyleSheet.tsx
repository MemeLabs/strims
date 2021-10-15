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

export type ExtraEmoteRules = { [key: string]: PropList };

export interface StyleSheetProps {
  liveEmotes: Emote[];
  styles: ChatStyles;
  uiConfig: UIConfig;
  extraEmoteRules?: ExtraEmoteRules;
}

interface EmoteState {
  uris: string[];
  name: string;
}

type Prop = [string, string];
type PropList = Prop[];

const upsertProps = (prev: PropList, ...vs: Prop[]): PropList => [
  ...prev.filter(([pp]) => !vs.some(([vp]) => pp === vp)),
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
}) => {
  const ref = useRef<HTMLStyleElement>(null);
  const [, setEmotes] = useState(new Map<Emote, EmoteState>());

  useLayoutEffect(() => {
    setEmotes((prev) => {
      const next = new Map(Array.from(prev.entries()));
      const added = liveEmotes.filter((e) => !prev.has(e));
      const removed = Array.from(prev.entries()).filter(([e]) => !liveEmotes.includes(e));

      removed.forEach(([e, { name, uris }]) => {
        uris.forEach((uri) => URL.revokeObjectURL(uri));
        next.delete(e);
        deleteMatchingCSSRules(ref.current.sheet, (r) => r.cssText.includes(name));
      });
      added.forEach((e) => {
        const uris = e.images.map(({ data, fileType }) =>
          URL.createObjectURL(new Blob([data], { type: toMimeType(fileType) }))
        );
        const { name } = styles.emotes[e.name];
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

        e.effects.forEach(({ effect }) => {
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
        });

        if (e.name in extraEmoteRules) {
          rules = upsertProps(rules, ...extraEmoteRules[e.name]);
        }

        ref.current.sheet.insertRule(
          `.${name} {${[...rules, ...containerRules].map((r) => r.join(":")).join(";")}}`
        );
        ref.current.sheet.insertRule(
          `.${name}_container {${containerRules.map((r) => r.join(":")).join(";")}}`
        );

        next.set(e, { name, uris });
      });

      return next;
    });
  }, [liveEmotes, styles]);

  useLayoutEffect(() => {
    deleteMatchingCSSRules(ref.current.sheet, (r) => r.cssText.includes("chat__message--tag_"));

    styles.tags.forEach(({ name, color, sensitive }) => {
      let rules: PropList = [["--tag-color", color]];
      if (sensitive && uiConfig.hideNsfw) {
        rules = upsertProps(rules, ["display", "none"]);
      }

      ref.current.sheet.insertRule(
        `.chat__message--tag_${name} {${rules.map((r) => r.join(":")).join(";")}}`
      );
    });
  }, [styles, uiConfig]);

  useEffect(() => {
    return () =>
      setEmotes((prev) => {
        Array.from(prev.entries()).forEach(([, { uris }]) =>
          uris.forEach((uri) => URL.revokeObjectURL(uri))
        );
        return prev;
      });
  }, []);

  return <style ref={ref} />;
};

export default StyleSheet;

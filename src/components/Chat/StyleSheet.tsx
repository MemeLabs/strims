import React, { useLayoutEffect, useRef, useState } from "react";

import { Emote, EmoteEffect, EmoteFileType, EmoteScale } from "../../apis/strims/chat/v1/chat";
import { StyleMap } from "../../contexts/Chat";

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

interface StyleSheetProps {
  liveEmotes: Emote[];
  styles: StyleMap;
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

const StyleSheet: React.FC<StyleSheetProps> = ({ liveEmotes, styles }) => {
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

        for (let i = 0; i < ref.current.sheet.cssRules.length; ) {
          if (ref.current.sheet.cssRules.item(i).cssText.includes(name)) {
            ref.current.sheet.deleteRule(i);
          }
        }
      });
      added.forEach((e) => {
        const uris = e.images.map(({ data, fileType }) =>
          URL.createObjectURL(new Blob([data], { type: toMimeType(fileType) }))
        );
        const { name } = styles[e.name];
        const imageSet = e.images.map(({ scale }, i) => `url(${uris[i]}) ${toCSSScale(scale)}x`);
        const sample = e.images[0];
        const sampleScale = toCSSScale(sample.scale);
        const height = sample.height / sampleScale;
        const width = sample.width / sampleScale;

        let rules: PropList = [
          ["background-image", `image-set(${imageSet.join(", ")})`],
          ["background-image", `-webkit-image-set(${imageSet.join(", ")})`],
          ["background-repeat", `"no-repeat"`],
          ["width", `${width}px`],
          ["height", `${height}px`],
          ["margin-top", `calc(0.5em - ${height / 2}px)`],
          ["margin-bottom", `calc(0.5em - ${height / 2}px)`],
        ];
        let containerRules: PropList = [
          ["--width", `${width}px`],
          ["--height", `${height}px`],
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
                  loopForever,
                  alternateDirection,
                } = effect.spriteAnimation;
                const frameWidth = width / frameCount;
                const direction = alternateDirection ? "alternate" : "normal";
                const animName = `${name}_anim`;
                const iterations = iterationCount + endOnFrame / frameCount;

                rules = upsertProps(
                  rules,
                  ["width", `${frameWidth}px`],
                  ["background-position-x", `-${endOnFrame * frameWidth}px`],
                  [
                    "animation",
                    `${animName} ${durationMs}ms steps(${frameCount}) ${iterations} ${direction}`,
                  ]
                );
                containerRules = upsertProps(
                  containerRules,
                  ["--width", `${frameWidth}px`],
                  ["--animation-duration-ms", durationMs.toString()],
                  ["--animation-iterations", iterations.toString()]
                );

                const loopRuleSelector = [`.${name}:hover`];
                if (loopForever) {
                  loopRuleSelector.push(`.${name}.chat__emote--animate_forever`);
                }
                ref.current.sheet.insertRule(
                  `${loopRuleSelector.join(", ")} {` +
                    `animation: ${animName} ${durationMs}ms steps(${frameCount}) infinite ${direction}` +
                    `}`
                );

                ref.current.sheet.insertRule(
                  [
                    `@keyframes ${animName} {`,
                    `0% { background-position: 0; }`,
                    `100% { background-position: -${width}px; }`,
                    `}`,
                  ].join("\n")
                );
              }
              break;
          }
        });

        ref.current.sheet.insertRule(`.${name} {${rules.map((r) => r.join(":")).join(";")}}`);
        ref.current.sheet.insertRule(
          `.${name}_container {${containerRules.map((r) => r.join(":")).join(";")}}`
        );

        next.set(e, { name, uris });
      });

      return next;
    });
  }, [liveEmotes, styles]);

  return <style ref={ref} />;
};

export default StyleSheet;

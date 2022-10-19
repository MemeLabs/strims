// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import * as csstree from "css-tree";
import React, { useEffect, useLayoutEffect, useRef, useState } from "react";

import {
  Emote,
  EmoteEffect,
  EmoteFileType,
  EmoteScale,
  Modifier,
  UIConfig,
  UIConfigHighlight,
  UIConfigIgnore,
  UIConfigTag,
} from "../../apis/strims/chat/v1/chat";
import { ChatStyles } from "../../contexts/Chat";
import { createImageObjectURL } from "../../lib/image";

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
  id: number;
  liveEmotes: Emote[];
  styles: ChatStyles;
  uiConfig: UIConfig;
  uiConfigHighlights?: Map<string, UIConfigHighlight>;
  uiConfigTags?: Map<string, UIConfigTag>;
  uiConfigIgnores?: Map<string, UIConfigIgnore>;
  extraEmoteRules?: ExtraRules;
  extraContainerRules?: ExtraRules;
}

interface EmoteState {
  uris: string[];
  name: string;
}

interface ModifierState {
  uris: Map<string, string>;
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

const defaultUIConfigHighlights: Map<string, UIConfigHighlight> = new Map();
const defaultUIConfigTags: Map<string, UIConfigTag> = new Map();
const defaultUIConfigIgnores: Map<string, UIConfigIgnore> = new Map();

const StyleSheet: React.FC<StyleSheetProps> = ({
  id,
  liveEmotes,
  styles,
  uiConfig,
  uiConfigHighlights = defaultUIConfigHighlights,
  uiConfigTags = defaultUIConfigTags,
  uiConfigIgnores = defaultUIConfigIgnores,
  extraEmoteRules = {},
  extraContainerRules = {},
}) => {
  const ref = useRef<HTMLStyleElement>(null);
  const [, setEmotes] = useState(new Map<Emote, EmoteState>());
  const scope = `chat-${id}`;

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
          `#${scope} .${name} {${[...rules, ...containerRules].map((r) => r.join(":")).join(";")}}`
        );
        ref.current.sheet.insertRule(
          `#${scope} .${name}_container {${containerRules.map((r) => r.join(":")).join(";")}}`
        );

        next.set(e, { name, uris });
      }

      return next;
    });
  }, [liveEmotes, styles]);

  const [, setModifiers] = useState(new Map<Modifier, ModifierState>());

  useLayoutEffect(() => {
    const isNodeType = (target: csstree.CssNode["type"]) => {
      return ({ type }: csstree.CssNode) => type === target;
    };

    const sanitizeSelector = (node: csstree.Selector, name: string) => {
      const { first } = node.children;
      if (first.type !== "ClassSelector") {
        return first.type === "Percentage";
      }
      if (first.name !== name) {
        return false;
      }

      first.name = `chat__emote_container--${name}`;
      node.children.prependList(
        new csstree.List<csstree.CssNode>().fromArray([
          csstree.fromPlainObject({
            type: "IdSelector",
            name: scope,
          }),
          csstree.fromPlainObject({
            type: "Combinator",
            name: " ",
          }),
        ])
      );
      return true;
    };

    const sanitizeDeclaration = (node: csstree.Declaration) => {
      if (node.property !== "position") {
        return true;
      }

      const value = csstree.find(node, isNodeType("Identifier")) as csstree.Identifier;
      return value?.name === "relative" || value?.name === "absolute";
    };

    const sanitizeUrl = (node: csstree.Url, uris: Map<string, string>) => {
      if (typeof node.value === "string") {
        const ok = uris.has(node.value);
        node.value = uris.get(node.value) as unknown as csstree.Raw;
        return ok;
      }
      return false;
    };

    const sanitizeFunction = (node: csstree.FunctionNode, uris: Map<string, string>) => {
      if (node.name !== "image-set") {
        return true;
      }

      const isValue = ({ type }: csstree.CssNode) => type === "String" || type === "Raw";
      const values = csstree.findAll(node, isValue) as (csstree.StringNode | csstree.Raw)[];
      for (const value of values) {
        if (!uris.has(value.value)) {
          return false;
        }
        value.value = uris.get(value.value);
      }
      return true;
    };

    const sanitizeStyleSheet = (css: string = "", name: string, uris: Map<string, string>) => {
      const ast = csstree.parse(css);
      let ok = true;
      csstree.walk(ast, (node) => {
        switch (node.type) {
          case "Selector":
            ok &&= sanitizeSelector(node, name);
            break;
          case "Declaration":
            ok &&= sanitizeDeclaration(node);
            break;
          case "Url":
            ok &&= sanitizeUrl(node, uris);
            break;
          case "Function":
            ok &&= sanitizeFunction(node, uris);
            break;
        }
      });
      if (!ok) {
        return [];
      }

      const sheet = csstree.find(ast, isNodeType("StyleSheet")) as csstree.StyleSheet;
      return ok ? sheet.children.toArray().map((node) => csstree.generate(node)) : [];
    };

    setModifiers((prev) => {
      const next = new Map(prev);
      const modifiers = Array.from(styles.modifiers.values());
      const added = modifiers.filter((e) => !prev.has(e));
      const removed = Array.from(prev.entries()).filter(([e]) => !modifiers.includes(e));

      for (const [m, { name, uris }] of removed) {
        for (const uri of uris.values()) {
          URL.revokeObjectURL(uri);
        }
        next.delete(m);
        deleteMatchingCSSRules(ref.current.sheet, (r) =>
          r.cssText.includes(`#${scope} chat__emote_container--${name}`)
        );
      }
      for (const m of added) {
        const { name, styleSheet } = m;
        const uris = new Map(
          styleSheet?.assets.map(({ name, image }) => [name, createImageObjectURL(image)])
        );

        const rules = sanitizeStyleSheet(styleSheet?.css, name, uris);
        for (const rule of rules) {
          ref.current.sheet.insertRule(rule);
        }

        next.set(m, { name, uris });
      }

      return next;
    });
  }, [styles]);

  useLayoutEffect(() => {
    deleteMatchingCSSRules(ref.current.sheet, (r) => r.cssText.includes("chat__message--tag_"));

    for (const { name, color, sensitive } of styles.tags) {
      let rules: PropList = [["--tag-color", color]];
      if (sensitive && uiConfig.hideNsfw) {
        rules = upsertProps(rules, ["display", "none"]);
      }

      ref.current.sheet.insertRule(
        `#${scope} .chat__message--tag_${name} {${rules.map((r) => r.join(":")).join(";")}}`
      );
    }

    deleteMatchingCSSRules(ref.current.sheet, (r) => r.cssText.includes("chat__message--author_"));
    deleteMatchingCSSRules(ref.current.sheet, (r) => r.cssText.includes("chat__message--mention_"));

    const props = new Map<string, PropList>();

    for (const [key, { color }] of uiConfigTags) {
      const rules: PropList = [["--color-chat-author-tag", color]];
      if (uiConfig.taggedVisibility) {
        rules.push(["--color-background-chat-message", "var(--color-background-chat-tagged)"]);
      }
      props.set(key, upsertProps(props.get(key), ...rules));
    }

    for (const key of uiConfigHighlights.keys()) {
      const rules: PropList = [
        ["--color-background-chat-message", "var(--color-background-chat-highlight)"],
      ];
      props.set(key, upsertProps(props.get(key), ...rules));
    }

    for (const key of uiConfigIgnores.keys()) {
      props.set(key, upsertProps(props.get(key), ["display", "none"]));
    }

    for (const [key, rules] of props) {
      ref.current.sheet.insertRule(
        `#${scope} .chat__message--author_${key} {${rules.map((r) => r.join(":")).join(";")}}`
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
      const selector = `#${scope} .chat__message:not(${keys.join(", ")})`;
      ref.current.sheet.insertRule(
        `${selector} { --opacity-chat-message: var(--opacity-chat-unselected); }`
      );
    }

    if (uiConfig.ignoreMentions) {
      for (const key of uiConfigIgnores.keys()) {
        ref.current.sheet.insertRule(`#${scope} .chat__message--mention_${key} { display: none; }`);
      }
    }
  }, [styles, uiConfig, uiConfigHighlights, uiConfigTags, uiConfigIgnores]);

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

// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Emote.scss";

import clsx from "clsx";
import React, { ReactNode } from "react";

import { Modifier } from "../../apis/strims/chat/v1/chat";
import { useRoom } from "../../contexts/Chat";

type EmoteProps = {
  name: string;
  modifiers?: string[];
  shouldAnimateForever?: boolean;
  shouldShowModifiers?: boolean;
  compactSpacing?: boolean;
  children?: ReactNode;
  [key: string]: any;
};

const Emote: React.FC<EmoteProps> = ({
  children,
  name,
  modifiers,
  shouldAnimateForever = false,
  shouldShowModifiers = true,
  compactSpacing = false,
  ...props
}) => {
  const [room] = useRoom();
  const style = room.styles.emotes.get(name);

  if (style === undefined) {
    return null;
  }

  const effectiveModifiers = new Set<Modifier>();
  for (const m of style.modifiers) {
    if (room.styles.modifiers.has(m)) {
      effectiveModifiers.add(room.styles.modifiers.get(m));
    }
  }
  if (modifiers?.length > 0 && shouldShowModifiers) {
    for (const m of modifiers) {
      if (room.styles.modifiers.has(m)) {
        effectiveModifiers.add(room.styles.modifiers.get(m));
      }
    }
  }

  let rootDepth = effectiveModifiers.size;

  let emote = (
    <span
      {...props}
      title={name + (modifiers || []).map((m) => ":" + m).join("")}
      className={clsx(style.name, "chat__emote", `chat__emote--${name}`, {
        "chat__emote--animated": style.animated,
        "chat__emote--animate_forever": shouldAnimateForever,
        "chat__emote--compact_spacing": compactSpacing,
        "chat__emote--root": rootDepth === 0,
      })}
    >
      {children}
    </span>
  );

  for (const modifier of Array.from(effectiveModifiers).sort((a, b) => b.priority - a.priority)) {
    rootDepth--;

    for (let i = modifier.extraWrapCount; i >= 0; i--) {
      emote = (
        <span
          className={clsx(
            `${style.name}_container`,
            "chat__emote_container",
            `chat__emote_container--emote_${name}`,
            {
              [`chat__emote_container--${modifier.name}`]: i === 0,
              [`chat__emote_container--${modifier.name}__extra_${i}`]: i > 0,
              "chat__emote_container--animated": style.animated,
              "chat__emote_container--animate_forever": shouldAnimateForever,
              "chat__emote_container--compact_spacing": compactSpacing,
              "chat__emote_container--root": rootDepth === 0 && i === 0,
            }
          )}
        >
          {emote}
        </span>
      );
    }
  }

  return emote;
};

export default Emote;

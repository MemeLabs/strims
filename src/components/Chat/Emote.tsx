import clsx from "clsx";
import React from "react";

import { useChat } from "../../contexts/Chat";

type EmoteProps = {
  name: string;
  modifiers?: string[];
  shouldAnimateForever?: boolean;
  shouldShowModifiers?: boolean;
  [key: string]: any;
};

const Emote: React.FC<EmoteProps> = ({
  children,
  name,
  modifiers,
  shouldAnimateForever = false,
  shouldShowModifiers = true,
  ...props
}) => {
  const [{ styles }] = useChat();
  const style = styles[name];

  if (style === undefined) {
    return null;
  }

  let emote = (
    <span
      {...props}
      title={name + (modifiers || []).map((m) => ":" + m).join()}
      className={clsx(style.name, "chat__emote", `chat__emote--${name}`, {
        "chat__emote--animated": style.animated,
        "chat__emote--animate_forever": shouldAnimateForever,
      })}
    >
      {children}
    </span>
  );

  const effectiveModifiers = new Set<string>(style.modifiers);
  if (modifiers?.length > 0 && shouldShowModifiers) {
    modifiers.forEach((m) => effectiveModifiers.add(m));
  }
  for (const modifier of effectiveModifiers) {
    emote = (
      <span
        className={clsx(
          `${style.name}_container`,
          "chat__emote_container",
          `chat__emote_container--emote_${name}`,
          `chat__emote_container--${modifier}`,
          {
            "chat__emote_container--animated": style.animated,
            "chat__emote_container--animate_forever": shouldAnimateForever,
          }
        )}
      >
        {emote}
      </span>
    );
  }

  return emote;
};

export default Emote;

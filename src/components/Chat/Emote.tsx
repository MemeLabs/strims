import { css } from "aphrodite/no-important";
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

  let emote = (
    <span
      {...props}
      title={name + (modifiers || []).map((m) => ":" + m).join()}
      className={clsx(styles[name], "chat__emote", `chat__emote--${name}`, {
        "chat__emote--animate_forever": shouldAnimateForever,
      })}
    >
      {children}
    </span>
  );

  if (modifiers?.length > 0 && shouldShowModifiers) {
    emote = modifiers.reduce(
      (emote, modifier) => (
        <span
          className={clsx(
            "chat__emote_container",
            `chat__emote_container--emote_${name}`,
            `chat__emote_container--${modifier}`,
            {
              "chat__emote_container--animate_forever": shouldAnimateForever,
            }
          )}
        >
          {emote}
        </span>
      ),
      emote
    );
  }

  return emote;
};

export default Emote;

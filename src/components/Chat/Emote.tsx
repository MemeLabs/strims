import { css } from "aphrodite/no-important";
import clsx from "clsx";
import * as React from "react";
import { FunctionComponent } from "react";

import { useChat } from "../../contexts/Chat";

type EmoteProps = {
  name: string;
  modifiers?: string[];
  [key: string]: any;
};
const Emote: FunctionComponent<EmoteProps> = ({ children, name, modifiers, ...props }) => {
  const [{ styles }] = useChat();

  // TODO: optionally disable emotes/modifiers
  let emote = (
    <span
      {...props}
      title={name + (modifiers || []).map((m) => ":" + m).join()}
      className={clsx(css(styles[name]), "chat__emote", `chat__emote--${name}`)}
    >
      {children}
    </span>
  );

  if (modifiers && modifiers.length > 0) {
    emote = modifiers.reduce(
      (emote, modifier, i) => (
        <span
          className={clsx(
            "chat__emote_container",
            `chat__emote_container--emote_${name}`,
            `chat__emote_container--${modifier}`
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

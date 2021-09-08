import clsx from "clsx";
import date from "date-and-time";
import React, { ReactNode, useEffect, useRef } from "react";

import { UIConfig, Message as chatv1_Message } from "../../apis/strims/chat/v1/chat";
import Emote from "./Emote";

// TODO: in app links
interface MessageLinkProps {
  entity: chatv1_Message.Entities.Link;
}

const MessageLink: React.FC<MessageLinkProps> = ({ children, entity }) => (
  <a className="chat__message__link" target="_blank" rel="nofollow" href={entity.url}>
    {children}
  </a>
);

interface MessageEmoteProps {
  entity: chatv1_Message.Entities.Emote;
  shouldAnimateForever: boolean;
  shouldShowModifiers: boolean;
}

const MessageEmote: React.FC<MessageEmoteProps> = ({
  children,
  entity,
  shouldAnimateForever,
  shouldShowModifiers,
}) => (
  <Emote
    name={entity.name}
    modifiers={entity.modifiers}
    shouldAnimateForever={shouldAnimateForever}
    shouldShowModifiers={shouldShowModifiers}
  >
    {children}
  </Emote>
);

interface MessageNickProps {
  entity: chatv1_Message.Entities.Nick;
}

const MessageNick: React.FC<MessageNickProps> = ({ children }) => (
  <span className="chat__message__nick">{children}</span>
);

interface MessageTagProps {
  entity: chatv1_Message.Entities.Tag;
}

const MessageTag: React.FC<MessageTagProps> = ({ children }) => (
  <span className="tag">{children}</span>
);

// TODO: extract spoiler body bounds in parser
const spoilerPrefix = /^[|\s]+/;
const spoilerSuffix = /[|\s]+$/;

const trimSpoiler = (node: React.ReactNode, rx: RegExp) =>
  typeof node === "string" ? node.replace(rx, "") : node;

interface MessageSpoilerProps {
  entity: chatv1_Message.Entities.Spoiler;
}

const MessageSpoiler: React.FC<MessageSpoilerProps> = ({ children: childrenNode }) => {
  const children = React.Children.toArray(childrenNode);
  const prefix = trimSpoiler(children.shift(), spoilerPrefix);
  const suffix = trimSpoiler(children.pop(), spoilerSuffix);
  const [hidden, setHidden] = React.useState(true);

  const handleClick = React.useCallback(() => setHidden((v) => !v), []);

  return (
    <span
      className={clsx({
        "chat__message__spoiler": true,
        "chat__message__spoiler--hidden": hidden,
      })}
      onClick={handleClick}
    >
      {prefix}
      {children}
      {suffix}
    </span>
  );
};

interface MessageCodeBlockProps {
  entity: chatv1_Message.Entities.CodeBlock;
}

const MessageCodeBlock: React.FC<MessageCodeBlockProps> = ({ children }) => (
  <span className="chat__message__code">{children}</span>
);

interface MessageGreenTextProps {
  entity: chatv1_Message.Entities.GenericEntity;
}

// TODO: optionally disable
const MessageGreenText: React.FC<MessageGreenTextProps> = ({ children }) => (
  <span className="chat__message__greentext">{children}</span>
);

type EntityComponent =
  | typeof MessageLink
  | typeof MessageEmote
  | typeof MessageNick
  | typeof MessageTag
  | typeof MessageSpoiler
  | typeof MessageCodeBlock
  | typeof MessageGreenText;

class MessageFormatter {
  private bounds: number[];
  public body: ReactNode[];

  constructor(body: string) {
    this.bounds = [0, body.length];
    this.body = [body];
  }

  // alignSlices cuts text spans in body to ensure they align with the bounds
  // of new entities during insertion. returns false if the index is inside a
  // node created for a previously inserted entity.
  private alignSlices(index: number) {
    for (let i = 0; i < this.bounds.length; i++) {
      if (this.bounds[i] < index && this.bounds[i + 1] > index) {
        const elem = this.body[i];
        if (typeof elem !== "string") {
          return false;
        }

        const splitIndex = index - this.bounds[i];
        this.body.splice(i, 1, elem.substr(0, splitIndex), elem.substr(splitIndex));
        this.bounds.splice(i + 1, 0, index);
        return true;
      }
    }
    return true;
  }

  // replaces the message body from the bounds in the supplied entity with a
  // react node of type C.
  public insertEntity<C extends EntityComponent>(
    component: C,
    entity: React.ComponentProps<C>["entity"],
    props?: Omit<React.ComponentProps<C>, "entity">
  ) {
    const { start, end } = entity.bounds;
    if (!this.alignSlices(start) || !this.alignSlices(end)) {
      return false;
    }

    const startIndex = this.bounds.findIndex((i) => i === start);
    const endIndex = this.bounds.findIndex((i) => i === end);
    this.body.splice(
      startIndex,
      endIndex - startIndex,
      React.createElement(component, {
        key: `${component.name}(${start},${end})`,
        children: this.body.slice(startIndex, endIndex),
        entity,
        ...props,
      })
    );
  }
}

interface MessageTimeProps {
  timestamp: bigint;
  format: string;
}

const MessageTime: React.FC<MessageTimeProps> = ({ timestamp, format }) => {
  const time = new Date(Number(timestamp));
  return (
    <time className="chat__message__time" title={time.toLocaleString()}>
      {date.format(time, format)}
    </time>
  );
};

interface MessageProps extends React.HTMLProps<HTMLDivElement> {
  uiConfig: UIConfig;
  message: chatv1_Message;
  isMostRecent?: boolean;
}

const Message: React.FC<MessageProps> = ({ isMostRecent, ...props }) => {
  const { emotes } = props.message.entities;
  return emotes?.length === 1 && emotes[0].combo ? (
    <ComboMessage {...props} isMostRecent={isMostRecent} />
  ) : (
    <StandardMessage {...props} />
  );
};

const ComboMessage: React.FC<MessageProps> = ({
  uiConfig,
  message: { serverTime, body, entities },
  className: baseClassName,
  isMostRecent,
  ...props
}) => {
  const formatter = new MessageFormatter(body);
  entities.emotes.forEach((entity) => formatter.insertEntity(MessageEmote, entity, {
    shouldAnimateForever: uiConfig.animateForever,
    shouldShowModifiers: uiConfig.emoteModifiers,
  }));

  const count = entities.emotes[0].combo;
  const scale = Math.min(Math.floor(count / 5) * 5, 50);
  const className = clsx([baseClassName, "chat__combo_message"], {
    [`chat__combo_message--scale_${scale}`]: scale > 0,
    "chat__combo_message--complete": !isMostRecent,
  });

  const ref = useRef<HTMLDivElement>();
  useEffect(() => {
    ref.current.classList.remove(`chat__combo_message--hit`);
    const rafId = window.requestAnimationFrame(() =>
      ref.current?.classList.add(`chat__combo_message--hit`)
    );
    return () => window.cancelAnimationFrame(rafId);
  }, [count]);

  return (
    <div {...props} className={className} ref={ref}>
      {uiConfig.showTime && (
        <MessageTime timestamp={serverTime} format={uiConfig.timestampFormat} />
      )}
      <span className="chat__combo_message__body">
        {formatter.body}
        <i className="chat__combo_message__count">{count}</i>
        <i className="chat__combo_message__x">x</i>
        <i className="chat__combo_message__hits">hits</i>
        <i className="chat__combo_message__combo">c-c-c-combo</i>
      </span>
    </div>
  );
};

// TODO: mark sensitive tags in server config
const sensitiveTags = ["nsfw", "nsfl"];

const StandardMessage: React.FC<MessageProps> = ({
  uiConfig,
  message: { nick, serverTime, body, entities },
  className,
  ...props
}) => {
  if (uiConfig.hideNsfw && entities.tags?.some(({ name }) => sensitiveTags.includes(name))) {
    return null;
  }

  const formatter = new MessageFormatter(body);
  entities.codeBlocks.forEach((entity) => formatter.insertEntity(MessageCodeBlock, entity));
  entities.links.forEach((entity) => formatter.insertEntity(MessageLink, entity));
  if (uiConfig.formatterEmote) {
    entities.emotes.forEach((entity) =>
      formatter.insertEntity(MessageEmote, entity, {
        shouldAnimateForever: uiConfig.animateForever,
        shouldShowModifiers: uiConfig.emoteModifiers,
      })
    );
  }
  entities.nicks.forEach((entity) => formatter.insertEntity(MessageNick, entity));
  entities.tags.forEach((entity) => formatter.insertEntity(MessageTag, entity));
  if (!uiConfig.disableSpoilers) {
    entities.spoilers.forEach((entity) => formatter.insertEntity(MessageSpoiler, entity));
  }
  if (uiConfig.formatterGreen && entities.greenText) {
    formatter.insertEntity(MessageGreenText, entities.greenText);
  }

  const classNames = clsx([
    "chat__message",
    entities.selfMessage && "chat__message--self",
    ...entities.tags.map(({ name }) => `chat__message--tag_${name}`),
    className,
  ]);

  return (
    <div {...props} className={classNames}>
      {uiConfig.showTime && (
        <MessageTime timestamp={serverTime} format={uiConfig.timestampFormat} />
      )}
      <span className="chat__message__author">{nick}</span>
      <span className="chat__message__colon">{": "}</span>
      <span className="chat__message__body">{formatter.body}</span>
    </div>
  );
};

export default Message;

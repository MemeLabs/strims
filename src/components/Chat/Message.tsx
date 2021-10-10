import clsx from "clsx";
import date from "date-and-time";
import React, { ReactNode, useEffect, useRef } from "react";

import { UIConfig, Message as chatv1_Message } from "../../apis/strims/chat/v1/chat";
import Emote from "./Emote";

const LINK_SHORTEN_THRESHOLD = 75;
const LINK_SHORTEN_AFFIX_LENGTH = 35;

// TODO: in app links
interface MessageLinkProps {
  entity: chatv1_Message.Entities.Link;
  shouldShorten: boolean;
}

const MessageLink: React.FC<MessageLinkProps> = ({ children, entity, shouldShorten }) => {
  let nodes = React.Children.toArray(children);
  if (shouldShorten && typeof nodes[0] === "string" && nodes[0].length > LINK_SHORTEN_THRESHOLD) {
    nodes = [
      nodes[0].substring(0, LINK_SHORTEN_AFFIX_LENGTH),
      <span className="chat__message__link__ellipsis">&hellip;</span>,
      <span className="chat__message__link__overflow">
        {nodes[0].substring(LINK_SHORTEN_AFFIX_LENGTH, nodes[0].length - LINK_SHORTEN_AFFIX_LENGTH)}
      </span>,
      nodes[0].substring(nodes[0].length - LINK_SHORTEN_AFFIX_LENGTH),
    ];
  }

  return (
    <a className="chat__message__link" target="_blank" rel="nofollow" href={entity.url}>
      {nodes}
    </a>
  );
};

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
  <span className="chat__message__tag">{children}</span>
);

// TODO: extract spoiler body bounds in parser
const spoilerPrefix = /^[|\s]+/;
const spoilerSuffix = /[|\s]+$/;
const codePrefix = /^[`\s]+/;
const codeSuffix = /[`\s]+$/;

const trimTextNode = (node: React.ReactNode, rx: RegExp) =>
  typeof node === "string" ? node.replace(rx, "") : node;

const trimChildren = (children: React.ReactNode, prefix: RegExp, suffix: RegExp) => {
  const nodes = React.Children.toArray(children);
  nodes[0] = trimTextNode(nodes[0], prefix);
  nodes[nodes.length - 1] = trimTextNode(nodes[nodes.length - 1], suffix);
  return nodes;
};

interface MessageSpoilerProps {
  entity: chatv1_Message.Entities.Spoiler;
}

const MessageSpoiler: React.FC<MessageSpoilerProps> = ({ children }) => {
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
      {trimChildren(children, spoilerPrefix, spoilerSuffix)}
    </span>
  );
};

interface MessageCodeBlockProps {
  entity: chatv1_Message.Entities.CodeBlock;
}

const MessageCodeBlock: React.FC<MessageCodeBlockProps> = ({ children }) => (
  <span className="chat__message__code">{trimChildren(children, codePrefix, codeSuffix)}</span>
);

interface MessageGreenTextProps {
  entity: chatv1_Message.Entities.GenericEntity;
}

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

  // splitSpan splits the text span in body at the given character offset and
  // returns the span index. returns -1 if the offset is in a non-text span.
  private splitSpan(offset: number) {
    let l = 0;
    let r = this.bounds.length;
    while (l !== r) {
      const m = (r + l) >> 1;
      if (this.bounds[m] <= offset) {
        l = m + 1;
      } else {
        r = m;
      }
    }

    const i = l - 1;
    if (this.bounds[i] === offset) {
      return i;
    }
    const span = this.body[i];
    if (typeof span !== "string") {
      return -1;
    }

    const splitOffset = offset - this.bounds[i];
    this.body.splice(i, 1, span.substr(0, splitOffset), span.substr(splitOffset));
    this.bounds.splice(i + 1, 0, offset);
    return i + 1;
  }

  // replaces the message body from the bounds in the supplied entity with a
  // react node of type C.
  public insertEntity<C extends EntityComponent>(
    component: C,
    entity: React.ComponentProps<C>["entity"],
    props?: Omit<React.ComponentProps<C>, "entity">
  ) {
    const { start, end } = entity.bounds;
    const startIndex = this.splitSpan(start);
    const endIndex = this.splitSpan(end);
    if (startIndex === -1 || endIndex === -1) {
      return false;
    }

    const node = React.createElement(component, {
      key: `${component.name}(${start},${end})`,
      children: this.body.slice(startIndex, endIndex),
      entity,
      ...props,
    });

    this.body.splice(startIndex, endIndex - startIndex, node);
    this.bounds.splice(startIndex + 1, endIndex - startIndex - 1);
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
  entities.emotes.forEach((entity) =>
    formatter.insertEntity(MessageEmote, entity, {
      shouldAnimateForever: uiConfig.animateForever,
      shouldShowModifiers: uiConfig.emoteModifiers,
    })
  );

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

const StandardMessage: React.FC<MessageProps> = ({
  uiConfig,
  message: { nick, serverTime, body, entities },
  className,
  ...props
}) => {
  const formatter = new MessageFormatter(body);
  entities.codeBlocks.forEach((entity) => formatter.insertEntity(MessageCodeBlock, entity));
  entities.links.forEach((entity) =>
    formatter.insertEntity(MessageLink, entity, {
      shouldShorten: uiConfig.shortenLinks,
    })
  );
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
    {
      "chat__message--self": entities.selfMessage,
      "chat__message--tagged": entities.tags.length > 0,
    },
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

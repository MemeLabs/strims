// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Message.scss";

import clsx from "clsx";
import date from "date-and-time";
import { Base64 } from "js-base64";
import { uniq } from "lodash";
import React, { ReactNode, useEffect, useMemo, useRef } from "react";

import { UIConfig, Message as chatv1_Message } from "../../apis/strims/chat/v1/chat";
import { useRoom } from "../../contexts/Chat";
import { useStableCallback } from "../../hooks/useStableCallback";
import Emote from "./Emote";
import { UserPresenceIndicator } from "./UserPresenceIndicator";

const LINK_SHORTEN_THRESHOLD = 75;
const LINK_SHORTEN_AFFIX_LENGTH = 35;

// TODO: in app links
interface MessageLinkProps {
  entity: chatv1_Message.Entities.Link;
  shouldShorten: boolean;
}

const MessageLink: React.FC<MessageLinkProps> = ({ children, entity, shouldShorten }) => {
  const [node] = React.Children.toArray(children);
  if (shouldShorten && typeof node === "string" && node.length > LINK_SHORTEN_THRESHOLD) {
    children = (
      <>
        {node.substring(0, LINK_SHORTEN_AFFIX_LENGTH)}
        <span className="chat__message__link__ellipsis">&hellip;</span>
        <span className="chat__message__link__overflow">
          {node.substring(LINK_SHORTEN_AFFIX_LENGTH, node.length - LINK_SHORTEN_AFFIX_LENGTH)}
        </span>
        {node.substring(node.length - LINK_SHORTEN_AFFIX_LENGTH)}
      </>
    );
  }

  return (
    <a className="chat__message__link" target="_blank" rel="nofollow" href={entity.url}>
      {children}
    </a>
  );
};

interface MessageEmoteProps {
  entity: chatv1_Message.Entities.Emote;
  shouldAnimateForever?: boolean;
  shouldShowModifiers?: boolean;
  compactSpacing?: boolean;
}

const MessageEmote: React.FC<MessageEmoteProps> = ({
  children,
  entity,
  shouldAnimateForever,
  shouldShowModifiers,
  compactSpacing,
}) => (
  <Emote
    name={entity.name}
    modifiers={entity.modifiers}
    shouldAnimateForever={shouldAnimateForever}
    shouldShowModifiers={shouldShowModifiers}
    compactSpacing={compactSpacing}
  >
    {children}
  </Emote>
);

interface MessageEmojiProps {
  entity: chatv1_Message.Entities.Emoji;
}

const MessageEmoji: React.FC<MessageEmojiProps> = ({ children, entity }) => (
  <span className="chat__message__emoji" title={entity.description}>
    {children}
  </span>
);

interface MessageNickProps {
  entity: chatv1_Message.Entities.Nick;
  normalizeCase: boolean;
  onClick: (e: React.MouseEvent, entity: chatv1_Message.Entities.Nick) => void;
}

const MessageNick: React.FC<MessageNickProps> = ({ children, entity, normalizeCase, onClick }) => {
  const handleClick = useStableCallback((e: React.MouseEvent) => onClick(e, entity));

  return (
    <span className="chat__message__nick" onClick={handleClick}>
      {normalizeCase ? entity.nick : children}
    </span>
  );
};

interface MessageTagProps {
  entity: chatv1_Message.Entities.Tag;
}

const MessageTag: React.FC<MessageTagProps> = ({ children }) => (
  <span className="chat__message__tag">{children}</span>
);

// TODO: extract spoiler body bounds in parser
const SPOILER_PREFIX = /^[|\s]+/;
const SPOILER_SUFFIX = /[|\s]+$/;
const CODE_PREFIX = /^[`\s]+/;
const CODE_SUFFIX = /[`\s]+$/;

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
      {trimChildren(children, SPOILER_PREFIX, SPOILER_SUFFIX)}
    </span>
  );
};

interface MessageCodeBlockProps {
  entity: chatv1_Message.Entities.CodeBlock;
}

const MessageCodeBlock: React.FC<MessageCodeBlockProps> = ({ children }) => (
  <span className="chat__message__code">{trimChildren(children, CODE_PREFIX, CODE_SUFFIX)}</span>
);

interface MessageGreenTextProps {
  entity: chatv1_Message.Entities.GenericEntity;
}

const MessageGreenText: React.FC<MessageGreenTextProps> = ({ children }) => (
  <span className="chat__message__greentext">{children}</span>
);

interface MessageSelfProps {
  entity: chatv1_Message.Entities.GenericEntity;
}

const MessageSelf: React.FC<MessageSelfProps> = ({ children }) => (
  <span className="chat__message__self">{children}</span>
);

type EntityComponent =
  | typeof MessageLink
  | typeof MessageEmote
  | typeof MessageEmoji
  | typeof MessageNick
  | typeof MessageTag
  | typeof MessageSpoiler
  | typeof MessageCodeBlock
  | typeof MessageGreenText
  | typeof MessageSelf;

class MessageFormatter {
  private bounds: number[];
  private runes: string[];
  public body: ReactNode[];

  constructor(body: string) {
    this.body = [body];
    this.runes = Array.from(body);
    this.bounds = [0, this.runes.length];
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

    const left = this.runes.slice(this.bounds[i], offset).join("");
    const right = this.runes.slice(offset, this.bounds[i + 1]).join("");
    this.body.splice(i, 1, left, right);
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
  isContinued?: boolean;
}

const Message: React.FC<MessageProps> = (props) => {
  const { emotes } = props.message.entities;
  return emotes.length === 1 && emotes[0].combo ? (
    <ComboMessage {...props} />
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
  const formattedBody = useMemo(() => {
    const formatter = new MessageFormatter(body);
    entities.emotes.forEach((entity) =>
      formatter.insertEntity(MessageEmote, entity, {
        shouldAnimateForever: uiConfig.animateForever,
        shouldShowModifiers: uiConfig.emoteModifiers,
      })
    );
    return formatter.body;
  }, [uiConfig]);

  const count = entities.emotes[0].combo;
  const scale = Math.min(Math.floor(count / 5) * 5, 50);
  const className = clsx(baseClassName, "chat__combo_message", {
    [`chat__combo_message--scale_${scale}`]: scale > 0,
    "chat__combo_message--complete": !isMostRecent,
    "chat__combo_message--repeatable": isMostRecent,
  });

  const ref = useRef<HTMLDivElement>();
  useEffect(() => {
    ref.current.classList.remove(`chat__combo_message--hit`);
    const rafId = requestAnimationFrame(() =>
      ref.current?.classList.add(`chat__combo_message--hit`)
    );
    return () => cancelAnimationFrame(rafId);
  }, [count]);

  const [, { sendMessage }] = useRoom();

  const handleBodyClick = useStableCallback(() => {
    if (isMostRecent) {
      sendMessage(body);
    }
  });

  return (
    <div {...props} className={className} ref={ref}>
      {uiConfig.showTime && (
        <MessageTime timestamp={serverTime} format={uiConfig.timestampFormat} />
      )}
      <span className="chat__combo_message__body" onClick={handleBodyClick}>
        {formattedBody}
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
  message: { nick, peerKey, viewedListing, serverTime, body, entities },
  className: baseClassName,
  isMostRecent,
  isContinued,
  ...props
}) => {
  const [, { toggleSelectedPeer, sendMessage }] = useRoom();

  const handleNickClick = useStableCallback(
    (e: React.MouseEvent, entity: chatv1_Message.Entities.Nick) => {
      e.stopPropagation();
      toggleSelectedPeer(entity.peerKey);
      toggleSelectedPeer(peerKey, true);
    }
  );

  const formattedBody = useMemo(() => {
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
          compactSpacing: uiConfig.compactEmoteSpacing,
        })
      );
      entities.emojis.forEach((entity) => formatter.insertEntity(MessageEmoji, entity));
    }
    entities.nicks.forEach((entity) =>
      formatter.insertEntity(MessageNick, entity, {
        normalizeCase: uiConfig.normalizeAliasCase,
        onClick: handleNickClick,
      })
    );
    entities.tags.forEach((entity) => formatter.insertEntity(MessageTag, entity));
    if (!uiConfig.disableSpoilers) {
      entities.spoilers.forEach((entity) => formatter.insertEntity(MessageSpoiler, entity));
    }
    if (uiConfig.formatterGreen && entities.greenText) {
      formatter.insertEntity(MessageGreenText, entities.greenText);
    }
    if (entities.selfMessage) {
      formatter.insertEntity(MessageSelf, entities.selfMessage);
    }
    return formatter.body;
  }, [uiConfig, entities]);

  const authorKey = Base64.fromUint8Array(peerKey, true);

  const isRepeatable = useMemo(
    () => isMostRecent && entities.emotes[0]?.canCombo,
    [isMostRecent, entities]
  );

  const classNames = useMemo(() => {
    return clsx(
      baseClassName,
      "chat__message",
      `chat__message--author_${authorKey}`,
      {
        "chat__message--continued": isContinued,
        "chat__message--repeatable": isRepeatable,
        "chat__message--self": entities.selfMessage,
        "chat__message--tagged": entities.tags.length > 0,
      },
      uniq(entities.tags.map(({ name }) => `chat__message--tag_${name}`)),
      uniq(
        entities.nicks.map(
          ({ peerKey }) => `chat__message--mention_${Base64.fromUint8Array(peerKey, true)}`
        )
      )
    );
  }, [baseClassName, isContinued, isRepeatable, entities]);

  const handleAuthorClick = useStableCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    toggleSelectedPeer(peerKey);
  });

  const handleBodyClick = useStableCallback(() => {
    if (isRepeatable) {
      sendMessage(body);
    }
  });

  return (
    <div {...props} className={classNames}>
      {uiConfig.showTime && (
        <MessageTime timestamp={serverTime} format={uiConfig.timestampFormat} />
      )}
      <span className="chat__message__author" onClick={handleAuthorClick}>
        {!!uiConfig.userPresenceIndicator && (
          <UserPresenceIndicator
            style={uiConfig.userPresenceIndicator}
            directoryRef={viewedListing}
          />
        )}
        <span className="chat__message__author__text">{nick}</span>
      </span>
      <span className="chat__message__colon">{": "}</span>
      <span className="chat__message__body" onClick={handleBodyClick}>
        {formattedBody}
      </span>
      <br />
    </div>
  );
};

export default Message;

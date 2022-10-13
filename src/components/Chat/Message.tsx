// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Message.scss";

import clsx from "clsx";
import date from "date-and-time";
import { Base64 } from "js-base64";
import { isEqual, uniq } from "lodash";
import React, { ReactNode, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { FiExternalLink } from "react-icons/fi";
import { Link } from "react-router-dom";

import { MessageState, UIConfig, Message as chatv1_Message } from "../../apis/strims/chat/v1/chat";
import { Listing } from "../../apis/strims/network/v1/directory/directory";
import { useRoom } from "../../contexts/Chat";
import { useSession } from "../../contexts/Session";
import { useOpenListing } from "../../hooks/directory";
import useRefs from "../../hooks/useRefs";
import { useStableCallback } from "../../hooks/useStableCallback";
import * as directory from "../../lib/directory";
import ExternalLink from "../ExternalLink";
import Emoji from "./Emoji";
import Emote from "./Emote";
import EmoteDetails from "./EmoteDetails";
import MessageStateIcon from "./MessageStateIcon";
import { useUserContextMenu } from "./UserContextMenu";
import { UserPresenceIndicator } from "./UserPresenceIndicator";

const LINK_SHORTEN_THRESHOLD = 75;
const LINK_SHORTEN_AFFIX_LENGTH = 35;

interface MessageLinkProps {
  entity: chatv1_Message.Entities.Link;
  shouldShorten: boolean;
  children: ReactNode;
}

const MessageLink: React.FC<MessageLinkProps> = ({ children, entity, shouldShorten }) => {
  const embed = directory.createEmbedFromURL(entity.url);
  if (embed) {
    const [, { getNetworkKeys }] = useRoom();
    const openListing = useOpenListing();

    const [networkKey] = getNetworkKeys();
    const listing = new Listing({ content: { embed } });

    const handleClick = useStableCallback((e: React.MouseEvent) => {
      e.preventDefault();
      openListing(Base64.fromUint8Array(networkKey, true), listing);
    });

    return (
      <>
        <Link
          className="chat__message__link"
          to={directory.formatUri(Base64.fromUint8Array(networkKey, true), listing)}
          onClick={handleClick}
        >
          {directory.serviceToSlug(embed.service)}/{embed.id}
        </Link>
        <ExternalLink
          className="chat__message__link chat__message__link--external_embed"
          href={entity.url}
        >
          <FiExternalLink />
        </ExternalLink>
      </>
    );
  }

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
    <ExternalLink className="chat__message__link" href={entity.url}>
      {children}
    </ExternalLink>
  );
};

interface MessageEmoteProps {
  entity: chatv1_Message.Entities.Emote;
  shouldAnimateForever?: boolean;
  shouldShowModifiers?: boolean;
  compactSpacing?: boolean;
  hidden?: boolean;
  children: ReactNode[];
}

const MessageEmote: React.FC<MessageEmoteProps> = ({
  children,
  entity,
  shouldAnimateForever,
  shouldShowModifiers,
  compactSpacing,
  hidden,
}) => {
  const [detailsAnchor, setDetailsAnchor] = useState<[number, number]>(null);
  const handleClick = useCallback((e: React.MouseEvent) => {
    setDetailsAnchor([e.pageX, e.pageY]);
  }, []);
  const handleDetailsClose = useCallback(() => setDetailsAnchor(null), []);

  return hidden ? (
    <span className="chat__hidden_emote">{entity.name}</span>
  ) : (
    <>
      <Emote
        name={entity.name}
        modifiers={entity.modifiers}
        shouldAnimateForever={shouldAnimateForever}
        shouldShowModifiers={shouldShowModifiers}
        compactSpacing={compactSpacing}
        onClick={handleClick}
      >
        {children}
      </Emote>
      {detailsAnchor && (
        <EmoteDetails name={entity.name} anchor={detailsAnchor} onClose={handleDetailsClose} />
      )}
    </>
  );
};

interface MessageEmojiProps {
  entity: chatv1_Message.Entities.Emoji;
  children: ReactNode[];
}

// TODO: load shortcode from emoji in chat context
const MessageEmoji: React.FC<MessageEmojiProps> = ({ children }) => <Emoji>{children}</Emoji>;

interface MessageNickProps {
  entity: chatv1_Message.Entities.Nick;
  normalizeCase: boolean;
  children: ReactNode[];
  onClick: (e: React.MouseEvent, entity: chatv1_Message.Entities.Nick) => void;
}

const MessageNick: React.FC<MessageNickProps> = ({ children, entity, normalizeCase, onClick }) => {
  const handleClick = useStableCallback((e: React.MouseEvent) => onClick(e, entity));

  const { UserContextMenu, openUserContextMenu } = useUserContextMenu(entity);
  const handleContextMenu = useStableCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    e.preventDefault();
    openUserContextMenu(e);
  });

  return (
    <span className="chat__message__nick" onClick={handleClick} onContextMenu={handleContextMenu}>
      {normalizeCase ? entity.nick : children}
      <UserContextMenu />
    </span>
  );
};

interface MessageTagProps {
  entity: chatv1_Message.Entities.Tag;
  children: ReactNode[];
}

const MessageTag: React.FC<MessageTagProps> = ({ children }) => (
  <span className="chat__message__tag">{children}</span>
);

// TODO: extract spoiler body bounds in parser
const SPOILER_PREFIX = /^[|\s]+/;
const SPOILER_SUFFIX = /[|\s]+$/;
const CODE_PREFIX = /^[`\s]+/;
const CODE_SUFFIX = /[`\s]+$/;

const trimTextNode = (node: ReactNode, rx: RegExp): ReactNode =>
  typeof node === "string" ? node.replace(rx, "") : node;

const trimChildren = (children: ReactNode, prefix: RegExp, suffix: RegExp) => {
  const nodes: ReactNode[] = React.Children.toArray(children);
  nodes[0] = trimTextNode(nodes[0], prefix);
  nodes[nodes.length - 1] = trimTextNode(nodes[nodes.length - 1], suffix);
  return nodes;
};

interface MessageSpoilerProps {
  entity: chatv1_Message.Entities.Spoiler;
  children: ReactNode[];
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
  children: ReactNode[];
}

const MessageCodeBlock: React.FC<MessageCodeBlockProps> = ({ children }) => (
  <span className="chat__message__code">{trimChildren(children, CODE_PREFIX, CODE_SUFFIX)}</span>
);

interface MessageGreenTextProps {
  entity: chatv1_Message.Entities.GenericEntity;
  children: ReactNode[];
}

const MessageGreenText: React.FC<MessageGreenTextProps> = ({ children }) => (
  <span className="chat__message__greentext">{children}</span>
);

interface MessageSelfProps {
  entity: chatv1_Message.Entities.GenericEntity;
  children: ReactNode[];
}

const MessageSelf: React.FC<MessageSelfProps> = ({ children }) => (
  <span className="chat__message__self">{children}</span>
);

type Entity =
  | chatv1_Message.Entities.Link
  | chatv1_Message.Entities.Emote
  | chatv1_Message.Entities.Emoji
  | chatv1_Message.Entities.Nick
  | chatv1_Message.Entities.Tag
  | chatv1_Message.Entities.Spoiler
  | chatv1_Message.Entities.CodeBlock
  | chatv1_Message.Entities.GenericEntity;

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
  public insertEntity<C extends React.ComponentType<{ entity: E }>, E extends Entity>(
    component: C,
    entity: E,
    props?: Omit<React.ComponentProps<C>, "entity" | "children">
  ) {
    const { start, end } = entity.bounds;
    const startIndex = this.splitSpan(start);
    const endIndex = this.splitSpan(end);
    if (startIndex === -1 || endIndex === -1) {
      return;
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
  messageState?: MessageState;
  isMostRecent?: boolean;
  isContinued?: boolean;
}

const Message = React.forwardRef<HTMLDivElement, MessageProps>(({ isContinued, ...props }, ref) => {
  const { emotes } = props.message.entities;
  return emotes.length === 1 && emotes[0].combo ? (
    <ComboMessage {...props} fwRef={ref} />
  ) : (
    <StandardMessage {...props} isContinued={isContinued} fwRef={ref} />
  );
});

Message.displayName = "Message";

interface MessageImplProps extends MessageProps {
  fwRef: React.ForwardedRef<HTMLDivElement>;
}

const ComboMessage: React.FC<MessageImplProps> = ({
  uiConfig,
  message: { serverTime, body, entities },
  className: baseClassName,
  isMostRecent,
  fwRef,
  ...props
}) => {
  const formattedBody = useMemo(() => {
    const formatter = new MessageFormatter(body);
    entities.emotes.forEach((entity) =>
      formatter.insertEntity(MessageEmote, entity, {
        shouldAnimateForever: uiConfig.animateForever,
        shouldShowModifiers: uiConfig.emoteModifiers,
        hidden: uiConfig.hiddenEmotes.includes(entity.name),
      })
    );
    return formatter.body;
  }, [uiConfig, entities]);

  const count = entities.emotes[0].combo;
  const scale = Math.min(Math.floor(count / 5) * 5, 50);
  const className = clsx(baseClassName, "chat__combo_message", {
    [`chat__combo_message--scale_${scale}`]: scale > 0,
    "chat__combo_message--complete": !isMostRecent,
    "chat__combo_message--clickable": isMostRecent,
  });

  const ref = useRef<HTMLElement>();
  useEffect(() => {
    ref.current.classList.remove(`chat__combo_message--hit`);
    const rafId = requestAnimationFrame(() =>
      ref.current?.classList.add(`chat__combo_message--hit`)
    );
    return () => cancelAnimationFrame(rafId);
  }, [count]);

  const [, { sendMessage }] = useRoom();

  const handleBodyClick = useStableCallback((e: React.MouseEvent) => {
    if (isMostRecent) {
      sendMessage(body);
      e.stopPropagation();
    }
  });

  return (
    <div {...props} className={className} ref={useRefs(ref, fwRef)}>
      {uiConfig.showTime && (
        <MessageTime timestamp={serverTime} format={uiConfig.timestampFormat} />
      )}
      <span className="chat__combo_message__body" onClickCapture={handleBodyClick}>
        {formattedBody}
        <i className="chat__combo_message__count">{count}</i>
        <i className="chat__combo_message__x">x</i>
        <i className="chat__combo_message__hits">hits</i>
        <i className="chat__combo_message__combo">c-c-c-combo</i>
      </span>
    </div>
  );
};

const StandardMessage: React.FC<MessageImplProps> = ({
  uiConfig,
  message: { nick, peerKey, viewedListing, serverTime, body, entities },
  messageState,
  className: baseClassName,
  isMostRecent,
  isContinued,
  fwRef,
  ...props
}) => {
  const [, { toggleSelectedPeer, sendMessage }] = useRoom();
  const [{ profile }] = useSession();

  const handleNickClick = useStableCallback(
    (e: React.MouseEvent, entity: chatv1_Message.Entities.Nick) => {
      e.stopPropagation();
      toggleSelectedPeer(entity.peerKey);
      toggleSelectedPeer(peerKey, true);
    }
  );

  const handleAuthorClick = useStableCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    toggleSelectedPeer(peerKey);
  });

  const { UserContextMenu, openUserContextMenu } = useUserContextMenu({
    nick,
    peerKey,
    viewedListing,
  });
  const handleAuthorContextMenu = useStableCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    e.preventDefault();
    openUserContextMenu(e);
  });

  const canCombo = isMostRecent && entities.emotes[0]?.canCombo;
  const handleBodyClick = useStableCallback((e: React.MouseEvent) => {
    if (canCombo) {
      sendMessage(body);
      e.stopPropagation();
    }
  });

  const classNames = useMemo(() => {
    const authorKey = Base64.fromUint8Array(peerKey, true);
    const sent = isEqual(peerKey, profile.key.public);
    const highlight =
      (uiConfig.highlight && entities.nicks.some((n) => isEqual(n.peerKey, profile.key.public))) ||
      uiConfig.customHighlight.some((h) => body.includes(h));

    return clsx(
      baseClassName,
      "chat__message",
      `chat__message--author_${authorKey}`,
      {
        "chat__message--continued": isContinued,
        "chat__message--clickable": canCombo,
        "chat__message--self": entities.selfMessage,
        "chat__message--tagged": entities.tags.length > 0,
        "chat__message--sent": sent,
        "chat__message--enqueued": messageState === MessageState.MESSAGE_STATE_ENQUEUED,
        "chat__message--failed": messageState === MessageState.MESSAGE_STATE_FAILED,
        "chat__message--highlight": !sent && highlight,
      },
      uniq(entities.tags.map(({ name }) => `chat__message--tag_${name}`)),
      uniq(
        entities.nicks.map(
          ({ peerKey }) => `chat__message--mention_${Base64.fromUint8Array(peerKey, true)}`
        )
      )
    );
  }, [baseClassName, isContinued, canCombo, entities, uiConfig, messageState]);

  const content = useMemo(() => {
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
          hidden: uiConfig.hiddenEmotes.includes(entity.name),
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
    return (
      <>
        {messageState === MessageState.MESSAGE_STATE_FAILED && (
          <MessageStateIcon messageState={messageState} />
        )}
        {uiConfig.showTime && (
          <MessageTime timestamp={serverTime} format={uiConfig.timestampFormat} />
        )}
        <span
          className="chat__message__author"
          onClick={handleAuthorClick}
          onContextMenu={handleAuthorContextMenu}
        >
          {!!uiConfig.userPresenceIndicator && (
            <UserPresenceIndicator
              style={uiConfig.userPresenceIndicator}
              directoryRef={viewedListing}
            />
          )}
          <span className="chat__message__author__text">{nick}</span>
        </span>
        <span className="chat__message__colon">{": "}</span>
        <span className="chat__message__body" onClickCapture={handleBodyClick}>
          {formatter.body}
        </span>
        <br />
      </>
    );
  }, [uiConfig, entities, messageState]);

  return (
    <div {...props} className={classNames} ref={fwRef}>
      {content}
      <UserContextMenu />
    </div>
  );
};

export default Message;

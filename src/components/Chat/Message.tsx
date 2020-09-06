import clsx from "clsx";
import * as React from "react";
import { FunctionComponent } from "react";
import { ReactNode } from "react";

import { ChatClientEvent, MessageEntities } from "../../lib/pb";

// TODO: in app links
interface LinkProps {
  entity: MessageEntities.ILink;
}
const Link: FunctionComponent<LinkProps> = ({ children, entity }) => (
  <a className="chat_message__link" target="_blank" rel="nofollow" href={entity.url}>
    {children}
  </a>
);

interface EmoteProps {
  entity: MessageEntities.IEmote;
}
const Emote: FunctionComponent<EmoteProps> = ({ children, entity }) =>
  (entity.modifiers || []).reduce(
    (emote, modifier) => (
      <span
        className={clsx([
          "chat_message__emote_container",
          `chat_message__emote_container--emote_${entity.name}`,
          `chat_message__emote_container--${modifier}`,
        ])}
      >
        {emote}
      </span>
    ),
    <span
      title={children.toString()}
      className={clsx(["chat_message__emote", `chat_message__emote--${entity.name}`])}
    >
      {children}
    </span>
  );

interface NickProps {
  entity: MessageEntities.INick;
}
const Nick: FunctionComponent<NickProps> = ({ children }) => (
  <span className="chat_message__nick">{children}</span>
);

interface TagProps {
  entity: MessageEntities.ITag;
}
const Tag: FunctionComponent<TagProps> = ({ children }) => <span className="tag">{children}</span>;

// TODO: extract spoiler body bounds in parser
const spoilerPrefix = /^[|\s]+/;
const spoilerSuffix = /[|\s]+$/;

const trimSpoiler = (node: React.ReactNode, rx: RegExp) =>
  typeof node === "string" ? node.replace(rx, "") : node;

interface SpoilerProps {
  entity: MessageEntities.ISpoiler;
}
const Spoiler: FunctionComponent<SpoilerProps> = ({ children: childrenNode }) => {
  const children = React.Children.toArray(childrenNode);
  const prefix = trimSpoiler(children.shift(), spoilerPrefix);
  const suffix = trimSpoiler(children.pop(), spoilerSuffix);

  return (
    <span className="chat_message__spoiler">
      {prefix}
      {children}
      {suffix}
    </span>
  );
};

interface CodeBlockProps {
  entity: MessageEntities.ICodeBlock;
}
const CodeBlock: FunctionComponent<CodeBlockProps> = ({ children }) => (
  <span className="chat_message__code">{children}</span>
);

interface GreenTextProps {
  entity: MessageEntities.IGenericEntity;
}
const GreenText: FunctionComponent<GreenTextProps> = ({ children }) => (
  <span className="chat_message__greentext">{children}</span>
);

type EntityComponent =
  | typeof Link
  | typeof Emote
  | typeof Nick
  | typeof Tag
  | typeof Spoiler
  | typeof CodeBlock
  | typeof GreenText;

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

interface MessageProps {
  message: ChatClientEvent.IMessage;
}

const Message: FunctionComponent<MessageProps> = ({ message }) => {
  const { emotes } = message.entities;
  return emotes?.length === 1 && emotes[0].combo ? (
    <ComboMessage message={message} />
  ) : (
    <StandardMessage message={message} />
  );
};

const ComboMessage: FunctionComponent<MessageProps> = ({ message: { body, entities } }) => {
  const formatter = new MessageFormatter(body);
  entities.emotes.forEach((entity) => formatter.insertEntity(Emote, entity));

  return (
    <div className={clsx(["chat__message", "chat__message--emote"])}>
      <span className="body">
        {formatter.body}
        <i className="count">{entities.emotes[0].combo}</i>
        <i className="x">x</i>
        <i className="hits">hits</i>
        <i className="combo">c-c-c-combo</i>
      </span>
    </div>
  );
};

const StandardMessage: FunctionComponent<MessageProps> = ({
  message: { nick, serverTime, body, entities },
}) => {
  const formatter = new MessageFormatter(body);
  entities.codeBlocks.forEach((entity) => formatter.insertEntity(CodeBlock, entity));
  entities.links.forEach((entity) => formatter.insertEntity(Link, entity));
  entities.emotes.forEach((entity) => formatter.insertEntity(Emote, entity));
  entities.nicks.forEach((entity) => formatter.insertEntity(Nick, entity));
  entities.tags.forEach((entity) => formatter.insertEntity(Tag, entity));
  entities.spoilers.forEach((entity) => formatter.insertEntity(Spoiler, entity));
  if (entities.greenText) {
    formatter.insertEntity(GreenText, entities.greenText);
  }

  const classNames = clsx([
    "chat__message",
    entities.selfMessage && "chat__message--self",
    ...entities.tags.map(({ name }) => `chat__message--tag_${name}`),
  ]);

  const time = new Date(serverTime);

  return (
    <div className={classNames}>
      <time className="time" title={time.toLocaleString()}>
        {time.toLocaleTimeString()}
      </time>
      <span className="nick">{nick}</span>
      <span className="body">{formatter.body}</span>
    </div>
  );
};

export default Message;

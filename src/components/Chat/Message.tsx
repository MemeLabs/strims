import clsx from "clsx";
import * as React from "react";
import { FunctionComponent } from "react";
import { ReactNode } from "react";

import { ChatClientEvent, MessageEntities } from "../../lib/pb";
import Emote from "./Emote";

// TODO: in app links
interface MessageLinkProps {
  entity: MessageEntities.ILink;
}
const MessageLink: FunctionComponent<MessageLinkProps> = ({ children, entity }) => (
  <a className="chat__message__link" target="_blank" rel="nofollow" href={entity.url}>
    {children}
  </a>
);

interface MessageEmoteProps {
  entity: MessageEntities.IEmote;
}
const MessageEmote: FunctionComponent<MessageEmoteProps> = ({ children, entity }) => (
  <Emote name={entity.name} modifiers={entity.modifiers}>
    {children}
  </Emote>
);

interface MessageNickProps {
  entity: MessageEntities.INick;
}
const MessageNick: FunctionComponent<MessageNickProps> = ({ children }) => (
  <span className="chat__message__nick">{children}</span>
);

interface MessageTagProps {
  entity: MessageEntities.ITag;
}
const MessageTag: FunctionComponent<MessageTagProps> = ({ children }) => (
  <span className="tag">{children}</span>
);

// TODO: extract spoiler body bounds in parser
const spoilerPrefix = /^[|\s]+/;
const spoilerSuffix = /[|\s]+$/;

const trimSpoiler = (node: React.ReactNode, rx: RegExp) =>
  typeof node === "string" ? node.replace(rx, "") : node;

interface MessageSpoilerProps {
  entity: MessageEntities.ISpoiler;
}
const MessageSpoiler: FunctionComponent<MessageSpoilerProps> = ({ children: childrenNode }) => {
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
  entity: MessageEntities.ICodeBlock;
}
const MessageCodeBlock: FunctionComponent<MessageCodeBlockProps> = ({ children }) => (
  <span className="chat__message__code">{children}</span>
);

interface MessageGreenTextProps {
  entity: MessageEntities.IGenericEntity;
}
// TODO: optionally disable
const MessageGreenText: FunctionComponent<MessageGreenTextProps> = ({ children }) => (
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

interface MessageProps extends React.HTMLProps<HTMLDivElement> {
  message: ChatClientEvent.IMessage;
}

const Message: FunctionComponent<MessageProps> = (props) => {
  const { emotes } = props.message.entities;
  return emotes?.length === 1 && emotes[0].combo ? (
    <ComboMessage {...props} />
  ) : (
    <StandardMessage {...props} />
  );
};

const ComboMessage: FunctionComponent<MessageProps> = ({
  message: { body, entities },
  className,
  ...props
}) => {
  const formatter = new MessageFormatter(body);
  entities.emotes.forEach((entity) => formatter.insertEntity(MessageEmote, entity));

  return (
    <div {...props} className={clsx(["chat__message", "chat__message--emote", className])}>
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
  className,
  ...props
}) => {
  const formatter = new MessageFormatter(body);
  entities.codeBlocks.forEach((entity) => formatter.insertEntity(MessageCodeBlock, entity));
  entities.links.forEach((entity) => formatter.insertEntity(MessageLink, entity));
  entities.emotes.forEach((entity) => formatter.insertEntity(MessageEmote, entity));
  entities.nicks.forEach((entity) => formatter.insertEntity(MessageNick, entity));
  entities.tags.forEach((entity) => formatter.insertEntity(MessageTag, entity));
  entities.spoilers.forEach((entity) => formatter.insertEntity(MessageSpoiler, entity));
  if (entities.greenText) {
    formatter.insertEntity(MessageGreenText, entities.greenText);
  }

  const classNames = clsx([
    "chat__message",
    entities.selfMessage && "chat__message--self",
    ...entities.tags.map(({ name }) => `chat__message--tag_${name}`),
    className,
  ]);

  const time = new Date(serverTime);

  return (
    <div {...props} className={classNames}>
      <time className="chat__message__time" title={time.toLocaleString()}>
        {time.toLocaleTimeString()}
      </time>
      <span className="chat__message__author">{nick}</span>
      <span className="chat__message__colon">{": "}</span>
      <span className="chat__message__body">{formatter.body}</span>
    </div>
  );
};

export default Message;

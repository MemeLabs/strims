// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Message } from "../../../apis/strims/chat/v1/chat";
import { FIXTURE_EPOCH } from "../../storybook/fixtures/ids";
import { listingItem } from "../../storybook/fixtures/listings";
import { testNetworkKey } from "../../storybook/fixtures/networks";
import { peerKey } from "./MessageEmitter";

type Bounds = Message.Entities.IBounds;

// The rune bounds of the `nth` (from 0) `text` in `body`, as the server's
// entity parser gives them.
export const bounds = (body: string, text: string, nth = 0): Bounds => {
  let i = -1;
  for (let k = 0; k <= nth; k++) {
    i = body.indexOf(text, i + 1);
    if (i === -1) {
      throw new Error(
        `mock message: no ${JSON.stringify(text)} #${nth} in ${JSON.stringify(body)}`
      );
    }
  }
  const start = Array.from(body.substring(0, i)).length;
  return { start, end: start + Array.from(text).length };
};

// A reference to the directory mock's listing `id` (an author's viewed
// listing), coloured by the listing unless `themeColor` (0xRRGGBB) is given.
export const directoryRef = (id: number, themeColor = 0): Message.IDirectoryRef => ({
  directoryId: BigInt(id),
  networkKey: testNetworkKey,
  listing: listingItem(BigInt(id))?.listing,
  themeColor,
});

// Entity constructors over one message's body.
export const entitiesOf = (body: string) => ({
  emote: (
    name: string,
    modifiers: string[] = [],
    opts: { nth?: number; combo?: number; canCombo?: boolean } = {}
  ): Message.Entities.IEmote => ({
    bounds: bounds(body, [name, ...modifiers].join(":"), opts.nth),
    name,
    modifiers,
    combo: opts.combo ?? 0,
    canCombo: opts.canCombo ?? false,
  }),
  emoji: (text: string, nth?: number): Message.Entities.IEmoji => ({
    bounds: bounds(body, text, nth),
  }),
  nick: (nick: string, viewedListing?: Message.IDirectoryRef): Message.Entities.INick => ({
    bounds: bounds(body, nick),
    nick,
    peerKey: peerKey(nick),
    viewedListing,
  }),
  link: (text: string, url = `https://${text}`): Message.Entities.ILink => ({
    bounds: bounds(body, text),
    url,
  }),
  tag: (name: string): Message.Entities.ITag => ({ bounds: bounds(body, name), name }),
  code: (text: string): Message.Entities.ICodeBlock => ({ bounds: bounds(body, text) }),
  spoiler: (text: string): Message.Entities.ISpoiler => ({ bounds: bounds(body, text) }),
  span: (text: string): Message.Entities.IGenericEntity => ({ bounds: bounds(body, text) }),
});

export type EntityBuilder = ReturnType<typeof entitiesOf>;

export interface ChatMessageOptions {
  /** Minutes after the fixtures' epoch (2026-01-01T00:00Z). */
  minute?: number;
  /** The author's key; default peerKey(nick). */
  peerKey?: Uint8Array;
  viewedListing?: Message.IDirectoryRef;
}

// A message from `nick`, its entities made by `entities` over its body.
export const chatMessage = (
  nick: string,
  body: string,
  entities: (e: EntityBuilder) => Message.IEntities = () => ({}),
  { minute = 0, peerKey: key = peerKey(nick), viewedListing }: ChatMessageOptions = {}
): Message =>
  new Message({
    nick,
    peerKey: key,
    serverTime: (FIXTURE_EPOCH + BigInt(minute * 60)) * BigInt(1000),
    body,
    entities: entities(entitiesOf(body)),
    viewedListing,
  });

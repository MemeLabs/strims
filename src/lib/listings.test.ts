// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import {
  FrontendWatchListingsResponse,
  IFrontendWatchListingsResponse,
} from "../apis/strims/network/v1/directory/directory";
import { NetworkListingsMap, foldListingEvents } from "./listings";

const network = (id: number) => ({ id: BigInt(id), name: `n${id}`, key: new Uint8Array([id]) });

const item = (id: number, userCount = 0) => ({
  id: BigInt(id),
  listing: { content: { media: { mimeType: "video/x.strims-units", swarmUri: `magnet:${id}` } } },
  snippet: { title: `listing ${id}` },
  userCount,
});

const events = (...e: IFrontendWatchListingsResponse["events"]) =>
  new FrontendWatchListingsResponse({ events: e }).events;

const change = (networkId: number, ...items: ReturnType<typeof item>[]) => ({
  event: { change: { listings: { network: network(networkId), listings: items } } },
});

const ids = (m: NetworkListingsMap) =>
  Array.from(m, ([n, { listings }]) => [n, Array.from(listings.keys())]);

describe("listing events", () => {
  it("a change adds listings and replaces those it names again", () => {
    let m = foldListingEvents(new Map(), events(change(1, item(1), item(2)), change(2, item(1))));
    assert.deepEqual(ids(m), [
      [BigInt(1), [BigInt(1), BigInt(2)]],
      [BigInt(2), [BigInt(1)]],
    ]);
    m = foldListingEvents(m, events(change(1, { ...item(2), snippet: { title: "renamed" } })));
    assert.equal(m.get(BigInt(1)).listings.get(BigInt(2)).snippet.title, "renamed");
    assert.equal(m.get(BigInt(1)).listings.size, 2);
  });

  it("an unpublish drops its listing, and one for an unknown network is ignored", () => {
    let m = foldListingEvents(new Map(), events(change(1, item(1), item(2))));
    m = foldListingEvents(
      m,
      events(
        { event: { unpublish: { networkId: BigInt(1), listingId: BigInt(1) } } },
        { event: { unpublish: { networkId: BigInt(9), listingId: BigInt(1) } } }
      )
    );
    assert.deepEqual(ids(m), [[BigInt(1), [BigInt(2)]]]);
  });

  it("a user count change updates its listing's count only", () => {
    let m = foldListingEvents(new Map(), events(change(1, item(1, 3))));
    m = foldListingEvents(
      m,
      events(
        {
          event: {
            userCountChange: {
              networkId: BigInt(1),
              listingId: BigInt(1),
              userCount: 7,
              recentUserCount: 9,
            },
          },
        },
        {
          event: {
            userCountChange: { networkId: BigInt(1), listingId: BigInt(5), userCount: 1 },
          },
        }
      )
    );
    const l = m.get(BigInt(1)).listings.get(BigInt(1));
    assert.equal(l.userCount, 7);
    assert.equal(l.snippet.title, "listing 1");
    assert.equal(m.get(BigInt(1)).listings.size, 1);
  });

  it("leaves the listings it folds onto as they were", () => {
    const before = foldListingEvents(new Map(), events(change(1, item(1))));
    foldListingEvents(
      before,
      events(change(1, item(2)), {
        event: { unpublish: { networkId: BigInt(1), listingId: BigInt(1) } },
      })
    );
    assert.deepEqual(ids(before), [[BigInt(1), [BigInt(1)]]]);
  });
});

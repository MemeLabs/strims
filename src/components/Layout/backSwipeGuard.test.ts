// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import { BACK_SWIPE_EDGE_PX, BackSwipeGuard, TAP_SLOP_PX, TouchPoint } from "./backSwipeGuard";

const at = (clientX: number, clientY: number, identifier = 0): TouchPoint => ({
  identifier,
  clientX,
  clientY,
});

describe("BackSwipeGuard", () => {
  it("cancels touches starting near the left edge, and only those", () => {
    const guard = new BackSwipeGuard();
    assert.strictEqual(guard.start(at(0, 400), 1), true);
    assert.strictEqual(guard.start(at(BACK_SWIPE_EDGE_PX - 1, 400), 1), true);
    assert.strictEqual(guard.start(at(BACK_SWIPE_EDGE_PX, 400), 1), false);
    assert.strictEqual(guard.start(at(200, 400), 1), false);
  });

  it("clicks where a tap ends", () => {
    const guard = new BackSwipeGuard();
    // The swapped drawer's arrow, closed, in a portrait phone's strip.
    guard.start(at(25, 300), 1);
    guard.move([at(27, 302)]);
    assert.deepStrictEqual(guard.end([at(26, 301)]), at(26, 301));
    // Each touch is its own: the next ends with nothing to click.
    assert.strictEqual(guard.end([at(26, 301)]), null);
  });

  it("takes a touch within the slop for a tap", () => {
    const guard = new BackSwipeGuard();
    guard.start(at(20, 300), 1);
    assert.deepStrictEqual(guard.end([at(20 + TAP_SLOP_PX, 300)]), at(20 + TAP_SLOP_PX, 300));
  });

  it("doesn't click for a drag: the drawer's swipe from its arrow", () => {
    const guard = new BackSwipeGuard();
    guard.start(at(25, 300), 1);
    for (let x = 35; x <= 300; x += 10) guard.move([at(x, 300)]);
    assert.strictEqual(guard.end([at(300, 300)]), null);
  });

  it("doesn't click for a touch that moved off and came back", () => {
    const guard = new BackSwipeGuard();
    guard.start(at(25, 300), 1);
    guard.move([at(25 + TAP_SLOP_PX + 1, 300)]);
    guard.move([at(25, 300)]);
    assert.strictEqual(guard.end([at(25, 300)]), null);
  });

  it("doesn't click for a touch ending past the slop with no move seen", () => {
    const guard = new BackSwipeGuard();
    guard.start(at(25, 300), 1);
    assert.strictEqual(guard.end([at(25, 300 + TAP_SLOP_PX + 1)]), null);
  });

  it("doesn't click for a touch away from the edge, which the browser clicks itself", () => {
    const guard = new BackSwipeGuard();
    guard.start(at(200, 300), 1);
    assert.strictEqual(guard.end([at(200, 300)]), null);
  });

  it("doesn't click once a second finger lands, or the touch is cancelled", () => {
    const guard = new BackSwipeGuard();
    guard.start(at(20, 300, 1), 1);
    assert.strictEqual(guard.start(at(200, 300, 2), 2), false);
    assert.strictEqual(guard.end([at(200, 300, 2)]), null);
    assert.strictEqual(guard.end([at(20, 300, 1)]), null);

    guard.start(at(20, 300, 3), 1);
    guard.cancel();
    assert.strictEqual(guard.end([at(20, 300, 3)]), null);
  });

  it("leaves a touch on a field to the browser", () => {
    const guard = new BackSwipeGuard();
    assert.strictEqual(guard.start(at(15, 800), 1, true), false);
    assert.strictEqual(guard.end([at(15, 800)]), null);
  });

  it("doesn't click for a guarded touch that lands beside another", () => {
    const guard = new BackSwipeGuard();
    assert.strictEqual(guard.start(at(20, 300, 5), 2), true);
    assert.strictEqual(guard.end([at(20, 300, 5)]), null);
  });
});

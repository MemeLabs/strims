// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import { MAX_BUFFER_SECONDS, UnitBuffer } from "./buffer";

const unit = (
  track: number,
  kind: "video" | "audio",
  us: number,
  keyframe: boolean,
  bytes = 10
) => ({
  track,
  kind,
  keyframe,
  timestamp: us,
  decodeTimestamp: us,
  duration: kind === "audio" ? 21_333 : 0,
  published: 0,
  data: new Uint8Array(bytes),
});

// Video at 30 fps with a keyframe each second, from `from` to `to` seconds.
const video = (b: UnitBuffer, from: number, to: number) => {
  for (let k = Math.round(from * 30); k < to * 30; k++) {
    b.push(unit(1, "video", Math.round((k * 1e6) / 30), k % 30 === 0));
  }
};

describe("UnitBuffer", () => {
  it("seeks a track to its latest keyframe at or before a time", () => {
    const b = new UnitBuffer();
    video(b, 0, 3);
    assert.ok(!b.seekKey(1, -1));
    assert.ok(b.seekKey(1, 2_500_000));
    assert.strictEqual(b.peek(1).timestamp, 2_000_000);
    assert.ok(b.peek(1).keyframe);
    assert.strictEqual(b.count(1), 30);
    assert.strictEqual(b.firstKey(1), 2_000_000);
  });

  it("drops a track's units that end before a time, and to its next keyframe", () => {
    const b = new UnitBuffer();
    for (let i = 0; i < 100; i++) b.push(unit(2, "audio", i * 21_333, true));
    assert.strictEqual(b.dropBefore(2, 1_000_000), 46);
    assert.ok(b.peek(2).timestamp + 21_333 >= 1_000_000);
    video(b, 0, 2);
    assert.strictEqual(b.dropToKey(1), 30);
    assert.strictEqual(b.peek(1).timestamp, 1_000_000);
  });

  it("holds at most its bound of seconds, dropping whole GOPs of video", () => {
    const b = new UnitBuffer();
    video(b, 0, MAX_BUFFER_SECONDS + 5);
    const first = b.peek(1);
    assert.ok(first.keyframe);
    assert.ok(first.timestamp >= 5_000_000, `${first.timestamp}`);
    assert.ok(b.evicted > 0);
    assert.strictEqual(b.bytes, b.count(1) * 10);
  });
});

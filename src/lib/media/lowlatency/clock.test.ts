// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import { AudioClock, AudioTimeline, FreeClock } from "./clock";

const close = (a: number, b: number, eps = 1e-6) =>
  assert.ok(Math.abs(a - b) <= eps, `${a} != ${b} within ${eps}`);

describe("FreeClock", () => {
  it("runs at its rate from its anchor and holds while stopped", () => {
    const c = new FreeClock();
    c.start(10, 100);
    close(c.position(101), 11);
    c.setRate(1.02, 101);
    close(c.position(102), 12.02);
    c.hold(102);
    close(c.position(110), 12.02);
    c.start(c.position(110), 110);
    close(c.position(111), 13.04);
  });
});

describe("AudioTimeline", () => {
  it("maps samples to media through runs, holding at the end", () => {
    const t = new AudioTimeline();
    assert.strictEqual(t.mediaAt(0), null);
    t.append(5, 480, 1 / 48000);
    t.append(5.01, 480, 1.02 / 48000);
    close(t.mediaAt(240), 5.005);
    close(t.mediaAt(480 + 240), 5.01 + (240 * 1.02) / 48000);
    // Past what was sent: where it ends.
    close(t.mediaAt(5000), 5.01 + (480 * 1.02) / 48000);
    close(t.end, 5.01 + (480 * 1.02) / 48000);
    t.prune(600);
    close(t.mediaAt(600), 5.01 + (120 * 1.02) / 48000);
    assert.strictEqual(t.mediaAt(100), null);
    t.clear();
    assert.strictEqual(t.mediaAt(1000), null);
    // Samples go on being numbered after a clear.
    t.append(9, 100, 1 / 48000);
    close(t.mediaAt(960 + 50), 9 + 50 / 48000);
  });
});

describe("AudioClock", () => {
  it("finds the sample heard now from late, jittered reports", () => {
    const rate = 48000;
    const clock = new AudioClock(rate);
    assert.strictEqual(clock.heard(1, 0), null);
    // The context's frame f renders at local time 50 + f / rate; reports
    // arrive 1 to 9 ms after.
    let x = 1;
    for (let q = 0; q < 2000; q += 4) {
      const frame = q * 128;
      x = (x * 7919) % 101;
      const arrival = 50 + frame / rate + 0.001 + (x / 100) * 0.008;
      clock.report({ type: "report", frame, consumed: frame, queued: 100000, starved: 0 }, arrival);
    }
    const now = 50 + (2000 * 128) / rate;
    // Within the least delay's millisecond, a 20 ms output latency behind.
    close(clock.heard(now, 0.02), (now - 50 - 0.02) * rate, 0.0012 * rate);
  });

  it("holds at the last sample queued", () => {
    const clock = new AudioClock(48000);
    clock.report({ type: "report", frame: 0, consumed: 1000, queued: 960, starved: 0 }, 1);
    assert.strictEqual(clock.heard(5, 0), 1960);
  });
});

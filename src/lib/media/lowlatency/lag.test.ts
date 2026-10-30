// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import {
  INITIAL_TARGET,
  JOIN_MAX_WAIT,
  LagTarget,
  MAX_RATE_CHANGE,
  MIN_TARGET,
  STALL_BUMP,
  TARGET_MARGIN,
  rateFor,
} from "./lag";

// Media at `ahead` seconds past arrival, arriving every 0.05 s from `from`
// to `to` with `late(t)` lateness, ticked every 0.1 s.
const feed = (
  lag: LagTarget,
  from: number,
  to: number,
  late: (t: number) => number = () => 0,
  ahead = -0.2
) => {
  let tick = from;
  for (let t = from; t < to; t += 0.05) {
    lag.arrive(t, t + ahead - late(t));
    if (t >= tick) {
      lag.tick(t);
      tick += 0.1;
    }
  }
};

describe("LagTarget", () => {
  it("the live edge follows the most prompt arrival", () => {
    const lag = new LagTarget();
    assert.strictEqual(lag.edge(0), null);
    feed(lag, 0, 5, (t) => (Math.round(t * 20) % 10 === 0 ? 0.3 : 0));
    // The edge is the clock less the prompt arrivals' 0.2 s.
    assert.ok(Math.abs(lag.edge(5) - 4.8) < 1e-6);
  });

  it("the target covers the arrivals' lateness plus the margin, within its bounds", () => {
    const lag = new LagTarget();
    assert.strictEqual(lag.target, INITIAL_TARGET);
    // Steady arrivals: the target falls to the least.
    feed(lag, 0, 60);
    assert.ok(Math.abs(lag.target - Math.max(MIN_TARGET, TARGET_MARGIN)) < 1e-6, `${lag.target}`);
    // Every fifth arrival 0.4 s late: the target rises at once to cover it.
    feed(lag, 60, 70, (t) => (Math.round(t * 20) % 5 === 0 ? 0.4 : 0));
    assert.ok(Math.abs(lag.target - (0.4 + TARGET_MARGIN)) < 0.01, `${lag.target}`);
  });

  it("a stall bumps the target", () => {
    const lag = new LagTarget();
    feed(lag, 0, 30);
    const before = lag.target;
    lag.stall();
    assert.ok(Math.abs(lag.target - before - STALL_BUMP) < 1e-9);
  });

  it("a join settles once the backlog stops arriving faster than real time, or waits its most", () => {
    const lag = new LagTarget();
    // A backlog of 2 s arriving within 0.2 s, then live arrivals.
    for (let i = 0; i <= 20; i++) lag.arrive(i * 0.01, -2 + i * 0.1);
    lag.tick(0.2);
    assert.ok(!lag.settled(0.2));
    feed(lag, 0.25, 0.9, () => 0, -0.25);
    assert.ok(lag.settled(0.9));
    // Arrivals that keep running ahead settle only at the wait's end.
    const racing = new LagTarget();
    for (let t = 0; t < JOIN_MAX_WAIT + 0.1; t += 0.05) {
      racing.arrive(t, t * 3);
      racing.tick(t);
      if (t < JOIN_MAX_WAIT - 0.01) assert.ok(!racing.settled(t), `${t}`);
    }
    assert.ok(racing.settled(JOIN_MAX_WAIT + 0.1));
  });

  it("the rate closes the lag's error within 2%", () => {
    assert.strictEqual(rateFor(0), 1);
    assert.strictEqual(rateFor(0.01), 1);
    assert.ok(rateFor(0.1) > 1 && rateFor(0.1) < 1 + MAX_RATE_CHANGE);
    assert.strictEqual(rateFor(5), 1 + MAX_RATE_CHANGE);
    assert.strictEqual(rateFor(-5), 1 - MAX_RATE_CHANGE);
  });
});

// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import {
  Action,
  INITIAL_TARGET,
  LiveEdge,
  MAX_RATE,
  MIN_TARGET,
  PlaybackState,
  RATE_GAIN,
  RATE_HOLD,
  SEEK_EXCESS,
  STALL_BUMP,
  TARGET_MARGIN,
  quantile,
} from "./live";
import { Range } from "./ranges";

const TICK = 0.2;
const ARRIVAL = 0.05;

// A stream whose media at `frontier(t)` arrives at t, every ARRIVAL seconds
// from `from` to `to`, ticked every TICK seconds. The video is `video(t)`'s
// ranges and fields; without a currentTime, a simulated one follows the
// actions, playing at their rate once asked to play. Returns the actions.
const run = (
  live: LiveEdge,
  from: number,
  to: number,
  frontier: (t: number) => number | null,
  video: (t: number) => Partial<PlaybackState> & { ranges: Range[] }
): { t: number; action: Action }[] => {
  const actions: { t: number; action: Action }[] = [];
  let nextTick = from;
  let currentTime = 0;
  let playing = false;
  let rate = 1;
  for (let t = from; t <= to + 1e-9; t += ARRIVAL) {
    const f = frontier(t);
    if (f !== null) live.arrive(t, f);
    if (t >= nextTick - 1e-9) {
      nextTick += TICK;
      const v = video(t);
      const end = v.ranges[v.ranges.length - 1]?.[1] ?? 0;
      if (playing) currentTime = Math.min(end, currentTime + TICK * rate);
      const state: PlaybackState = {
        now: t,
        currentTime,
        paused: false,
        seeking: false,
        waiting: false,
        ...v,
      };
      const action = live.tick(state);
      actions.push({ t, action });
      rate = action.rate;
      if (action.seek !== undefined) currentTime = action.seek;
      if (action.play) playing = true;
    }
  }
  return actions;
};

const seeks = (actions: { t: number; action: Action }[]) =>
  actions.filter(({ action }) => action.seek !== undefined);

// Joins a live stream whose media at 100 + t arrives at t, buffered from
// 95, playing at `lag` behind the edge once joined.
const joined = (lag: number) => {
  const live = new LiveEdge();
  const actions = run(
    live,
    0,
    5,
    (t) => 100 + t,
    (t) => ({ ranges: [[95, 100 + t]], ...(live.stats().joined && { currentTime: 100 + t - lag }) })
  );
  assert.strictEqual(seeks(actions).length, 1);
  return live;
};

describe("LiveEdge", () => {
  it("joins once the backlog has arrived, at the target behind the edge", () => {
    const live = new LiveEdge();
    // A 4 s GOP's backlog arrives over 0.5 s, then the stream goes on live.
    const frontier = (t: number) => (t < 0.5 ? 100 + 8 * t : 104 + (t - 0.5));
    const actions = run(live, 0, 3, frontier, (t) => ({ ranges: [[100, frontier(t)]] }));
    const [join] = seeks(actions);
    assert.ok(join.t >= 0.5 + 0.3 - 1e-9, `joined at ${join.t}, before the backlog settled`);
    assert.ok(join.action.play);
    const edge = frontier(join.t);
    assert.ok(Math.abs(join.action.seek - (edge - INITIAL_TARGET)) < 0.06, `${join.action.seek}`);
  });

  it("joins a stream without a backlog once the target lag of media is buffered", () => {
    const live = new LiveEdge();
    const actions = run(
      live,
      0,
      3,
      (t) => 100 + t,
      (t) => ({ ranges: [[100, 100 + t]] })
    );
    const [join] = seeks(actions);
    assert.ok(join.t >= INITIAL_TARGET - 1e-9, `joined at ${join.t}`);
    assert.ok(join.action.seek >= 100);
  });

  it("speeds playback up in proportion to the lag beyond the target", () => {
    const live = joined(INITIAL_TARGET + 0.25);
    const { rate } = live.stats();
    assert.ok(Math.abs(rate - (1 + RATE_GAIN * 0.25)) < 0.01, `rate ${rate}`);
  });

  it("holds a playback rate RATE_HOLD seconds before changing it", () => {
    const live = joined(INITIAL_TARGET + 0.5);
    const lag = (t: number) => (t < 5.5 ? INITIAL_TARGET + 1 : INITIAL_TARGET);
    const actions = run(
      live,
      5,
      9,
      (t) => 100 + t,
      (t) => ({ ranges: [[95, 100 + t]], currentTime: 100 + t - lag(t) })
    );
    const changes = actions.filter((a, i) => i > 0 && a.action.rate !== actions[i - 1].action.rate);
    assert.ok(changes.length <= 2, `${changes.length} changes`);
    for (let i = 1; i < changes.length; i++) {
      assert.ok(changes[i].t - changes[i - 1].t >= RATE_HOLD - 1e-9);
    }
    assert.strictEqual(actions[actions.length - 1].action.rate, 1);
  });

  it("caps the playback rate", () => {
    const live = joined(INITIAL_TARGET + 1.9);
    assert.strictEqual(live.stats().rate, MAX_RATE);
  });

  it("slows playback down when the lag falls short of the target", () => {
    const live = joined(INITIAL_TARGET - 0.3);
    assert.ok(live.stats().rate < 1);
  });

  it("plays at rate 1 at the target", () => {
    const live = joined(INITIAL_TARGET);
    assert.strictEqual(live.stats().rate, 1);
  });

  it("seeks to the target once the lag passes it by SEEK_EXCESS", () => {
    const live = joined(INITIAL_TARGET);
    const actions = run(
      live,
      5,
      5,
      (t) => 100 + t,
      (t) => ({ ranges: [[100, 100 + t]], currentTime: 100 + t - INITIAL_TARGET - SEEK_EXCESS - 1 })
    );
    const [seek] = seeks(actions);
    assert.ok(Math.abs(seek.action.seek - (100 + seek.t - INITIAL_TARGET)) < 0.06);
    assert.strictEqual(live.stats().seeks, 1);
  });

  it("jumps a position at a range's end to the next range", () => {
    const live = joined(INITIAL_TARGET);
    const actions = run(
      live,
      5,
      5,
      (t) => 100 + t,
      () => ({
        ranges: [
          [100, 103],
          [103.04, 105],
        ],
        currentTime: 102.98,
      })
    );
    assert.strictEqual(seeks(actions)[0].action.seek, 103.04);
    assert.strictEqual(live.stats().holeJumps, 1);
  });

  it("jumps a stalled position short of a hole to the next range", () => {
    const live = joined(INITIAL_TARGET);
    const actions = run(
      live,
      5,
      5,
      (t) => 100 + t,
      () => ({
        ranges: [
          [100, 103],
          [103.08, 105],
        ],
        currentTime: 102.89,
        waiting: true,
      })
    );
    assert.strictEqual(seeks(actions)[0].action.seek, 103.08);
  });

  it("leaves a playing position short of a hole to play on", () => {
    const live = joined(INITIAL_TARGET);
    const actions = run(
      live,
      5,
      5,
      (t) => 100 + t,
      () => ({
        ranges: [
          [100, 103],
          [103.08, 105],
        ],
        currentTime: 102.7,
      })
    );
    assert.strictEqual(seeks(actions).length, 0);
  });

  it("raises the target to the arrivals' lateness quantile plus its margin", () => {
    const live = new LiveEdge();
    // Every other arrival comes 1.3 s late.
    run(
      live,
      0,
      20,
      (t) => 100 + t - (Math.round(t / ARRIVAL) % 2 ? 1.3 : 0),
      (t) => ({ ranges: [[100, 100 + t]], currentTime: 100 + t - 1.5 })
    );
    assert.ok(Math.abs(live.stats().target - (1.3 + TARGET_MARGIN)) < 0.01);
  });

  it("leaves outages out of the target", () => {
    const live = new LiveEdge();
    // The stream stops for 10 s, then its backlog comes at once.
    const frontier = (t: number) => (t < 5 ? 100 + t : t < 15 ? null : 100 + t);
    run(live, 0, 16, frontier, (t) => ({ ranges: [[100, 100 + Math.min(t, 5)]] }));
    live.arrive(15.01, 105.5);
    run(
      live,
      15.05,
      16,
      (t) => 100 + t,
      (t) => ({ ranges: [[100, 100 + t]] })
    );
    assert.ok(live.stats().target <= INITIAL_TARGET);
    assert.ok(live.stats().outages > 0);
  });

  it("lowers the target slowly toward prompt arrivals' once they inform it", () => {
    const live = new LiveEdge();
    const lag = (t: number) => (t < 12 ? INITIAL_TARGET : MIN_TARGET);
    const actions = run(
      live,
      0,
      40,
      (t) => 100 + t,
      (t) => ({ ranges: [[100, 100 + t]], currentTime: 100 + t - lag(t) })
    );
    assert.ok(actions.length > 0);
    assert.strictEqual(live.stats().target, MIN_TARGET);
  });

  it("bumps the target for a stall", () => {
    const live = joined(INITIAL_TARGET);
    const before = live.stats().target;
    run(
      live,
      5,
      5.4,
      (t) => 100 + t,
      (t) => ({ ranges: [[100, 100 + t]], currentTime: 100 + t - before, waiting: true })
    );
    assert.strictEqual(live.stats().stalls, 1);
    assert.ok(live.stats().target >= before + STALL_BUMP - 0.01);
  });

  it("holds the rate at 1 after relearning, until arrivals come", () => {
    const live = joined(INITIAL_TARGET + 1);
    live.relearn();
    const actions = run(
      live,
      5,
      5.4,
      () => null,
      () => ({ ranges: [[100, 105]], currentTime: 103 })
    );
    assert.ok(actions.every(({ action }) => action.rate === 1 && action.seek === undefined));
  });

  it("joins again after rejoining", () => {
    const live = joined(INITIAL_TARGET);
    live.rejoin();
    const actions = run(
      live,
      5,
      8,
      (t) => 100 + t,
      (t) => ({ ranges: [[105, 100 + t]] })
    );
    assert.strictEqual(seeks(actions).length, 1);
    assert.ok(seeks(actions)[0].action.play);
  });
});

describe("quantile", () => {
  it("takes the value at the quantile's rank", () => {
    assert.strictEqual(quantile([5, 1, 4, 2, 3], 0.5), 3);
    assert.strictEqual(quantile([5, 1, 4, 2, 3], 0.98), 5);
    assert.strictEqual(quantile([], 0.5), 0);
  });
});

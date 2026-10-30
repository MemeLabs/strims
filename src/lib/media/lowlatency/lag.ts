// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Times are seconds: arrivals and ticks on the worker's clock, media times
// on the demuxer's presentation timeline.

/**
 * The target lag before arrivals say how late media comes: above most
 * streams' signing interval and swarm jitter, since a stall at the start
 * bumps the target for minutes.
 */
export const INITIAL_TARGET = 0.75;
/** The least and most target lag. */
export const MIN_TARGET = 0.1;
export const MAX_TARGET = 3;
/**
 * Lag added to the arrival jitter the target covers: the decoder's
 * reordering and latency, and a display frame.
 */
export const TARGET_MARGIN = 0.08;
/** The quantile of arrival lateness the target covers. */
export const JITTER_QUANTILE = 0.98;
/** Seconds of arrivals the jitter quantile is taken over. */
export const JITTER_WINDOW = 30;
/** Seconds of arrivals the live edge is estimated from. */
export const EDGE_WINDOW = 60;
/**
 * Arrivals later than this behind the live edge are outages, such as a
 * swarm reader blocked at a missing chunk: the target doesn't cover them.
 */
export const OUTAGE_LATENESS = 2;
/**
 * Added to the target per stall, and to the lag the target covers, where
 * it halves every STALL_HALF_LIFE seconds.
 */
export const STALL_BUMP = 0.15;
export const STALL_HALF_LIFE = 60;
/** The most the target falls per second, once JITTER_INFORMED seconds of arrivals came. */
export const TARGET_FALL_RATE = 0.02;
export const JITTER_INFORMED = 10;
/**
 * A join waits while the live edge rose more than JOIN_SETTLE_RISE in the
 * last JOIN_SETTLE seconds beyond real time: the reader's backlog, its GOP
 * from the keyframe, is still arriving. It waits JOIN_MAX_WAIT at most.
 */
export const JOIN_SETTLE = 0.3;
export const JOIN_SETTLE_RISE = 0.1;
export const JOIN_MAX_WAIT = 3;
/** Lag beyond the target past which the player skips ahead to the target. */
export const SKIP_EXCESS = 0.5;
/**
 * The playout rate's change per second of lag beyond the target, within
 * 1 +- MAX_RATE_CHANGE: audio is resampled to change it, which shifts its
 * pitch by up to a third of a semitone.
 */
export const RATE_GAIN = 0.1;
export const MAX_RATE_CHANGE = 0.02;
/** Errors within this many seconds of the target leave the rate at 1. */
export const RATE_DEADBAND = 0.02;

const clamp = (x: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, x));

// The q-quantile of values, 0 without any.
export const quantile = (values: number[], q: number): number => {
  if (!values.length) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.min(sorted.length - 1, Math.floor(q * sorted.length))];
};

/**
 * The live edge of a stream and the lag behind it to aim for.
 *
 * The live edge is where media would be at the latest had it come as
 * promptly as the most prompt arrival of the last EDGE_WINDOW seconds: the
 * clock plus the most media time ahead of arrival time any arrival had. An
 * arrival's lateness is how far it fell behind that edge. The target lag
 * covers the JITTER_QUANTILE of lateness plus TARGET_MARGIN and a bump per
 * stall, within MIN_TARGET and MAX_TARGET; it rises at once and falls
 * slowly.
 */
export class LagTarget {
  /** Arrivals, kept for their media time ahead of arrival time, `ahead`, decreasing. */
  private edgeArrivals: { at: number; ahead: number }[] = [];
  private lateness: { at: number; late: number }[] = [];
  /** `ahead` of the edge at recent ticks, for a join's settling. */
  private history: { now: number; ahead: number }[] = [];
  private firstAt: number | null = null;
  private lastTick: number | null = null;
  private bump = 0;
  public target = INITIAL_TARGET;
  public jitter = 0;
  public outages = 0;

  // Media up to `frontier` became presentable at `at`.
  arrive(at: number, frontier: number): void {
    this.firstAt ??= at;
    this.expire(at);
    const ahead = frontier - at;
    while (
      this.edgeArrivals.length &&
      this.edgeArrivals[this.edgeArrivals.length - 1].ahead <= ahead
    ) {
      this.edgeArrivals.pop();
    }
    this.edgeArrivals.push({ at, ahead });
    const late = this.edgeArrivals[0].ahead - ahead;
    if (late > OUTAGE_LATENESS) {
      this.outages++;
    } else {
      this.lateness.push({ at, late });
    }
  }

  /** The timeline changed: the arrivals so far say nothing of the edge. */
  relearn(): void {
    this.edgeArrivals = [];
    this.lateness = [];
    this.history = [];
    this.firstAt = null;
  }

  // When the first arrival since the last relearn came.
  get since(): number | null {
    return this.firstAt;
  }

  // The live edge's media time at `now`; null before any arrival.
  edge(now: number): number | null {
    return this.edgeArrivals.length ? now + this.edgeArrivals[0].ahead : null;
  }

  /** The playout stalled: aim further behind. */
  stall(): void {
    this.bump += STALL_BUMP;
    this.target = Math.min(MAX_TARGET, this.target + STALL_BUMP);
  }

  // Updates the target and the join's history at `now`.
  tick(now: number): void {
    this.expire(now);
    const dt = this.lastTick === null ? 0 : Math.max(0, now - this.lastTick);
    this.lastTick = now;
    this.bump *= Math.pow(0.5, dt / STALL_HALF_LIFE);
    if (this.lateness.length) {
      this.jitter = quantile(
        this.lateness.map((l) => l.late),
        JITTER_QUANTILE
      );
    }
    const want = clamp(this.jitter + TARGET_MARGIN + this.bump, MIN_TARGET, MAX_TARGET);
    const informed = this.lateness.length > 0 && this.lateness[0].at <= now - JITTER_INFORMED;
    if (want >= this.target) {
      this.target = want;
    } else if (informed) {
      this.target = Math.max(want, this.target - TARGET_FALL_RATE * dt);
    }
    if (this.edgeArrivals.length) {
      this.history.push({ now, ahead: this.edgeArrivals[0].ahead });
      while (this.history.length > 1 && this.history[1].now <= now - JOIN_SETTLE) {
        this.history.shift();
      }
    }
  }

  // A join may start at `now`: the backlog stopped arriving faster than
  // real time, or the join waited long enough.
  settled(now: number): boolean {
    if (this.firstAt === null || !this.edgeArrivals.length) return false;
    if (now - this.firstAt >= JOIN_MAX_WAIT) return true;
    const first = this.history[0];
    return (
      first !== undefined &&
      first.now <= now - JOIN_SETTLE &&
      this.edgeArrivals[0].ahead - first.ahead < JOIN_SETTLE_RISE
    );
  }

  private expire(now: number): void {
    while (this.edgeArrivals.length && this.edgeArrivals[0].at < now - EDGE_WINDOW) {
      this.edgeArrivals.shift();
    }
    while (this.lateness.length && this.lateness[0].at < now - JITTER_WINDOW) {
      this.lateness.shift();
    }
  }
}

// The playout rate that closes `error` seconds of lag beyond the target:
// 1 within RATE_DEADBAND, else proportional, within 1 +- MAX_RATE_CHANGE.
export const rateFor = (error: number): number => {
  if (Math.abs(error) <= RATE_DEADBAND) return 1;
  return 1 + clamp(RATE_GAIN * error, -MAX_RATE_CHANGE, MAX_RATE_CHANGE);
};

// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Range } from "./ranges";

// Times are seconds: arrival times on the window's clock, media times on
// the SourceBuffer's timeline.

/** The target lag before arrivals say how late media comes. */
export const INITIAL_TARGET = 1;
/** The least and most target lag. */
export const MIN_TARGET = 0.5;
export const MAX_TARGET = 3;
/**
 * Lag added to the arrival jitter the target covers: appending, decoding,
 * the remuxer holding each track's last unit, and the media an engine
 * still holds when it stops for more (Chromium waits with about 0.1 s
 * buffered).
 */
export const TARGET_MARGIN = 0.3;
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
export const STALL_BUMP = 0.25;
export const STALL_HALF_LIFE = 60;
/** The most the target falls per second, once JITTER_INFORMED seconds of arrivals came. */
export const TARGET_FALL_RATE = 0.05;
export const JITTER_INFORMED = 10;
/** Lag beyond the target past which the player seeks to the target. */
export const SEEK_EXCESS = 2;
/** The playback rate's change per second of lag beyond the target. */
export const RATE_GAIN = 0.2;
export const MIN_RATE = 0.95;
export const MAX_RATE = 1.1;
/**
 * Playback rates are multiples of RATE_STEP, and a rate holds at least
 * RATE_HOLD seconds: Safari loses about 50 ms of playback at each change,
 * so changing it every tick plays slower than rate 1.
 */
export const RATE_STEP = 0.025;
export const RATE_HOLD = 2;
/**
 * A join waits while the live edge rose more than JOIN_SETTLE_RISE in the
 * last JOIN_SETTLE seconds beyond real time: the reader's backlog, its
 * GOP from the keyframe, is still arriving.
 */
export const JOIN_SETTLE = 0.3;
export const JOIN_SETTLE_RISE = 0.1;
/** The longest a join waits for the backlog and for target lag of media. */
export const JOIN_MAX_WAIT = 3;
/** Media a seek leaves ahead of the playback position. */
export const MIN_AHEAD = 0.1;
/**
 * A playback position this close to its range's end, or stalled anywhere,
 * with a later range, jumps to it.
 */
export const RANGE_END_EPSILON = 0.15;

/** What the video shows at a tick. */
export interface PlaybackState {
  /** The window's clock. */
  now: number;
  currentTime: number;
  /** The buffered ranges, normalized. */
  ranges: Range[];
  paused: boolean;
  seeking: boolean;
  /** The video waits for media. */
  waiting: boolean;
}

/** What a tick asks of the video. */
export interface Action {
  seek?: number;
  rate: number;
  /** Start playing: the join's seek is set. */
  play?: boolean;
}

export interface LiveStats {
  joined: boolean;
  /** Seconds from the first arrival to the join's seek. */
  joinTime: number | null;
  /** Seconds behind the estimated live edge. */
  lag: number | null;
  target: number;
  /** The arrival lateness quantile the target covers. */
  jitter: number;
  rate: number;
  stalls: number;
  stallTime: number;
  seeks: number;
  holeJumps: number;
  /** Arrivals that came as outages. */
  outages: number;
}

const clamp = (x: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, x));

// The q-quantile of values, 0 without any.
export const quantile = (values: number[], q: number): number => {
  if (!values.length) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.min(sorted.length - 1, Math.floor(q * sorted.length))];
};

/**
 * Holds a live stream's playback at a target lag behind its live edge.
 *
 * The live edge is where media would be at the latest had it come as
 * promptly as the most prompt arrival of the last EDGE_WINDOW seconds: the
 * clock plus the most media time ahead of arrival time any arrival had. An
 * arrival's lateness is how far it fell behind that edge. The target lag
 * covers the JITTER_QUANTILE of lateness plus TARGET_MARGIN, a bump per
 * stall, within MIN_TARGET and MAX_TARGET; it rises at once and falls
 * slowly.
 *
 * A join waits for the backlog its reader rewound to, then seeks to the
 * target lag behind the edge within the newest buffered range. Playing,
 * it holds the target with the playback rate, which keeps audio and
 * picture going, in steps that hold a while. It seeks only when the lag
 * passes the target by SEEK_EXCESS, since a seek costs a decode from the
 * keyframe before. A playback position in a hole, or at a range's end
 * with a later range, jumps to the next range.
 */
export class LiveEdge {
  /** Arrivals, kept for their media time ahead of arrival time, `ahead`, decreasing. */
  private edge: { at: number; ahead: number }[] = [];
  private lateness: { at: number; late: number }[] = [];
  /** `ahead` of the edge, at recent ticks, for the join's settling. */
  private history: { now: number; ahead: number }[] = [];
  private firstAt: number | null = null;
  private joined = false;
  private target = INITIAL_TARGET;
  private jitter = 0;
  private bump = 0;
  private lastTick: number | null = null;
  private rate = 1;
  private rateSince = -Infinity;
  private stallStart: number | null = null;
  private counts = { stalls: 0, stallTime: 0, seeks: 0, holeJumps: 0, outages: 0 };
  private joinTime: number | null = null;
  private lag: number | null = null;

  // Media up to `frontier` became playable at `at`.
  arrive(at: number, frontier: number): void {
    this.firstAt ??= at;
    this.expire(at);
    const ahead = frontier - at;
    while (this.edge.length && this.edge[this.edge.length - 1].ahead <= ahead) {
      this.edge.pop();
    }
    this.edge.push({ at, ahead });
    const late = this.edge[0].ahead - ahead;
    if (late > OUTAGE_LATENESS) {
      this.counts.outages++;
    } else {
      this.lateness.push({ at, late });
    }
  }

  /** The timeline changed: the arrivals so far say nothing of the edge. */
  relearn(): void {
    this.edge = [];
    this.lateness = [];
    this.history = [];
    this.firstAt = null;
  }

  /** The video lost its media, such as for a new MediaSource: join again. */
  rejoin(): void {
    this.relearn();
    this.joined = false;
    this.stallStart = null;
  }

  tick(s: PlaybackState): Action {
    this.expire(s.now);
    this.updateTarget(s.now);
    const edge = this.edge.length ? s.now + this.edge[0].ahead : null;
    this.lag = edge === null || !this.joined ? null : edge - s.currentTime;
    if (edge !== null) {
      this.history.push({ now: s.now, ahead: this.edge[0].ahead });
      while (this.history.length > 1 && this.history[1].now <= s.now - JOIN_SETTLE) {
        this.history.shift();
      }
    }
    if (!this.joined) {
      return this.join(s, edge);
    }

    if (s.waiting && !s.seeking && !s.paused) {
      if (this.stallStart === null) {
        this.stallStart = s.now;
        this.counts.stalls++;
        this.bump += STALL_BUMP;
        this.target = Math.min(MAX_TARGET, this.target + STALL_BUMP);
      }
    } else if (this.stallStart !== null) {
      this.counts.stallTime += s.now - this.stallStart;
      this.stallStart = null;
    }
    if (s.paused || s.seeking) {
      return { rate: this.rate };
    }

    // Browsers stall short of a hole and don't cross it. The remuxer fills
    // audio holes with silence; those it leaves are over 3 s or of a codec
    // without silent frames, and a browser drops video from one up to the
    // next keyframe.
    const t = s.currentTime;
    const inRange = s.ranges.some(([a, b]) => t >= a && t < b - RANGE_END_EPSILON);
    const next = s.ranges.find(([a]) => a > t);
    if (next && (!inRange || s.waiting)) {
      this.counts.holeJumps++;
      return { ...this.setRate(1), seek: next[0] };
    }

    if (edge === null) {
      return this.holdRate(s.now, 1);
    }
    const err = edge - t - this.target;
    if (err > SEEK_EXCESS) {
      const p = this.position(edge, s.ranges);
      if (p !== null && p > t) {
        this.counts.seeks++;
        return { ...this.setRate(1), seek: p };
      }
    }
    const steps = Math.round((clamp(1 + RATE_GAIN * err, MIN_RATE, MAX_RATE) - 1) / RATE_STEP);
    return this.holdRate(s.now, 1 + steps * RATE_STEP);
  }

  stats(): LiveStats {
    return {
      joined: this.joined,
      joinTime: this.joinTime,
      lag: this.lag,
      target: this.target,
      jitter: this.jitter,
      rate: this.rate,
      ...this.counts,
    };
  }

  // Sets the rate at once, as for a seek, which interrupts playback anyway.
  private setRate(rate: number): Action {
    this.rate = rate;
    return { rate };
  }

  // Moves to `rate` once the current one has held RATE_HOLD seconds.
  private holdRate(now: number, rate: number): Action {
    if (Math.abs(rate - this.rate) > 1e-9 && now - this.rateSince >= RATE_HOLD) {
      this.rate = rate;
      this.rateSince = now;
    }
    return { rate: this.rate };
  }

  private join(s: PlaybackState, edge: number | null): Action {
    const last = s.ranges[s.ranges.length - 1];
    if (edge === null || !last) {
      return this.setRate(1);
    }
    const waited = s.now - this.firstAt;
    const first = this.history[0];
    const settled =
      first !== undefined &&
      first.now <= s.now - JOIN_SETTLE &&
      this.edge[0].ahead - first.ahead < JOIN_SETTLE_RISE;
    if (!settled && waited < JOIN_MAX_WAIT) {
      return this.setRate(1);
    }
    let p = edge - this.target;
    if (p < last[0]) {
      if (waited < JOIN_MAX_WAIT) {
        return this.setRate(1);
      }
      p = last[0];
    }
    p = Math.min(p, last[1] - MIN_AHEAD);
    if (p < last[0]) {
      return this.setRate(1);
    }
    this.joined = true;
    this.joinTime = waited;
    return { ...this.setRate(1), seek: p, play: true };
  }

  // The target lag behind `edge`, within the newest range; null without room.
  private position(edge: number, ranges: Range[]): number | null {
    const last = ranges[ranges.length - 1];
    if (!last) return null;
    const p = Math.min(Math.max(edge - this.target, last[0]), last[1] - MIN_AHEAD);
    return p >= last[0] ? p : null;
  }

  private expire(now: number): void {
    while (this.edge.length && this.edge[0].at < now - EDGE_WINDOW) {
      this.edge.shift();
    }
    while (this.lateness.length && this.lateness[0].at < now - JITTER_WINDOW) {
      this.lateness.shift();
    }
  }

  private updateTarget(now: number): void {
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
  }
}

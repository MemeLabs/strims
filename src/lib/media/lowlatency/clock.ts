// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The playout clocks. Times are seconds: local times on the worker's clock,
// media times on the demuxer's presentation timeline.

import type { WorkletReport } from "./protocol";

/** Seconds of worklet reports the audio clock's offset is the least over. */
export const OFFSET_WINDOW = 2;

/**
 * A clock on the worker's: media advances at `rate` from an anchor while
 * running, and holds while not.
 */
export class FreeClock {
  private anchorAt = 0;
  private anchorMedia = 0;
  public rate = 1;
  public running = false;

  // Starts at `media` at `now`.
  start(media: number, now: number): void {
    this.anchorAt = now;
    this.anchorMedia = media;
    this.running = true;
  }

  position(now: number): number {
    return this.running ? this.anchorMedia + (now - this.anchorAt) * this.rate : this.anchorMedia;
  }

  setRate(rate: number, now: number): void {
    this.anchorMedia = this.position(now);
    this.anchorAt = now;
    this.rate = rate;
  }

  // Holds at the position reached at `now`, or at `media`.
  hold(now: number, media = this.position(now)): void {
    this.anchorMedia = media;
    this.anchorAt = now;
    this.running = false;
  }
}

/** Samples from number `index` playing media from `media`, `perFrame` each. */
interface Run {
  index: number;
  frames: number;
  media: number;
  perFrame: number;
}

/**
 * Which media each sample sent to the worklet plays: runs of samples, each
 * from a media time at a fixed media length per sample (the playout rate
 * over the context's sample rate).
 *
 * Bound: runs are pruned behind the sample played; the worker sends a run
 * per decoded block, which the worklet's queue bounds.
 */
export class AudioTimeline {
  private runs: Run[] = [];
  /** The next sample's number. */
  public sent = 0;

  // The next `frames` samples play media from `media`, `perFrame` each.
  append(media: number, frames: number, perFrame: number): void {
    this.runs.push({ index: this.sent, frames, media, perFrame });
    this.sent += frames;
  }

  // The media time just after the samples sent; null without any.
  get end(): number | null {
    const last = this.runs[this.runs.length - 1];
    return last ? last.media + last.frames * last.perFrame : null;
  }

  // The media playing at sample `index`, which may be fractional: within
  // the runs sent, or the end of the last when past them; null before the
  // first run.
  mediaAt(index: number): number | null {
    let run: Run | null = null;
    for (const r of this.runs) {
      if (r.index > index) break;
      run = r;
    }
    if (!run) return null;
    return run.media + Math.min(index - run.index, run.frames) * run.perFrame;
  }

  // Forgets the runs that end before `index`, keeping the last.
  prune(index: number): void {
    while (this.runs.length > 1 && this.runs[0].index + this.runs[0].frames <= index) {
      this.runs.shift();
    }
  }

  /** Forgets every run: the samples sent from here start over. */
  clear(): void {
    this.runs = [];
  }
}

/**
 * The audio output's clock as the worker sees it: which sample is heard at
 * a time on the worker's clock.
 *
 * Each worklet report gives the sample played at a context frame. The
 * context's clock maps to the worker's by an offset, the least of each
 * report's arrival less its frame's time over OFFSET_WINDOW: a report can
 * only arrive after its quantum was rendered, and the least delay tracks
 * the two clocks' drift. Samples play one a frame from the latest report
 * that had samples to play, up to the last one sent; a sample is heard
 * `latency` after it is rendered.
 */
export class AudioClock {
  private offsets: { at: number; offset: number }[] = [];
  private last: WorkletReport | null = null;
  /** The latest report whose quantum had samples to play. */
  private anchor: WorkletReport | null = null;

  constructor(private readonly sampleRate: number) {}

  report(r: WorkletReport, now: number): void {
    const offset = now - r.frame / this.sampleRate;
    while (this.offsets.length && this.offsets[this.offsets.length - 1].offset >= offset) {
      this.offsets.pop();
    }
    this.offsets.push({ at: now, offset });
    while (this.offsets.length > 1 && this.offsets[0].at < now - OFFSET_WINDOW) {
      this.offsets.shift();
    }
    this.last = r;
    if (r.queued > 0) this.anchor = r;
  }

  // The latest report.
  get latest(): WorkletReport | null {
    return this.last;
  }

  // The worklet has run out of samples, as of the latest report.
  get starving(): boolean {
    return this.last !== null && this.last.queued === 0;
  }

  // The sample, possibly fractional, heard at `now`, `latency` seconds
  // after its rendering; no later than the last sample queued at the
  // latest report. Null before the worklet played any.
  heard(now: number, latency: number): number | null {
    const a = this.anchor;
    const r = this.last;
    if (!a || !r || !this.offsets.length) return null;
    const frame = (now - this.offsets[0].offset - latency) * this.sampleRate;
    return Math.min(a.consumed + frame - a.frame, r.consumed + r.queued);
  }
}

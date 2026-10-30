// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The capture's clock: microseconds since the page's time origin.
export const nowUs = (): number => Math.round(performance.now() * 1000);

/** The least step between two timestamps of a track, in microseconds: more
 * than a tick of any track's timescale.
 */
const MIN_STEP_US = 1000;

/**
 * A track's timeline on the capture's clock. Sources stamp their samples
 * on clocks of their own, a stream's video and audio apart, so each track
 * maps its first sample to the moment it arrived and keeps its source's
 * spacing from there; the tracks then line up to within their delivery
 * delays. Output timestamps increase by at least MIN_STEP_US.
 */
export class TrackClock {
  private source0: number | null = null;
  private arrived0 = 0;
  private last = -Infinity;

  // Anchors the track at its first sample.
  public anchor(sourceUs: number, arrivedUs = nowUs()): void {
    if (this.source0 === null) {
      this.source0 = sourceUs;
      this.arrived0 = arrivedUs;
    }
  }

  // Whether the track has its first sample.
  public get anchored(): boolean {
    return this.source0 !== null;
  }

  // A capture-clock time on the source's clock.
  public toSource(captureUs: number): number {
    return captureUs - this.arrived0 + (this.source0 ?? 0);
  }

  // The output timestamp of a source time: after the track's last.
  public output(sourceUs: number): number {
    const t = Math.max(sourceUs - (this.source0 ?? 0) + this.arrived0, this.last + MIN_STEP_US);
    this.last = t;
    return t;
  }
}

/** An offset that follows its target by at most a step at a time. */
export class Slew {
  public target = 0;
  private value = 0;

  public constructor(private readonly maxStep: number) {}

  // Moves the offset toward the target, and returns it.
  public step(): number {
    const d = this.target - this.value;
    this.value += Math.max(-this.maxStep, Math.min(this.maxStep, d));
    return this.value;
  }
}

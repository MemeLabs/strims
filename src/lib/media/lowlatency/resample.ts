// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Decoded audio placed on the media timeline and resampled to the output's
// rate at the playout rate. Media times are seconds.

/** Output channels: the worklet plays stereo. */
export const OUTPUT_CHANNELS = 2;
/**
 * A block starting within this many seconds of where the audio before it
 * ends follows it at once: timestamps in whole microseconds round.
 */
export const PLACEMENT_TOLERANCE = 0.001;
/** The most input seconds held: past it, the oldest are dropped. */
export const MAX_HELD_SECONDS = 10;

/** Decoded samples from `start`, planar, at `sampleRate`. */
export interface PcmBlock {
  start: number;
  sampleRate: number;
  channels: Float32Array[];
}

/** Samples for the worklet, from media `media`, `perFrame` media seconds each. */
export interface PcmOutput {
  channels: Float32Array<ArrayBuffer>[];
  frames: number;
  media: number;
  perFrame: number;
}

// Catmull-Rom interpolation between b and c, at f in [0, 1).
const cubic = (a: number, b: number, c: number, d: number, f: number): number =>
  b + 0.5 * f * (c - a + f * (2 * a - 5 * b + 4 * c - d + f * (3 * (b - c) + d - a)));

// Stereo planes of a block's channels: mono doubled, the first two of more.
const stereo = (channels: Float32Array[]): Float32Array[] =>
  channels.length >= OUTPUT_CHANNELS
    ? channels.slice(0, OUTPUT_CHANNELS)
    : [channels[0], channels[0]];

/**
 * Lays decoded blocks end to end by their start times, a hole filled with
 * silence and an overlap dropped, and resamples them to the output rate at
 * a playout rate: `rate` media seconds per second, which also changes
 * their pitch. The same rate and output rate play the input as it is.
 *
 * Input is held from the sample the next output reads on, bounded by
 * MAX_HELD_SECONDS.
 */
export class AudioOut {
  /** The held input, stereo planes from absolute sample `base` on. */
  private input: Float32Array[] = [new Float32Array(0), new Float32Array(0)];
  private base = 0;
  /** The run's input rate and the media time of its sample 0. */
  private inRate = 0;
  private origin = 0;
  /** The input position of the next output sample. */
  private pos = 0;
  /** The media time a reset set the next run to start at. */
  private pending: number | null = null;
  /** Silence filled, and input dropped as overlap or for the bound, in seconds. */
  public filled = 0;
  public dropped = 0;

  constructor(private readonly outRate: number) {}

  // The media time the held input ends at; null without a run.
  get end(): number | null {
    return this.inRate ? this.origin + (this.base + this.input[0].length) / this.inRate : null;
  }

  // The media time of the next output sample; null without a run.
  get next(): number | null {
    return this.inRate ? this.origin + this.pos / this.inRate : null;
  }

  // Seconds of input held ahead of the next output sample.
  get ahead(): number {
    const end = this.end;
    const next = this.next;
    return end === null || next === null ? 0 : Math.max(0, end - next);
  }

  // Drops everything: the next block starts a run, from `start` when given.
  reset(start: number | null = null): void {
    this.input = [new Float32Array(0), new Float32Array(0)];
    this.base = 0;
    this.pos = 0;
    this.inRate = 0;
    this.pending = start;
  }

  // Places a block after the input held.
  push(block: PcmBlock): void {
    let planes = stereo(block.channels);
    let start = block.start;
    if (!this.inRate || block.sampleRate !== this.inRate) {
      // A new run, at the media time a reset asked for when given.
      this.input = [new Float32Array(0), new Float32Array(0)];
      this.inRate = block.sampleRate;
      this.base = 0;
      this.origin = this.pending ?? start;
      this.pos = 0;
      this.pending = null;
    }
    const gap = start - (this.end ?? start);
    if (gap > PLACEMENT_TOLERANCE) {
      const n = Math.round(gap * this.inRate);
      this.append([new Float32Array(n), new Float32Array(n)]);
      this.filled += n / this.inRate;
    } else if (gap < -PLACEMENT_TOLERANCE) {
      const n = Math.min(planes[0].length, Math.round(-gap * this.inRate));
      this.dropped += n / this.inRate;
      planes = planes.map((p) => p.subarray(n));
      start += n / this.inRate;
    }
    if (planes[0].length) this.append(planes);
    const over = this.input[0].length - Math.ceil(MAX_HELD_SECONDS * this.inRate);
    if (over > 0) {
      this.input = this.input.map((p) => p.slice(over));
      this.base += over;
      this.pos = Math.max(this.pos, this.base + 1);
      this.dropped += over / this.inRate;
    }
  }

  // Up to `maxFrames` output samples at playout rate `rate`, as far as the
  // input held allows; null when it allows none.
  pull(maxFrames: number, rate: number): PcmOutput | null {
    if (!this.inRate) return null;
    const step = (this.inRate * rate) / this.outRate;
    if (step === 1) {
      // Back at the input's own rate: whole samples, copied as they are.
      this.pos = Math.round(this.pos);
    }
    const len = this.input[0].length;
    // The last output position must have the sample after it held.
    const last = this.base + len - 2;
    const frames = Math.min(maxFrames, Math.floor((last - this.pos) / step) + 1);
    if (!(frames > 0)) return null;
    const media = this.origin + this.pos / this.inRate;
    const out = [new Float32Array(frames), new Float32Array(frames)];
    const exact = step === 1 && Number.isInteger(this.pos);
    for (let c = 0; c < OUTPUT_CHANNELS; c++) {
      const x = this.input[c];
      const y = out[c];
      if (exact) {
        y.set(x.subarray(this.pos - this.base, this.pos - this.base + frames));
        continue;
      }
      const at = (i: number) => x[Math.min(len - 1, Math.max(0, i))];
      for (let k = 0; k < frames; k++) {
        const p = this.pos + k * step - this.base;
        const i = Math.floor(p);
        y[k] = cubic(at(i - 1), x[i], at(i + 1), at(i + 2), p - i);
      }
    }
    this.pos += frames * step;
    // Keep the sample before the next position, which interpolation reads.
    const keep = Math.max(0, Math.floor(this.pos) - 1 - this.base);
    if (keep > 0) {
      this.input = this.input.map((p) => p.subarray(keep));
      this.base += keep;
    }
    return { channels: out, frames, media, perFrame: step / this.inRate };
  }

  private append(planes: Float32Array[]): void {
    this.input = this.input.map((p, c) => {
      const n = new Float32Array(p.length + planes[c].length);
      n.set(p);
      n.set(planes[c], p.length);
      return n;
    });
  }
}

// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The low-latency player's worker logic: the demuxer's outputs in; decoded
// pictures to a renderer and samples to the audio output out, on a playout
// clock. Times are seconds on the worker's clock and the media timeline but
// for units' microseconds and publish times' Unix milliseconds.

import { UnitBuffer } from "./buffer";
import { AudioClock, AudioTimeline, FreeClock } from "./clock";
import { JOIN_MAX_WAIT, LagTarget, SKIP_EXCESS, quantile, rateFor } from "./lag";
import type {
  AudioDecoderTrack,
  AudioOutput,
  CoreStats,
  DecoderTrack,
  DemuxedUnitOutput,
  DemuxerOutput,
  PlayoutClock,
  VideoDecoderTrack,
  WorkletInput,
  WorkletReport,
} from "./protocol";
import { AudioOut, PcmOutput } from "./resample";

/** A picture shows this long after it is handed to the renderer: a display frame. */
export const DISPLAY_LEAD = 0.016;
/** Pictures decode this far ahead of the playout position. */
export const VIDEO_LOOKAHEAD = 0.3;
/** The most pictures decoding or decoded and not yet shown: hardware decoders hold few. */
export const MAX_FRAMES_AHEAD = 6;
/** The most units queued in a decoder, playing and while decoding up to a start. */
export const MAX_DECODE_QUEUE = 3;
export const PREROLL_DECODE_QUEUE = 8;
/**
 * A start that fell this many seconds behind its target lag while decoding
 * up to it moves on, at most MAX_RETARGETS times a start: the live edge
 * moved on while it decoded.
 */
export const RETARGET_SLACK = 0.05;
export const MAX_RETARGETS = 3;
/** Seconds of samples the worklet is kept fed with. */
export const AUDIO_QUEUE = 0.1;
/** Seconds of audio decoded ahead of what the worklet holds. */
export const AUDIO_DECODE_AHEAD = 0.25;
/** Audio decodes from this long before a start, for the decoder to settle. */
export const AUDIO_PREROLL = 0.1;
/** The longest a start waits for the audio output to play its first samples. */
export const AUDIO_START_TIMEOUT = 0.6;
/** Output samples per message to the worklet. */
export const AUDIO_BLOCK = 1024;
/** A stalled playout resumes once this many seconds of media are ahead of it. */
export const RESUME_AHEAD = 0.1;
/**
 * The audio output running out within this many seconds of the media
 * arrived is a stall; further from it, the worker fell behind.
 */
export const STALL_MARGIN = 0.05;
/** Seconds of publish times kept for the playout position's. */
export const PUBLISHED_WINDOW = 60;
/** Decoder errors within DECODER_ERROR_WINDOW seconds past which the player gives up. */
export const MAX_DECODER_ERRORS = 5;
export const DECODER_ERROR_WINDOW = 10;
/** Measurements kept for the stats' quantiles. */
const SAMPLES = 512;

type Unit = Omit<DemuxedUnitOutput, "type">;

export interface FrameLike {
  readonly timestamp: number;
  readonly displayWidth: number;
  readonly displayHeight: number;
  close(): void;
}

export interface AudioDataLike {
  readonly timestamp: number;
  readonly numberOfFrames: number;
  readonly sampleRate: number;
  close(): void;
}

export interface DecoderLike<Config> {
  readonly state: string;
  readonly decodeQueueSize: number;
  configure(config: Config): void;
  decode(unit: Unit): void;
  close(): void;
}

/** What the core needs of its worker: the clocks, decoders, renderer, worklet and timer. */
export interface CoreEnv {
  /** The worker's clock, in seconds. */
  now(): number;
  /** Date.now(). */
  wall(): number;
  videoDecoder(
    output: (f: FrameLike) => void,
    error: (e: unknown) => void
  ): DecoderLike<VideoDecoderConfig>;
  audioDecoder(
    output: (d: AudioDataLike) => void,
    error: (e: unknown) => void
  ): DecoderLike<AudioDecoderConfig>;
  /** Whether the engine decodes a track. */
  supported(track: DecoderTrack): Promise<boolean>;
  /** An AudioData's samples as planar 32-bit floats. */
  planes(d: AudioDataLike): Float32Array[];
  /** Shows a picture now; takes it. */
  render(frame: FrameLike): void;
  /** Posts to the worklet. */
  worklet(message: WorkletInput, transfer: Transferable[]): void;
  /** Asks for a tick `delay` seconds from now. */
  schedule(delay: number): void;
  /** The player can't go on. */
  fail(message: string, final: boolean, fallback: boolean): void;
}

const enum State {
  /** Waiting for enough of the stream to join. */
  Waiting,
  /** Decoding from a keyframe up to the start without showing. */
  Preroll,
  /** The first picture shows; waiting for the audio output to play. */
  Starting,
  Playing,
  Paused,
}

// The q-quantile of values, null without any.
const pct = (values: number[], q: number): number | null =>
  values.length ? +quantile(values, q).toFixed(1) : null;

const push = (values: number[], x: number) => {
  values.push(x);
  if (values.length > SAMPLES) values.shift();
};

const configKey = (t: DecoderTrack): string =>
  `${t.codec}:${Array.from(t.description, (b) => b.toString(16).padStart(2, "0")).join("")}`;

/**
 * Plays a demuxed stream: a jitter buffer of units, WebCodecs decoders, an
 * audio output fed through a worklet, and a playout clock that the audio
 * output drives while it plays and the worker's clock does otherwise.
 *
 * - **Joining.** Once the reader's backlog has arrived, the player starts
 *   at its target lag behind the live edge: it decodes from the keyframe
 *   before that point without showing the pictures before it, and audio
 *   from a little before it.
 * - **Playing.** A picture shows when the clock reaches it; pictures that
 *   fall due together show the latest. The playout rate holds the target
 *   lag within 2%, and a lag past the target by SKIP_EXCESS skips ahead.
 * - **Losses.** After a lost picture the demuxer gives video from the next
 *   keyframe, so the last picture holds while audio plays on. A decoder
 *   error rebuilds the decoder, which resumes at a keyframe.
 * - **Stalls.** When the master track (audio, else video) runs out the
 *   clock holds, the target rises, and play resumes once RESUME_AHEAD of
 *   media came.
 *
 * Bounds: the jitter buffer's (MAX_BUFFER_BYTES), MAX_FRAMES_AHEAD pictures,
 * AUDIO_QUEUE in the worklet and AUDIO_DECODE_AHEAD decoded.
 */
export class PlayerCore {
  private buffer = new UnitBuffer();
  private lag = new LagTarget();
  private free = new FreeClock();
  private timeline = new AudioTimeline();
  private audioClock: AudioClock | null = null;
  private out: AudioOut | null = null;
  private state = State.Waiting;
  private paused = false;
  private configs = new Map<number, DecoderTrack>();
  private videoTrack: VideoDecoderTrack | null = null;
  private audioTrack: AudioDecoderTrack | null = null;
  private video: DecoderLike<VideoDecoderConfig> | null = null;
  private videoKey: string | null = null;
  private needKey = true;
  /** Units sent to the video decoder and not yet output: their send times. */
  private decoding = new Map<number, number>();
  /** Decoded pictures not yet shown, in presentation order. */
  private frames: FrameLike[] = [];
  private audio: DecoderLike<AudioDecoderConfig> | null = null;
  private audioKey: string | null = null;
  /** Audio units sent to the decoder and not yet output: their durations, by timestamp. */
  private audioInputs = new Map<number, number>();
  private firstAudioOutput = true;
  /** Pictures and samples before this media time aren't shown or played. */
  private floor = 0;
  private startedAt = 0;
  private retargets = 0;
  private rate = 1;
  /** Each track's furthest presentation end arrived. */
  private ends = new Map<number, number>();
  private published: { pts: number; published: number }[] = [];
  private stallStart: number | null = null;
  private errorTimes: number[] = [];
  private supportChecked = new Set<string>();
  private failed = false;
  private counts = {
    stalls: 0,
    stallTime: 0,
    skips: 0,
    gaps: 0,
    discontinuities: 0,
    timelineRestarts: 0,
    framesRendered: 0,
    framesDropped: 0,
    framesSkipped: 0,
    decoderErrors: 0,
  };
  private joinTime: number | null = null;
  private avOffsets: number[] = [];
  private decodeTimes: number[] = [];
  private renderLates: number[] = [];
  private width = 0;
  private height = 0;

  constructor(private readonly env: CoreEnv, private output: AudioOutput | null) {
    if (output) this.openAudio(output);
  }

  // The demuxer's outputs for a Data message that arrived at `at`.
  outputs(outputs: DemuxerOutput[], at: number): void {
    for (const o of outputs) {
      switch (o.type) {
        case "configs":
          this.setConfigs(o.tracks);
          break;
        case "unit":
          this.arrive(o, at);
          break;
        case "gap":
          this.counts.gaps++;
          break;
        case "discontinuity":
          this.counts.discontinuities++;
          break;
        case "restart":
          this.counts.timelineRestarts++;
          this.lag.relearn();
          break;
      }
    }
    this.tick();
  }

  // The audio output changed.
  setAudio(output: AudioOutput): void {
    const was = this.output;
    this.output = output;
    if (!was || was.sampleRate !== output.sampleRate) this.openAudio(output);
    if (was?.running && !output.running) {
      // The clock goes on, silently, where the audio left it.
      const now = this.env.now();
      const p = this.position(now);
      this.resetAudio(null);
      if (p !== null && this.state === State.Playing && this.stallStart === null) {
        this.free.start(p, now);
      }
    } else if (!was?.running && output.running && this.state === State.Playing) {
      const p = this.position(this.env.now());
      if (p !== null) this.resetAudio(p + AUDIO_QUEUE + output.latency);
    }
    this.tick();
  }

  // The worklet's report on what it played.
  report(r: WorkletReport): void {
    this.audioClock?.report(r, this.env.now());
  }

  // The viewer paused or played.
  setPaused(paused: boolean): void {
    if (paused === this.paused) return;
    this.paused = paused;
    const now = this.env.now();
    if (paused) {
      this.free.hold(now);
      this.resetAudio(null);
      this.closeFrames();
      if (this.state !== State.Waiting) this.state = State.Paused;
    } else if (this.state === State.Paused) {
      this.seek(now);
    }
    this.tick();
  }

  tick(): void {
    if (this.failed) return;
    const now = this.env.now();
    this.lag.tick(now);
    if (this.state === State.Waiting) this.join(now);
    this.feedVideo(now);
    this.feedAudio(now);
    if (this.state === State.Preroll) this.prerolled(now);
    if (this.state === State.Starting) this.started(now);
    if (this.state === State.Playing) this.play(now);
    this.env.schedule(this.nextTick(now));
  }

  clock(): PlayoutClock {
    const now = this.env.now();
    const p = this.playing() ? this.position(now) : null;
    const edge = this.lag.edge(now);
    return {
      at: this.env.wall(),
      published: p === null ? null : this.publishedAt(p),
      rate: this.state === State.Playing && this.stallStart === null ? this.rate : 0,
      targetLag: this.lag.target,
      lag: p === null || edge === null ? null : edge - p,
    };
  }

  stats(): CoreStats {
    const now = this.env.now();
    const c = this.clock();
    const latest = this.audioClock?.latest;
    const queued =
      latest && this.output ? (this.timeline.sent - latest.consumed) / this.output.sampleRate : 0;
    return {
      ...this.counts,
      stallTime: this.counts.stallTime + (this.stallStart === null ? 0 : now - this.stallStart),
      clock:
        this.state === State.Waiting || this.state === State.Preroll
          ? "none"
          : this.audioMode(now)
          ? "audio"
          : "free",
      silent: !!this.audioTrack && !this.output?.running,
      joined: this.joinTime !== null,
      joinTime: this.joinTime,
      lag: c.lag,
      target: this.lag.target,
      jitter: this.lag.jitter,
      rate: this.rate,
      outages: this.lag.outages,
      audioUnderruns: latest?.starved ?? 0,
      audioQueued: Math.round(Math.max(0, queued) * 1000),
      avOffset: { p50: pct(this.avOffsets, 0.5), p95: pct(this.avOffsets, 0.95) },
      decodeMs: pct(this.decodeTimes, 0.5),
      renderLate: { p50: pct(this.renderLates, 0.5), p95: pct(this.renderLates, 0.95) },
      publishLag: c.published === null ? null : c.at - c.published,
      busy: 0,
      width: this.width,
      height: this.height,
    };
  }

  close(): void {
    this.failed = true;
    this.closeFrames();
    for (const d of [this.video, this.audio]) {
      try {
        d?.close();
      } catch {
        // Already closed.
      }
    }
    this.video = this.audio = null;
    this.buffer.clear();
  }

  private openAudio(output: AudioOutput): void {
    this.audioClock = new AudioClock(output.sampleRate);
    this.out = new AudioOut(output.sampleRate);
    this.timeline = new AudioTimeline();
  }

  private fail(message: string, fallback: boolean): void {
    if (this.failed) return;
    this.close();
    this.env.fail(message, true, fallback);
  }

  private setConfigs(tracks: DecoderTrack[]): void {
    this.configs = new Map(tracks.map((t) => [t.id, t]));
    this.videoTrack = tracks.find((t): t is VideoDecoderTrack => t.kind === "video") ?? null;
    this.audioTrack = tracks.find((t): t is AudioDecoderTrack => t.kind === "audio") ?? null;
    for (const t of [this.videoTrack, this.audioTrack]) {
      if (!t) continue;
      const key = configKey(t);
      if (this.supportChecked.has(key)) continue;
      this.supportChecked.add(key);
      this.env.supported(t).then(
        (ok) => ok || this.fail(`this browser can't decode ${t.codec}`, true),
        () => this.fail(`this browser can't decode ${t.codec}`, true)
      );
    }
  }

  private master(): number | null {
    return (this.audioTrack ?? this.videoTrack)?.id ?? null;
  }

  private arrive(u: DemuxedUnitOutput, at: number): void {
    const t = this.configs.get(u.track);
    if (!t || (t.id !== this.videoTrack?.id && t.id !== this.audioTrack?.id)) return;
    this.buffer.push(u);
    const pts = u.timestamp / 1e6;
    const end = pts + (u.duration > 0 ? u.duration / 1e6 : 1 / 30);
    this.ends.set(u.track, Math.max(this.ends.get(u.track) ?? -Infinity, end));
    if (u.track === this.master()) {
      this.lag.arrive(at, this.ends.get(u.track));
      this.published.push({ pts, published: u.published });
      while (this.published.length && this.published[0].pts < pts - PUBLISHED_WINDOW) {
        this.published.shift();
      }
    }
  }

  // The furthest the master track's media arrived.
  private frontier(): number | null {
    const m = this.master();
    return m === null ? null : this.ends.get(m) ?? null;
  }

  private publishedAt(media: number): number | null {
    let u: { pts: number; published: number } | undefined;
    for (let i = this.published.length - 1; i >= 0; i--) {
      if (this.published[i].pts <= media) {
        u = this.published[i];
        break;
      }
    }
    u ??= this.published[0];
    return u ? u.published + (media - u.pts) * 1000 : null;
  }

  private playing(): boolean {
    return this.state === State.Starting || this.state === State.Playing;
  }

  private audioMode(now: number): boolean {
    return (
      !!this.output?.running &&
      !!this.audioTrack &&
      this.state === State.Playing &&
      this.audioPosition(now) !== null
    );
  }

  private audioPosition(now: number): number | null {
    if (!this.output?.running || !this.audioClock) return null;
    const heard = this.audioClock.heard(now, this.output.latency);
    return heard === null ? null : this.timeline.mediaAt(heard);
  }

  // The media presented at `now`.
  private position(now: number): number | null {
    if (this.state === State.Waiting || this.state === State.Preroll) return null;
    if (this.state === State.Playing) {
      const a = this.audioPosition(now);
      if (a !== null) return a;
    }
    return this.free.position(now);
  }

  private join(now: number): void {
    if (this.paused || !this.lag.settled(now)) return;
    const edge = this.lag.edge(now);
    const v = this.videoTrack?.id;
    if (edge === null || (v !== undefined && !this.buffer.hasKey(v))) return;
    this.startAt(now, edge - this.lag.target, true);
  }

  // Plays from about `start`: decoding from the keyframe before it. A join
  // without one waits for the reader's backlog, then starts at the first
  // keyframe held; a skip without one decodes on from where it is.
  private startAt(now: number, start: number, joining: boolean): void {
    const v = this.videoTrack?.id;
    if (v !== undefined && !this.buffer.seekKey(v, start * 1e6) && joining) {
      const key = this.buffer.firstKey(v);
      if (key === null || now - (this.lag.since ?? now) < JOIN_MAX_WAIT) return;
      start = key / 1e6;
      this.buffer.seekKey(v, key);
    }
    this.floor = start;
    this.startedAt = now;
    this.retargets = 0;
    const a = this.audioTrack?.id;
    if (a !== undefined) this.buffer.dropBefore(a, (start - AUDIO_PREROLL) * 1e6);
    this.resetAudio(start);
    this.closeFrames(start);
    this.state = State.Preroll;
  }

  // Decoded up to the start: show its picture, then start the clock.
  private prerolled(now: number): void {
    const v = this.videoTrack;
    const edge = this.lag.edge(now);
    if (edge !== null && this.retargets < MAX_RETARGETS) {
      const start = edge - this.lag.target;
      if (start - this.floor > RETARGET_SLACK) {
        // Decoded blocks before the new start are trimmed as they come.
        this.retargets++;
        this.floor = start;
        this.out?.reset(start);
        this.closeFrames(start);
        const a = this.audioTrack?.id;
        if (a !== undefined) this.buffer.dropBefore(a, (start - AUDIO_PREROLL) * 1e6);
      }
    }
    const first = this.frames[0];
    if (v && !first) return;
    const audio = this.audioTrack && this.output?.running;
    const waited = now - this.startedAt >= AUDIO_START_TIMEOUT;
    if (audio && (this.out?.ahead ?? 0) <= 0 && !waited) return;
    if (first) this.show(first, 0);
    this.joinTime ??= now - (this.lag.since ?? now);
    if (audio) {
      this.free.hold(now, this.floor);
      this.startedAt = now;
      this.state = State.Starting;
    } else {
      this.free.start(this.floor, now);
      this.state = State.Playing;
    }
  }

  // The audio output played its first samples, or took too long to.
  private started(now: number): void {
    const heard = this.audioPosition(now);
    if (heard !== null || now - this.startedAt >= AUDIO_START_TIMEOUT) {
      this.free.start(heard ?? this.floor, now);
      this.state = State.Playing;
    }
  }

  private play(now: number): void {
    const p = this.position(now);
    const frontier = this.frontier();
    if (p === null) return;
    // Stalls: the master track ran out, as the audio output found it or
    // the clock reached it.
    const out =
      frontier !== null &&
      (this.audioMode(now)
        ? this.audioClock.starving && frontier - p < STALL_MARGIN
        : frontier <= p + 0.001);
    if (this.stallStart === null && out) {
      this.stallStart = now;
      this.counts.stalls++;
      this.lag.stall();
      this.free.hold(now, p);
    } else if (this.stallStart !== null && frontier !== null && frontier - p >= RESUME_AHEAD) {
      this.counts.stallTime += now - this.stallStart;
      this.stallStart = null;
      this.free.start(p, now);
    }
    const edge = this.lag.edge(now);
    if (edge !== null && this.stallStart === null) {
      const error = edge - p - this.lag.target;
      if (error > SKIP_EXCESS) {
        this.counts.skips++;
        this.seek(now);
        return;
      }
      const rate = rateFor(error);
      if (rate !== this.rate) {
        this.rate = rate;
        this.free.setRate(rate, now);
      }
    }
    // Pictures due: the latest shows, the rest are dropped.
    let due = -1;
    while (
      due + 1 < this.frames.length &&
      this.frames[due + 1].timestamp / 1e6 <= p + DISPLAY_LEAD
    ) {
      due++;
    }
    if (due >= 0) {
      for (let i = 0; i < due; i++) {
        this.frames[i].close();
        this.counts.framesDropped++;
      }
      const f = this.frames[due];
      this.frames.splice(0, due + 1);
      this.show(f, f.timestamp / 1e6 - (p + DISPLAY_LEAD));
    }
  }

  private show(f: FrameLike, offset: number): void {
    this.width = f.displayWidth;
    this.height = f.displayHeight;
    if (this.frames[0] === f) this.frames.shift();
    push(this.avOffsets, Math.abs(offset * 1000));
    push(this.renderLates, Math.max(0, -offset * 1000));
    this.counts.framesRendered++;
    this.env.render(f);
  }

  // Skips to the target lag behind the live edge.
  private seek(now: number): void {
    const edge = this.lag.edge(now);
    if (edge === null) {
      this.state = State.Waiting;
      return;
    }
    if (this.stallStart !== null) {
      this.counts.stallTime += now - this.stallStart;
      this.stallStart = null;
    }
    this.startAt(now, edge - this.lag.target, false);
  }

  private closeFrames(before = Infinity): void {
    const keep: FrameLike[] = [];
    for (const f of this.frames) {
      if (f.timestamp / 1e6 < before) {
        f.close();
      } else {
        keep.push(f);
      }
    }
    this.frames = keep;
  }

  // Drops the audio queued and played: samples from `start` play next, or none.
  private resetAudio(start: number | null): void {
    this.timeline.clear();
    this.env.worklet({ type: "reset", index: this.timeline.sent }, []);
    this.out?.reset(start);
    this.audioInputs.clear();
    this.firstAudioOutput = true;
    // A new decoder: what the old one still holds is of before the reset.
    if (this.audio) this.recreateAudio();
  }

  private recreateAudio(): void {
    try {
      this.audio?.close();
    } catch {
      // Already closed.
    }
    this.audio = null;
    this.audioKey = null;
  }

  private ensureVideo(): DecoderLike<VideoDecoderConfig> | null {
    const t = this.videoTrack;
    if (!t) return null;
    if (!this.video || this.video.state === "closed") {
      const decoder = this.env.videoDecoder(
        (f) => this.videoOutput(f),
        (e) => this.video === decoder && this.decoderError("video", e)
      );
      this.video = decoder;
      this.videoKey = null;
      this.decoding.clear();
    }
    const key = configKey(t);
    if (key !== this.videoKey) {
      this.video.configure({
        codec: t.codec,
        description: t.description,
        optimizeForLatency: true,
      });
      this.videoKey = key;
      this.needKey = true;
    }
    return this.video;
  }

  private ensureAudio(): DecoderLike<AudioDecoderConfig> | null {
    const t = this.audioTrack;
    if (!t) return null;
    if (!this.audio || this.audio.state === "closed") {
      const decoder = this.env.audioDecoder(
        (d) => (this.audio === decoder ? this.audioOutput(d) : d.close()),
        (e) => this.audio === decoder && this.decoderError("audio", e)
      );
      this.audio = decoder;
      this.audioKey = null;
    }
    const key = configKey(t);
    if (key !== this.audioKey) {
      this.audio.configure({
        codec: t.codec,
        sampleRate: t.sampleRate,
        numberOfChannels: t.numberOfChannels || 2,
        description: t.description,
      });
      this.audioKey = key;
      this.firstAudioOutput = true;
    }
    return this.audio;
  }

  private decoderError(kind: "video" | "audio", e: unknown): void {
    this.counts.decoderErrors++;
    const now = this.env.now();
    this.errorTimes = this.errorTimes.filter((t) => t > now - DECODER_ERROR_WINDOW);
    this.errorTimes.push(now);
    if (this.errorTimes.length > MAX_DECODER_ERRORS) {
      this.fail(`the ${kind} decoder failed: ${String(e)}`, true);
      return;
    }
    if (kind === "video") {
      try {
        this.video?.close();
      } catch {
        // Closed by its error.
      }
      this.video = null;
      this.decoding.clear();
      this.needKey = true;
    } else {
      this.recreateAudio();
    }
    this.env.schedule(0);
  }

  private feedVideo(now: number): void {
    const t = this.videoTrack;
    if (!t || this.state === State.Waiting || this.state === State.Paused) return;
    const p = this.position(now) ?? this.floor;
    for (;;) {
      const u = this.buffer.peek(t.id);
      if (!u) return;
      if (this.needKey && !u.keyframe) {
        this.buffer.shift(t.id);
        continue;
      }
      const prerolling = this.state === State.Preroll || u.timestamp / 1e6 < this.floor;
      if (!prerolling && u.timestamp / 1e6 > p + VIDEO_LOOKAHEAD) return;
      const d = this.ensureVideo();
      const queue = this.state === State.Preroll ? PREROLL_DECODE_QUEUE : MAX_DECODE_QUEUE;
      if (!d || d.decodeQueueSize >= queue) return;
      // Pictures before the floor are closed as they come out.
      let kept = this.frames.length;
      for (const ts of this.decoding.keys()) if (ts >= this.floor * 1e6) kept++;
      if (kept >= MAX_FRAMES_AHEAD) return;
      this.buffer.shift(t.id);
      try {
        d.decode(u);
      } catch (e) {
        this.decoderError("video", e);
        return;
      }
      this.needKey = false;
      this.decoding.set(u.timestamp, now);
    }
  }

  private videoOutput(f: FrameLike): void {
    const sent = this.decoding.get(f.timestamp);
    if (sent !== undefined) {
      push(this.decodeTimes, (this.env.now() - sent) * 1000);
      this.decoding.delete(f.timestamp);
    } else if (this.decoding.size) {
      // Outputs come in presentation order: what was sent before it is out.
      const first = Math.min(...this.decoding.keys());
      if (first < f.timestamp) this.decoding.delete(first);
    }
    if (f.timestamp / 1e6 < this.floor || this.state === State.Paused) {
      f.close();
      this.counts.framesSkipped++;
    } else {
      let i = this.frames.length;
      while (i > 0 && this.frames[i - 1].timestamp > f.timestamp) i--;
      this.frames.splice(i, 0, f);
    }
    this.tick();
  }

  private feedAudio(now: number): void {
    const t = this.audioTrack;
    const out = this.out;
    if (!t) return;
    const running = !!this.output?.running && !!out;
    const active =
      running &&
      (this.state === State.Preroll ||
        this.state === State.Starting ||
        this.state === State.Playing);
    if (!active) {
      // Silent or not started: audio behind the playout goes undecoded.
      const p = this.position(now);
      if (p !== null) this.buffer.dropBefore(t.id, (p - AUDIO_PREROLL) * 1e6);
      return;
    }
    // Decode ahead of what the worklet holds.
    for (;;) {
      const u = this.buffer.peek(t.id);
      if (!u) break;
      const queuedInDecoder = [...this.audioInputs.values()].reduce((a, d) => a + d, 0) / 1e6;
      if (out.ahead + queuedInDecoder >= AUDIO_DECODE_AHEAD) break;
      const d = this.ensureAudio();
      if (!d || d.decodeQueueSize >= MAX_DECODE_QUEUE * 4) break;
      this.buffer.shift(t.id);
      try {
        d.decode(u);
      } catch (e) {
        this.decoderError("audio", e);
        break;
      }
      this.audioInputs.set(u.timestamp, u.duration);
    }
    // Keep the worklet fed while playing, but not over a stall.
    if (this.state === State.Preroll || this.stallStart !== null || !this.output) return;
    const rate = this.output.sampleRate;
    const latest = this.audioClock?.latest;
    const played = latest ? latest.consumed : this.timeline.sent;
    while ((this.timeline.sent - played) / rate < AUDIO_QUEUE) {
      const block: PcmOutput | null = out.pull(AUDIO_BLOCK, this.rate);
      if (!block) break;
      this.timeline.append(block.media, block.frames, block.perFrame);
      this.env.worklet(
        { type: "pcm", channels: block.channels, frames: block.frames },
        block.channels.map((c) => c.buffer)
      );
    }
    // The samples heard trail those played by the output's latency.
    this.timeline.prune(played - (this.output.latency + 0.1) * rate);
  }

  private audioOutput(d: AudioDataLike): void {
    const out = this.out;
    if (!out || !this.output?.running) {
      d.close();
      return;
    }
    const input = this.audioInputs.get(d.timestamp);
    this.audioInputs.delete(d.timestamp);
    let start = d.timestamp / 1e6;
    let planes = this.env.planes(d);
    const frames = d.numberOfFrames;
    const rate = d.sampleRate;
    if (input !== undefined) {
      const expected = Math.round((input / 1e6) * rate);
      if (frames < expected - 1) {
        // The decoder dropped the start of what this input decoded to (an
        // Opus pre-skip): what remains is its end.
        start += (expected - frames) / rate;
      } else if (this.firstAudioOutput && this.audioTrack?.preSkip > 0) {
        // A decoder that keeps the pre-skip: drop it here.
        const skip = Math.min(frames, Math.round((this.audioTrack.preSkip * rate) / 48000));
        planes = planes.map((p) => p.subarray(skip));
        start += skip / rate;
      }
    } else {
      // Outputs are in order: inputs before this one are out.
      for (const ts of this.audioInputs.keys()) {
        if (ts < d.timestamp) this.audioInputs.delete(ts);
      }
    }
    this.firstAudioOutput = false;
    d.close();
    if (planes[0]?.length) out.push({ start, sampleRate: rate, channels: planes });
    this.tick();
  }

  private nextTick(now: number): number {
    if (this.state === State.Waiting || this.state === State.Paused) return 0.02;
    if (this.state !== State.Playing || !this.frames.length) return 0.01;
    const p = this.position(now);
    if (p === null) return 0.01;
    const due = (this.frames[0].timestamp / 1e6 - DISPLAY_LEAD - p) / Math.max(this.rate, 0.5);
    return Math.min(0.01, Math.max(0.002, due));
  }
}

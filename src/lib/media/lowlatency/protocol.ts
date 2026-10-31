// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The messages between the low-latency player's window side, its worker and
// its AudioWorklet.

import type {
  CueEndOutput,
  CueOutput,
  DemuxerOutput,
} from "../../../../crates/strims-player/Cargo.toml";

export type {
  AudioDecoderTrack,
  ConfigsOutput,
  CueEndOutput,
  CueOutput,
  DecoderTrack,
  DemuxedUnitOutput,
  DemuxerOutput,
  VideoDecoderTrack,
} from "../../../../crates/strims-player/Cargo.toml";

/** The demuxer behind the worker: Egress Data messages in, decoder configs and units out. */
export interface Core {
  /** Never throws on hostile input; a throw means the core is broken. */
  push(data: Uint8Array, segmentEnd: boolean, discontinuity: boolean): DemuxerOutput[];
  /** The stream ended: an end output, unless one came after the last unit. */
  finish(): DemuxerOutput[];
  metrics(): unknown;
  free(): void;
}

/**
 * How the player puts pictures on screen:
 * - `vtg`: a VideoTrackGenerator in the worker, whose track the window's
 *   video plays (Safari);
 * - `mstg`: frames posted to a MediaStreamTrackGenerator in the window,
 *   whose track the video plays (Chromium);
 * - `canvas`: frames posted to a canvas in the window, whose captureStream
 *   the video plays;
 * - `offscreen`: frames drawn by the worker on a canvas the window
 *   transferred, whose captureStream the video plays where the engine
 *   allows it, and which shows in the video's place where it doesn't
 *   (Firefox).
 */
export const enum Renderer {
  VideoTrackGenerator = "vtg",
  MediaStreamTrackGenerator = "mstg",
  Canvas = "canvas",
  Offscreen = "offscreen",
}

/** The window's audio output, as the worker plans for it. */
export interface AudioOutput {
  /** The AudioContext's rate. */
  sampleRate: number;
  /** The context runs: its clock advances and what the worklet plays is heard. */
  running: boolean;
  /** Seconds from a sample's rendering to its being heard: base plus output latency. */
  latency: number;
}

/** What the window posts the worker. */
export type WindowMessage =
  | {
      type: "open";
      renderer: Renderer;
      /** The transferred canvas, for the offscreen renderer. */
      canvas?: OffscreenCanvas;
      /** A port to the worklet, and the output's state; none without audio output. */
      audioPort?: MessagePort;
      audio?: AudioOutput;
      /**
       * The port of the stream's feed (../feed) when a node feeds it
       * straight from the swarm; without one, the window posts the stream's
       * batches.
       */
      feed?: MessagePort;
    }
  /**
   * A batch of Egress Data messages (../feed). `at` is when the window
   * received them, in Unix milliseconds (performance.timeOrigin + now).
   */
  | { type: "data"; batch: ArrayBuffer; at: number }
  /** The stream ended, after the batches before. */
  | { type: "end" }
  /** The audio output changed: started, suspended, or its latency moved. */
  | { type: "audio"; audio: AudioOutput }
  /** The video was paused or played by the viewer. */
  | { type: "paused"; paused: boolean };

/** Where the player is, for the window and the deadline-aware request scheduler. */
export interface PlayoutClock {
  /** When this was so: Date.now(). */
  at: number;
  /**
   * The publish time, on the writer's Unix clock in milliseconds, of the
   * media presented at `at`; null before the player joins. Media published
   * at p is presented at about `at + (p - published) / rate`.
   */
  published: number | null;
  /** Media seconds the playout advances per second; 0 while stalled or paused. */
  rate: number;
  /** The media time presented at `at`, in seconds; null before the player joins. */
  media: number | null;
  /** Seconds behind the stream's live edge the player aims for. */
  targetLag: number;
  /** Seconds behind the live edge it is; null before it joins. */
  lag: number | null;
}

/** The worker's counts and measurements. */
export interface CoreStats {
  /** How the playout clock runs: on the audio output, free on the worker's clock, or not yet. */
  clock: "audio" | "free" | "none";
  /** The audio output's context is suspended (no tap yet), so the stream plays silently. */
  silent: boolean;
  joined: boolean;
  /** Seconds from the first unit to the first picture. */
  joinTime: number | null;
  lag: number | null;
  target: number;
  /** The arrival lateness quantile the target covers. */
  jitter: number;
  rate: number;
  stalls: number;
  stallTime: number;
  /** Jumps ahead to the target lag, from lag past it. */
  skips: number;
  outages: number;
  gaps: number;
  discontinuities: number;
  timelineRestarts: number;
  framesRendered: number;
  /** Decoded pictures dropped for a later one being due. */
  framesDropped: number;
  /** Pictures decoded and not shown, on a join or a skip. */
  framesSkipped: number;
  decoderErrors: number;
  /** Times the worklet ran out of samples while playing. */
  audioUnderruns: number;
  /** Milliseconds of audio queued in the worklet. */
  audioQueued: number;
  /** Milliseconds each picture shows ahead of (positive) or behind its audio, p50 and p95 of the magnitude. */
  avOffset: { p50: number | null; p95: number | null };
  /** Milliseconds from a picture's decode call to its output, p50. */
  decodeMs: number | null;
  /** Milliseconds from a picture's due time to its hand-off to the renderer, p50 and p95. */
  renderLate: { p50: number | null; p95: number | null };
  /** Milliseconds between the presented media's publishing and now. */
  publishLag: number | null;
  /** The share of the worker's time spent in the player's code. */
  busy: number;
  width: number;
  height: number;
  /** The stream ended, and plays out to its end. */
  ended: boolean;
}

/** What the worker posts the window. */
export type WorkerMessage =
  /** The worker started: what it has. */
  | { type: "hello"; videoTrackGenerator: boolean }
  /** The VideoTrackGenerator's track, for the video. */
  | { type: "track"; track: MediaStreamTrack }
  /**
   * A picture to show now, for the window's renderers, handed off at `at`
   * (Unix milliseconds, performance.timeOrigin + now).
   */
  | { type: "frame"; frame: VideoFrame; at: number }
  /**
   * The worker's state, every STATE_INTERVAL: the clock, the stats, and when
   * it handed pictures to the renderer since the last, in Unix milliseconds
   * (performance.timeOrigin + now).
   */
  | {
      type: "state";
      clock: PlayoutClock;
      stats: CoreStats;
      handoffs: number[];
      /** The Data messages the demuxer took so far, and their bytes. */
      messages: number;
      bytes: number;
    }
  /**
   * The demuxer's cues, on the media timeline: the window shows them on
   * its video's.
   */
  | { type: "cues"; cues: (CueOutput | CueEndOutput)[] }
  /**
   * The worker has taken `batches` of the window's batches: an ack, for a
   * batch that asked.
   */
  | { type: "consumed"; batches: number }
  /**
   * The worker can't go on: the window replaces it, or gives up when
   * `final`. `fallback` says another player may play the stream.
   */
  | { type: "error"; message: string; final: boolean; fallback: boolean };

/** What the worker posts the worklet. */
export type WorkletInput =
  /** Samples to play next, planar, at the context's rate. */
  | { type: "pcm"; channels: Float32Array<ArrayBuffer>[]; frames: number }
  /** Drop what is queued; the next samples sent are number `index`. */
  | { type: "reset"; index: number };

/** What the worklet posts the worker. */
export interface WorkletReport {
  type: "report";
  /** The context frame a quantum it rendered starts at. */
  frame: number;
  /** The number of the sample it played at that frame, or would have. */
  consumed: number;
  /** Samples it held from that one on, when the quantum started. */
  queued: number;
  /** Times it ran out of samples while playing, since the start. */
  starved: number;
}

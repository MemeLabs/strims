// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The messages between a unit-format player's window side and its worker.

import type { RemuxerOutput } from "../../../crates/strims-player/Cargo.toml";

export type { RemuxerOutput, TrackInfo, UnitInfo } from "../../../crates/strims-player/Cargo.toml";

/**
 * A remuxer output as the worker passes it on: init and media segments
 * that the worker's own SourceBuffer takes reach the window without their
 * bytes.
 */
export type CoreOutput = RemuxerOutput extends infer O
  ? O extends { data: Uint8Array<ArrayBuffer> }
    ? Omit<O, "data"> & { data?: Uint8Array<ArrayBuffer> }
    : O
  : never;

/** The remuxer behind a worker: Egress Data messages in, fMP4 out. */
export interface Core {
  /** Never throws on hostile input; a throw means the core is broken. */
  push(data: Uint8Array, segmentEnd: boolean, discontinuity: boolean): RemuxerOutput[];
  /**
   * The stream ended: the outputs of what the core holds, then an end
   * output, unless one came after the last unit.
   */
  finish(): RemuxerOutput[];
  metrics(): unknown;
  free(): void;
}

/** Where the MediaSource lives. */
export const enum SinkMode {
  /** In the worker, attached to the window's video through its handle. */
  Worker = "worker",
  /** In the window; the worker posts the fMP4 bytes there. */
  Window = "window",
}

/** What the window posts the worker. */
export type WindowMessage =
  /**
   * The stream's sink, the window's `performance.timeOrigin`, and the port
   * of the stream's feed (see ./feed) when a node feeds it straight from
   * the swarm; without one, the window posts the stream's batches.
   */
  | { type: "open"; mode: SinkMode; timeOrigin: number; feed?: MessagePort }
  /**
   * A batch of Egress Data messages (see ./feed). `at` is when the window
   * received them, on its clock.
   */
  | { type: "data"; batch: ArrayBuffer; at: number }
  /** The stream ended, after the batches before. */
  | { type: "end" }
  /** The window took the outputs of `batches` batches of the feed: the worker acks them on the feed's port. */
  | { type: "ack"; batches: number }
  /** The video's playback position, which bounds what the sink evicts. */
  | { type: "position"; currentTime: number };

/** What the worker posts the window. */
export type WorkerMessage =
  /** The worker's MediaSource for the video, at the start and after each rebuild. */
  | { type: "attach"; handle: MediaSourceHandle }
  /**
   * The core's outputs for a batch received at `at`, on the window's clock,
   * and the Data messages and bytes the core has taken so far. An `end`
   * output ends the stream after the outputs before it: a window sink ends
   * its stream there.
   */
  | { type: "outputs"; at: number; outputs: CoreOutput[]; messages: number; bytes: number }
  /**
   * The worker has taken `batches` batches, the outputs before this message
   * included: an ack, for a batch that asked, of the window's batches, or
   * in a window's sink mode of the feed's, once the window has taken their
   * outputs.
   */
  | { type: "consumed"; batches: number }
  /** The worker can't go on: the window replaces it, or gives up when `final`. */
  | { type: "error"; message: string; final: boolean };

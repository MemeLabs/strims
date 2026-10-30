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
  | { type: "open"; mode: SinkMode }
  /**
   * An Egress Data message: bytes of the record in progress, the record's
   * end, and whether the bytes before were dropped. `at` is when the window
   * received it, on its clock.
   */
  | { type: "data"; data: ArrayBuffer; segmentEnd: boolean; discontinuity: boolean; at: number }
  /** The video's playback position, which bounds what the sink evicts. */
  | { type: "position"; currentTime: number };

/** What the worker posts the window. */
export type WorkerMessage =
  /** The worker's MediaSource for the video, at the start and after each rebuild. */
  | { type: "attach"; handle: MediaSourceHandle }
  /** The core's outputs for the Data message the window received at `at`. */
  | { type: "outputs"; at: number; outputs: CoreOutput[] }
  /** The worker can't go on: the window replaces it, or gives up when `final`. */
  | { type: "error"; message: string; final: boolean };

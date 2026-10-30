// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Core, CoreOutput, SinkMode, WindowMessage, WorkerMessage } from "./protocol";
import { MediaSink, MediaSourceConstructor } from "./sink";

/** The most Data message bytes held while the core loads. */
export const MAX_PENDING_BYTES = 32 * 1024 * 1024;

/** Loads a core, resolving to a constructor of new ones. */
export type CoreLoader = () => Promise<() => Core>;

interface MediaSourceScope {
  MediaSource?: MediaSourceConstructor;
  ManagedMediaSource?: MediaSourceConstructor;
}

// The worker's own MediaSource type: ManagedMediaSource where it has one.
const workerMediaSource = (scope: MediaSourceScope): MediaSourceConstructor | undefined =>
  scope.ManagedMediaSource ?? scope.MediaSource;

// Runs a unit-format player's worker on `scope`: the window's Data messages
// go through the core, and its init and media segments into the worker's
// MediaSource, whose handle goes to the window, or to the window in its
// outputs. Every output goes to the window for its timing and events.
//
// Any failure posts an error and stops the worker taking messages: the
// window replaces it, which gives the stream a new core and MediaSource.
export const runPlayerWorker = (scope: DedicatedWorkerGlobalScope, load: CoreLoader): void => {
  let core: Core | null = null;
  let sink: MediaSink | null = null;
  let mode: SinkMode | null = null;
  let pending: Extract<WindowMessage, { type: "data" }>[] = [];
  let pendingBytes = 0;
  let stopped = false;

  const post = (message: WorkerMessage, transfer: Transferable[] = []) =>
    scope.postMessage(message, transfer);

  const stop = (message: string, final = false) => {
    if (stopped) return;
    stopped = true;
    pending = [];
    sink?.close();
    try {
      core?.free();
    } catch {
      // A core that trapped can't be freed; the worker goes with it.
    }
    core = null;
    post({ type: "error", message, final });
  };

  const push = ({
    data,
    segmentEnd,
    discontinuity,
    at,
  }: Extract<WindowMessage, { type: "data" }>) => {
    let outputs: CoreOutput[];
    try {
      outputs = core.push(new Uint8Array(data), segmentEnd, discontinuity);
    } catch (e) {
      stop(`the remuxer failed: ${String(e)}`);
      return;
    }
    if (!outputs.length) return;
    const transfer: ArrayBuffer[] = [];
    for (const out of outputs) {
      if (out.type !== "init" && out.type !== "media") continue;
      if (mode === SinkMode.Window) {
        // Each output's bytes have a buffer of their own.
        transfer.push(out.data.buffer);
        continue;
      }
      if (out.type === "init") {
        sink?.init(out.data, out.mime, out.compatible);
      } else {
        sink?.media(out.data);
      }
      delete out.data;
    }
    if (!stopped) post({ type: "outputs", at, outputs }, transfer);
  };

  scope.onmessage = ({ data: message }: MessageEvent<WindowMessage>) => {
    if (stopped) return;
    switch (message.type) {
      case "open": {
        mode = message.mode;
        if (mode === SinkMode.Worker) {
          const MediaSourceType = workerMediaSource(scope as MediaSourceScope);
          if (!MediaSourceType) {
            stop("this worker has no MediaSource", true);
            return;
          }
          sink = new MediaSink(MediaSourceType, {
            attach: (ms) => post({ type: "attach", handle: ms.handle }, [ms.handle]),
            error: (e) => stop(e),
          });
        }
        load().then(
          (create) => {
            if (stopped) return;
            core = create();
            for (const m of pending) {
              push(m);
              if (stopped) return;
            }
            pending = [];
          },
          (e) => stop(`the remuxer didn't load: ${String(e)}`, true)
        );
        return;
      }
      case "data":
        if (core) {
          push(message);
          return;
        }
        pendingBytes += message.data.byteLength;
        if (pendingBytes > MAX_PENDING_BYTES) {
          stop("the remuxer is loading too slowly");
          return;
        }
        pending.push(message);
        return;
      case "position":
        sink?.position(message.currentTime);
        return;
    }
  };
};

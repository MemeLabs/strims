// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { DataMessage, parseBatch } from "./feed";
import { Core, CoreOutput, SinkMode, WindowMessage, WorkerMessage } from "./protocol";
import { MediaSink, MediaSourceConstructor } from "./sink";

/** The most batch bytes held while the core loads. */
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

/** What the worker takes in stream order: a batch received at `at`, or the end. */
type Input = { batch: ArrayBuffer; at: number } | { end: true };

// Runs a unit-format player's worker on `scope`: the stream's batches of
// Data messages (./feed) go through the core, and its init and media
// segments into the worker's MediaSource, whose handle goes to the window,
// or to the window in its outputs. The batches come from the node on the
// open message's feed port, or from the window. Every batch's outputs go to
// the window in one message, for their timing and events.
//
// The worker acks a batch that asks once its outputs are taken: at once,
// to whoever posted it, when its own sink took them; through the window,
// after the outputs, when the window's sink takes them. A sender that stops
// getting acks stops sending, so a worker that falls behind holds a bounded
// queue.
//
// At the stream's end the core writes out what it holds, a worker sink ends
// its stream, and the window hears `ended`.
//
// Any failure posts an error and stops the worker taking messages: the
// window replaces it, which gives the stream a new core and MediaSource.
export const runPlayerWorker = (scope: DedicatedWorkerGlobalScope, load: CoreLoader): void => {
  let core: Core | null = null;
  let sink: MediaSink | null = null;
  let mode: SinkMode | null = null;
  let feed: MessagePort | null = null;
  /** The window's performance.timeOrigin, for arrival times on its clock. */
  let windowOrigin = 0;
  let pending: Input[] = [];
  let pendingBytes = 0;
  /** Data batches taken, and Data messages and their bytes. */
  let batches = 0;
  let messages = 0;
  let bytes = 0;
  let stopped = false;

  const post = (message: WorkerMessage, transfer: Transferable[] = []) =>
    scope.postMessage(message, transfer);

  const stop = (message: string, final = false) => {
    if (stopped) return;
    stopped = true;
    pending = [];
    if (feed) {
      feed.onmessage = null;
      feed.close();
    }
    sink?.close();
    try {
      core?.free();
    } catch {
      // A core that trapped can't be freed; the worker goes with it.
    }
    core = null;
    post({ type: "error", message, final });
  };

  // Gives the window a batch's outputs, the media in them to the sink that
  // takes it.
  const deliver = (outputs: CoreOutput[], at: number) => {
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
    post({ type: "outputs", at, outputs, messages, bytes }, transfer);
  };

  // Acks the batches taken so far.
  const ack = () => {
    if (feed && mode === SinkMode.Worker) {
      feed.postMessage(batches);
    } else {
      post({ type: "consumed", batches });
    }
  };

  const push = (input: Input) => {
    if ("end" in input) {
      end();
      return;
    }
    const batch = parseBatch(input.batch);
    if (typeof batch === "string") {
      stop(`a malformed batch: ${batch}`);
      return;
    }
    if (batch.kind === "end") {
      end();
      return;
    }
    const outputs: CoreOutput[] = [];
    for (const m of batch.messages) {
      if (!take(m, outputs)) return;
    }
    batches++;
    deliver(outputs, input.at);
    if (batch.ack) ack();
  };

  const take = ({ data, segmentEnd, discontinuity }: DataMessage, outputs: CoreOutput[]) => {
    messages++;
    bytes += data.byteLength;
    try {
      outputs.push(...core.push(data, segmentEnd, discontinuity));
      return true;
    } catch (e) {
      stop(`the remuxer failed: ${String(e)}`);
      return false;
    }
  };

  const end = () => {
    let outputs: CoreOutput[];
    try {
      outputs = core.finish();
    } catch (e) {
      stop(`the remuxer failed: ${String(e)}`);
      return;
    }
    deliver(outputs, now());
    sink?.end();
    post({ type: "ended" });
  };

  // Now, on the window's clock, in seconds.
  const now = () => (performance.timeOrigin + performance.now() - windowOrigin) / 1000;

  const input = (i: Input) => {
    if (stopped) return;
    if (core) {
      push(i);
      return;
    }
    if ("batch" in i) {
      pendingBytes += i.batch.byteLength;
      if (pendingBytes > MAX_PENDING_BYTES) {
        stop("the remuxer is loading too slowly");
        return;
      }
    }
    pending.push(i);
  };

  scope.onmessage = ({ data: message }: MessageEvent<WindowMessage>) => {
    if (stopped) return;
    switch (message.type) {
      case "open": {
        mode = message.mode;
        windowOrigin = message.timeOrigin;
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
        if (message.feed) {
          feed = message.feed;
          feed.onmessage = ({ data }: MessageEvent<ArrayBuffer>) => {
            if (data instanceof ArrayBuffer) {
              input({ batch: data, at: now() });
            } else {
              stop("the feed posted something other than a batch");
            }
          };
        }
        load().then(
          (create) => {
            if (stopped) return;
            core = create();
            for (const i of pending) {
              push(i);
              if (stopped) return;
            }
            pending = [];
            pendingBytes = 0;
          },
          (e) => stop(`the remuxer didn't load: ${String(e)}`, true)
        );
        return;
      }
      case "data":
        input({ batch: message.batch, at: message.at });
        return;
      case "end":
        input({ end: true });
        return;
      case "ack":
        feed?.postMessage(message.batches);
        return;
      case "position":
        sink?.position(message.currentTime);
        return;
    }
  };
};

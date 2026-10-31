// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { parseBatch } from "../feed";
import { AudioDataLike, CoreEnv, FrameLike, PlayerCore } from "./core";
import {
  AudioDecoderTrack,
  Core,
  CueEndOutput,
  CueOutput,
  DecoderTrack,
  DemuxerOutput,
  Renderer,
  WindowMessage,
  WorkerMessage,
  WorkletInput,
  WorkletReport,
} from "./protocol";

/** The most batch bytes held while the demuxer loads. */
export const MAX_PENDING_BYTES = 32 * 1024 * 1024;
/** Milliseconds between the worker's state messages. */
export const STATE_INTERVAL_MS = 250;
/** The most hand-off times a state message carries. */
const MAX_HANDOFFS = 64;

/** Loads a demuxer, resolving to a constructor of new ones. */
export type CoreLoader = () => Promise<() => Core>;

/**
 * What the worker takes in stream order: a batch received at `at`, on the
 * worker's clock in seconds, or the end.
 */
type Input = { batch: ArrayBuffer; at: number } | { end: true };

interface GeneratorScope {
  VideoTrackGenerator?: new () => { track: MediaStreamTrack; writable: WritableStream<VideoFrame> };
}

/** Puts pictures on screen from the worker, as the window chose. */
interface WorkerRenderer {
  render(frame: VideoFrame): void;
  close(): void;
}

const createRenderer = (
  scope: DedicatedWorkerGlobalScope,
  renderer: Renderer,
  canvas: OffscreenCanvas | undefined,
  post: (m: WorkerMessage, transfer?: Transferable[]) => void
): WorkerRenderer => {
  switch (renderer) {
    case Renderer.VideoTrackGenerator: {
      const Generator = (scope as GeneratorScope).VideoTrackGenerator;
      const gen = new Generator();
      post({ type: "track", track: gen.track }, [gen.track]);
      const writer = gen.writable.getWriter();
      return {
        render: (frame) => {
          // A writer still busy with the last picture takes no more.
          if (writer.desiredSize !== null && writer.desiredSize <= 0) {
            frame.close();
            return;
          }
          writer.write(frame).catch(() => frame.close());
        },
        close: () => void writer.close().catch(() => undefined),
      };
    }
    case Renderer.Offscreen: {
      const g = canvas.getContext("2d");
      return {
        render: (frame) => {
          if (canvas.width !== frame.displayWidth || canvas.height !== frame.displayHeight) {
            canvas.width = frame.displayWidth;
            canvas.height = frame.displayHeight;
          }
          g.drawImage(frame, 0, 0, canvas.width, canvas.height);
          frame.close();
        },
        close: () => undefined,
      };
    }
    default:
      return {
        render: (frame) =>
          post({ type: "frame", frame, at: performance.timeOrigin + performance.now() }, [frame]),
        close: () => undefined,
      };
  }
};

// An AudioData's samples as planar 32-bit floats: converted by copyTo where
// the engine converts, else copied interleaved and split.
const planes = (d: AudioData): Float32Array[] => {
  const n = d.numberOfFrames;
  const channels = d.numberOfChannels;
  try {
    return Array.from({ length: channels }, (_, c) => {
      const p = new Float32Array(n);
      d.copyTo(p, { planeIndex: c, format: "f32-planar" });
      return p;
    });
  } catch {
    // Copied as it is below.
  }
  try {
    const all = new Float32Array(n * channels);
    d.copyTo(all, { planeIndex: 0, format: "f32" });
    return Array.from({ length: channels }, (_, c) => {
      const p = new Float32Array(n);
      for (let i = 0; i < n; i++) p[i] = all[i * channels + c];
      return p;
    });
  } catch {
    return Array.from({ length: channels }, () => new Float32Array(n));
  }
};

const videoConfig = (t: DecoderTrack): VideoDecoderConfig => ({
  codec: t.codec,
  description: t.description,
  optimizeForLatency: true,
});

const audioConfig = (t: AudioDecoderTrack): AudioDecoderConfig => ({
  codec: t.codec,
  sampleRate: t.sampleRate,
  numberOfChannels: t.numberOfChannels || 2,
  description: t.description,
});

// Runs the low-latency player's worker on `scope`: the stream's batches of
// Data messages (../feed) go through the demuxer into a PlayerCore on
// WebCodecs, whose pictures go to the renderer the window chose and whose
// samples go to the worklet's port. The batches come from the node on the
// open message's feed port, or from the window. It says what it has at once
// ("hello"), and posts its state every STATE_INTERVAL_MS.
//
// The worker acks a batch that asks once the core took its outputs, to
// whoever posted it. A sender that stops getting acks stops sending, so a
// worker that falls behind holds a bounded queue.
//
// The stream ends at a writer session's END record, which the demuxer
// finds in the stream, or at the end of the input: either way the core
// plays out to it.
//
// Any failure posts an error and stops the worker taking messages: the
// window replaces it, or falls back to another player.
export const runLowLatencyWorker = (scope: DedicatedWorkerGlobalScope, load: CoreLoader): void => {
  let demuxer: Core | null = null;
  let core: PlayerCore | null = null;
  let renderer: WorkerRenderer | null = null;
  let port: MessagePort | null = null;
  let feed: MessagePort | null = null;
  let pending: Input[] = [];
  let pendingBytes = 0;
  /** Batches taken, and Data messages and their bytes. */
  let batches = 0;
  let messages = 0;
  let bytes = 0;
  let stopped = false;
  let timer: ReturnType<typeof setTimeout> | null = null;
  let timerAt = Infinity;
  let busy = 0;
  let busySince = performance.now();
  let handoffs: number[] = [];
  let stateTimer: ReturnType<typeof setInterval> | null = null;

  const post = (message: WorkerMessage, transfer: Transferable[] = []) =>
    scope.postMessage(message, transfer);

  const stop = (message: string, final: boolean, fallback: boolean) => {
    if (stopped) return;
    stopped = true;
    pending = [];
    if (feed) {
      feed.onmessage = null;
      feed.close();
    }
    if (timer) clearTimeout(timer);
    if (stateTimer) clearInterval(stateTimer);
    core?.close();
    renderer?.close();
    try {
      demuxer?.free();
    } catch {
      // A demuxer that trapped can't be freed; the worker goes with it.
    }
    demuxer = null;
    post({ type: "error", message, final, fallback });
  };

  // Runs `f`, counting its time as the player's.
  const timed = <T>(f: () => T): T => {
    const t0 = performance.now();
    try {
      return f();
    } finally {
      busy += performance.now() - t0;
    }
  };

  const env: CoreEnv = {
    now: () => performance.now() / 1000,
    wall: () => Date.now(),
    videoDecoder: (output, error) => {
      const d = new VideoDecoder({
        output: (f: VideoFrame) => timed(() => output(f)),
        error: (e) => timed(() => error(e)),
      });
      return {
        get state() {
          return d.state;
        },
        get decodeQueueSize() {
          return d.decodeQueueSize;
        },
        configure: (c) => d.configure(c),
        decode: (u) =>
          d.decode(
            new EncodedVideoChunk({
              type: u.keyframe ? "key" : "delta",
              timestamp: u.timestamp,
              data: u.data,
            })
          ),
        close: () => d.close(),
      };
    },
    audioDecoder: (output, error) => {
      const d = new AudioDecoder({
        output: (a) => timed(() => output(a as unknown as AudioDataLike)),
        error: (e) => timed(() => error(e)),
      });
      return {
        get state() {
          return d.state;
        },
        get decodeQueueSize() {
          return d.decodeQueueSize;
        },
        configure: (c) => d.configure(c),
        decode: (u) =>
          d.decode(
            new EncodedAudioChunk({
              type: "key",
              timestamp: u.timestamp,
              duration: u.duration || undefined,
              data: u.data,
            })
          ),
        close: () => d.close(),
      };
    },
    supported: async (t) => {
      try {
        const r =
          t.kind === "video"
            ? await VideoDecoder.isConfigSupported(videoConfig(t))
            : await AudioDecoder.isConfigSupported(audioConfig(t));
        return !!r.supported;
      } catch {
        return false;
      }
    },
    planes: (d) => planes(d as unknown as AudioData),
    render: (frame: FrameLike) => {
      handoffs.push(performance.timeOrigin + performance.now());
      if (handoffs.length > MAX_HANDOFFS) handoffs.shift();
      renderer?.render(frame as VideoFrame);
    },
    worklet: (message: WorkletInput, transfer) => port?.postMessage(message, transfer),
    schedule: (delay) => {
      const at = performance.now() + delay * 1000;
      if (timer && timerAt <= at) return;
      if (timer) clearTimeout(timer);
      timerAt = at;
      timer = setTimeout(() => {
        timer = null;
        timerAt = Infinity;
        if (!stopped) timed(() => core?.tick());
      }, Math.max(0, delay * 1000));
    },
    fail: (message, final, fallback) => stop(message, final, fallback),
  };

  // Acks the batches taken so far, to whoever posts them.
  const ack = () => {
    if (feed) {
      feed.postMessage(batches);
    } else {
      post({ type: "consumed", batches });
    }
  };

  const finish = () => {
    let outputs: DemuxerOutput[];
    try {
      outputs = demuxer.finish();
    } catch (e) {
      stop(`the demuxer failed: ${String(e)}`, false, false);
      return;
    }
    core.outputs(outputs, performance.now() / 1000);
    postCues(outputs);
  };

  // Passes the window the outputs' cues, which it shows on its video.
  const postCues = (outputs: DemuxerOutput[]) => {
    const cues = outputs.filter(
      (o): o is CueOutput | CueEndOutput => o.type === "cue" || o.type === "cueEnd"
    );
    if (cues.length) post({ type: "cues", cues });
  };

  const push = (input: Input) => {
    if ("end" in input) {
      finish();
      return;
    }
    const batch = parseBatch(input.batch);
    if (typeof batch === "string") {
      stop(`a malformed batch: ${batch}`, false, false);
      return;
    }
    if (batch.kind === "end") {
      finish();
      return;
    }
    const outputs: DemuxerOutput[] = [];
    for (const m of batch.messages) {
      messages++;
      bytes += m.data.byteLength;
      try {
        outputs.push(...demuxer.push(m.data, m.segmentEnd, m.discontinuity));
      } catch (e) {
        stop(`the demuxer failed: ${String(e)}`, false, false);
        return;
      }
    }
    batches++;
    core.outputs(outputs, input.at);
    postCues(outputs);
    if (batch.ack && !stopped) ack();
  };

  const input = (i: Input) => {
    if (stopped) return;
    if (ready()) {
      push(i);
      return;
    }
    if ("batch" in i) {
      pendingBytes += i.batch.byteLength;
      if (pendingBytes > MAX_PENDING_BYTES) {
        stop("the demuxer is loading too slowly", false, false);
        return;
      }
    }
    pending.push(i);
  };

  const postState = () => {
    if (!core || stopped) return;
    const now = performance.now();
    const stats = { ...core.stats(), busy: +(busy / Math.max(1, now - busySince)).toFixed(4) };
    busy = 0;
    busySince = now;
    post({ type: "state", clock: core.clock(), stats, handoffs, messages, bytes });
    handoffs = [];
  };

  const ready = () => demuxer && core;

  scope.onmessage = ({ data: message }: MessageEvent<WindowMessage>) =>
    timed(() => {
      if (stopped) return;
      switch (message.type) {
        case "open": {
          try {
            renderer = createRenderer(scope, message.renderer, message.canvas, post);
          } catch (e) {
            stop(`the renderer failed: ${String(e)}`, true, true);
            return;
          }
          port = message.audioPort ?? null;
          core = new PlayerCore(env, message.audio ?? null);
          if (port) {
            port.onmessage = ({ data }: MessageEvent<WorkletReport>) =>
              timed(() => core?.report(data));
          }
          stateTimer = setInterval(() => timed(postState), STATE_INTERVAL_MS);
          if (message.feed) {
            feed = message.feed;
            feed.onmessage = ({ data }: MessageEvent<ArrayBuffer>) =>
              timed(() => {
                if (data instanceof ArrayBuffer) {
                  input({ batch: data, at: performance.now() / 1000 });
                } else {
                  stop("the feed posted something other than a batch", false, false);
                }
              });
          }
          load().then(
            (create) => {
              if (stopped) return;
              demuxer = create();
              for (const i of pending) {
                timed(() => push(i));
                if (stopped) return;
              }
              pending = [];
              pendingBytes = 0;
            },
            (e) => stop(`the demuxer didn't load: ${String(e)}`, true, true)
          );
          return;
        }
        case "data":
          input({ batch: message.batch, at: (message.at - performance.timeOrigin) / 1000 });
          return;
        case "end":
          input({ end: true });
          return;
        case "audio":
          core?.setAudio(message.audio);
          return;
        case "paused":
          core?.setPaused(message.paused);
          return;
      }
    });

  post({
    type: "hello",
    videoTrackGenerator: typeof (scope as GeneratorScope).VideoTrackGenerator === "function",
  });
};

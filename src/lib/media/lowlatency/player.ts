// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Feed, WindowBatches } from "../feed";
import { WINDOW_BATCHES, WINDOW_BYTES } from "../units";
import { AudioPlayout, sharedAudioContext } from "./audio";
import { quantile } from "./lag";
import { CoreStats, PlayoutClock, Renderer, WindowMessage, WorkerMessage } from "./protocol";
import { defaultRenderer, offscreenCaptureStream } from "./support";

/** Restarts within RESTART_WINDOW_MS after which the player gives up. */
export const MAX_RESTARTS = 3;
export const RESTART_WINDOW_MS = 60_000;
/** Measurements kept for the stats' quantiles. */
const SAMPLES = 512;
/** Displays are matched to hand-offs this long after they show, once every hand-off before them came. */
const MATCH_AFTER_MS = 300;

export interface LowLatencyHandlers {
  /**
   * Points the video at the player's pictures, or at nothing when they
   * show on a canvas in its place. Sets the video's srcObject when not
   * given.
   */
  attach?: (stream: MediaStream | null) => void;
  /** Starts the video. Calls play() when not given. */
  play?: () => Promise<void> | void;
  /**
   * The player gave up; `fallback` says another player may play the
   * stream (the engine can't decode it, or lacks what this player needs).
   */
  error?: (message: string, fallback: boolean) => void;
  /** The context audio plays on: the page's shared one when not given, none when null. */
  audioContext?: AudioContext | null;
  /** The renderer to use, for trying them; the engine's default when not given. */
  renderer?: Renderer;
  /**
   * Opens the stream's feed to a player worker on `port`, straight from
   * the node's swarm, taking over from `replaces`, the player's last feed.
   * With it, the player opens a feed for each worker and closes the last
   * one when it's done; without it, the owner pushes the stream's Data
   * messages.
   */
  feed?: (port: MessagePort, replaces: Feed | null) => Feed;
}

export interface LowLatencyStats extends CoreStats {
  kind: "lowlatency";
  renderer: Renderer | null;
  restarts: number;
  /** Data messages, and their bytes: pushed, or fed and taken by the worker. */
  messages: number;
  bytes: number;
  /** Data messages dropped for the worker falling behind. */
  dropped: number;
  /** Pictures the video presented, by its frame callbacks. */
  presented: number;
  /** Milliseconds from a picture's hand-off in the worker to its expected display, p50 and p95. */
  presentDelay: { p50: number | null; p95: number | null };
  /** Milliseconds between presented pictures: p50, p95, the most, and how many took over twice the p50. */
  frameIntervals: { p50: number | null; p95: number | null; max: number | null; long: number };
  /** Milliseconds from a picture's hand-off to the window taking it, for the window's renderers. */
  postDelay: { p50: number | null; p95: number | null };
  /** Pictures the window's renderer dropped for being busy with the last. */
  windowDropped: number;
  droppedFrames: number | null;
  totalFrames: number | null;
  error: string | null;
}

interface TrackGenerator extends MediaStreamTrack {
  readonly writable: WritableStream<VideoFrame>;
}

type TrackGeneratorConstructor = new (init: { kind: "video" }) => TrackGenerator;

/** A registry of the players on the page, for the debugger and the harnesses. */
interface PlayerRegistry {
  __strims_players__?: unknown[];
}

const defaultWorker = () =>
  new Worker(
    /* webpackChunkName: "lowlatency" */ new URL(
      "../../../web/lowlatency.worker.ts",
      import.meta.url
    ),
    { name: "lowlatency" }
  );

const pct = (xs: number[], q: number): number | null =>
  xs.length ? +quantile(xs, q).toFixed(1) : null;

const push = (xs: number[], x: number) => {
  xs.push(x);
  if (xs.length > SAMPLES) xs.shift();
};

const nowAbs = () => performance.timeOrigin + performance.now();

/**
 * Plays a unit-format stream with WebCodecs in a player worker, at a low
 * target lag behind the live edge, with audio through an AudioWorklet that
 * drives the playout clock. Pictures reach the video through a
 * VideoTrackGenerator in the worker (Safari), a MediaStreamTrackGenerator
 * in the window (Chromium), or a canvas whose stream the video plays (a
 * window canvas, or one the worker draws on). Where the engine can't
 * stream a canvas the worker draws on, that canvas shows in the video's
 * place.
 *
 * It has UnitsPlayer's shape: push each Egress Data message, end() at the
 * stream's end, read stats(), destroy(). clock() is where the playout is,
 * for request scheduling. At the stream's end, a writer session's END
 * record in the stream or the end `end` says, the player plays out to it
 * and holds the last picture; a later session's media plays on.
 *
 * The messages come straight from the node's swarm, on a feed of the
 * worker's own (`handlers.feed`), or from the owner's `push`, which the
 * window batches per task and keeps within WINDOW_BYTES and WINDOW_BATCHES
 * unacked by dropping to the next whole record.
 *
 * A failed worker is replaced, at most MAX_RESTARTS times in
 * RESTART_WINDOW_MS: fed by a new feed, which joins at the stream's
 * current GOP, or from the next whole record pushed. Then, or when the
 * stream can't be decoded here, the error handler hears whether another
 * player may play it.
 */
export class LowLatencyPlayer {
  private worker: Worker | null = null;
  private renderer: Renderer | null = null;
  private audio: AudioPlayout | null = null;
  private generator: TrackGenerator | null = null;
  private writer: WritableStreamDefaultWriter<VideoFrame> | null = null;
  private canvas: HTMLCanvasElement | null = null;
  private canvasContext: CanvasRenderingContext2D | null = null;
  private stream: MediaStream | null = null;
  /** The pushed messages, batched for the worker. */
  private batches = new WindowBatches(WINDOW_BYTES, WINDOW_BATCHES, (b) => this.postBatch(b));
  /** The worker's feed, while there is one, and the port its worker takes. */
  private feed: Feed | null = null;
  private feedPort: MessagePort | null = null;
  private endSent = false;
  private restartTimes: number[] = [];
  private destroyed = false;
  private failure: string | null = null;
  private generation = 0;
  private core: CoreStats | null = null;
  private clockState: PlayoutClock | null = null;
  private listeners = new Set<(c: PlayoutClock) => void>();
  private handoffs: number[] = [];
  private displays: number[] = [];
  private lastPresentation: number | null = null;
  private presentDelays: number[] = [];
  private intervals: number[] = [];
  private postDelays: number[] = [];
  private counts = { restarts: 0, messages: 0, bytes: 0, presented: 0, windowDropped: 0 };

  constructor(
    private readonly video: HTMLVideoElement,
    private readonly handlers: LowLatencyHandlers = {},
    private readonly createWorker: () => Worker = defaultWorker
  ) {
    video.disableRemotePlayback = true;
    video.addEventListener("pause", this.onPause);
    video.addEventListener("play", this.onPlay);
    ((window as PlayerRegistry).__strims_players__ ??= []).push(this);
    this.start();
  }

  // Takes an Egress Data message, when the player has no feed.
  push(data: Uint8Array, segmentEnd: boolean, discontinuity: boolean): void {
    if (this.destroyed || this.failure || this.endSent || this.handlers.feed) return;
    this.counts.messages++;
    this.counts.bytes += data.byteLength;
    this.batches.push({ data: data as Uint8Array<ArrayBuffer>, segmentEnd, discontinuity });
  }

  /** The stream ended: the player plays out to its end, after what was pushed. */
  end(): void {
    if (this.destroyed || this.failure || this.endSent) return;
    this.endSent = true;
    if (this.handlers.feed) return;
    this.batches.flush();
    this.post({ type: "end" });
  }

  // Where the playout is; its target lag before the worker's first state.
  clock(): PlayoutClock {
    return (
      this.clockState ?? {
        at: Date.now(),
        published: null,
        rate: 0,
        targetLag: this.core?.target ?? 0.5,
        lag: null,
      }
    );
  }

  // Hears every clock state the worker posts; answers a function that stops it.
  onClock(listener: (c: PlayoutClock) => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  stats(): LowLatencyStats {
    const q = this.video.getVideoPlaybackQuality?.();
    const p50 = pct(this.intervals, 0.5);
    const empty: CoreStats = {
      clock: "none",
      silent: false,
      joined: false,
      joinTime: null,
      lag: null,
      target: 0.5,
      jitter: 0,
      rate: 1,
      stalls: 0,
      stallTime: 0,
      skips: 0,
      outages: 0,
      gaps: 0,
      discontinuities: 0,
      timelineRestarts: 0,
      framesRendered: 0,
      framesDropped: 0,
      framesSkipped: 0,
      decoderErrors: 0,
      audioUnderruns: 0,
      audioQueued: 0,
      avOffset: { p50: null, p95: null },
      decodeMs: null,
      renderLate: { p50: null, p95: null },
      publishLag: null,
      busy: 0,
      width: 0,
      height: 0,
      ended: false,
    };
    const core = this.core ?? empty;
    return {
      ...core,
      // The clock's publish lag, carried on to now.
      publishLag: this.publishLag() ?? core.publishLag,
      kind: "lowlatency",
      renderer: this.renderer,
      ...this.counts,
      presentDelay: { p50: pct(this.presentDelays, 0.5), p95: pct(this.presentDelays, 0.95) },
      frameIntervals: {
        p50,
        p95: pct(this.intervals, 0.95),
        max: this.intervals.length ? +Math.max(...this.intervals).toFixed(1) : null,
        long: p50 === null ? 0 : this.intervals.filter((x) => x > 2 * p50).length,
      },
      postDelay: { p50: pct(this.postDelays, 0.5), p95: pct(this.postDelays, 0.95) },
      dropped: this.batches.dropped,
      droppedFrames: q?.droppedVideoFrames ?? null,
      totalFrames: q?.totalVideoFrames ?? null,
      error: this.failure,
    };
  }

  destroy(): void {
    if (this.destroyed) return;
    this.destroyed = true;
    this.stop();
    this.closeFeed();
    this.video.removeEventListener("pause", this.onPause);
    this.video.removeEventListener("play", this.onPlay);
    this.attach(null);
    const players = (window as PlayerRegistry).__strims_players__ ?? [];
    if (players.includes(this)) players.splice(players.indexOf(this), 1);
  }

  private start(): void {
    const generation = ++this.generation;
    const worker = this.createWorker();
    this.worker = worker;
    worker.onmessage = ({ data }: MessageEvent<WorkerMessage>) => {
      if (generation === this.generation) this.onMessage(data);
    };
    worker.onerror = (e) => this.restart(`the player worker failed: ${e.message}`);
    if (this.handlers.feed) {
      // The new feed takes over from the last one once it holds the
      // stream, so the swarm stays open between them. Its port goes to
      // the worker with the open message.
      const channel = new MessageChannel();
      this.feed = this.handlers.feed(channel.port1, this.feed);
      this.feedPort = channel.port2;
    } else if (this.endSent) {
      this.post({ type: "end" });
    }
  }

  private closeFeed(): void {
    this.feed?.close();
    this.feed = null;
  }

  // Posts a batch of the pushed messages to the worker.
  private postBatch(batch: ArrayBuffer): void {
    const message: WindowMessage = { type: "data", batch, at: nowAbs() };
    this.worker?.postMessage(message, [batch]);
  }

  private stop(): void {
    this.generation++;
    this.worker?.terminate();
    this.worker = null;
    // A feed port its worker never took.
    this.feedPort?.close();
    this.feedPort = null;
    this.audio?.close();
    this.audio = null;
    void this.writer?.close().catch(() => undefined);
    this.writer = null;
    this.generator?.stop();
    this.generator = null;
    this.stream?.getTracks().forEach((t) => t.stop());
    this.stream = null;
    this.canvas?.remove();
    this.canvas = null;
    this.canvasContext = null;
  }

  private restart(reason: string): void {
    if (this.destroyed || this.failure) return;
    const now = performance.now();
    this.restartTimes = this.restartTimes.filter((t) => t > now - RESTART_WINDOW_MS);
    if (this.restartTimes.length >= MAX_RESTARTS) {
      this.fail(reason, false);
      return;
    }
    this.restartTimes.push(now);
    this.counts.restarts++;
    this.stop();
    this.batches.restart((b) => this.postBatch(b));
    this.start();
  }

  private fail(message: string, fallback: boolean): void {
    this.failure = message;
    this.stop();
    this.closeFeed();
    this.handlers.error?.(message, fallback);
  }

  private attach(stream: MediaStream | null): void {
    if (this.handlers.attach) {
      this.handlers.attach(stream);
    } else {
      this.video.srcObject = stream;
    }
    if (stream) {
      void (this.handlers.play ? this.handlers.play() : this.video.play().catch(() => undefined));
      this.watchPresentation(stream);
    }
  }

  // Counts the pictures the video presents while it plays `stream`.
  private watchPresentation(stream: MediaStream): void {
    const v = this.video;
    if (!v.requestVideoFrameCallback) return;
    const generation = this.generation;
    const cb: VideoFrameRequestCallback = (_, md) => {
      if (generation !== this.generation || v.srcObject !== stream) return;
      this.counts.presented++;
      this.displays.push(performance.timeOrigin + md.expectedDisplayTime);
      if (this.displays.length > SAMPLES) this.displays.shift();
      if (this.lastPresentation !== null) {
        push(this.intervals, md.presentationTime - this.lastPresentation);
      }
      this.lastPresentation = md.presentationTime;
      v.requestVideoFrameCallback(cb);
    };
    v.requestVideoFrameCallback(cb);
  }

  // Matches displays to the hand-offs before them, once those all came.
  private matchDisplays(): void {
    const before = nowAbs() - MATCH_AFTER_MS;
    let i = 0;
    for (; i < this.displays.length && this.displays[i] <= before; i++) {
      const d = this.displays[i];
      let h: number | null = null;
      for (const x of this.handoffs) {
        if (x <= d) h = x;
        else break;
      }
      if (h !== null) push(this.presentDelays, d - h);
    }
    this.displays.splice(0, i);
    while (this.handoffs.length > SAMPLES) this.handoffs.shift();
  }

  private async open(videoTrackGenerator: boolean): Promise<void> {
    const generation = this.generation;
    const scope = globalThis as { MediaStreamTrackGenerator?: TrackGeneratorConstructor };
    const renderer =
      this.handlers.renderer ??
      defaultRenderer({
        videoTrackGenerator,
        mediaStreamTrackGenerator: typeof scope.MediaStreamTrackGenerator === "function",
        offscreenCaptureStream: offscreenCaptureStream(),
      });
    this.renderer = renderer;
    const message: Extract<WindowMessage, { type: "open" }> = { type: "open", renderer };
    const transfer: Transferable[] = [];
    switch (renderer) {
      case Renderer.MediaStreamTrackGenerator: {
        this.generator = new scope.MediaStreamTrackGenerator({ kind: "video" });
        this.writer = this.generator.writable.getWriter();
        this.stream = new MediaStream([this.generator]);
        this.attach(this.stream);
        break;
      }
      case Renderer.Canvas: {
        this.canvas = this.makeCanvas(true);
        this.canvasContext = this.canvas.getContext("2d");
        this.stream = this.canvas.captureStream();
        this.attach(this.stream);
        break;
      }
      case Renderer.Offscreen: {
        this.canvas = this.makeCanvas(false);
        const offscreen = this.canvas.transferControlToOffscreen();
        message.canvas = offscreen;
        transfer.push(offscreen);
        try {
          this.stream = this.canvas.captureStream();
          this.hideCanvas(this.canvas);
          this.attach(this.stream);
        } catch {
          // The canvas shows in the video's place.
          this.attach(null);
        }
        break;
      }
      case Renderer.VideoTrackGenerator:
        break;
    }
    const ctx =
      this.handlers.audioContext === undefined ? sharedAudioContext() : this.handlers.audioContext;
    if (ctx) {
      try {
        const { playout, port } = await AudioPlayout.open(ctx, this.video, (audio) =>
          this.post({ type: "audio", audio })
        );
        if (generation !== this.generation) {
          playout.close();
          return;
        }
        this.audio = playout;
        message.audioPort = port;
        message.audio = playout.output();
        transfer.push(port);
      } catch {
        // No audio output: the clock runs on the worker's.
      }
    }
    if (generation !== this.generation) return;
    if (this.feedPort) {
      message.feed = this.feedPort;
      transfer.push(this.feedPort);
      this.feedPort = null;
    }
    this.worker?.postMessage(message, transfer);
  }

  private makeCanvas(hidden: boolean): HTMLCanvasElement {
    const c = document.createElement("canvas");
    c.className = this.video.className;
    c.style.objectFit = "contain";
    c.style.pointerEvents = "none";
    if (hidden) this.hideCanvas(c);
    this.video.insertAdjacentElement("afterend", c);
    return c;
  }

  // A canvas whose stream the video plays stays in the document, unseen.
  private hideCanvas(c: HTMLCanvasElement): void {
    c.style.width = c.style.height = "1px";
    c.style.opacity = "0";
  }

  private post(message: WindowMessage): void {
    this.worker?.postMessage(message);
  }

  private onMessage(m: WorkerMessage): void {
    switch (m.type) {
      case "hello":
        this.open(m.videoTrackGenerator).catch((e) =>
          this.fail(`the renderer failed: ${String(e)}`, true)
        );
        return;
      case "track":
        this.stream = new MediaStream([m.track]);
        this.attach(this.stream);
        return;
      case "frame":
        this.frame(m.frame, m.at);
        return;
      case "state":
        this.core = m.stats;
        this.clockState = m.clock;
        if (this.handlers.feed) {
          this.counts.messages = m.messages;
          this.counts.bytes = m.bytes;
        }
        this.handoffs.push(...m.handoffs);
        this.matchDisplays();
        if (this.renderer === Renderer.Offscreen && !this.stream) {
          // No video to watch: the hand-offs are the presentation.
          for (let i = 1; i < m.handoffs.length; i++) {
            push(this.intervals, m.handoffs[i] - m.handoffs[i - 1]);
          }
        }
        for (const l of this.listeners) l(m.clock);
        return;
      case "consumed":
        this.batches.ack(m.batches);
        return;
      case "error":
        if (m.final) {
          this.fail(m.message, m.fallback);
        } else {
          this.restart(m.message);
        }
        return;
    }
  }

  private frame(frame: VideoFrame, at: number): void {
    push(this.postDelays, nowAbs() - at);
    if (this.writer) {
      if (this.writer.desiredSize !== null && this.writer.desiredSize <= 0) {
        this.counts.windowDropped++;
        frame.close();
        return;
      }
      this.writer.write(frame).catch(() => frame.close());
      return;
    }
    const c = this.canvas;
    const g = this.canvasContext;
    if (c && g) {
      if (c.width !== frame.displayWidth || c.height !== frame.displayHeight) {
        c.width = frame.displayWidth;
        c.height = frame.displayHeight;
      }
      g.drawImage(frame, 0, 0, c.width, c.height);
    }
    frame.close();
  }

  // Milliseconds between the presented media's publishing and now.
  private publishLag(): number | null {
    const c = this.clockState;
    if (!c || c.published === null) return null;
    const now = Date.now();
    return now - (c.published + (now - c.at) * c.rate);
  }

  private onPause = () => this.post({ type: "paused", paused: true });

  private onPlay = () => this.post({ type: "paused", paused: false });
}

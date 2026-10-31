// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Where a capture's video frames come from: MediaStreamTrackProcessor in
// the window, or in a worker where the engine has it only there, and a
// <video> element elsewhere.

import { nowUs } from "./clock";

/** A video track's frames. */
export interface FrameSource {
  readonly kind: "processor" | "worker" | "turned" | "element" | "upright";
  // The next frame, which the caller closes, or null once the track ended or
  // the source closed.
  next(): Promise<VideoFrame | null>;
  close(): void;
}

/** A source that can turn its frames, by quarter turns clockwise. */
export interface TurningSource extends FrameSource {
  /** The quarter turns the source's last frame was turned by. */
  readonly turns: number;
  /** Whether a turn the source was asked for didn't take. */
  readonly cannotTurn: boolean;
  /** Turns the frames from the next one on. */
  turn(quarterTurns: number): void;
}

export type FramesRequest =
  | { type: "open"; track: MediaStreamTrack }
  | { type: "next" }
  | { type: "turn"; quarterTurns: number }
  | { type: "close" };

export type FramesResponse =
  | { type: "opened" }
  /** A frame, and the quarter turns clockwise it was turned by. */
  | { type: "frame"; frame: VideoFrame | null; turns: number }
  | { type: "error"; message: string };

/**
 * Tells each new picture a <video> presents from its frame callbacks:
 * presentedFrames counts them, where a camera stream's mediaTime needn't
 * move.
 */
export class PresentedPictures {
  private presented = -1;

  // Whether a callback's picture is one not seen before.
  isNew({ presentedFrames }: Pick<VideoFrameCallbackMetadata, "presentedFrames">): boolean {
    if (presentedFrames === this.presented) return false;
    this.presented = presentedFrames;
    return true;
  }
}

// Runs `tick` every `intervalMs` from a worker's timer, which background tabs
// don't throttle as they do the window's.
export const ticker = (intervalMs: number, tick: () => void): (() => void) => {
  try {
    const url = URL.createObjectURL(
      new Blob([`setInterval(() => postMessage(0), ${intervalMs});`], { type: "text/javascript" })
    );
    const worker = new Worker(url);
    worker.onmessage = () => tick();
    return () => {
      worker.terminate();
      URL.revokeObjectURL(url);
    };
  } catch {
    const id = setInterval(tick, intervalMs);
    return () => clearInterval(id);
  }
};

class ProcessorFrames implements FrameSource {
  public readonly kind = "processor";
  private reader: ReadableStreamDefaultReader<VideoFrame>;

  public constructor(track: MediaStreamTrack) {
    const processor = new MediaStreamTrackProcessor({ track });
    this.reader = (processor.readable as ReadableStream<VideoFrame>).getReader();
  }

  public async next(): Promise<VideoFrame | null> {
    let r: ReadableStreamReadResult<VideoFrame>;
    try {
      r = await this.reader.read();
    } catch {
      return null;
    }
    return r.done ? null : r.value;
  }

  public close(): void {
    this.reader.cancel().catch(() => undefined);
  }
}

class WorkerFrames implements TurningSource {
  public turns = 0;
  public cannotTurn = false;
  /** The quarter turns asked for, until a frame turned by them comes. */
  private asked: number | null = null;
  private waiting: ((m: FramesResponse) => void) | null = null;
  private closed = false;

  public constructor(private readonly worker: Worker) {
    worker.onmessage = ({ data }: MessageEvent<FramesResponse>) => {
      const w = this.waiting;
      this.waiting = null;
      if (w) w(data);
      else if (data.type === "frame") data.frame?.close();
    };
  }

  // Opens the source on `track`, which the worker takes. Its frames come
  // as the track gives them until it's asked to turn them.
  public static async open(worker: Worker, track: MediaStreamTrack): Promise<WorkerFrames | null> {
    const source = new WorkerFrames(worker);
    try {
      const opened = source.request({ type: "open", track }, [track]);
      const m = await Promise.race([
        opened,
        new Promise<FramesResponse>((resolve) =>
          setTimeout(() => resolve({ type: "error", message: "timeout" }), 3000)
        ),
      ]);
      if (m.type === "opened") return source;
    } catch {
      // The track can't be transferred.
    }
    source.close();
    return null;
  }

  private request(m: FramesRequest, transfer: Transferable[] = []): Promise<FramesResponse> {
    return new Promise((resolve) => {
      this.waiting = resolve;
      this.worker.postMessage(m, transfer);
    });
  }

  public get kind(): "worker" | "turned" {
    return this.turns ? "turned" : "worker";
  }

  public turn(quarterTurns: number): void {
    if (this.closed) return;
    this.asked = quarterTurns;
    this.worker.postMessage({ type: "turn", quarterTurns } satisfies FramesRequest);
  }

  public async next(): Promise<VideoFrame | null> {
    if (this.closed) return null;
    const m = await this.request({ type: "next" });
    if (m.type !== "frame" || this.closed) {
      if (m.type === "frame") m.frame?.close();
      return null;
    }
    this.turns = m.turns;
    // A frame asked for after the turn: the worker turns frames from there.
    if (this.asked !== null) {
      this.cannotTurn = m.turns !== this.asked;
      this.asked = null;
    }
    return m.frame;
  }

  public close(): void {
    if (this.closed) return;
    this.closed = true;
    this.worker.postMessage({ type: "close" } satisfies FramesRequest);
    this.worker.terminate();
    const w = this.waiting;
    this.waiting = null;
    w?.({ type: "frame", frame: null, turns: this.turns });
  }
}

/** Frames taken from a <video> element playing the track, each picture it
 * presents (PresentedPictures), stamped with the capture's clock. Hidden
 * pages present no frames, so a worker's ticks take one while none comes.
 */
class ElementFrames implements FrameSource {
  public readonly kind: "element" | "upright";
  private video: HTMLVideoElement;
  /** Where an upright source draws the element's picture. */
  private canvas: HTMLCanvasElement | null = null;
  private queued: VideoFrame | null = null;
  private waiting: ((f: VideoFrame | null) => void) | null = null;
  private closed = false;
  private lastAt = 0;
  private stopTicks: () => void;

  // An upright source draws each picture as the element presents it, turned
  // as the track's settings have it, and stops `track` when it closes.
  public constructor(
    private readonly track: MediaStreamTrack,
    private readonly frameIntervalMs: number,
    upright = false
  ) {
    this.kind = upright ? "upright" : "element";
    if (upright) this.canvas = document.createElement("canvas");
    const video = document.createElement("video");
    video.muted = true;
    video.playsInline = true;
    video.autoplay = true;
    // Rendered, so the page presents its frames, and out of the way.
    Object.assign(video.style, {
      position: "fixed",
      left: "0",
      top: "0",
      width: "2px",
      height: "2px",
      opacity: "0.01",
      pointerEvents: "none",
    });
    video.srcObject = new MediaStream([track]);
    document.body.appendChild(video);
    this.video = video;
    video.play().catch(() => undefined);
    if (video.requestVideoFrameCallback) {
      const pictures = new PresentedPictures();
      const onFrame: VideoFrameRequestCallback = (_, metadata) => {
        if (this.closed) return;
        if (pictures.isNew(metadata)) this.take();
        video.requestVideoFrameCallback(onFrame);
      };
      video.requestVideoFrameCallback(onFrame);
    }
    this.stopTicks = ticker(frameIntervalMs, () => {
      if (performance.now() - this.lastAt >= 2 * this.frameIntervalMs) this.take();
    });
  }

  private take(): void {
    if (this.video.readyState < HTMLMediaElement.HAVE_CURRENT_DATA || !this.video.videoWidth) {
      return;
    }
    let frame: VideoFrame;
    try {
      frame = new VideoFrame(this.picture(), { timestamp: nowUs() });
    } catch {
      return;
    }
    this.lastAt = performance.now();
    const w = this.waiting;
    this.waiting = null;
    if (w) {
      w(frame);
      return;
    }
    this.queued?.close();
    this.queued = frame;
  }

  // The element's picture, drawn upright on the canvas for an upright source:
  // a frame made from the element itself keeps the camera's own orientation.
  private picture(): HTMLVideoElement | HTMLCanvasElement {
    const c = this.canvas;
    if (!c) return this.video;
    const { videoWidth: w, videoHeight: h } = this.video;
    if (c.width !== w || c.height !== h) {
      c.width = w;
      c.height = h;
    }
    c.getContext("2d").drawImage(this.video, 0, 0, w, h);
    return c;
  }

  public next(): Promise<VideoFrame | null> {
    if (this.closed) return Promise.resolve(null);
    const q = this.queued;
    this.queued = null;
    if (q) return Promise.resolve(q);
    return new Promise((resolve) => (this.waiting = resolve));
  }

  public close(): void {
    if (this.closed) return;
    this.closed = true;
    this.stopTicks();
    this.queued?.close();
    this.queued = null;
    this.waiting?.(null);
    this.waiting = null;
    this.video.srcObject = null;
    this.video.remove();
    if (this.canvas) this.track.stop();
  }
}

// The frames of a video track, from the best source the engine has. The
// source reads `track` until it closes; `createWorker` makes a frames worker,
// for engines whose MediaStreamTrackProcessor is worker-only, which can turn
// the frames (TurningSource).
export const openFrameSource = async (
  track: MediaStreamTrack,
  frameRate: number,
  createWorker?: () => Worker
): Promise<FrameSource> => {
  if (typeof MediaStreamTrackProcessor !== "undefined") {
    return new ProcessorFrames(track);
  }
  if (createWorker) {
    try {
      // A transferred track leaves the window, so the worker takes a clone.
      const clone = track.clone();
      const source = await WorkerFrames.open(createWorker(), clone);
      if (source) return source;
      clone.stop();
    } catch {
      // No worker.
    }
  }
  return new ElementFrames(track, 1000 / frameRate);
};

// Whether a track's frames come turned from the way the track's settings and
// its <video> show them: they carry a rotation, or they are landscape where
// the settings are portrait or the other way round. A camera held in
// portrait can give frames of the sensor's landscape with no rotation set.
export const framesTurned = (
  frame: { displayWidth: number; displayHeight: number; rotation?: number },
  settings: { width?: number; height?: number }
): boolean => {
  if (frame.rotation && frame.rotation % 360 !== 0) return true;
  const { width, height } = settings;
  if (!width || !height || width === height || frame.displayWidth === frame.displayHeight) {
    return false;
  }
  return width > height !== frame.displayWidth > frame.displayHeight;
};

// Whether `source` can turn its frames.
export const turning = (source: FrameSource): source is TurningSource =>
  source instanceof WorkerFrames && !source.cannotTurn;

// Frames of `track` drawn upright from a <video> element, which presents
// them as the track's settings have them. The source stops `track` when it
// closes: pass a clone.
export const openUprightFrameSource = (track: MediaStreamTrack, frameRate: number): FrameSource =>
  new ElementFrames(track, 1000 / frameRate, true);

// Makes the frames worker, built with the app.
export const createFramesWorker = (): Worker =>
  new Worker(
    /* webpackChunkName: "capture-frames" */ new URL("./frames.worker.ts", import.meta.url)
  );

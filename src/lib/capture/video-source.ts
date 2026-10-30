// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Where a capture's video frames come from: MediaStreamTrackProcessor in
// the window, or in a worker where the engine has it only there, and a
// <video> element elsewhere.

import { nowUs } from "./clock";

/** A video track's frames. */
export interface FrameSource {
  readonly kind: "processor" | "worker" | "element";
  // The next frame, which the caller closes, or null once the track ended or
  // the source closed.
  next(): Promise<VideoFrame | null>;
  close(): void;
}

export type FramesRequest =
  | { type: "open"; track: MediaStreamTrack }
  | { type: "next" }
  | { type: "close" };

export type FramesResponse =
  | { type: "opened" }
  | { type: "frame"; frame: VideoFrame | null }
  | { type: "error"; message: string };

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

class WorkerFrames implements FrameSource {
  public readonly kind = "worker";
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

  // Opens the source on `track`, which the worker takes.
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

  public async next(): Promise<VideoFrame | null> {
    if (this.closed) return null;
    const m = await this.request({ type: "next" });
    if (m.type !== "frame" || this.closed) {
      if (m.type === "frame") m.frame?.close();
      return null;
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
    w?.({ type: "frame", frame: null });
  }
}

/** Frames taken from a <video> element playing the track, as each is
 * presented, stamped with the capture's clock. Hidden pages present no
 * frames, so a worker's ticks take one while none comes.
 */
class ElementFrames implements FrameSource {
  public readonly kind = "element";
  private video: HTMLVideoElement;
  private queued: VideoFrame | null = null;
  private waiting: ((f: VideoFrame | null) => void) | null = null;
  private closed = false;
  private lastAt = 0;
  private stopTicks: () => void;

  public constructor(track: MediaStreamTrack, private readonly frameIntervalMs: number) {
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
      const onFrame = () => {
        if (this.closed) return;
        this.take();
        video.requestVideoFrameCallback(onFrame);
      };
      video.requestVideoFrameCallback(onFrame);
    }
    this.stopTicks = ticker(frameIntervalMs, () => {
      if (performance.now() - this.lastAt >= 2 * this.frameIntervalMs) this.take();
    });
  }

  private take(): void {
    if (
      this.video.readyState < HTMLMediaElement.HAVE_CURRENT_DATA ||
      !this.video.videoWidth ||
      performance.now() - this.lastAt < this.frameIntervalMs / 2
    ) {
      return;
    }
    let frame: VideoFrame;
    try {
      frame = new VideoFrame(this.video, { timestamp: nowUs() });
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
  }
}

// The frames of a video track, from the best source the engine has. The
// source reads `track` until it closes; `createWorker` makes a frames worker,
// for engines whose MediaStreamTrackProcessor is worker-only.
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

// Makes the frames worker, built with the app.
export const createFramesWorker = (): Worker =>
  new Worker(
    /* webpackChunkName: "capture-frames" */ new URL("./frames.worker.ts", import.meta.url)
  );

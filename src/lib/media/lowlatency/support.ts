// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Renderer } from "./protocol";

interface Scope {
  VideoDecoder?: unknown;
  AudioDecoder?: unknown;
  EncodedVideoChunk?: unknown;
  AudioWorkletNode?: unknown;
  Worker?: unknown;
  MediaStreamTrackGenerator?: unknown;
  HTMLCanvasElement?: { prototype: object };
}

// Whether the window can run the low-latency player: WebCodecs decoders
// (in its workers too, on every engine that has them in the window), an
// AudioWorklet, and a way to show pictures (a MediaStreamTrackGenerator
// or a canvas it can hand to a worker). Firefox before 130 has no
// WebCodecs.
export const lowLatencySupport = (scope: Scope = globalThis): boolean =>
  typeof scope.VideoDecoder === "function" &&
  typeof scope.AudioDecoder === "function" &&
  typeof scope.EncodedVideoChunk === "function" &&
  typeof scope.AudioWorkletNode === "function" &&
  typeof scope.Worker === "function" &&
  (typeof scope.MediaStreamTrackGenerator === "function" ||
    "transferControlToOffscreen" in (scope.HTMLCanvasElement?.prototype ?? {}));

/** What the window and its player worker have to show pictures with. */
export interface RendererCaps {
  /** A VideoTrackGenerator in workers (Safari). */
  videoTrackGenerator: boolean;
  /** A MediaStreamTrackGenerator in the window (Chromium). */
  mediaStreamTrackGenerator: boolean;
  /** A canvas handed to a worker can still feed a video (not Firefox). */
  offscreenCaptureStream: boolean;
}

// The renderer a player uses by default: a VideoTrackGenerator where
// workers have one; else the window's MediaStreamTrackGenerator; else a
// window canvas whose stream the video plays. Each gives the player a
// native video.
export const defaultRenderer = (caps: RendererCaps): Renderer => {
  if (caps.videoTrackGenerator) return Renderer.VideoTrackGenerator;
  if (caps.mediaStreamTrackGenerator) return Renderer.MediaStreamTrackGenerator;
  return Renderer.Canvas;
};

let offscreenStream: boolean | null = null;

// Whether a canvas handed to a worker still gives a stream: tried once, on a canvas of its own.
export const offscreenCaptureStream = (): boolean => {
  if (offscreenStream !== null) return offscreenStream;
  try {
    const c = document.createElement("canvas");
    c.transferControlToOffscreen();
    const stream = c.captureStream();
    stream.getTracks().forEach((t) => t.stop());
    offscreenStream = true;
  } catch {
    offscreenStream = false;
  }
  return offscreenStream;
};

// The renderer a `renderer` query parameter names, for trying them.
export const rendererParam = (search: string): Renderer | null => {
  const r = new URLSearchParams(search).get("renderer");
  return [
    Renderer.VideoTrackGenerator,
    Renderer.MediaStreamTrackGenerator,
    Renderer.Canvas,
    Renderer.Offscreen,
  ].includes(r as Renderer)
    ? (r as Renderer)
    : null;
};

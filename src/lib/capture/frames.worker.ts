// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Reads a video track's frames where MediaStreamTrackProcessor exists only
// in workers: the window transfers the track in, then asks for each frame,
// which comes back transferred, or null once the track ends. Asked to turn
// them, it draws each frame turned on a canvas of its own and sends that,
// keeping the frame's timestamp; without an OffscreenCanvas it sends them
// as they come, and says so by their turns.

import { turnedDraw } from "./turn";
import type { FramesRequest, FramesResponse } from "./video-source";

declare let self: DedicatedWorkerGlobalScope;

let reader: ReadableStreamDefaultReader<VideoFrame> | null = null;
/** Quarter turns clockwise each frame is turned by. */
let turns = 0;
let canvas: OffscreenCanvas | null = null;
let context: OffscreenCanvasRenderingContext2D | null = null;

const reply = (m: FramesResponse, transfer: Transferable[] = []) => self.postMessage(m, transfer);

// `frame` turned by `turns`, which closes it; as it is when unturned or the
// worker has no canvas to turn it on.
const turned = (frame: VideoFrame): { frame: VideoFrame; turns: number } => {
  if (!turns || typeof OffscreenCanvas === "undefined") return { frame, turns: 0 };
  const { size, transform } = turnedDraw(frame.displayWidth, frame.displayHeight, turns);
  if (!canvas || canvas.width !== size[0] || canvas.height !== size[1]) {
    canvas = new OffscreenCanvas(size[0], size[1]);
    context = canvas.getContext("2d");
  }
  if (!context) return { frame, turns: 0 };
  context.setTransform(...transform);
  context.drawImage(frame, 0, 0, frame.displayWidth, frame.displayHeight);
  const out = new VideoFrame(canvas, {
    timestamp: frame.timestamp,
    duration: frame.duration ?? undefined,
  });
  frame.close();
  return { frame: out, turns };
};

self.onmessage = ({ data }: MessageEvent<FramesRequest>) => {
  switch (data.type) {
    case "open":
      try {
        const processor = new MediaStreamTrackProcessor({ track: data.track });
        reader = (processor.readable as ReadableStream<VideoFrame>).getReader();
        reply({ type: "opened" });
      } catch (e) {
        reply({ type: "error", message: String(e) });
      }
      break;
    case "next":
      reader?.read().then(
        ({ value, done }) => {
          if (done) {
            reply({ type: "frame", frame: null, turns });
            return;
          }
          let out: { frame: VideoFrame; turns: number };
          try {
            out = turned(value);
          } catch {
            out = { frame: value, turns: 0 };
          }
          reply({ type: "frame", ...out }, [out.frame]);
        },
        (e) => reply({ type: "error", message: String(e) })
      );
      break;
    case "turn":
      turns = ((data.quarterTurns % 4) + 4) % 4;
      break;
    case "close":
      void reader?.cancel().catch(() => undefined);
      reader = null;
      break;
  }
};

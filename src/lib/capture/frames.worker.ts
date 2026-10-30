// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Reads a video track's frames where MediaStreamTrackProcessor exists only
// in workers: the window transfers the track in, then asks for each frame,
// which comes back transferred, or null once the track ends.

import type { FramesRequest, FramesResponse } from "./video-source";

declare let self: DedicatedWorkerGlobalScope;

let reader: ReadableStreamDefaultReader<VideoFrame> | null = null;

const reply = (m: FramesResponse, transfer: Transferable[] = []) => self.postMessage(m, transfer);

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
        ({ value, done }) =>
          done
            ? reply({ type: "frame", frame: null })
            : reply({ type: "frame", frame: value }, [value]),
        (e) => reply({ type: "error", message: String(e) })
      );
      break;
    case "close":
      void reader?.cancel().catch(() => undefined);
      reader = null;
      break;
  }
};

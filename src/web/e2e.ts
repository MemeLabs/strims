// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The page for the browser link tests and the testbed's browser nodes. It
// starts ?count= e2e nodes (one by default) and relays node i's frontend
// connection to the WebSocket at /control?i=<i> on the page's host, as
// binary messages both ways. ?logLevel= sets the nodes' zap level.
//
// A testbed node reading a media stream sends the page its Data messages
// (MEDIA_BYTES, then flags, then the bytes), which play on a video through
// the baseline player and aren't relayed. Each second the page sends the
// harness a `player` event: the player's stats as window.__strims_players__
// has them, the video's events since the last one, and when the first
// data, `playing` and rendered frame came.

import { UnitsPlayer } from "../lib/media/units";
import { WSReadWriter } from "../lib/ws";
import { startWasmNode } from "./wasm";

const params = new URLSearchParams(location.search);
const count = Math.max(1, parseInt(params.get("count") ?? "1") >> 0);
const logLevel = parseInt(params.get("logLevel") ?? "0") >> 0;
const scheme = location.protocol === "https:" ? "wss:" : "ws:";

/** The first byte of a Data message from the node (strims-wasm's e2e). */
const MEDIA_BYTES = 0x02;
const MEDIA_SEGMENT_END = 0x01;
const MEDIA_DISCONTINUITY = 0x02;
/** Milliseconds between `player` events. */
const PLAYER_EVENT_MS = 1000;
/** The video events a `player` event lists. */
const VIDEO_EVENTS = [
  "waiting",
  "playing",
  "seeking",
  "seeked",
  "stalled",
  "pause",
  "ratechange",
  "error",
];

interface PlayerRegistry {
  __strims_players__?: UnitsPlayer[];
}

interface VideoEvent {
  type: string;
  t: number;
  currentTime: number;
  readyState: number;
  /** Seconds buffered ahead of the playback position. */
  ahead: number;
  rate: number;
}

// Seconds buffered ahead of the video's playback position.
const bufferedAhead = (v: HTMLVideoElement): number => {
  const t = v.currentTime;
  for (let i = 0; i < v.buffered.length; i++) {
    if (v.buffered.start(i) <= t && t < v.buffered.end(i)) {
      return v.buffered.end(i) - t;
    }
  }
  return 0;
};

// A media stream's viewer: its Data messages play on a video of its own.
class Viewer {
  private readonly video: HTMLVideoElement;
  private readonly player: UnitsPlayer;
  private events: VideoEvent[] = [];
  private readonly first: {
    dataMs: number;
    playingMs: number | null;
    frameMs: number | null;
  };
  private frames = 0;

  constructor(private readonly ws: WSReadWriter) {
    this.first = { dataMs: Date.now(), playingMs: null, frameMs: null };
    const video = document.createElement("video");
    video.width = 640;
    video.height = 360;
    document.body.appendChild(video);
    for (const type of VIDEO_EVENTS) {
      video.addEventListener(type, () => this.event(type));
    }
    const onFrame = () => {
      this.frames++;
      this.first.frameMs ??= Date.now();
      video.requestVideoFrameCallback(onFrame);
    };
    video.requestVideoFrameCallback(onFrame);
    this.video = video;
    this.player = new UnitsPlayer(video);
    setInterval(() => this.post(), PLAYER_EVENT_MS);
  }

  push(message: Uint8Array): void {
    const flags = message[1] ?? 0;
    this.player.push(
      message.subarray(2),
      (flags & MEDIA_SEGMENT_END) !== 0,
      (flags & MEDIA_DISCONTINUITY) !== 0
    );
  }

  private event(type: string): void {
    const v = this.video;
    if (type === "playing") this.first.playingMs ??= Date.now();
    this.events.push({
      type,
      t: Date.now(),
      currentTime: v.currentTime,
      readyState: v.readyState,
      ahead: bufferedAhead(v),
      rate: v.playbackRate,
    });
  }

  private post(): void {
    const players = (window as PlayerRegistry).__strims_players__ ?? [];
    const player = players.find((p) => p === this.player);
    const line = {
      type: "player",
      postMs: Date.now(),
      stats: player?.stats() ?? null,
      readyState: this.video.readyState,
      ahead: bufferedAhead(this.video),
      paused: this.video.paused,
      frames: this.frames,
      first: this.first,
      events: this.events,
    };
    this.events = [];
    this.ws.write(new TextEncoder().encode(JSON.stringify(line)));
  }
}

for (let i = 0; i < count; i++) {
  const node = startWasmNode({ logLevel, allowInsecureWs: true, iceServers: [], e2e: true });
  const ws = new WSReadWriter(`${scheme}//${location.host}/control?i=${i}`);
  let viewer: Viewer | null = null;

  node.bus.on("data", (d: Uint8Array) => {
    if (d[0] === MEDIA_BYTES) {
      viewer ??= new Viewer(ws);
      viewer.push(d);
      return;
    }
    ws.write(d);
  });
  node.bus.on("close", () => ws.close());
  ws.on("data", (d: Uint8Array) => node.bus.write(d));
  ws.on("close", () => node.close());
}

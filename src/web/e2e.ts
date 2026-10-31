// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The page for the browser link tests and the testbed's browser nodes. It
// starts ?count= e2e nodes (one by default) and relays node i's frontend
// connection to the WebSocket at /control?i=<i> on the page's host, as
// binary messages both ways. ?logLevel= sets the nodes' zap level.
//
// A testbed node that starts reading a media stream tells the page so
// (MEDIA_OPEN, which isn't relayed): the stream then plays on a video
// through the baseline player, whose worker the node feeds on the feeds
// port as an app page's node does (crates/strims-wasm's e2e module). Each
// second the page sends the harness a `player` event: the player's stats
// as window.__strims_players__ has them, the video's events since the last
// one, and when the stream started (`dataMs`), and the first `playing` and
// rendered frame came.

import { Feed } from "../lib/media/feed";
import { UNITS_MIME_TYPE } from "../lib/media/mime";
import { UnitsPlayer } from "../lib/media/units";
import { WSReadWriter } from "../lib/ws";
import { startWasmNode } from "./wasm";

const params = new URLSearchParams(location.search);
const count = Math.max(1, parseInt(params.get("count") ?? "1") >> 0);
const logLevel = parseInt(params.get("logLevel") ?? "0") >> 0;
const scheme = location.protocol === "https:" ? "wss:" : "ws:";

/** The node's message that a media stream starts (strims-wasm's e2e). */
const MEDIA_OPEN = 0x02;
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

// A media stream's viewer: its stream plays on a video of its own, from the
// feeds `feed` opens.
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

  constructor(
    private readonly ws: WSReadWriter,
    feed: (port: MessagePort, replaces: Feed | null) => Feed
  ) {
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
    this.player = new UnitsPlayer(video, { feed });
    setInterval(() => this.post(), PLAYER_EVENT_MS);
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

  // The node's own stream: the request's fields name nothing it reads.
  const request = { swarmUri: "", networkKeys: [], mimeType: UNITS_MIME_TYPE };
  // eslint-disable-next-line no-console
  const closed = (message: string) => console.warn("the stream's feed ended:", message);
  node.bus.on("data", (d: Uint8Array) => {
    if (d[0] === MEDIA_OPEN && d.byteLength === 1) {
      viewer ??= new Viewer(ws, (port, replaces) =>
        node.feeds.open(request, port, { replaces, closed })
      );
      return;
    }
    ws.write(d);
  });
  node.bus.on("close", () => ws.close());
  ws.on("data", (d: Uint8Array) => node.bus.write(d));
  ws.on("close", () => node.close());
}

// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { BatchWriter, Credit, DataMessage, Feed, MAX_BATCH_BYTES } from "./feed";
import { LiveEdge, LiveStats } from "./live";
import { CoreOutput, SinkMode, UnitInfo, WindowMessage, WorkerMessage } from "./protocol";
import { timeRanges } from "./ranges";
import { MediaSink, MediaSourceConstructor } from "./sink";

/** Milliseconds between the player's looks at the video. */
export const TICK_MS = 200;
/** Milliseconds between the playback positions the sink evicts behind. */
export const POSITION_MS = 1000;
/** Restarts within RESTART_WINDOW_MS after which the player gives up. */
export const MAX_RESTARTS = 3;
export const RESTART_WINDOW_MS = 60_000;
/** Seconds of publish times kept for the playback position's. */
const PUBLISHED_WINDOW = 60;
/**
 * The most bytes and batches the window posts the worker unacked. Past it,
 * Data messages are dropped to the next whole record, which goes with a
 * discontinuity. It covers a join's rewound GOP while the worker loads.
 */
export const WINDOW_BYTES = 32 * 1024 * 1024;
export const WINDOW_BATCHES = 1024;

export interface PlayerHandlers {
  /**
   * Points the video at the stream's media: a worker MediaSource's handle
   * or a window MediaSource's object URL. Sets the video's srcObject or src
   * when not given.
   */
  attach?: (src: MediaSourceHandle | string) => void;
  /** Starts the video. Calls play() when not given. */
  play?: () => Promise<void> | void;
  /** The player gave up: the stream can't play here. */
  error?: (message: string) => void;
  /**
   * Opens the stream's feed to a player worker on `port`, straight from
   * the node's swarm, taking over from `replaces`, the player's last feed.
   * With it, the player opens a feed for each worker and closes the last
   * one when it's done; without it, the owner pushes the stream's Data
   * messages.
   */
  feed?: (port: MessagePort, replaces: Feed | null) => Feed;
}

export interface PlayerStats extends LiveStats {
  mode: SinkMode;
  restarts: number;
  messages: number;
  bytes: number;
  gaps: number;
  discontinuities: number;
  timelineRestarts: number;
  /** Milliseconds from publishing to playing, on the writer's and this clock. */
  publishLag: number | null;
  currentTime: number;
  buffered: [number, number][];
  playbackRate: number;
  droppedFrames: number | null;
  totalFrames: number | null;
  /** Data messages dropped for the worker falling behind. */
  dropped: number;
  /** The stream ended, and plays out to its end. */
  ended: boolean;
  error: string | null;
}

/** A registry of the players on the page, for the debugger. */
interface PlayerRegistry {
  __strims_players__?: UnitsPlayer[];
}

// Where the stream's MediaSource can live: in the player's worker where
// the browser allows it, else in the window; null without one.
export const chooseSinkMode = (scope: typeof globalThis = globalThis): SinkMode | null => {
  if (
    scope.ManagedMediaSource?.canConstructInDedicatedWorker ||
    scope.MediaSource?.canConstructInDedicatedWorker
  ) {
    return SinkMode.Worker;
  }
  if (scope.ManagedMediaSource || scope.MediaSource) {
    return SinkMode.Window;
  }
  return null;
};

// Attaches a worker MediaSource's handle to a video, which the DOM typings'
// MediaProvider doesn't name.
export const attachHandle = (video: HTMLMediaElement, handle: MediaSourceHandle): void => {
  Reflect.set(video, "srcObject", handle);
};

const defaultWorker = () =>
  new Worker(
    /* webpackChunkName: "player" */ new URL("../../web/player.worker.ts", import.meta.url),
    {
      name: "player",
    }
  );

/**
 * Plays a unit-format stream on a video: its Egress Data messages go to a
 * player worker, whose remuxer turns them into fMP4 for a MediaSource in
 * the worker, or in the window where workers have none (Firefox). A
 * LiveEdge holds playback at its target lag.
 *
 * The messages come straight from the node's swarm, on a feed of the
 * worker's own (`handlers.feed`), or from the owner's `push`, which the
 * window batches per task and keeps within WINDOW_BYTES and WINDOW_BATCHES
 * unacked by dropping to the next whole record.
 *
 * A failed worker, sink or video is replaced by a new worker and
 * MediaSource, at most MAX_RESTARTS times in RESTART_WINDOW_MS: fed by a
 * new feed, which joins at the stream's current GOP, or from the next
 * whole record pushed.
 *
 * At the stream's end, a writer session's END record in the stream or the
 * end `end` says, the worker writes out what the remuxer holds, the
 * MediaSource's stream ends, and the video plays out to its end at rate 1.
 * Media the worker gives after that, a later session's, goes on playing; a
 * video that played out to the end, and stopped there, resumes there once
 * the later session's media is the target lag past it.
 */
export class UnitsPlayer {
  private readonly mode: SinkMode;
  private worker: Worker | null = null;
  private sink: MediaSink | null = null;
  private url: string | null = null;
  private live = new LiveEdge();
  private timer: ReturnType<typeof setInterval>;
  private lastPosition = 0;
  /** Data messages are dropped until a whole record starts. */
  private skipping = false;
  /** The next Data message starts a whole record. */
  private atRecordStart = true;
  private restartTimes: number[] = [];
  private destroyed = false;
  private failure: string | null = null;
  /** Each track's furthest presentation time arrived, for the playable frontier. */
  private frontiers = new Map<number, number>();
  /** Recent units' presentation and publish times, video's when the stream has video. */
  private published: { pts: number; published: number }[] = [];
  private hasVideo = false;
  private waiting = false;
  private lastCurrentTime = -1;
  /** The video's error a restart answered, which lasts until its new media loads. */
  private handledError: MediaError | null = null;
  /** The worker's feed, while there is one. */
  private feed: Feed | null = null;
  /** The pushed messages for the worker's next batch, and its unacked ones. */
  private batch = new BatchWriter();
  private batchQueued = false;
  private credit = new Credit(WINDOW_BYTES, WINDOW_BATCHES);
  /** The next pushed message goes with a discontinuity: those before it were dropped. */
  private dropDiscontinuity = false;
  private endSent = false;
  private ended = false;
  /** The video played out to the end of an ended stream. */
  private playedOut = false;
  /** Where a video that played out resumes, once a later session's media is past it. */
  private resumeAt: number | null = null;
  private counts = {
    restarts: 0,
    messages: 0,
    bytes: 0,
    gaps: 0,
    discontinuities: 0,
    timelineRestarts: 0,
    dropped: 0,
  };

  constructor(
    private readonly video: HTMLVideoElement,
    private readonly handlers: PlayerHandlers = {},
    private readonly createWorker: () => Worker = defaultWorker
  ) {
    this.mode = chooseSinkMode();
    video.disableRemotePlayback = true;
    video.addEventListener("waiting", this.onWaiting);
    video.addEventListener("playing", this.onPlaying);
    video.addEventListener("seeked", this.onPlaying);
    video.addEventListener("ended", this.onEnded);
    const players = ((window as PlayerRegistry).__strims_players__ ??= []);
    players.push(this);
    if (this.mode === null) {
      this.fail("this browser has no MediaSource");
      return;
    }
    this.start();
    this.timer = setInterval(this.tick, TICK_MS);
  }

  // Takes an Egress Data message, when the player has no feed.
  push(data: Uint8Array, segmentEnd: boolean, discontinuity: boolean): void {
    if (this.destroyed || this.failure || this.endSent || this.handlers.feed) return;
    this.counts.messages++;
    this.counts.bytes += data.byteLength;
    if (discontinuity) {
      this.skipping = false;
      this.counts.discontinuities++;
      this.relearn();
    }
    const skip = this.skipping;
    this.skipping &&= !segmentEnd;
    this.atRecordStart = segmentEnd;
    if (skip) {
      // The record in progress began before this worker, or was dropped.
      return;
    }
    if (!this.credit.open) {
      // The worker fell behind: the record goes, and the next whole one
      // tells the worker so.
      this.counts.dropped++;
      this.skipping = !segmentEnd;
      this.dropDiscontinuity = true;
      return;
    }
    const message: DataMessage = {
      data: data as Uint8Array<ArrayBuffer>,
      segmentEnd,
      discontinuity: discontinuity || this.dropDiscontinuity,
    };
    this.dropDiscontinuity = false;
    this.batch.add(message);
    if (this.batch.size >= MAX_BATCH_BYTES) {
      this.flush();
    } else if (!this.batchQueued) {
      this.batchQueued = true;
      queueMicrotask(this.flush);
    }
  }

  /**
   * The stream ended: the worker writes out what it holds, after what was
   * pushed, and the video plays out to its end.
   */
  end(): void {
    if (this.destroyed || this.failure || this.endSent) return;
    this.endSent = true;
    if (this.handlers.feed) return;
    this.flush();
    const message: WindowMessage = { type: "end" };
    this.worker?.postMessage(message);
  }

  stats(): PlayerStats {
    const q = this.video.getVideoPlaybackQuality?.();
    return {
      ...this.live.stats(),
      ...this.counts,
      mode: this.mode,
      publishLag: this.publishLag(),
      currentTime: this.video.currentTime,
      buffered: timeRanges(this.video.buffered),
      playbackRate: this.video.playbackRate,
      droppedFrames: q?.droppedVideoFrames ?? null,
      totalFrames: q?.totalVideoFrames ?? null,
      ended: this.ended,
      error: this.failure,
    };
  }

  destroy(): void {
    if (this.destroyed) return;
    this.destroyed = true;
    clearInterval(this.timer);
    this.stop();
    this.closeFeed();
    this.video.removeEventListener("waiting", this.onWaiting);
    this.video.removeEventListener("playing", this.onPlaying);
    this.video.removeEventListener("seeked", this.onPlaying);
    this.video.removeEventListener("ended", this.onEnded);
    this.video.playbackRate = 1;
    if (!this.handlers.attach) {
      this.video.srcObject = null;
      this.video.removeAttribute("src");
      this.video.load();
    }
    const players = (window as PlayerRegistry).__strims_players__ ?? [];
    if (players.includes(this)) players.splice(players.indexOf(this), 1);
  }

  private start(): void {
    this.worker = this.createWorker();
    this.worker.onmessage = this.onMessage;
    this.worker.onerror = (e) => this.restart(`the player worker failed: ${e.message}`);
    if (this.mode === SinkMode.Window) {
      const MediaSourceType = (globalThis.ManagedMediaSource ??
        globalThis.MediaSource) as MediaSourceConstructor;
      this.sink = new MediaSink(MediaSourceType, {
        attach: (ms) => {
          const url = URL.createObjectURL(ms);
          this.attach(url);
          if (this.url) URL.revokeObjectURL(this.url);
          this.url = url;
        },
        error: (e) => this.restart(e),
      });
    }
    this.credit = new Credit(WINDOW_BYTES, WINDOW_BATCHES);
    this.batch = new BatchWriter();
    this.dropDiscontinuity = false;
    let feed: MessagePort | undefined;
    if (this.handlers.feed) {
      // The new feed takes over from the last one once it holds the
      // stream, so the swarm stays open between them.
      const channel = new MessageChannel();
      this.feed = this.handlers.feed(channel.port1, this.feed);
      feed = channel.port2;
    }
    const open: WindowMessage = {
      type: "open",
      mode: this.mode,
      timeOrigin: performance.timeOrigin,
      feed,
    };
    this.worker.postMessage(open, feed ? [feed] : []);
    if (this.endSent && !feed) {
      const end: WindowMessage = { type: "end" };
      this.worker.postMessage(end);
    }
  }

  private closeFeed(): void {
    this.feed?.close();
    this.feed = null;
  }

  // Posts the pushed messages to the worker as one batch.
  private flush = () => {
    this.batchQueued = false;
    if (this.batch.empty || !this.worker) return;
    const ask = this.credit.post(this.batch.size);
    const batch = this.batch.take(ask);
    const message: WindowMessage = { type: "data", batch, at: performance.now() / 1000 };
    this.worker.postMessage(message, [batch]);
  };

  private stop(): void {
    this.worker?.terminate();
    this.worker = null;
    this.sink?.close();
    this.sink = null;
    if (this.url) {
      URL.revokeObjectURL(this.url);
      this.url = null;
    }
  }

  private restart(reason: string): void {
    if (this.destroyed || this.failure || this.ended) return;
    const now = performance.now();
    this.restartTimes = this.restartTimes.filter((t) => t > now - RESTART_WINDOW_MS);
    if (this.restartTimes.length >= MAX_RESTARTS) {
      this.fail(reason);
      return;
    }
    this.restartTimes.push(now);
    this.counts.restarts++;
    this.stop();
    this.skipping = !this.atRecordStart;
    this.rejoin();
    this.start();
  }

  private fail(message: string): void {
    this.failure = message;
    this.stop();
    this.closeFeed();
    clearInterval(this.timer);
    this.handlers.error?.(message);
  }

  private attach(src: MediaSourceHandle | string): void {
    this.rejoin();
    if (this.handlers.attach) {
      this.handlers.attach(src);
    } else if (typeof src === "string") {
      this.video.srcObject = null;
      this.video.src = src;
    } else {
      attachHandle(this.video, src);
    }
  }

  private relearn(): void {
    this.live.relearn();
    this.frontiers.clear();
    this.published = [];
  }

  private rejoin(): void {
    this.live.rejoin();
    this.frontiers.clear();
    this.published = [];
    this.resumeAt = null;
  }

  private onMessage = ({ data: message }: MessageEvent<WorkerMessage>) => {
    switch (message.type) {
      case "attach":
        this.attach(message.handle);
        return;
      case "outputs":
        if (this.handlers.feed) {
          this.counts.messages = message.messages;
          this.counts.bytes = message.bytes;
        }
        for (const out of message.outputs) {
          this.output(out, message.at);
        }
        return;
      case "consumed":
        if (this.handlers.feed) {
          const ack: WindowMessage = { type: "ack", batches: message.batches };
          this.worker?.postMessage(ack);
        } else {
          this.credit.ack(message.batches);
        }
        return;
      case "error":
        if (message.final) {
          this.fail(message.message);
        } else {
          this.restart(message.message);
        }
        return;
    }
  };

  private output(out: CoreOutput, at: number): void {
    switch (out.type) {
      case "init":
        this.hasVideo = out.tracks.some((t) => t.kind === "video");
        this.sink?.init(out.data, out.mime, out.compatible);
        return;
      case "media": {
        // Media after the end, a later session's: the stream goes on, and
        // a video that played out to the end and stopped there resumes.
        this.ended = false;
        if (this.playedOut) {
          this.playedOut = false;
          this.resumeAt = this.video.currentTime;
        }
        this.sink?.media(out.data);
        const keep: UnitInfo["kind"] = this.hasVideo ? "video" : "audio";
        for (const u of out.units) {
          this.frontiers.set(u.track, Math.max(this.frontiers.get(u.track) ?? -Infinity, u.pts));
          if (u.kind === keep) {
            this.published.push({ pts: u.pts, published: u.published });
          }
        }
        while (
          this.published.length &&
          this.published[0].pts < out.units[0]?.pts - PUBLISHED_WINDOW
        ) {
          this.published.shift();
        }
        if (out.units.length) {
          this.live.arrive(at, Math.min(...this.frontiers.values()));
        }
        return;
      }
      case "gap":
        this.counts.gaps++;
        return;
      case "discontinuity":
        return;
      case "restart":
        this.counts.timelineRestarts++;
        this.relearn();
        return;
      case "end":
        this.ended = true;
        this.sink?.end();
        this.video.playbackRate = 1;
        return;
    }
  }

  // Milliseconds between the playing picture's publishing and now.
  private publishLag(): number | null {
    const t = this.video.currentTime;
    let unit: { pts: number; published: number } | undefined;
    for (const u of this.published) {
      if (u.pts <= t && (!unit || u.pts > unit.pts)) unit = u;
    }
    return unit ? Date.now() - (unit.published + (t - unit.pts) * 1000) : null;
  }

  private onWaiting = () => {
    this.waiting = true;
  };

  private onPlaying = () => {
    this.waiting = false;
  };

  private onEnded = () => {
    this.playedOut = this.ended;
  };

  private tick = () => {
    const v = this.video;
    if (v.error && v.error !== this.handledError) {
      this.handledError = v.error;
      this.restart(`the video failed: ${v.error.message || v.error.code}`);
      return;
    }
    const now = performance.now() / 1000;
    if (this.resumeAt !== null) {
      // A video that played out resumes where it stopped, the remuxer's
      // later session starting there, once the target lag of media is
      // past it or the stream ended again. It seeks first: playing a video
      // that ended starts it over.
      const ranges = timeRanges(v.buffered);
      const end = ranges.length ? ranges[ranges.length - 1][1] : 0;
      if (this.ended || end - this.resumeAt >= this.live.stats().target) {
        v.currentTime = this.resumeAt;
        this.resumeAt = null;
        void (this.handlers.play ? this.handlers.play() : v.play().catch(() => undefined));
      }
      this.position(now);
      return;
    }
    if (this.ended) {
      // Played out to the end: nothing more to chase.
      this.position(now);
      return;
    }
    const stuck =
      v.currentTime === this.lastCurrentTime && v.readyState < HTMLMediaElement.HAVE_FUTURE_DATA;
    this.lastCurrentTime = v.currentTime;
    const action = this.live.tick({
      now,
      currentTime: v.currentTime,
      ranges: timeRanges(v.buffered),
      paused: v.paused,
      seeking: v.seeking,
      waiting: this.waiting || stuck,
    });
    if (action.seek !== undefined) {
      v.currentTime = action.seek;
    }
    if (Math.abs(v.playbackRate - action.rate) > 0.001) {
      v.playbackRate = action.rate;
    }
    if (action.play) {
      void (this.handlers.play ? this.handlers.play() : v.play().catch(() => undefined));
    }
    this.position(now);
  };

  // Tells the sink the playback position, once a POSITION_MS.
  private position(now: number): void {
    if (now * 1000 - this.lastPosition < POSITION_MS) return;
    this.lastPosition = now * 1000;
    const { currentTime } = this.video;
    if (this.sink) {
      this.sink.position(currentTime);
    } else {
      const message: WindowMessage = { type: "position", currentTime };
      this.worker?.postMessage(message);
    }
  }
}

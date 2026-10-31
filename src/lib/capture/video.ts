// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// A capture's video: frames from its source, encoded as H.264 with
// WebCodecs.

import { TrackClock, nowUs } from "./clock";
import {
  BASELINE,
  MAX_FRAME_RATE,
  PROFILES,
  avcConfig,
  chooseConfig,
  encodeSize,
  sameBytes,
} from "./h264";
import { CaptureSink, VIDEO_TRACK } from "./sink";
import { UPRIGHT_TURNS } from "./turn";
import {
  FrameSource,
  framesTurned,
  openFrameSource,
  openUprightFrameSource,
  ticker,
  turning,
} from "./video-source";

/**
 * The longest a GOP runs: the capture asks for a keyframe once its last is
 * this old. A viewer joining mid-GOP fetches the GOP so far, at most 2 s
 * of media (1.5 MB at 6 Mbit/s), within the egress's 2 s rewind timeout
 * for viewers that download at 6 Mbit/s or more; a unit lost past repair
 * holds the picture at most this long; and HLS segments end on these
 * keyframes. At VBR, a keyframe of a still screen costs little.
 */
export const KEYFRAME_INTERVAL_US = 2_000_000;
/**
 * A still source still gets a frame this often: the held frame, encoded
 * again. Screen capture sends no frames while nothing changes, and players
 * hold a video unit until the next one times it, so without these a still
 * screen would stall their buffers and leave its GOP's keyframe to fall
 * out of the live window.
 */
export const STILL_REPEAT_MS = 250;
/** Frames between checks that the source's frames aren't turned. */
const TURN_CHECK_FRAMES = 30;
/** Frames the encoder may hold before new ones are dropped. */
const MAX_ENCODE_QUEUE = 3;
/** How long a new source size holds before the encoder takes it; frames
 * in between are scaled by the encoder.
 */
const RESIZE_SETTLE_MS = 500;

export interface VideoCaptureInfo {
  source: string;
  profile: string;
  codec: string;
  width: number;
  height: number;
  bitrate: number;
  frameRate: number;
  /** Times the encoder reordered frames and the capture fell back to
   * Constrained Baseline.
   */
  reorderFallbacks: number;
  /** Whether the source's frames came turned, and the capture has them
   * drawn upright: turned in the frames worker, or drawn from a <video>.
   */
  turned: boolean;
  framesIn: number;
  framesDropped: number;
  stillRepeats: number;
  units: number;
  keyframes: number;
}

export interface VideoCaptureOptions {
  createFramesWorker?: () => Worker;
  onerror: (e: unknown) => void;
}

// A copy of a decoder config's description. SharedArrayBuffer exists only
// in cross-origin isolated pages, so views are told apart from buffers.
export const bytesOf = (b: AllowSharedBufferSource): Uint8Array =>
  ArrayBuffer.isView(b)
    ? new Uint8Array(b.buffer, b.byteOffset, b.byteLength).slice()
    : new Uint8Array(b).slice();

export class VideoCapture {
  public readonly info: VideoCaptureInfo;
  private encoder: VideoEncoder | null = null;
  private readonly clock = new TrackClock();
  private profileIndex = 0;
  /** Profiles whose configs were seen not to reorder. */
  private readonly verified = new Set<number>();
  private size: [number, number] | null = null;
  private pending: { size: [number, number]; since: number } | null = null;
  private reconfiguring = false;
  private lastKeyUs: number | null = null;
  private forceKey = true;
  private lastOutUs = -Infinity;
  private dropUntilKey = false;
  private sentConfig: Uint8Array | null = null;
  private held: VideoFrame | null = null;
  private lastFrameAt = 0;
  private lastSourceUs = -Infinity;
  private stopped = false;
  private failed = false;
  private loop: Promise<void> = Promise.resolve();
  private stopTicks: () => void = () => undefined;

  private constructor(
    private readonly track: MediaStreamTrack,
    private source: FrameSource,
    private readonly sink: CaptureSink,
    private readonly frameRate: number,
    private readonly onerror: (e: unknown) => void
  ) {
    this.info = {
      source: source.kind,
      profile: "",
      codec: "",
      width: 0,
      height: 0,
      bitrate: 0,
      frameRate,
      reorderFallbacks: 0,
      turned: false,
      framesIn: 0,
      framesDropped: 0,
      stillRepeats: 0,
      units: 0,
      keyframes: 0,
    };
  }

  // Starts capturing `track`'s video into `sink`.
  public static async start(
    track: MediaStreamTrack,
    sink: CaptureSink,
    options: VideoCaptureOptions
  ): Promise<VideoCapture> {
    const frameRate = Math.min(track.getSettings().frameRate || MAX_FRAME_RATE, MAX_FRAME_RATE);
    const source = await openFrameSource(track, frameRate, options.createFramesWorker);
    const v = new VideoCapture(track, source, sink, frameRate, options.onerror);
    v.loop = v.run();
    v.stopTicks = ticker(50, () => v.repeatStill());
    return v;
  }

  // Asks for a keyframe next.
  public requestKeyframe(): void {
    this.forceKey = true;
  }

  private async run(): Promise<void> {
    while (!this.stopped) {
      const source = this.source;
      const frame = await source.next();
      if (!frame) {
        if (source !== this.source) continue;
        break;
      }
      try {
        await this.take(frame);
      } catch (e) {
        this.fail(e);
      } finally {
        frame.close();
      }
    }
  }

  private async take(frame: VideoFrame): Promise<void> {
    this.info.framesIn++;
    const source = this.source;
    if ("cannotTurn" in source && source.cannotTurn && source.kind !== "turned") {
      // The source couldn't turn its frames as asked.
      this.drawUpright();
      return;
    }
    if (
      source.kind !== "upright" &&
      this.info.framesIn % TURN_CHECK_FRAMES === 1 &&
      framesTurned(frame, this.track.getSettings())
    ) {
      this.turnUpright();
      return;
    }
    this.info.source = source.kind;
    this.clock.anchor(frame.timestamp);
    // A source may stamp every frame alike: a frame whose time doesn't move
    // on takes its arrival's.
    const sourceUs =
      frame.timestamp > this.lastSourceUs ? frame.timestamp : this.clock.toSource(nowUs());
    this.lastSourceUs = frame.timestamp;
    this.held?.close();
    this.held = frame.clone();
    this.lastFrameAt = performance.now();
    const size = encodeSize(frame.displayWidth, frame.displayHeight);
    if (!this.size) {
      await this.configure(size, this.profileIndex);
    } else if (size[0] !== this.size[0] || size[1] !== this.size[1]) {
      const p = this.pending;
      if (!p || p.size[0] !== size[0] || p.size[1] !== size[1]) {
        this.pending = { size, since: performance.now() };
      } else if (performance.now() - p.since >= RESIZE_SETTLE_MS) {
        await this.configure(size, this.profileIndex);
      }
    } else {
      this.pending = null;
    }
    this.encode(frame, sourceUs);
  }

  // Has the frames turned upright: the source's come turned from the track's
  // settings, and an encoder doesn't turn them. A source that turns its
  // frames turns them by a quarter, or back, when they are found turned
  // again: the camera turned with the page.
  private turnUpright(): void {
    const s = this.source;
    if (turning(s)) {
      const turns = s.turns ? 0 : UPRIGHT_TURNS;
      s.turn(turns);
      this.info.turned = turns !== 0;
      return;
    }
    this.drawUpright();
  }

  // Moves to frames drawn upright from a <video>, which shows the track as
  // its settings have it.
  private drawUpright(): void {
    const old = this.source;
    this.source = openUprightFrameSource(this.track.clone(), this.frameRate);
    this.info.source = this.source.kind;
    this.info.turned = true;
    this.clock.restart();
    this.lastSourceUs = -Infinity;
    this.held?.close();
    this.held = null;
    old.close();
  }

  // Encodes the held frame again when the source sent none for a while.
  private repeatStill(): void {
    if (
      this.stopped ||
      !this.held ||
      !this.encoder ||
      performance.now() - this.lastFrameAt < STILL_REPEAT_MS
    ) {
      return;
    }
    this.lastFrameAt = performance.now();
    this.info.stillRepeats++;
    try {
      this.encode(this.held, this.clock.toSource(nowUs()));
    } catch (e) {
      this.fail(e);
    }
  }

  // Encodes `frame` at `sourceUs` on its source's clock, unless the encoder is
  // busy.
  private encode(frame: VideoFrame, sourceUs: number): void {
    const encoder = this.encoder;
    if (
      this.reconfiguring ||
      this.failed ||
      !encoder ||
      encoder.state !== "configured" ||
      encoder.encodeQueueSize > MAX_ENCODE_QUEUE
    ) {
      this.info.framesDropped++;
      return;
    }
    const t = this.clock.output(sourceUs);
    const keyFrame =
      this.forceKey || this.lastKeyUs === null || t - this.lastKeyUs >= KEYFRAME_INTERVAL_US;
    const stamped = new VideoFrame(frame, { timestamp: t });
    try {
      encoder.encode(stamped, { keyFrame });
    } finally {
      stamped.close();
    }
    if (keyFrame) {
      this.forceKey = false;
      this.lastKeyUs = t;
    }
  }

  // (Re)configures the encoder for `size`, from profile `from` on.
  private async configure(size: [number, number], from: number): Promise<void> {
    this.reconfiguring = true;
    try {
      if (this.encoder?.state === "configured") {
        await this.encoder.flush().catch(() => undefined);
      }
      const choice = await chooseConfig(size[0], size[1], this.frameRate, from, this.verified);
      if (!choice) throw new Error(`no H.264 encoder config at ${size[0]}x${size[1]}`);
      if (!this.encoder || this.encoder.state === "closed") {
        this.encoder = new VideoEncoder({
          output: (chunk: EncodedVideoChunk, meta?: EncodedVideoChunkMetadata) =>
            this.output(chunk, meta),
          error: (e) => this.fail(e),
        });
      }
      this.encoder.configure(choice.config);
      this.profileIndex = choice.index;
      this.size = size;
      this.pending = null;
      this.forceKey = true;
      Object.assign(this.info, {
        profile: PROFILES[choice.index].name,
        codec: choice.config.codec,
        width: size[0],
        height: size[1],
        bitrate: choice.config.bitrate,
      });
    } finally {
      this.reconfiguring = false;
    }
  }

  private output(chunk: EncodedVideoChunk, meta?: EncodedVideoChunkMetadata): void {
    if (this.failed) return;
    if (chunk.timestamp <= this.lastOutUs) {
      // The encoder reorders frames: their units would need decode times.
      this.dropUntilKey = true;
      if (this.profileIndex !== BASELINE && !this.reconfiguring && this.size) {
        this.info.reorderFallbacks++;
        const size = this.size;
        this.configure(size, BASELINE).catch((e) => this.fail(e));
      }
      return;
    }
    this.lastOutUs = chunk.timestamp;
    const data = new Uint8Array(chunk.byteLength);
    chunk.copyTo(data);
    const keyframe = chunk.type === "key";
    if (keyframe) this.lastKeyUs = chunk.timestamp;
    const description = meta?.decoderConfig?.description;
    if (description) {
      const config = avcConfig(bytesOf(description), keyframe ? data : new Uint8Array());
      if (!this.sentConfig || !sameBytes(config, this.sentConfig)) {
        this.sentConfig = config;
        this.sink.push({ type: "config", track: VIDEO_TRACK, codec: "h264", config });
      }
    }
    if (this.dropUntilKey) {
      if (!keyframe) return;
      this.dropUntilKey = false;
    }
    this.info.units++;
    if (keyframe) this.info.keyframes++;
    this.sink.push({
      type: "unit",
      track: VIDEO_TRACK,
      timestampUs: chunk.timestamp,
      keyframe,
      data,
    });
  }

  private fail(e: unknown): void {
    if (this.failed) return;
    this.failed = true;
    this.onerror(e);
  }

  // Stops the source and writes out what the encoder holds.
  public async stop(): Promise<void> {
    this.stopped = true;
    this.stopTicks();
    this.source.close();
    await this.loop;
    const encoder = this.encoder;
    if (encoder?.state === "configured") await encoder.flush().catch(() => undefined);
    if (encoder && encoder.state !== "closed") encoder.close();
    this.held?.close();
    this.held = null;
  }
}

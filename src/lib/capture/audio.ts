// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// A capture's audio: samples from MediaStreamTrackProcessor in the window,
// or an AudioWorklet where the engine has none there (a worker's processor
// may take video only), encoded as Opus with WebCodecs.

import { TrackClock, nowUs } from "./clock";
import { encoderConfig, opusConfig } from "./opus";
import { AUDIO_TRACK, CaptureSink } from "./sink";
import { bytesOf } from "./video";

/** Samples per block the worklet hands over: 20 ms at 48 kHz, an Opus
 * frame, so encoders stamp each packet with its own block's time.
 */
const WORKLET_BLOCK = 960;

/** A track's audio as AudioData, on the capture's clock or the source's. */
interface SampleSource {
  readonly kind: "processor" | "worklet";
  // The next block, which the caller closes, or null once the track ended or
  // the source closed.
  next(): Promise<AudioData | null>;
  close(): void;
}

class ProcessorSamples implements SampleSource {
  public readonly kind = "processor";
  private reader: ReadableStreamDefaultReader<AudioData>;

  public constructor(track: MediaStreamTrack) {
    const processor = new MediaStreamTrackProcessor({ track });
    this.reader = (processor.readable as ReadableStream<AudioData>).getReader();
  }

  public async next(): Promise<AudioData | null> {
    let r: ReadableStreamReadResult<AudioData>;
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

/** The worklet: gathers each input channel into blocks of `frames` and
 * posts them with the context frame of their first sample.
 */
const WORKLET_SOURCE = `
registerProcessor("strims-capture", class extends AudioWorkletProcessor {
  constructor(options) {
    super();
    const { channels, frames } = options.processorOptions;
    this.channels = channels;
    this.frames = frames;
    this.buf = Array.from({ length: channels }, () => new Float32Array(frames));
    this.fill = 0;
    this.start = 0;
    this.stopped = false;
    this.port.onmessage = () => (this.stopped = true);
  }
  process(inputs) {
    if (this.stopped) return false;
    const input = inputs[0] || [];
    const n = input[0] ? input[0].length : 128;
    for (let i = 0; i < n; ) {
      if (this.fill === 0) this.start = currentFrame + i;
      const take = Math.min(n - i, this.frames - this.fill);
      for (let c = 0; c < this.channels; c++) {
        const src = input[Math.min(c, input.length - 1)];
        if (src) this.buf[c].set(src.subarray(i, i + take), this.fill);
        else this.buf[c].fill(0, this.fill, this.fill + take);
      }
      this.fill += take;
      i += take;
      if (this.fill === this.frames) {
        const data = new Float32Array(this.frames * this.channels);
        this.buf.forEach((b, c) => data.set(b, c * this.frames));
        this.port.postMessage({ frame: this.start, data }, [data.buffer]);
        this.fill = 0;
      }
    }
    return true;
  }
});
`;

interface WorkletBlock {
  frame: number;
  data: Float32Array<ArrayBuffer>;
}

/**
 * Blocks from an AudioWorklet, stamped by their context frame: the first
 * block's start maps to its arrival less its length, and the rest keep the
 * audio clock's spacing. `driftUs` follows how far arrivals have moved
 * from that mapping since.
 */
class WorkletSamples implements SampleSource {
  public readonly kind = "worklet";
  private queue: AudioData[] = [];
  private waiting: ((d: AudioData | null) => void) | null = null;
  private closed = false;
  private frame0: number | null = null;
  private at0 = 0;
  /** The least arrival delay of the blocks in the window so far, less the
   * first block's.
   */
  public driftUs = 0;
  private windowMin = Infinity;
  private windowBlocks = 0;

  private constructor(
    private readonly context: AudioContext,
    private readonly ownsContext: boolean,
    private readonly node: AudioWorkletNode,
    private readonly input: MediaStreamAudioSourceNode,
    private readonly channels: number
  ) {
    node.port.onmessage = ({ data }: MessageEvent<WorkletBlock>) => this.block(data);
  }

  public static async open(
    track: MediaStreamTrack,
    channels: number,
    context?: AudioContext
  ): Promise<WorkletSamples> {
    const ownsContext = !context;
    const ctx = context ?? new AudioContext({ sampleRate: 48_000 });
    const url = URL.createObjectURL(new Blob([WORKLET_SOURCE], { type: "text/javascript" }));
    try {
      await ctx.audioWorklet.addModule(url);
    } finally {
      URL.revokeObjectURL(url);
    }
    const node = new AudioWorkletNode(ctx, "strims-capture", {
      numberOfInputs: 1,
      numberOfOutputs: 0,
      channelCount: channels,
      channelCountMode: "explicit",
      processorOptions: { channels, frames: WORKLET_BLOCK },
    });
    const input = ctx.createMediaStreamSource(new MediaStream([track]));
    input.connect(node);
    await ctx.resume();
    return new WorkletSamples(ctx, ownsContext, node, input, channels);
  }

  private block({ frame, data }: WorkletBlock): void {
    if (this.closed) return;
    const rate = this.context.sampleRate;
    const now = nowUs();
    const length = (WORKLET_BLOCK * 1e6) / rate;
    if (this.frame0 === null) {
      this.frame0 = frame;
      this.at0 = now - length;
    }
    const t = this.at0 + ((frame - this.frame0) * 1e6) / rate;
    this.windowMin = Math.min(this.windowMin, now - length - t);
    if (++this.windowBlocks === 250) {
      this.driftUs = Math.round(this.windowMin);
      this.windowMin = Infinity;
      this.windowBlocks = 0;
    }
    const d = new AudioData({
      format: "f32-planar",
      sampleRate: rate,
      numberOfFrames: WORKLET_BLOCK,
      numberOfChannels: this.channels,
      timestamp: Math.round(t),
      data,
    });
    const w = this.waiting;
    this.waiting = null;
    if (w) w(d);
    else this.queue.push(d);
  }

  public next(): Promise<AudioData | null> {
    if (this.closed) return Promise.resolve(null);
    const d = this.queue.shift();
    if (d) return Promise.resolve(d);
    return new Promise((resolve) => (this.waiting = resolve));
  }

  public close(): void {
    if (this.closed) return;
    this.closed = true;
    this.node.port.postMessage("stop");
    this.input.disconnect();
    this.queue.forEach((d) => d.close());
    this.queue = [];
    this.waiting?.(null);
    this.waiting = null;
    if (this.ownsContext) this.context.close().catch(() => undefined);
  }
}

// The first two channels of `data`.
const stereo = (data: AudioData): AudioData => {
  const n = data.numberOfFrames;
  const out = new Float32Array(2 * n);
  for (let p = 0; p < 2; p++) {
    data.copyTo(out.subarray(p * n, (p + 1) * n), { planeIndex: p, format: "f32-planar" });
  }
  return new AudioData({
    format: "f32-planar",
    sampleRate: data.sampleRate,
    numberOfFrames: n,
    numberOfChannels: 2,
    timestamp: data.timestamp,
    data: out,
  });
};

export interface AudioCaptureInfo {
  source: string;
  sampleRate: number;
  channels: number;
  bitrate: number;
  units: number;
  /** The worklet's arrival drift from its audio clock, in microseconds. */
  driftUs: number;
  /** Whether the OpusHead came from the encoder. */
  encoderHead: boolean;
}

export interface AudioCaptureOptions {
  /** A context made in the user's gesture: an engine may run audio only in
   * one.
   */
  audioContext?: AudioContext;
  onerror: (e: unknown) => void;
}

export class AudioCapture {
  public readonly info: AudioCaptureInfo;
  private audio: AudioEncoder | null = null;
  private readonly clock = new TrackClock();
  private sentConfig = false;
  private stopped = false;
  private failed = false;
  private loop: Promise<void> = Promise.resolve();

  private constructor(
    private readonly source: SampleSource,
    private readonly sink: CaptureSink,
    private readonly onerror: (e: unknown) => void
  ) {
    this.info = {
      source: source.kind,
      sampleRate: 0,
      channels: 0,
      bitrate: 0,
      units: 0,
      driftUs: 0,
      encoderHead: false,
    };
  }

  // The channels a capture of `track` encodes.
  public static channels(track: MediaStreamTrack): number {
    return track.getSettings().channelCount === 1 ? 1 : 2;
  }

  // Starts capturing `track`'s audio into `sink`.
  public static async start(
    track: MediaStreamTrack,
    sink: CaptureSink,
    options: AudioCaptureOptions
  ): Promise<AudioCapture> {
    const source: SampleSource =
      typeof MediaStreamTrackProcessor !== "undefined"
        ? new ProcessorSamples(track)
        : await WorkletSamples.open(track, AudioCapture.channels(track), options.audioContext);
    const a = new AudioCapture(source, sink, options.onerror);
    a.loop = a.run();
    return a;
  }

  private async run(): Promise<void> {
    while (!this.stopped) {
      const data = await this.source.next();
      if (!data) break;
      try {
        this.take(data);
      } catch (e) {
        this.fail(e);
      } finally {
        data.close();
      }
      if (this.source instanceof WorkletSamples) this.info.driftUs = this.source.driftUs;
    }
  }

  private take(data: AudioData): void {
    if (this.failed) return;
    // Worklet blocks are on the capture's clock already.
    this.clock.anchor(data.timestamp, this.source.kind === "worklet" ? data.timestamp : nowUs());
    if (!this.audio) {
      const channels = Math.min(2, data.numberOfChannels);
      const config = encoderConfig(data.sampleRate, channels);
      this.audio = new AudioEncoder({
        output: (chunk: EncodedAudioChunk, meta?: EncodedAudioChunkMetadata) =>
          this.output(chunk, meta),
        error: (e) => this.fail(e),
      });
      this.audio.configure(config);
      Object.assign(this.info, {
        sampleRate: data.sampleRate,
        channels,
        bitrate: config.bitrate,
      });
    }
    if (data.numberOfChannels > 2) {
      const two = stereo(data);
      this.audio.encode(two);
      two.close();
    } else {
      this.audio.encode(data);
    }
  }

  private output(chunk: EncodedAudioChunk, meta?: EncodedAudioChunkMetadata): void {
    if (this.failed) return;
    if (!this.sentConfig) {
      const d = meta?.decoderConfig?.description;
      const description = d ? bytesOf(d) : undefined;
      const config = opusConfig(description, this.info.channels, this.info.sampleRate);
      this.info.encoderHead = config === description;
      this.sink.push({ type: "config", track: AUDIO_TRACK, codec: "opus", config });
      this.sentConfig = true;
    }
    const data = new Uint8Array(chunk.byteLength);
    chunk.copyTo(data);
    this.info.units++;
    this.sink.push({
      type: "unit",
      track: AUDIO_TRACK,
      timestampUs: this.clock.output(chunk.timestamp),
      keyframe: true,
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
    this.source.close();
    await this.loop;
    const encoder = this.audio;
    if (encoder?.state === "configured") await encoder.flush().catch(() => undefined);
    if (encoder && encoder.state !== "closed") encoder.close();
  }
}

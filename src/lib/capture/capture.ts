// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Browser capture: a media stream's video and audio encoded with WebCodecs
// as H.264 and Opus, as decoder configs and access units for the node's
// unit-format writer.

import { AudioCapture, AudioCaptureInfo } from "./audio";
import { MAX_FRAME_RATE, encodeSize, videoBitrate } from "./h264";
import { audioBitrate } from "./opus";
import { AUDIO_TRACK, CaptureItem, CaptureSink, UnitItem, VIDEO_TRACK } from "./sink";
import { VideoCapture, VideoCaptureInfo } from "./video";
import { createFramesWorker } from "./video-source";

export interface CaptureOptions {
  /** A context made in the user's gesture, which an engine may need to run
   * audio: the capture takes audio through it where the engine has no
   * MediaStreamTrackProcessor.
   */
  audioContext?: AudioContext;
  /** Makes the worker that reads video frames where the engine's
   * MediaStreamTrackProcessor is worker-only.
   */
  createFramesWorker?: () => Worker;
  /** A failed encoder or source; the capture stops writing that track. */
  onerror?: (e: unknown) => void;
}

// The nominal bitrate a capture of `stream` encodes at, in bits a second: its
// video's target at its size and its audio's. With whether it has video, it
// picks how the capture's swarm signs.
export const captureBitrate = (stream: MediaStream): number => {
  let bitrate = 0;
  const [video] = stream.getVideoTracks();
  if (video) {
    const s = video.getSettings();
    const [w, h] = encodeSize(s.width || 1280, s.height || 720);
    bitrate += videoBitrate(w, h, Math.min(s.frameRate || MAX_FRAME_RATE, MAX_FRAME_RATE));
  }
  const [audio] = stream.getAudioTracks();
  if (audio) bitrate += audioBitrate(AudioCapture.channels(audio));
  return bitrate;
};

/** Audio units VideoFirst holds: 10 s of Opus packets. */
const MAX_HELD = 500;

/**
 * Holds a capture's audio units back until its video's first keyframe,
 * then sends those from the keyframe's time on. The node starts the
 * stream's GOPs at video keyframes once it knows the video track: audio
 * that came first would start a stream of audio alone, and audio between
 * the video's config and keyframe would be dropped. A video encoder can
 * take a second or two to give its first output, while audio flows.
 */
class VideoFirst implements CaptureSink {
  private held: UnitItem[] = [];
  private started = false;

  public constructor(private readonly sink: CaptureSink) {}

  public push(item: CaptureItem): void {
    if (this.started) {
      this.sink.push(item);
      return;
    }
    if (item.type === "unit" && item.track === AUDIO_TRACK) {
      this.held.push(item);
      if (this.held.length > MAX_HELD) this.held.shift();
      return;
    }
    this.sink.push(item);
    if (item.type === "unit" && item.track === VIDEO_TRACK && item.keyframe) {
      this.started = true;
      this.held.filter((a) => a.timestampUs >= item.timestampUs).forEach((a) => this.sink.push(a));
      this.held = [];
    }
  }
}

export interface CaptureInfo {
  video?: VideoCaptureInfo;
  audio?: AudioCaptureInfo;
}

/** A running capture of a stream's first video and audio tracks. */
export class Capture {
  private constructor(
    private readonly video: VideoCapture | null,
    private readonly audio: AudioCapture | null
  ) {}

  // Starts capturing `stream` into `sink`.
  public static async start(
    stream: MediaStream,
    sink: CaptureSink,
    options: CaptureOptions = {}
  ): Promise<Capture> {
    const onerror = options.onerror ?? (() => undefined);
    const [videoTrack] = stream.getVideoTracks();
    const [audioTrack] = stream.getAudioTracks();
    if (videoTrack) sink = new VideoFirst(sink);
    const [video, audio] = await Promise.all([
      videoTrack
        ? VideoCapture.start(videoTrack, sink, {
            createFramesWorker: options.createFramesWorker ?? createFramesWorker,
            onerror,
          })
        : null,
      audioTrack
        ? AudioCapture.start(audioTrack, sink, { audioContext: options.audioContext, onerror })
        : null,
    ]);
    return new Capture(video, audio);
  }

  public get info(): CaptureInfo {
    return { video: this.video?.info, audio: this.audio?.info };
  }

  // Asks the video encoder for a keyframe next.
  public requestKeyframe(): void {
    this.video?.requestKeyframe();
  }

  // Stops the sources and writes out what the encoders hold.
  public async stop(): Promise<void> {
    await Promise.all([this.video?.stop(), this.audio?.stop()]);
  }
}

// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { CaptureConfig, ICaptureAppendRequest } from "../../apis/strims/video/v1/capture";
import { CaptureItem, CaptureSink, ConfigItem, UnitItem, VIDEO_TRACK } from "./sink";

/** Units one append carries, below the node's bound of 512. */
const MAX_UNITS = 256;
/** Bytes one append carries, well below the frontend's 10 MiB messages. */
const MAX_BYTES = 2 * 1024 * 1024;
/** The queue the node may fall behind by before queued video is dropped. */
const MAX_QUEUED_BYTES = 8 * 1024 * 1024;
const MAX_QUEUED_US = 3_000_000;

const CODECS = {
  h264: CaptureConfig.Codec.CODEC_H264,
  opus: CaptureConfig.Codec.CODEC_OPUS,
};

/**
 * Sends a capture's configs and units to its node, one append at a time:
 * whatever queues while one is in flight goes in the next, so appends grow
 * with the node's delay. An append's configs come first, so a config
 * queued after units starts the next append. When the queue holds more
 * than MAX_QUEUED_BYTES or MAX_QUEUED_US of media, its video goes, later
 * video waits for a keyframe, and `onoverflow` asks the encoder for one.
 */
export class CaptureSender implements CaptureSink {
  private queue: CaptureItem[] = [];
  private queuedBytes = 0;
  private sending: Promise<void> | null = null;
  private scheduled = false;
  private failed = false;
  private videoNeedsKey = false;
  public sentUnits = 0;
  public droppedUnits = 0;

  public constructor(
    private readonly id: Uint8Array,
    private readonly append: (req: ICaptureAppendRequest) => Promise<unknown>,
    private readonly onerror: (e: unknown) => void,
    private readonly onoverflow: () => void = () => undefined
  ) {}

  public push(item: CaptureItem): void {
    if (this.failed) return;
    if (item.type === "unit" && item.track === VIDEO_TRACK && this.videoNeedsKey) {
      if (!item.keyframe) {
        this.droppedUnits++;
        return;
      }
      this.videoNeedsKey = false;
    }
    this.queue.push(item);
    this.queuedBytes += item.type === "unit" ? item.data.length : item.config.length;
    if (this.overflowing()) this.drop();
    // What an encoder's callback pushes at once, such as a config and its
    // keyframe, goes in one append.
    if (!this.scheduled) {
      this.scheduled = true;
      queueMicrotask(() => {
        this.scheduled = false;
        this.pump();
      });
    }
  }

  // Whether the queue holds more than `share` of its bounds.
  private overflowing(share = 1): boolean {
    if (this.queuedBytes > MAX_QUEUED_BYTES * share) return true;
    const units = this.queue.filter((i): i is UnitItem => i.type === "unit");
    return (
      units.length > 1 &&
      units[units.length - 1].timestampUs - units[0].timestampUs > MAX_QUEUED_US * share
    );
  }

  // Drops the queue's video, and then its oldest audio until it holds half
  // its bounds, so the keyframe asked for has room.
  private drop(): void {
    const video = (i: CaptureItem) => i.type === "unit" && i.track === VIDEO_TRACK;
    if (this.queue.some(video)) {
      const kept = this.queue.filter((i) => !video(i));
      this.droppedUnits += this.queue.length - kept.length;
      this.queue = kept;
      this.videoNeedsKey = true;
      this.onoverflow();
    }
    const recount = () =>
      (this.queuedBytes = this.queue.reduce(
        (n, i) => n + (i.type === "unit" ? i.data.length : i.config.length),
        0
      ));
    recount();
    while (this.overflowing(0.5)) {
      const at = this.queue.findIndex((i) => i.type === "unit");
      this.queue.splice(at, 1);
      this.droppedUnits++;
      recount();
    }
  }

  private pump(): void {
    if (this.sending || this.failed || this.queue.length === 0) return;
    const configs: ConfigItem[] = [];
    const units: UnitItem[] = [];
    let bytes = 0;
    let n = 0;
    for (const item of this.queue) {
      if (item.type === "config") {
        if (units.length > 0) break;
        configs.push(item);
        bytes += item.config.length;
      } else {
        if (
          units.length === MAX_UNITS ||
          (units.length > 0 && bytes + item.data.length > MAX_BYTES)
        ) {
          break;
        }
        units.push(item);
        bytes += item.data.length;
      }
      n++;
    }
    this.queue.splice(0, n);
    this.queuedBytes -= bytes;
    const req: ICaptureAppendRequest = {
      id: this.id,
      configs: configs.map(({ track, codec, config }) => ({ track, codec: CODECS[codec], config })),
      units: units.map(({ track, timestampUs, keyframe, data }) => ({
        track,
        timestampUs: BigInt(Math.round(timestampUs)),
        keyframe,
        data,
      })),
    };
    this.sending = this.append(req).then(
      () => {
        this.sentUnits += units.length;
        this.sending = null;
        this.pump();
      },
      (e) => {
        this.failed = true;
        this.sending = null;
        this.queue = [];
        this.onerror(e);
      }
    );
  }

  // Resolves once everything queued has been sent, or sending failed.
  public async drain(): Promise<void> {
    while (this.sending) await this.sending;
  }
}

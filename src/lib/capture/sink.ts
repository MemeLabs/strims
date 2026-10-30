// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

/** The track ids a capture writes. */
export const VIDEO_TRACK = 1;
export const AUDIO_TRACK = 2;

/** A decoder config, as its encoder gave it: an avcC record for H.264, an
 * OpusHead for Opus.
 */
export interface ConfigItem {
  type: "config";
  track: number;
  codec: "h264" | "opus";
  config: Uint8Array;
}

/** An access unit: its presentation time in microseconds on the capture's
 * clock, after its track's last.
 */
export interface UnitItem {
  type: "unit";
  track: number;
  timestampUs: number;
  keyframe: boolean;
  data: Uint8Array;
}

export type CaptureItem = ConfigItem | UnitItem;

/** Where a capture's configs and units go, in order: a track's config comes
 * before the units it decodes.
 */
export interface CaptureSink {
  push(item: CaptureItem): void;
}

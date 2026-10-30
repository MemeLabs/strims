// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Opus encoder configs for browser capture, and the OpusHead to send.

/** libopus's lookahead at 48 kHz: its encoders' pre-skip. */
export const LIBOPUS_PRE_SKIP = 312;

// The bitrate of a capture's audio: music and game audio, in stereo or mono.
export const audioBitrate = (channels: number): number => (channels > 1 ? 128_000 : 96_000);

// The encoder config of a capture's audio.
export const encoderConfig = (sampleRate: number, channels: number): AudioEncoderConfig => ({
  codec: "opus",
  sampleRate,
  numberOfChannels: channels,
  bitrate: audioBitrate(channels),
  bitrateMode: "variable",
  opus: { frameDuration: 20_000 },
});

// An OpusHead (RFC 7845 5.1) of mapping family 0.
export const opusHead = (
  channels: number,
  inputSampleRate: number,
  preSkip = LIBOPUS_PRE_SKIP
): Uint8Array => {
  const head = new Uint8Array(19);
  head.set(new TextEncoder().encode("OpusHead"));
  const v = new DataView(head.buffer);
  v.setUint8(8, 1);
  v.setUint8(9, channels);
  v.setUint16(10, preSkip, true);
  v.setUint32(12, inputSampleRate, true);
  return head;
};

// Whether `b` starts as an OpusHead does.
export const isOpusHead = (b: Uint8Array): boolean =>
  b.length >= 19 && new TextDecoder().decode(b.subarray(0, 8)) === "OpusHead";

// The OpusHead of an encoder's output: its description, or one built from its
// config when it gives none.
export const opusConfig = (
  description: Uint8Array | undefined,
  channels: number,
  sampleRate: number
): Uint8Array =>
  description && isOpusHead(description) ? description : opusHead(channels, sampleRate);

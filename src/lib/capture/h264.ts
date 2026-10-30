// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// H.264 encoder configs for browser capture: the encode size, bitrate,
// profile and level, and the avcC record to send.

/** The largest picture a capture encodes, by area. */
export const MAX_WIDTH = 1920;
export const MAX_HEIGHT = 1080;
/** The highest frame rate a capture asks its source for. */
export const MAX_FRAME_RATE = 30;

/** Bits per pixel per frame of the target bitrate. */
const BITS_PER_PIXEL = 0.08;
/**
 * The bitrate bounds. Below 1 Mbit/s the swarm sends padding anyway: a
 * capture's 8-chunk integrity units signed early are padded to 32 KiB, so
 * a slower stream costs as much on the wire. 6 Mbit/s is the top of the
 * bitrates the swarm is tuned for.
 */
export const MIN_VIDEO_BITRATE = 1_000_000;
export const MAX_VIDEO_BITRATE = 6_000_000;

// The size a source picture is encoded at: within the area of MAX_WIDTH by
// MAX_HEIGHT, at its aspect, in even dimensions.
export const encodeSize = (width: number, height: number): [number, number] => {
  const scale = Math.min(1, Math.sqrt((MAX_WIDTH * MAX_HEIGHT) / Math.max(1, width * height)));
  const even = (n: number) => Math.max(2, Math.floor((n * scale) / 2) * 2);
  return [even(width), even(height)];
};

// The target bitrate of a picture size at a frame rate.
export const videoBitrate = (width: number, height: number, frameRate: number): number =>
  Math.round(
    Math.min(
      MAX_VIDEO_BITRATE,
      Math.max(MIN_VIDEO_BITRATE, BITS_PER_PIXEL * width * height * frameRate)
    )
  );

/** H.264 levels (Table A-1): level_idc, MaxMBPS, MaxFS and MaxBR in kbit/s
 * (Baseline and Main; High allows 1.25 times it).
 */
const LEVELS: [number, number, number, number][] = [
  [31, 108_000, 3_600, 14_000],
  [32, 216_000, 5_120, 20_000],
  [40, 245_760, 8_192, 20_000],
  [41, 245_760, 8_192, 50_000],
  [42, 522_240, 8_704, 50_000],
  [50, 589_824, 22_080, 135_000],
  [51, 983_040, 36_864, 240_000],
  [52, 2_073_600, 36_864, 240_000],
];

// The lowest level, at least 3.1, whose limits hold a picture size at a frame
// rate and bitrate. Up to 720p30 always takes 3.1, so a capture that stays
// within it keeps its codec string.
export const levelFor = (
  width: number,
  height: number,
  frameRate: number,
  bitrate: number
): number => {
  const wMbs = Math.ceil(width / 16);
  const hMbs = Math.ceil(height / 16);
  const fs = wMbs * hMbs;
  for (const [idc, maxMbps, maxFs, maxBr] of LEVELS) {
    // A side is at most sqrt(8 MaxFS) macroblocks.
    const side = Math.sqrt(8 * maxFs);
    if (
      fs <= maxFs &&
      wMbs <= side &&
      hMbs <= side &&
      fs * frameRate <= maxMbps &&
      bitrate <= maxBr * 1000
    ) {
      return idc;
    }
  }
  return 52;
};

export interface Profile {
  name: string;
  /** profile_idc and the constraint flags, as a codec string writes them. */
  prefix: string;
}

/**
 * The profiles a capture tries, best compression first. High and Main
 * may reorder pictures (B-frames); a capture's units carry no decode time,
 * so an encoder that reorders takes the next profile, and Constrained
 * Baseline never reorders.
 */
export const PROFILES: Profile[] = [
  { name: "High", prefix: "6400" },
  { name: "Main", prefix: "4d40" },
  { name: "Constrained Baseline", prefix: "42e0" },
];
export const BASELINE = PROFILES.length - 1;

// The codec string of a profile at a level: avc1.PPCCLL.
export const codecString = (profile: Profile, level: number): string =>
  `avc1.${profile.prefix}${level.toString(16).padStart(2, "0")}`;

// The encoder config of a capture.
export const encoderConfig = (
  profile: Profile,
  width: number,
  height: number,
  frameRate: number
): VideoEncoderConfig => {
  const bitrate = videoBitrate(width, height, frameRate);
  return {
    codec: codecString(profile, levelFor(width, height, frameRate, bitrate)),
    width,
    height,
    bitrate,
    framerate: frameRate,
    // A still screen costs almost nothing and a busy one gets the bits.
    bitrateMode: "variable",
    // No lookahead, and the encoder may drop a frame to hold the rate.
    latencyMode: "realtime",
    hardwareAcceleration: "no-preference",
    avc: { format: "avc" },
  };
};

// Whether an encoder of `config` reorders pictures: it encodes a few small
// frames and checks their output order.
export const reordersFrames = async (config: VideoEncoderConfig): Promise<boolean> => {
  const width = 320;
  const height = 180;
  const codec = config.codec.slice(0, -2) + "1f";
  const timestamps: number[] = [];
  let failed = false;
  const encoder = new VideoEncoder({
    output: (chunk: EncodedVideoChunk) => timestamps.push(chunk.timestamp),
    error: () => (failed = true),
  });
  try {
    encoder.configure({ ...config, codec, width, height, bitrate: 500_000 });
    const canvas = new OffscreenCanvas(width, height);
    const g = canvas.getContext("2d");
    for (let i = 0; i < 12; i++) {
      g.fillStyle = "#123";
      g.fillRect(0, 0, width, height);
      g.fillStyle = "#fff";
      g.fillRect((i * 17) % width, 40, 64, 64);
      const frame = new VideoFrame(canvas, { timestamp: Math.round((i * 1e6) / 30) });
      encoder.encode(frame, { keyFrame: i === 0 });
      frame.close();
    }
    await encoder.flush();
  } catch {
    failed = true;
  } finally {
    if (encoder.state !== "closed") encoder.close();
  }
  return failed || timestamps.some((t, i) => i > 0 && t < timestamps[i - 1]);
};

// The first profile, from `from` on, whose config the encoder takes at this
// size and that doesn't reorder pictures.
export const chooseConfig = async (
  width: number,
  height: number,
  frameRate: number,
  from = 0,
  verified = new Set<number>()
): Promise<{ index: number; config: VideoEncoderConfig } | null> => {
  for (let index = from; index < PROFILES.length; index++) {
    const config = encoderConfig(PROFILES[index], width, height, frameRate);
    const { supported } = await VideoEncoder.isConfigSupported(config).catch(() => ({
      supported: false,
    }));
    if (!supported) continue;
    if (index !== BASELINE && !verified.has(index)) {
      if (await reordersFrames(config)) continue;
      verified.add(index);
    }
    return { index, config };
  }
  return null;
};

const NAL_SPS = 7;
const NAL_PPS = 8;

// The NAL units of an access unit in avc format.
export const nalUnits = (data: Uint8Array, lengthSize: number): Uint8Array[] => {
  const out: Uint8Array[] = [];
  let at = 0;
  while (at + lengthSize <= data.length) {
    let len = 0;
    for (let i = 0; i < lengthSize; i++) len = len * 256 + data[at + i];
    at += lengthSize;
    if (len === 0 || at + len > data.length) return [];
    out.push(data.subarray(at, at + len));
    at += len;
  }
  return at === data.length ? out : [];
};

// An avcC record of parameter sets.
export const avcC = (sps: Uint8Array[], pps: Uint8Array[], lengthSize: number): Uint8Array => {
  const sets = [...sps, ...pps];
  const out = new Uint8Array(7 + sets.reduce((n, s) => n + 2 + s.length, 0));
  const first = sps[0];
  out.set([1, first[1], first[2], first[3], 0xfc | (lengthSize - 1), 0xe0 | sps.length]);
  let at = 6;
  const put = (s: Uint8Array) => {
    out[at++] = s.length >> 8;
    out[at++] = s.length & 0xff;
    out.set(s, at);
    at += s.length;
  };
  sps.forEach(put);
  out[at++] = pps.length;
  pps.forEach(put);
  return out;
};

// The avcC record of an encoder's output: its description, or one of the
// parameter sets the keyframe it came with carries in band. A description may
// repeat each set's NAL header byte, which decoders can't read, while the sets
// a keyframe carries are the ones its encoder used.
export const avcConfig = (description: Uint8Array, keyframe: Uint8Array): Uint8Array => {
  const lengthSize = description.length > 4 ? (description[4] & 0x03) + 1 : 4;
  const nals = nalUnits(keyframe, lengthSize);
  const sps = nals.filter((n) => (n[0] & 0x1f) === NAL_SPS && n.length >= 4);
  const pps = nals.filter((n) => (n[0] & 0x1f) === NAL_PPS);
  if (sps.length === 0 || pps.length === 0) return description;
  return avcC(sps, pps, lengthSize);
};

// Whether two byte arrays hold the same bytes.
export const sameBytes = (a: Uint8Array, b: Uint8Array): boolean =>
  a.length === b.length && a.every((x, i) => x === b[i]);

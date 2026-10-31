// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { ListingRendition } from "../../apis/strims/network/v1/directory/directory";
import { UNITS_MIME_TYPE, mimeEssence } from "./mime";
import { PlayerKind } from "./players";

// Which of a stream's renditions a viewer plays.
//
// A listing's snippet names every swarm its stream is published in, each
// another encoding of it: the listing's own, H.264 that every viewer
// plays, and others, such as smaller H.264 rungs and the publisher's own
// HEVC or AV1. The viewer's renditions are those the browser says it can
// play, tried in the order rankRenditions gives, which ends at the
// listing's own.

/** The most renditions a snippet gives the viewer to choose from. */
export const MAX_CANDIDATES = 16;
/** The longest codec string taken from a rendition's MIME type. */
export const MAX_CODEC_LEN = 64;
/** The largest picture side taken. */
export const MAX_SIDE = 16384;
/** The highest frame rate taken; a higher one reads as unknown. */
export const MAX_FRAME_RATE = 1000;
/** A portable device prefers renditions whose shorter side is at most this. */
export const PORTABLE_MAX_SIDE = 720;
/** A rendition covers the player when showing it enlarges it by at most this fraction. */
export const FIT_SLACK = 0.05;
/** What decodingInfo is told of a rendition whose bitrate or frame rate is unknown. */
export const DEFAULT_BITRATE = 4_000_000;
export const DEFAULT_FRAME_RATE = 30;

/** The video codecs ranked, by their RFC 6381 sample entry. */
export const enum CodecFamily {
  H264 = "h264",
  Hevc = "hevc",
  Av1 = "av1",
}

const FAMILIES: [RegExp, CodecFamily][] = [
  [/^avc[13]\./i, CodecFamily.H264],
  [/^(hvc1|hev1)\./i, CodecFamily.Hevc],
  [/^av01\./i, CodecFamily.Av1],
];

const AUDIO = /^(mp4a\.[0-9a-f.]+|opus)$/i;
const CODEC = /^[A-Za-z0-9.]+$/;

/** The family of a video codec string, or null for one this choice doesn't rank. */
export const codecFamily = (codec: string): CodecFamily | null =>
  FAMILIES.find(([re]) => re.test(codec))?.[1] ?? null;

/** One encoding of a listing's stream, in a swarm of its own. */
export interface Rendition {
  swarmUri: string;
  /** Its swarm's id, which names it among the stream's renditions. */
  id: string;
  mimeType: string;
  /** Its video codec, as its MIME type names it (`hvc1.1.6.L123.B0`); empty when unknown. */
  video: string;
  /** Its audio codecs, as its MIME type names them. */
  audio: string[];
  /** Its picture's size in pixels; 0 when unknown. */
  width: number;
  height: number;
  /** Its frames a second; 0 when unknown. */
  frameRate: number;
  /** Its nominal bits a second; 0 when unknown. */
  bitrate: number;
  /** The listing's own swarm: H.264, which every viewer plays. */
  main: boolean;
}

/** What a listing says of its stream. */
export interface MainStream {
  swarmUri: string;
  mimeType: string;
}

/** What a snippet says of a stream's renditions. */
export interface SnippetRenditions {
  renditions: Pick<
    ListingRendition,
    "mimeType" | "swarmUri" | "bitrate" | "width" | "height" | "frameRate"
  >[];
  videoWidth: number;
  videoHeight: number;
}

// A swarm URI's swarm id, its magnet URI's `xt`, or the URI itself when
// it names none.
export const swarmId = (uri: string): string => {
  try {
    return new URL(uri).searchParams.get("xt") || uri;
  } catch {
    return uri;
  }
};

// The codecs a MIME type's codecs parameter lists, or null without one.
const mimeCodecs = (mime: string): string[] | null => {
  const m = /;\s*codecs\s*=\s*("([^"]*)"|[^;\s]*)/i.exec(mime ?? "");
  if (!m) return null;
  return (m[2] ?? m[1])
    .split(",")
    .map((c) => c.trim())
    .filter(Boolean);
};

// A unit-format MIME type's video codec and audio codecs, or null for
// another format, a codec this choice doesn't know, or more than one
// video codec.
const unitCodecs = (mime: string): { video: string; audio: string[] } | null => {
  if (mimeEssence(mime) !== UNITS_MIME_TYPE) return null;
  const codecs = mimeCodecs(mime);
  if (!codecs) return null;
  let video = "";
  const audio: string[] = [];
  for (const c of codecs) {
    if (c.length > MAX_CODEC_LEN || !CODEC.test(c)) return null;
    if (codecFamily(c)) {
      if (video) return null;
      video = c;
    } else if (AUDIO.test(c)) {
      audio.push(c);
    } else {
      return null;
    }
  }
  return { video, audio };
};

const side = (n: number): number => (Number.isInteger(n) && n > 0 && n <= MAX_SIDE ? n : 0);

const frameRate = (n: number): number =>
  Number.isFinite(n) && n > 0 && n <= MAX_FRAME_RATE ? n : 0;

const bitrate = (n: bigint | number): number => {
  const b = Number(n);
  return Number.isSafeInteger(b) && b > 0 ? b : 0;
};

/**
 * The renditions a viewer may choose from: the snippet's in the unit format
 * with a video codec and a picture size, each swarm once, at most
 * MAX_CANDIDATES, with the listing's own among them, taken from the
 * listing where the snippet doesn't name it (its size then the snippet's
 * video size). Without a snippet, or one of no renditions, it is the
 * listing's own alone.
 */
export const renditionCandidates = (
  main: MainStream,
  snippet: SnippetRenditions | null
): Rendition[] => {
  const mainId = swarmId(main.swarmUri);
  const mainCodecs = unitCodecs(main.mimeType);
  const out: Rendition[] = [];
  const seen = new Set<string>();
  let listedMain: Rendition | null = null;
  for (const r of snippet?.renditions ?? []) {
    if (out.length >= MAX_CANDIDATES - (listedMain ? 0 : 1)) break;
    if (!r.swarmUri) continue;
    const id = swarmId(r.swarmUri);
    if (seen.has(id)) continue;
    const isMain = r.swarmUri === main.swarmUri || id === mainId;
    const codecs = isMain ? mainCodecs : unitCodecs(r.mimeType);
    const width = side(r.width);
    const height = side(r.height);
    if (!isMain && (!codecs?.video || !width || !height)) continue;
    seen.add(id);
    const rendition: Rendition = {
      swarmUri: isMain ? main.swarmUri : r.swarmUri,
      id,
      mimeType: isMain ? main.mimeType : r.mimeType,
      video: codecs?.video ?? "",
      audio: codecs?.audio ?? [],
      width,
      height,
      frameRate: frameRate(r.frameRate),
      bitrate: bitrate(r.bitrate),
      main: isMain,
    };
    if (isMain) listedMain = rendition;
    out.push(rendition);
  }
  if (!listedMain) {
    out.push({
      swarmUri: main.swarmUri,
      id: mainId,
      mimeType: main.mimeType,
      video: mainCodecs?.video ?? "",
      audio: mainCodecs?.audio ?? [],
      width: side(snippet?.videoWidth ?? 0),
      height: side(snippet?.videoHeight ?? 0),
      frameRate: 0,
      bitrate: 0,
      main: true,
    });
  }
  return out;
};

// A codec as MediaSource takes it in MP4: Opus is `Opus`, which WebKit
// requires.
const mseCodec = (codec: string): string => (codec.toLowerCase() === "opus" ? "Opus" : codec);

/**
 * The type the player's sink creates a rendition's SourceBuffer with, as
 * the remuxer names its init: `video/mp4;codecs="<video>,<audio>"`.
 */
export const mseType = (r: Rendition): string => {
  const codecs = [r.video, ...r.audio].filter(Boolean).map(mseCodec);
  const type = r.video ? "video/mp4" : "audio/mp4";
  return codecs.length ? `${type};codecs="${codecs.join(",")}"` : type;
};

/** What decodingInfo says of a rendition's video. */
export interface DecodingInfo {
  supported: boolean;
  powerEfficient: boolean;
}

/** What the browser says it can play. */
export interface Probes {
  /** Whether the sink's MediaSource takes a SourceBuffer of `type`. */
  mse: (type: string) => boolean;
  /** Whether WebCodecs decodes `codec` at a size, for the low-latency player; absent for the baseline. */
  webCodecs?: (codec: string, width: number, height: number) => Promise<boolean>;
  /** decodingInfo's answer for a MediaSource video; null where it has none. */
  decodingInfo: (video: VideoConfiguration) => Promise<DecodingInfo | null>;
}

/** The parts of a window the probes use. */
export interface ProbeScope {
  ManagedMediaSource?: { isTypeSupported(type: string): boolean };
  MediaSource?: { isTypeSupported(type: string): boolean };
  VideoDecoder?: {
    isConfigSupported(c: { codec: string; codedWidth: number; codedHeight: number }): Promise<{
      supported?: boolean;
    }>;
  };
  navigator?: { mediaCapabilities?: Pick<MediaCapabilities, "decodingInfo"> };
}

/** The most decodingInfo answers a set of probes keeps. */
const MAX_CACHED = 64;

/**
 * The browser's probes for a player of `kind`: the MediaSource its sink
 * takes (ManagedMediaSource where there is one), WebCodecs for the
 * low-latency player, and decodingInfo of MediaSource video, its answers
 * kept per configuration. A probe that throws answers no.
 */
export const browserProbes = (kind: PlayerKind, scope: ProbeScope = globalThis): Probes => {
  const cache = new Map<string, Promise<DecodingInfo | null>>();
  const ms = scope.ManagedMediaSource ?? scope.MediaSource;
  const decoder = scope.VideoDecoder;
  const capabilities = scope.navigator?.mediaCapabilities;
  return {
    mse: (type) => {
      try {
        return !!ms?.isTypeSupported(type);
      } catch {
        return false;
      }
    },
    webCodecs:
      kind === PlayerKind.LowLatency
        ? async (codec, codedWidth, codedHeight) => {
            try {
              return !!(await decoder?.isConfigSupported({ codec, codedWidth, codedHeight }))
                ?.supported;
            } catch {
              return false;
            }
          }
        : undefined,
    decodingInfo: (video) => {
      const key = JSON.stringify(video);
      let info = cache.get(key);
      if (!info) {
        info = (async () => {
          try {
            if (!capabilities) return null;
            const r = await capabilities.decodingInfo({ type: "media-source", video });
            return { supported: !!r.supported, powerEfficient: !!r.powerEfficient };
          } catch {
            return null;
          }
        })();
        if (cache.size >= MAX_CACHED) cache.clear();
        cache.set(key, info);
      }
      return info;
    },
  };
};

/**
 * The video decodingInfo is asked about for a rendition: its video codec
 * alone (an audio codec in the content type makes Firefox and Safari
 * throw, and Chromium answer unsupported), and a frame rate and bitrate
 * always (every engine requires both).
 */
export const decodingConfig = (r: Rendition): VideoConfiguration => ({
  contentType: `video/mp4;codecs="${r.video}"`,
  width: r.width || 1920,
  height: r.height || 1080,
  bitrate: r.bitrate || DEFAULT_BITRATE,
  framerate: r.frameRate || DEFAULT_FRAME_RATE,
});

/** A rendition, and whether the browser says it plays it and how cheaply. */
export interface Assessment {
  rendition: Rendition;
  supported: boolean;
  powerEfficient: boolean;
}

/**
 * Asks the browser of each rendition. The listing's own is always
 * supported. Another is supported when it didn't fail here before
 * (`failed`), the sink's MediaSource takes its type, WebCodecs decodes it
 * for the low-latency player (whose fallback is the baseline, so both
 * must), and decodingInfo doesn't say otherwise. It is power-efficient
 * when decodingInfo says so.
 */
export const assessRenditions = (
  renditions: Rendition[],
  probes: Probes,
  failed: (r: Rendition) => boolean
): Promise<Assessment[]> =>
  Promise.all(
    renditions.map(async (rendition): Promise<Assessment> => {
      const no = { rendition, supported: false, powerEfficient: false };
      if (!rendition.main) {
        if (failed(rendition) || !probes.mse(mseType(rendition))) return no;
        const { video, width, height } = rendition;
        if (probes.webCodecs && !(await probes.webCodecs(video, width, height))) return no;
      }
      const info = rendition.video ? await probes.decodingInfo(decodingConfig(rendition)) : null;
      if (!rendition.main && info && !info.supported) return no;
      return { rendition, supported: true, powerEfficient: info?.powerEfficient ?? false };
    })
  );

/** The player's picture area, in device pixels, and whether the device is portable. */
export interface Viewport {
  width: number;
  height: number;
  portable: boolean;
}

// Codec families by what a power-efficient decoder makes of a bit, best first.
const GENERATION = [CodecFamily.Av1, CodecFamily.Hevc, CodecFamily.H264];
// Codec families by what decoding them in software costs, cheapest first.
const COST = [CodecFamily.H264, CodecFamily.Hevc, CodecFamily.Av1];

const familyIndex = (order: CodecFamily[], codec: string): number => {
  const i = order.indexOf(codecFamily(codec));
  return i < 0 ? order.length : i;
};

/**
 * The order the viewer tries renditions in: those supported, best first,
 * up to and including the listing's own, which every viewer plays, so
 * none after it is tried.
 *
 * - Power-efficient decoding first: it saves battery.
 * - Then fit. The smallest rendition that covers the player's picture
 *   (shown enlarged by at most FIT_SLACK) comes first; failing that, the
 *   largest. On a portable device those whose shorter side is over
 *   PORTABLE_MAX_SIDE come last, smallest first. A listing's own of
 *   unknown size is as large as the largest.
 * - Then, among power-efficient renditions, frame rate, then codec
 *   generation (AV1, HEVC, H.264), then bitrate: at one size and rate the
 *   newer codec is the publisher's own encoding, since renditions are
 *   transcoded to H.264. Among the others, the codec cheapest to decode
 *   in software (H.264, HEVC, AV1), then frame rate, then bitrate.
 * - Then the listing's own after the others.
 */
export const rankRenditions = (assessed: Assessment[], viewport: Viewport): Rendition[] => {
  const pool = assessed.filter((a) => a.supported || a.rendition.main);
  const largest = pool.reduce<Rendition | null>((l, { rendition: r }) => {
    if (!r.width || !r.height) return l;
    return !l || Math.min(r.width, r.height) > Math.min(l.width, l.height) ? r : l;
  }, null);
  const known = viewport.width > 0 && viewport.height > 0;
  const keyed = pool.map((a, order) => {
    const r = a.rendition;
    const [width, height] =
      r.width && r.height
        ? [r.width, r.height]
        : largest
        ? [largest.width, largest.height]
        : [0, 0];
    const short = width && height ? Math.min(width, height) : Infinity;
    const covers =
      known &&
      width > 0 &&
      Math.min(viewport.width / width, viewport.height / height) <= 1 + FIT_SLACK;
    let fit: number;
    if (viewport.portable && short > PORTABLE_MAX_SIDE) {
      fit = 2;
    } else if (covers) {
      fit = 0;
    } else {
      fit = 1;
    }
    const quality = a.powerEfficient
      ? [-r.frameRate, familyIndex(GENERATION, r.video), -r.bitrate]
      : [familyIndex(COST, r.video), -r.frameRate, -r.bitrate];
    const size = fit === 1 ? -short : short;
    return {
      r,
      key: [a.powerEfficient ? 0 : 1, fit, size, ...quality, r.main ? 1 : 0, order],
    };
  });
  keyed.sort((a, b) => {
    for (let i = 0; i < a.key.length; i++) {
      if (a.key[i] !== b.key[i]) return a.key[i] < b.key[i] ? -1 : 1;
    }
    return 0;
  });
  const chain: Rendition[] = [];
  for (const { r } of keyed) {
    chain.push(r);
    if (r.main) break;
  }
  return chain;
};

/** The local storage key of the codecs that failed to play here. */
export const FAILURES_STORAGE_KEY = "renditionFailures";
/** How long a codec that failed stays skipped. */
export const FAILURE_TTL_MS = 30 * 24 * 60 * 60 * 1000;
/** The most codecs kept as failed. */
export const MAX_FAILURES = 32;

interface StoredFailures {
  ua: string;
  /** When each player's codec last failed, in Unix milliseconds, by `<player>|<codec>`. */
  entries: Record<string, number>;
}

/**
 * The video codecs that failed to play on this device, per player, kept in
 * local storage: a rendition of one is skipped for FAILURE_TTL_MS. They
 * belong to the browser that failed them, so another user agent string
 * starts with none. At most MAX_FAILURES are kept, the oldest going first.
 *
 * Only HEVC and AV1 failures are kept: H.264 is what every viewer plays.
 * A publisher whose listing mislabels a rendition can make its codec fail
 * here, which skips that codec for other streams too; the entry lapses
 * with its TTL, and those streams play as H.264 meanwhile.
 */
export class RenditionFailures {
  private entries: Record<string, number> = {};

  constructor(
    private readonly storage: Pick<Storage, "getItem" | "setItem"> | null,
    private readonly userAgent: string,
    private readonly now: () => number = Date.now
  ) {
    try {
      const stored = JSON.parse(storage?.getItem(FAILURES_STORAGE_KEY) ?? "null") as unknown;
      if (isStored(stored) && stored.ua === userAgent) {
        this.entries = stored.entries;
      }
    } catch {
      // Storage the page may not read, or can't parse, holds none.
    }
    this.prune();
  }

  /** Whether `r`'s codec failed in a player of `kind` within the TTL. */
  failed(kind: PlayerKind, r: Rendition): boolean {
    const at = this.entries[key(kind, r.video)];
    return at !== undefined && this.now() - at < FAILURE_TTL_MS;
  }

  /** Keeps `r`'s codec as failed in a player of `kind`, unless it's H.264. */
  record(kind: PlayerKind, r: Rendition): void {
    const family = codecFamily(r.video);
    if (!family || family === CodecFamily.H264) return;
    this.entries[key(kind, r.video)] = this.now();
    this.prune();
    try {
      const stored: StoredFailures = { ua: this.userAgent, entries: this.entries };
      this.storage?.setItem(FAILURES_STORAGE_KEY, JSON.stringify(stored));
    } catch {
      // Storage the page may not write keeps the failure for this page only.
    }
  }

  private prune(): void {
    const now = this.now();
    const live = Object.entries(this.entries)
      .filter(([, at]) => now - at < FAILURE_TTL_MS)
      .sort(([, a], [, b]) => b - a)
      .slice(0, MAX_FAILURES);
    this.entries = Object.fromEntries(live);
  }
}

const key = (kind: PlayerKind, codec: string) => `${kind}|${codec}`;

const isStored = (v: unknown): v is StoredFailures => {
  if (typeof v !== "object" || v === null) return false;
  const { ua, entries } = v as Partial<StoredFailures>;
  return (
    typeof ua === "string" &&
    typeof entries === "object" &&
    entries !== null &&
    Object.values(entries).every((at) => typeof at === "number" && Number.isFinite(at))
  );
};

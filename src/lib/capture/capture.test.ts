// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import { ICaptureAppendRequest } from "../../apis/strims/video/v1/capture";
import { TrackClock } from "./clock";
import {
  PROFILES,
  avcConfig,
  codecString,
  encodeSize,
  levelFor,
  nalUnits,
  videoBitrate,
} from "./h264";
import { opusConfig, opusHead } from "./opus";
import { CaptureSender } from "./sender";
import { AUDIO_TRACK, VIDEO_TRACK } from "./sink";

const hex = (s: string): Uint8Array => Uint8Array.from(s.match(/../g).map((b) => parseInt(b, 16)));
const toHex = (b: Uint8Array): string =>
  Array.from(b, (x) => x.toString(16).padStart(2, "0")).join("");

const lengthPrefixed = (...nals: Uint8Array[]): Uint8Array => {
  const out = new Uint8Array(nals.reduce((n, x) => n + 4 + x.length, 0));
  let at = 0;
  for (const n of nals) {
    new DataView(out.buffer).setUint32(at, n.length);
    out.set(n, at + 4);
    at += 4 + n.length;
  }
  return out;
};

describe("capture h264", () => {
  it("encodes within 1080p's area at the source's aspect, in even sizes", () => {
    assert.deepEqual(encodeSize(1280, 720), [1280, 720]);
    assert.deepEqual(encodeSize(1920, 1080), [1920, 1080]);
    assert.deepEqual(encodeSize(3840, 2160), [1920, 1080]);
    assert.deepEqual(encodeSize(2560, 1600), [1820, 1138]);
    assert.deepEqual(encodeSize(1279, 719), [1278, 718]);
  });

  it("targets 0.08 bits a pixel, within 1 and 6 Mbit/s", () => {
    assert.equal(videoBitrate(1280, 720, 30), 2_211_840);
    assert.equal(videoBitrate(1920, 1080, 30), 4_976_640);
    assert.equal(videoBitrate(640, 360, 30), 1_000_000);
    assert.equal(videoBitrate(1920, 1080, 60), 6_000_000);
  });

  it("takes the lowest level from 3.1 that holds the size and rate", () => {
    assert.equal(levelFor(640, 360, 30, 1e6), 31);
    assert.equal(levelFor(1280, 720, 30, 2.2e6), 31);
    assert.equal(levelFor(1280, 720, 60, 4e6), 32);
    assert.equal(levelFor(1920, 1080, 30, 5e6), 40);
    assert.equal(levelFor(1920, 1080, 60, 6e6), 42);
    assert.equal(levelFor(3840, 2160, 30, 6e6), 51);
    assert.equal(codecString(PROFILES[0], 40), "avc1.640028");
    assert.equal(codecString(PROFILES[2], 31), "avc1.42e01f");
  });

  it("takes parameter sets a keyframe carries in band over the description", () => {
    // Firefox's description repeats each set's NAL header byte.
    const firefox = hex(
      "0142c01f03010018676742c01fd9005005ba10000003001001e84800f18324800100056868cb8cb2"
    );
    const sps = hex("6742c01fd9005005ba10000003001001e84800f1832480");
    const pps = hex("68cb8cb2");
    const idr = hex("6588840021");
    const keyframe = lengthPrefixed(sps, pps, idr);
    assert.deepEqual(nalUnits(keyframe, 4), [sps, pps, idr]);
    assert.equal(
      toHex(avcConfig(firefox, keyframe)),
      "0142c01fffe10017" + toHex(sps) + "010004" + toHex(pps)
    );
    // Chromium's keyframes carry none: its description stands.
    const chromium = hex(
      "01640c1fffe1001367640c1fac18d00a00b74dc0434041e1108d4001000468ce3c80fdf8f800"
    );
    assert.equal(avcConfig(chromium, lengthPrefixed(idr)), chromium);
    assert.equal(avcConfig(chromium, hex("00000009aa")), chromium);
  });
});

describe("capture opus", () => {
  it("builds the OpusHead the encoders give", () => {
    assert.equal(toHex(opusHead(2, 48_000)), "4f707573486561640102380180bb0000000000");
    assert.equal(toHex(opusHead(1, 44_100)), "4f707573486561640101380144ac0000000000");
    const given = hex("4f707573486561640101380180bb0000000000");
    assert.equal(opusConfig(given, 1, 48_000), given);
    assert.equal(toHex(opusConfig(undefined, 2, 48_000)), toHex(opusHead(2, 48_000)));
    assert.equal(toHex(opusConfig(hex("01"), 2, 48_000)), toHex(opusHead(2, 48_000)));
  });
});

describe("capture clock", () => {
  it("maps a track's first sample to its arrival and keeps its spacing", () => {
    const c = new TrackClock();
    c.anchor(933_000_000_000, 5_000);
    assert.equal(c.output(933_000_000_000), 5_000);
    assert.equal(c.output(933_000_033_333), 38_333);
    assert.equal(c.toSource(71_666), 933_000_066_666);
    // Never back, nor closer than a millisecond.
    assert.equal(c.output(933_000_000_000), 39_333);
    assert.equal(c.output(933_000_033_900), 40_333);
  });
});

describe("capture sender", () => {
  const unit = (track: number, timestampUs: number, keyframe = true, n = 10) => ({
    type: "unit" as const,
    track,
    timestampUs,
    keyframe,
    data: new Uint8Array(n),
  });
  const config = (track: number) => ({
    type: "config" as const,
    track,
    codec: track === VIDEO_TRACK ? ("h264" as const) : ("opus" as const),
    config: new Uint8Array(4),
  });

  it("sends one append at a time, its configs first", async () => {
    const sent: ICaptureAppendRequest[] = [];
    let release: () => void = () => undefined;
    const append = (req: ICaptureAppendRequest) => {
      sent.push(req);
      return new Promise<void>((resolve) => (release = resolve));
    };
    const s = new CaptureSender(new Uint8Array([1]), append, assert.fail);
    s.push(config(VIDEO_TRACK));
    s.push(unit(VIDEO_TRACK, 0));
    await Promise.resolve();
    assert.equal(sent.length, 1);
    s.push(unit(AUDIO_TRACK, 0));
    s.push(config(VIDEO_TRACK));
    s.push(unit(VIDEO_TRACK, 33_333));
    release();
    await new Promise((r) => setTimeout(r, 0));
    release();
    await new Promise((r) => setTimeout(r, 0));
    release();
    await s.drain();
    assert.deepEqual(
      sent.map((r) => [r.configs.length, r.units.map((u) => [u.track, u.timestampUs])]),
      [
        [1, [[1, BigInt(0)]]],
        [0, [[2, BigInt(0)]]],
        [1, [[1, BigInt(33_333)]]],
      ]
    );
    assert.equal(s.sentUnits, 3);
  });

  it("drops queued video past 3 s and waits for a keyframe", async () => {
    let overflows = 0;
    const sent: ICaptureAppendRequest[] = [];
    let release: () => void = () => undefined;
    const append = (req: ICaptureAppendRequest) => {
      sent.push(req);
      return new Promise<void>((resolve) => (release = resolve));
    };
    const s = new CaptureSender(new Uint8Array([1]), append, assert.fail, () => overflows++);
    s.push(unit(VIDEO_TRACK, 0));
    await Promise.resolve();
    for (let t = 33_333; t < 4_000_000; t += 33_333) {
      s.push(unit(VIDEO_TRACK, t, false));
      s.push(unit(AUDIO_TRACK, t));
    }
    assert.equal(overflows, 1);
    s.push(unit(VIDEO_TRACK, 4_100_000, false));
    s.push(unit(VIDEO_TRACK, 4_133_333, true));
    release();
    await new Promise((r) => setTimeout(r, 0));
    release();
    await s.drain();
    const second = sent[1].units;
    assert.deepEqual(
      second.filter((u) => u.track === VIDEO_TRACK).map((u) => u.timestampUs),
      [BigInt(4_133_333)]
    );
    // The audio left spans at most 3 s.
    const audio = second.filter((u) => u.track === AUDIO_TRACK).map((u) => Number(u.timestampUs));
    assert.ok(audio[audio.length - 1] - audio[0] <= 3_000_000);
    assert.ok(audio[audio.length - 1] > 3_900_000);
    assert.ok(s.droppedUnits > 90);
  });
});

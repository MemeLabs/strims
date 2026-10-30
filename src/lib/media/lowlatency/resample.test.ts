// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import { AudioOut, PcmOutput } from "./resample";

const tone = (rate: number, start: number, frames: number, hz = 440): Float32Array => {
  const x = new Float32Array(frames);
  for (let i = 0; i < frames; i++) x[i] = Math.sin(2 * Math.PI * hz * (start + i / rate));
  return x;
};

const pullAll = (out: AudioOut, rate = 1): PcmOutput[] => {
  const blocks: PcmOutput[] = [];
  for (let b = out.pull(1024, rate); b; b = out.pull(1024, rate)) blocks.push(b);
  return blocks;
};

const concat = (blocks: PcmOutput[], c = 0): Float32Array => {
  const n = blocks.reduce((a, b) => a + b.frames, 0);
  const y = new Float32Array(n);
  let at = 0;
  for (const b of blocks) {
    y.set(b.channels[c], at);
    at += b.frames;
  }
  return y;
};

describe("AudioOut", () => {
  it("plays input at the output's rate as it is", () => {
    const out = new AudioOut(48000);
    const x = tone(48000, 0, 4800);
    out.push({ start: 1, sampleRate: 48000, channels: [x] });
    const blocks = pullAll(out);
    assert.strictEqual(blocks[0].media, 1);
    assert.strictEqual(blocks[0].perFrame, 1 / 48000);
    const y = concat(blocks);
    // Mono plays on both channels.
    assert.deepStrictEqual(concat(blocks, 1), y);
    assert.ok(y.length >= 4797);
    for (let i = 0; i < y.length; i++) assert.strictEqual(y[i], x[i]);
  });

  it("resamples another rate closely, and plays faster at a higher rate", () => {
    const out = new AudioOut(48000);
    for (let k = 0; k < 10; k++) {
      out.push({ start: k * 0.1, sampleRate: 44100, channels: [tone(44100, k * 0.1, 4410)] });
    }
    const blocks = pullAll(out);
    const y = concat(blocks);
    // About a second at 48 kHz, close to the tone sampled at 48 kHz.
    assert.ok(Math.abs(y.length - 48000) < 10, `${y.length}`);
    let err = 0;
    for (let i = 100; i < y.length - 100; i++) {
      err = Math.max(err, Math.abs(y[i] - Math.sin(2 * Math.PI * 440 * (i / 48000))));
    }
    assert.ok(err < 0.01, `${err}`);

    const fast = new AudioOut(48000);
    fast.push({ start: 0, sampleRate: 48000, channels: [tone(48000, 0, 48000)] });
    const f = pullAll(fast, 1.02);
    assert.ok(Math.abs(concat(f).length - 48000 / 1.02) < 3);
    assert.strictEqual(f[0].perFrame, 1.02 / 48000);
  });

  it("fills a hole with silence and drops an overlap", () => {
    const out = new AudioOut(48000);
    out.push({ start: 0, sampleRate: 48000, channels: [new Float32Array(480).fill(1)] });
    // 10 ms of hole, then a block overlapping its predecessor by 5 ms.
    out.push({ start: 0.02, sampleRate: 48000, channels: [new Float32Array(480).fill(1)] });
    out.push({ start: 0.025, sampleRate: 48000, channels: [new Float32Array(480).fill(1)] });
    const y = concat(pullAll(out));
    assert.ok(Math.abs(out.filled - 0.01) < 1e-9);
    assert.ok(Math.abs(out.dropped - 0.005) < 1e-9);
    assert.strictEqual(y[479], 1);
    assert.strictEqual(y[480], 0);
    assert.strictEqual(y[959], 0);
    assert.strictEqual(y[960], 1);
    assert.ok(Math.abs(out.end - 0.035) < 1e-9);
  });

  it("a reset starts the next run at its time, trimming what comes before", () => {
    const out = new AudioOut(48000);
    out.push({ start: 0, sampleRate: 48000, channels: [new Float32Array(4800)] });
    out.reset(2);
    assert.strictEqual(out.end, null);
    // A preroll block from 1.9 s: only its part from 2 s plays.
    out.push({ start: 1.9, sampleRate: 48000, channels: [new Float32Array(9600).fill(0.5)] });
    const blocks = pullAll(out);
    assert.strictEqual(blocks[0].media, 2);
    assert.ok(Math.abs(concat(blocks).length - 4800) < 3);
  });
});

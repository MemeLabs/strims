// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";
import vm from "vm";

import { WORKLET_NAME, WORKLET_SOURCE } from "./audio";
import type { WorkletReport } from "./protocol";

interface Port {
  onmessage: ((m: { data: unknown }) => void) | null;
  postMessage(data: unknown): void;
}

interface Processor {
  port: Port;
  process(inputs: Float32Array[][], outputs: Float32Array[][]): boolean;
}

// The worklet's processor, run in a context of its own as a worklet's
// global scope has it, with its feed port and the reports it posts.
const load = () => {
  const reports: WorkletReport[] = [];
  const feed: Port = { onmessage: null, postMessage: (r: WorkletReport) => reports.push(r) };
  const context = {
    currentFrame: 0,
    processor: null as unknown,
    registerProcessor: (name: string, ctor: unknown) => {
      assert.strictEqual(name, WORKLET_NAME);
      context.processor = ctor;
    },
    AudioWorkletProcessor: class {
      port: Port = { onmessage: null, postMessage: () => undefined };
    },
  };
  vm.runInNewContext(WORKLET_SOURCE, context);
  const Ctor = context.processor as new () => Processor;
  const p = new Ctor();
  p.port.onmessage({ data: { port: feed } });
  const send = (m: unknown) => feed.onmessage({ data: m });
  const quantum = () => {
    const out = [new Float32Array(128), new Float32Array(128)];
    p.process([], [out]);
    context.currentFrame += 128;
    return out;
  };
  return { p, send, quantum, reports };
};

const pcm = (frames: number, value: number) => ({
  type: "pcm",
  channels: [new Float32Array(frames).fill(value), new Float32Array(frames).fill(-value)],
  frames,
});

describe("the playout worklet", () => {
  it("plays blocks in order across quanta, then silence, and reports what it played", () => {
    const w = load();
    w.send(pcm(200, 0.5));
    w.send(pcm(100, 0.25));
    const a = w.quantum();
    assert.ok(a[0].every((x) => x === 0.5));
    assert.ok(a[1].every((x) => x === -0.5));
    const b = w.quantum();
    assert.strictEqual(b[0][71], 0.5);
    assert.strictEqual(b[0][72], 0.25);
    const c = w.quantum();
    assert.strictEqual(c[0][43], 0.25);
    assert.strictEqual(c[0][44], 0);
    w.quantum();
    // Reports on running out, and every 4 quanta: the sample played at a
    // quantum's first frame, and what it held then.
    // Objects of the worklet's context: compared by their fields.
    const fields = (r: WorkletReport) => ({ ...r });
    const short = fields(w.reports.find((r) => r.frame === 256));
    assert.deepStrictEqual(short, {
      type: "report",
      frame: 256,
      consumed: 256,
      queued: 44,
      starved: 1,
    });
    const fourth = fields(w.reports.find((r) => r.frame === 384));
    assert.deepStrictEqual(fourth, {
      type: "report",
      frame: 384,
      consumed: 300,
      queued: 0,
      starved: 1,
    });
  });

  it("a reset drops what is queued and numbers the next samples from its index", () => {
    const w = load();
    w.send(pcm(1000, 0.5));
    w.quantum();
    w.send({ type: "reset", index: 5000 });
    w.send(pcm(256, 0.75));
    const a = w.quantum();
    assert.ok(a[0].every((x) => x === 0.75));
    w.quantum();
    w.quantum();
    const r = w.reports[w.reports.length - 1];
    assert.strictEqual(r.consumed, 5256);
    // Running out once, after playing.
    assert.strictEqual(r.starved, 1);
  });

  it("stops processing when told to", () => {
    const w = load();
    w.p.port.onmessage({ data: "stop" });
    assert.strictEqual(w.p.process([], [[new Float32Array(128)]]), false);
  });
});

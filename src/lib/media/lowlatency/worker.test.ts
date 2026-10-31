// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import { BatchWriter, endBatch } from "../feed";
import { Core, DemuxerOutput, Renderer, WindowMessage, WorkerMessage } from "./protocol";
import { STATE_INTERVAL_MS, runLowLatencyWorker } from "./worker";

// A demuxer that keeps what it's given and outputs nothing but its end.
class FakeDemuxer implements Core {
  pushes: [number[], boolean, boolean][] = [];
  finished = 0;

  push(data: Uint8Array, segmentEnd: boolean, discontinuity: boolean): DemuxerOutput[] {
    this.pushes.push([Array.from(data), segmentEnd, discontinuity]);
    return [];
  }

  finish(): DemuxerOutput[] {
    this.finished++;
    return [{ type: "end" }];
  }

  metrics(): unknown {
    return {};
  }

  free(): void {
    this.pushes = [];
  }
}

// A worker scope: what the worker posts the window, and a way to post it.
const scope = () => {
  const posted: WorkerMessage[] = [];
  const s = {
    onmessage: null as ((e: { data: WindowMessage }) => void) | null,
    postMessage: (message: WorkerMessage) => posted.push(message),
  };
  const send = (data: WindowMessage) => s.onmessage({ data });
  return { s: s as unknown as DedicatedWorkerGlobalScope, posted, send };
};

const deferred = <T>() => {
  let resolve: (v: T) => void;
  const promise = new Promise<T>((r) => (resolve = r));
  return { promise, resolve };
};

const settle = (ms = 20) => new Promise((r) => setTimeout(r, ms));

const batch = (ack: boolean, ...messages: [number[], boolean, boolean][]) => {
  const w = new BatchWriter();
  for (const [data, segmentEnd, discontinuity] of messages) {
    w.add({ data: new Uint8Array(data), segmentEnd, discontinuity });
  }
  return w.take(ack);
};

// What a port receives.
const receiver = (port: MessagePort) => {
  const got: unknown[] = [];
  port.onmessage = ({ data }: MessageEvent) => got.push(data);
  return got;
};

/** The tests' ports and workers, shut after each test so the process can exit. */
const ports: MessagePort[] = [];
const workers: ((m: WindowMessage) => void)[] = [];

// Starts a worker with the demuxer `load` gives it, fed on a port when `fed`.
const start = (
  demuxer: FakeDemuxer,
  fed: boolean,
  load = Promise.resolve(() => demuxer as Core)
) => {
  const w = scope();
  runLowLatencyWorker(w.s, () => load);
  workers.push(w.send);
  const channel = fed ? new MessageChannel() : null;
  if (channel) ports.push(channel.port1, channel.port2);
  w.send({ type: "open", renderer: Renderer.Canvas, feed: channel?.port2 });
  const node = channel?.port1 ?? null;
  const acks = node ? receiver(node) : [];
  return { ...w, node, acks };
};

describe("the low-latency player's worker", () => {
  afterEach(async () => {
    await settle();
    // A batch that doesn't parse stops a worker and its timers.
    for (const send of workers.splice(0)) {
      send({ type: "data", batch: new Uint8Array([9]).buffer, at: 0 });
    }
    for (const p of ports.splice(0)) p.close();
  });

  it("takes a feed's batches in order and acks those that ask on the feed's port", async () => {
    const d = new FakeDemuxer();
    const { node, acks, posted } = start(d, true);
    assert.deepStrictEqual(posted[0], { type: "hello", videoTrackGenerator: false });
    node.postMessage(batch(false, [[1, 2], true, false], [[3], false, false]));
    node.postMessage(batch(true, [[4, 5, 6], true, true]));
    await settle();
    assert.deepStrictEqual(d.pushes, [
      [[1, 2], true, false],
      [[3], false, false],
      [[4, 5, 6], true, true],
    ]);
    assert.deepStrictEqual(acks, [2]);
    assert.ok(!posted.some((m) => m.type === "consumed"), "a fed worker acks the feed alone");
  });

  it("holds the batches that come while the demuxer loads, and acks them once taken", async () => {
    const d = new FakeDemuxer();
    const load = deferred<() => Core>();
    const { node, acks } = start(d, true, load.promise);
    node.postMessage(batch(true, [[1], true, false]));
    await settle();
    assert.deepStrictEqual(d.pushes, []);
    assert.deepStrictEqual(acks, []);
    load.resolve(() => d);
    await settle();
    assert.deepStrictEqual(d.pushes, [[[1], true, false]]);
    assert.deepStrictEqual(acks, [1]);
  });

  it("ends the stream at the feed's end batch, after the batches before it", async () => {
    const d = new FakeDemuxer();
    const { node } = start(d, true);
    node.postMessage(batch(false, [[1], true, false]));
    node.postMessage(endBatch());
    await settle();
    assert.strictEqual(d.pushes.length, 1);
    assert.strictEqual(d.finished, 1);
  });

  it("takes the window's batches and end as it takes a feed's, acking the window", async () => {
    const d = new FakeDemuxer();
    const { posted, send } = start(d, false);
    send({ type: "data", batch: batch(false, [[1], true, false]), at: performance.timeOrigin });
    send({ type: "data", batch: batch(true, [[2], true, false]), at: performance.timeOrigin });
    send({ type: "end" });
    await settle();
    assert.deepStrictEqual(d.pushes, [
      [[1], true, false],
      [[2], true, false],
    ]);
    assert.strictEqual(d.finished, 1);
    assert.deepStrictEqual(
      posted.filter((m) => m.type === "consumed"),
      [{ type: "consumed", batches: 2 }]
    );
  });

  it("stops at a batch that doesn't parse", async () => {
    const d = new FakeDemuxer();
    const { node, posted } = start(d, true);
    node.postMessage(new Uint8Array([7, 0]).buffer);
    node.postMessage(batch(true, [[1], true, false]));
    await settle();
    assert.deepStrictEqual(d.pushes, []);
    const errors = posted.filter((m) => m.type === "error");
    assert.strictEqual(errors.length, 1);
    assert.ok(errors[0].type === "error" && !errors[0].final);
  });

  it("states the Data messages it took and their bytes", async () => {
    const d = new FakeDemuxer();
    const { node, posted } = start(d, true);
    node.postMessage(batch(false, [[1, 2], false, false], [[3], true, false]));
    await settle(STATE_INTERVAL_MS + 50);
    const state = posted.filter((m) => m.type === "state").pop();
    assert.ok(state?.type === "state");
    assert.deepStrictEqual([state.messages, state.bytes], [2, 3]);
  });
});

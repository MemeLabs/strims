// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import { BATCH_ACK, BatchWriter, endBatch } from "./feed";
import { Core, RemuxerOutput, SinkMode, WindowMessage, WorkerMessage } from "./protocol";
import { runPlayerWorker } from "./worker";

// A core whose every record end gives one media output: the record's
// last message's length.
class FakeCore implements Core {
  pushes: [number[], boolean, boolean][] = [];
  finished = 0;

  push(data: Uint8Array, segmentEnd: boolean, discontinuity: boolean): RemuxerOutput[] {
    this.pushes.push([Array.from(data), segmentEnd, discontinuity]);
    return segmentEnd ? [media(data.byteLength)] : [];
  }

  finish(): RemuxerOutput[] {
    this.finished++;
    return [media(99)];
  }

  metrics(): unknown {
    return {};
  }

  free(): void {
    this.pushes = [];
  }
}

const media = (n: number): RemuxerOutput => ({
  type: "media",
  data: new Uint8Array([n]),
  units: [],
});

// A MediaSource for a worker sink that never opens.
class FakeMediaSource extends EventTarget {
  handle = {};
  static isTypeSupported = () => true;
}

interface Posted {
  message: WorkerMessage;
  transfer: Transferable[];
}

// A worker scope: what the worker posts the window, and a way to post it.
const scope = (withMediaSource = false) => {
  const posted: Posted[] = [];
  const s = {
    onmessage: null as ((e: { data: WindowMessage }) => void) | null,
    postMessage: (message: WorkerMessage, transfer: Transferable[] = []) =>
      posted.push({ message, transfer }),
    MediaSource: withMediaSource ? FakeMediaSource : undefined,
  };
  const send = (data: WindowMessage) => s.onmessage({ data });
  return { s: s as unknown as DedicatedWorkerGlobalScope, posted, send };
};

const deferred = <T>() => {
  let resolve: (v: T) => void;
  const promise = new Promise<T>((r) => (resolve = r));
  return { promise, resolve };
};

const settle = () => new Promise((r) => setTimeout(r, 20));

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

const types = (posted: Posted[]) => posted.map((p) => p.message.type);

/** The tests' ports, closed after each test so the process can exit. */
const ports: MessagePort[] = [];

// Starts a worker on a feed, with the core `load` gives it.
const startFed = (mode: SinkMode, core: FakeCore, load = Promise.resolve(() => core as Core)) => {
  const w = scope(mode === SinkMode.Worker);
  runPlayerWorker(w.s, () => load);
  const channel = new MessageChannel();
  ports.push(channel.port1, channel.port2);
  w.send({ type: "open", mode, timeOrigin: performance.timeOrigin, feed: channel.port2 });
  const node = channel.port1;
  const acks = receiver(node);
  return { ...w, node, acks };
};

describe("the player worker", () => {
  afterEach(async () => {
    await settle();
    for (const p of ports.splice(0)) p.close();
  });

  it("takes a feed's batches in order and gives the window each one's outputs in one message", async () => {
    const core = new FakeCore();
    const { node, posted } = startFed(SinkMode.Window, core);
    node.postMessage(batch(false, [[1, 2], true, false], [[3], false, false]));
    node.postMessage(batch(false, [[4, 5, 6], true, true], [[7], true, false]));
    await settle();
    assert.deepStrictEqual(core.pushes, [
      [[1, 2], true, false],
      [[3], false, false],
      [[4, 5, 6], true, true],
      [[7], true, false],
    ]);
    assert.deepStrictEqual(types(posted), ["outputs", "outputs"]);
    const [first, second] = posted.map(
      (p) => p.message as Extract<WorkerMessage, { type: "outputs" }>
    );
    assert.deepStrictEqual(
      first.outputs.map((o) => o.type === "media" && Array.from(o.data)),
      [[2]]
    );
    assert.deepStrictEqual(
      second.outputs.map((o) => o.type === "media" && Array.from(o.data)),
      [[3], [1]]
    );
    assert.deepStrictEqual([second.messages, second.bytes], [4, 7]);
    assert.strictEqual(posted[1].transfer.length, 2, "a window sink takes the bytes");
    assert.ok(Math.abs(first.at - performance.now() / 1000) < 1, "arrival on the window's clock");
  });

  it("acks a feed's batch through the window when the window's sink takes the outputs", async () => {
    const core = new FakeCore();
    const { node, posted, acks, send } = startFed(SinkMode.Window, core);
    node.postMessage(batch(false, [[1], true, false]));
    node.postMessage(batch(true, [[2], true, false]));
    await settle();
    assert.deepStrictEqual(types(posted), ["outputs", "outputs", "consumed"]);
    assert.deepStrictEqual(posted[2].message, { type: "consumed", batches: 2 });
    assert.deepStrictEqual(acks, []);
    send({ type: "ack", batches: 2 });
    await settle();
    assert.deepStrictEqual(acks, [2]);
  });

  it("acks a feed's batch on the feed's port when its own sink takes the outputs", async () => {
    const core = new FakeCore();
    const { node, posted, acks } = startFed(SinkMode.Worker, core);
    node.postMessage(batch(true, [[1], true, false]));
    await settle();
    assert.deepStrictEqual(acks, [1]);
    assert.ok(!types(posted).includes("consumed"));
    const outputs = posted.find((p) => p.message.type === "outputs");
    assert.deepStrictEqual(outputs.transfer, [], "its sink keeps the bytes");
  });

  it("holds the batches that come while the core loads, and acks them once taken", async () => {
    const core = new FakeCore();
    const load = deferred<() => Core>();
    const { node, acks } = startFed(SinkMode.Worker, core, load.promise);
    node.postMessage(batch(true, [[1], true, false]));
    node.postMessage(endBatch());
    await settle();
    assert.deepStrictEqual([core.pushes.length, acks.length, core.finished], [0, 0, 0]);
    load.resolve(() => core);
    await settle();
    assert.deepStrictEqual(core.pushes, [[[1], true, false]]);
    assert.deepStrictEqual(acks, [1]);
    assert.strictEqual(core.finished, 1, "the end waits behind the data");
  });

  it("at the end, writes out what the core holds and then tells the window", async () => {
    const core = new FakeCore();
    const { node, posted } = startFed(SinkMode.Window, core);
    node.postMessage(batch(false, [[1], false, false]));
    node.postMessage(endBatch());
    await settle();
    assert.strictEqual(core.finished, 1);
    assert.deepStrictEqual(types(posted), ["outputs", "ended"]);
    const out = posted[0].message as Extract<WorkerMessage, { type: "outputs" }>;
    assert.deepStrictEqual(
      out.outputs.map((o) => o.type === "media" && Array.from(o.data)),
      [[99]]
    );
  });

  it("takes the window's batches and end as it takes a feed's, acking the window", async () => {
    const core = new FakeCore();
    const w = scope();
    runPlayerWorker(w.s, () => Promise.resolve(() => core as Core));
    w.send({ type: "open", mode: SinkMode.Window, timeOrigin: performance.timeOrigin });
    await settle();
    w.send({ type: "data", batch: batch(true, [[5], true, false]), at: 12.5 });
    w.send({ type: "end" });
    assert.deepStrictEqual(types(w.posted), ["outputs", "consumed", "outputs", "ended"]);
    assert.strictEqual((w.posted[0].message as { at: number }).at, 12.5);
    assert.deepStrictEqual(w.posted[1].message, { type: "consumed", batches: 1 });
  });

  it("stops at a batch that doesn't parse", async () => {
    const core = new FakeCore();
    const { node, posted } = startFed(SinkMode.Window, core);
    node.postMessage(new Uint8Array([7, 0]).buffer);
    node.postMessage(batch(false, [[1], true, false]));
    await settle();
    assert.deepStrictEqual(types(posted), ["error"]);
    assert.deepStrictEqual(core.pushes, []);
  });

  it("asks for acks only on the batches that ask", async () => {
    const core = new FakeCore();
    const { node, acks } = startFed(SinkMode.Worker, core);
    for (let i = 0; i < 5; i++) {
      node.postMessage(batch(i === 3, [[i], true, false]));
    }
    await settle();
    assert.deepStrictEqual(acks, [4]);
    assert.strictEqual(new Uint8Array(batch(true, [[1], true, false]))[1], BATCH_ACK);
  });
});

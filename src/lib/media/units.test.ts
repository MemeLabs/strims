// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import { Feed, parseBatch } from "./feed";
import { SinkMode, WindowMessage, WorkerMessage } from "./protocol";
import { UnitsPlayer, WINDOW_BYTES } from "./units";

// A player worker that keeps what the window posts it.
class FakeWorker {
  posted: { message: WindowMessage; transfer: Transferable[] }[] = [];
  terminated = false;
  onmessage: ((e: { data: WorkerMessage }) => void) | null = null;
  onerror: ((e: { message: string }) => void) | null = null;

  postMessage(message: WindowMessage, transfer: Transferable[] = []): void {
    this.posted.push({ message, transfer });
  }

  terminate(): void {
    this.terminated = true;
  }

  send(data: WorkerMessage): void {
    this.onmessage({ data });
  }

  messages(): WindowMessage[] {
    return this.posted.map((p) => p.message);
  }
}

class FakeVideo extends EventTarget {
  currentTime = 0;
  buffered = { length: 0, start: () => 0, end: () => 0 };
  paused = true;
  seeking = false;
  readyState = 0;
  playbackRate = 1;
  error: MediaError | null = null;
  disableRemotePlayback = false;
}

const globals = globalThis as unknown as Record<string, unknown>;

// A player whose MediaSource lives in its worker, and its workers.
const player = (feed?: (port: MessagePort, replaces: Feed | null) => Feed) => {
  const workers: FakeWorker[] = [];
  const video = new FakeVideo();
  const p = new UnitsPlayer(
    video as unknown as HTMLVideoElement,
    { attach: () => undefined, feed },
    () => {
      const w = new FakeWorker();
      workers.push(w);
      return w as unknown as Worker;
    }
  );
  return { p, video, workers };
};

// A feed opener that keeps what it's asked.
const feeds = () => {
  const opened: { port: MessagePort; replaces: Feed | null; feed: Feed; closes: number }[] = [];
  const open = (port: MessagePort, replaces: Feed | null): Feed => {
    const entry: typeof opened[number] = { port, replaces, feed: null, closes: 0 };
    entry.feed = { id: opened.length + 1, close: () => entry.closes++ };
    opened.push(entry);
    return entry.feed;
  };
  return { opened, open };
};

const tick = () => new Promise((r) => setTimeout(r, 0));

// The Data messages of the window's batches to `worker`.
const pushed = (worker: FakeWorker) =>
  worker
    .messages()
    .filter((m) => m.type === "data")
    .map((m) => {
      const b = parseBatch((m as { batch: ArrayBuffer }).batch);
      assert.ok(typeof b !== "string" && b.kind === "data");
      // Big messages by their length.
      return b.messages.map((d) => [
        d.data.byteLength > 64 ? d.data.byteLength : Array.from(d.data),
        d.segmentEnd,
        d.discontinuity,
      ]);
    });

describe("UnitsPlayer", () => {
  before(() => {
    globals.window = globalThis;
    globals.MediaSource = { canConstructInDedicatedWorker: true };
    globals.HTMLMediaElement = { HAVE_FUTURE_DATA: 3 };
  });

  after(() => {
    delete globals.window;
    delete globals.MediaSource;
    delete globals.HTMLMediaElement;
  });

  it("opens a feed for its worker and hands the worker the feed's other port", () => {
    const f = feeds();
    const { p, workers } = player(f.open);
    assert.strictEqual(f.opened.length, 1);
    assert.strictEqual(f.opened[0].replaces, null);
    const [{ message, transfer }] = workers[0].posted;
    assert.strictEqual(message.type, "open");
    // strictEqual narrows the message to the open one.
    const open = message;
    assert.strictEqual(open.mode, SinkMode.Worker);
    assert.ok(open.feed instanceof MessagePort);
    assert.notStrictEqual(open.feed, f.opened[0].port);
    assert.deepStrictEqual(transfer, [open.feed]);
    p.push(new Uint8Array([1]), true, false);
    assert.strictEqual(workers[0].posted.length, 1, "a fed player takes no pushes");
    p.destroy();
    f.opened[0].port.close();
    open.feed.close();
  });

  it("gives a restarted worker a feed that takes over from the last, and closes the last on destroy", () => {
    const f = feeds();
    const { p, workers } = player(f.open);
    workers[0].onerror({ message: "boom" });
    assert.ok(workers[0].terminated);
    assert.strictEqual(workers.length, 2);
    assert.strictEqual(f.opened.length, 2);
    assert.strictEqual(f.opened[1].replaces, f.opened[0].feed);
    assert.strictEqual(f.opened[0].closes, 0, "the new feed closes the old one itself");
    p.destroy();
    assert.deepStrictEqual(
      f.opened.map((o) => o.closes),
      [0, 1]
    );
    for (const o of f.opened) o.port.close();
    for (const w of workers) (w.posted[0].message as { feed: MessagePort }).feed.close();
  });

  it("relays the worker's acks of a feed back to it, and counts what it fed", () => {
    const f = feeds();
    const { p, workers } = player(f.open);
    workers[0].send({ type: "consumed", batches: 3 });
    const sent = workers[0].messages();
    assert.deepStrictEqual(sent[sent.length - 1], { type: "ack", batches: 3 });
    workers[0].send({ type: "outputs", at: 1, outputs: [], messages: 5, bytes: 70 });
    const s = p.stats();
    assert.deepStrictEqual([s.messages, s.bytes], [5, 70]);
    p.destroy();
    f.opened[0].port.close();
    (workers[0].posted[0].message as { feed: MessagePort }).feed.close();
  });

  it("posts the messages pushed in one task as one batch", async () => {
    const { p, workers } = player();
    p.push(new Uint8Array([1, 2]), false, false);
    p.push(new Uint8Array([3]), true, false);
    p.push(new Uint8Array([4]), true, false);
    assert.deepStrictEqual(pushed(workers[0]), []);
    await tick();
    assert.deepStrictEqual(pushed(workers[0]), [
      [
        [[1, 2], false, false],
        [[3], true, false],
        [[4], true, false],
      ],
    ]);
    p.destroy();
  });

  it("drops pushes to the next whole record while the worker is behind, which goes with a discontinuity", async () => {
    const { p, workers } = player();
    p.push(new Uint8Array(WINDOW_BYTES), true, false);
    assert.strictEqual(pushed(workers[0]).length, 1, "a full batch goes at once");
    p.push(new Uint8Array([1]), false, false);
    await tick();
    workers[0].send({ type: "consumed", batches: 1 });
    p.push(new Uint8Array([2]), true, false);
    p.push(new Uint8Array([3]), true, false);
    await tick();
    const batches = pushed(workers[0]);
    assert.deepStrictEqual(batches.slice(1), [[[[3], true, true]]]);
    assert.strictEqual(p.stats().dropped, 1);
    p.destroy();
  });

  it("ends the stream after what was pushed", () => {
    const { p, workers } = player();
    p.push(new Uint8Array([1]), true, false);
    p.end();
    p.end();
    p.push(new Uint8Array([2]), true, false);
    assert.deepStrictEqual(
      workers[0].messages().map((m) => m.type),
      ["open", "data", "end"]
    );
    p.destroy();
  });

  it("plays an ended stream out at rate 1, and restarts no worker after", () => {
    const { p, video, workers } = player();
    video.playbackRate = 1.05;
    workers[0].send({ type: "ended" });
    assert.ok(p.stats().ended);
    assert.strictEqual(video.playbackRate, 1);
    workers[0].onerror({ message: "late" });
    assert.strictEqual(workers.length, 1);
    p.destroy();
  });
});

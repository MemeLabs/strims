// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import { Feed, parseBatch } from "../feed";
import { cueTrack } from "../textTracks";
import { FakeCue, FakeTextTrack, FakeTextVideo } from "../textTracks.test";
import { WINDOW_BYTES } from "../units";
import { LowLatencyPlayer } from "./player";
import { CoreStats, Renderer, WindowMessage, WorkerMessage } from "./protocol";

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

class FakeVideo extends FakeTextVideo {
  disableRemotePlayback = false;
  srcObject: MediaStream | null = null;
  currentTime = 0;
}

const globals = globalThis as unknown as Record<string, unknown>;

// A player on a renderer the window needn't make, and its workers, each
// past its hello.
const player = (
  feed?: (port: MessagePort, replaces: Feed | null) => Feed,
  video = new FakeVideo()
) => {
  const workers: FakeWorker[] = [];
  const p = new LowLatencyPlayer(
    video as unknown as HTMLVideoElement,
    {
      attach: () => undefined,
      audioContext: null,
      renderer: Renderer.VideoTrackGenerator,
      feed,
    },
    () => {
      const w = new FakeWorker();
      workers.push(w);
      return w as unknown as Worker;
    }
  );
  return { p, workers };
};

const hello = async (w: FakeWorker) => {
  w.send({ type: "hello", videoTrackGenerator: true });
  await tick();
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

describe("LowLatencyPlayer", () => {
  before(() => {
    globals.window = globalThis;
  });

  after(() => {
    delete globals.window;
  });

  it("opens a feed for its worker and hands the worker the feed's other port as it opens", async () => {
    const f = feeds();
    const { p, workers } = player(f.open);
    assert.strictEqual(f.opened.length, 1);
    assert.strictEqual(f.opened[0].replaces, null);
    await hello(workers[0]);
    const [{ message, transfer }] = workers[0].posted;
    assert.ok(message.type === "open");
    assert.ok(message.feed instanceof MessagePort);
    assert.notStrictEqual(message.feed, f.opened[0].port);
    assert.deepStrictEqual(transfer, [message.feed]);
    p.push(new Uint8Array([1]), true, false);
    p.end();
    await tick();
    assert.strictEqual(workers[0].posted.length, 1, "a fed player takes no pushes or end");
    p.destroy();
    f.opened[0].port.close();
    message.feed.close();
  });

  it("gives a restarted worker a feed that takes over from the last, and closes the last on destroy", async () => {
    const f = feeds();
    const { p, workers } = player(f.open);
    await hello(workers[0]);
    workers[0].send({ type: "error", message: "boom", final: false, fallback: false });
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
    (workers[0].posted[0].message as { feed: MessagePort }).feed.close();
  });

  it("shows the worker's cues on the video's timeline, moved as the media drifts from it", () => {
    globals.VTTCue = FakeCue;
    const video = new FakeVideo();
    const { p, workers } = player(undefined, video);
    const stats = { ...p.stats() } as CoreStats;
    const state = (media: number | null, currentTime: number) => {
      video.currentTime = currentTime;
      const clock = { at: Date.now(), published: null, rate: 0, media, targetLag: 0.5, lag: null };
      workers[0].send({ type: "state", clock, stats, handoffs: [], messages: 0, bytes: 0 });
    };
    const list = () =>
      (
        cueTrack(video as unknown as HTMLVideoElement, "subtitles") as unknown as FakeTextTrack
      ).list.map((c) => [c.startTime, c.endTime]);
    // Before the player joins, cues wait for the video's timeline.
    workers[0].send({
      type: "cues",
      cues: [{ type: "cue", id: 1, kind: "subtitles", start: 100, end: 102, text: "a" }],
    });
    state(null, 0.5);
    assert.strictEqual(cueTrack(video as unknown as HTMLVideoElement, "subtitles"), null);
    // Media 99 plays at video time 1.
    state(99, 1);
    assert.deepStrictEqual(list(), [[2, 4]]);
    // Within the drift the cues stay; a stall past it moves them.
    state(99.5, 1.55);
    assert.deepStrictEqual(list(), [[2, 4]]);
    state(99.5, 2);
    assert.deepStrictEqual(list(), [[2.5, 4.5]]);
    workers[0].send({ type: "cues", cues: [{ type: "cueEnd", id: 1, end: 101 }] });
    assert.deepStrictEqual(list(), [[2.5, 3.5]]);
    p.destroy();
    assert.deepStrictEqual(list(), []);
    delete globals.VTTCue;
  });

  it("counts a fed worker's Data messages from its state", () => {
    const f = feeds();
    const { p, workers } = player(f.open);
    const clock = { at: 0, published: null, rate: 0, media: null, targetLag: 0.5, lag: null };
    const stats = { ...p.stats() } as CoreStats;
    workers[0].send({ type: "state", clock, stats, handoffs: [], messages: 5, bytes: 70 });
    const s = p.stats();
    assert.deepStrictEqual([s.messages, s.bytes], [5, 70]);
    p.destroy();
    f.opened[0].port.close();
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
    assert.deepStrictEqual(pushed(workers[0]).slice(1), [[[[3], true, true]]]);
    assert.strictEqual(p.stats().dropped, 1);
    p.destroy();
  });

  it("ends the stream after what was pushed, and a restarted worker hears the end", async () => {
    const { p, workers } = player();
    p.push(new Uint8Array([1]), true, false);
    p.end();
    p.end();
    p.push(new Uint8Array([2]), true, false);
    assert.deepStrictEqual(
      workers[0].messages().map((m) => m.type),
      ["data", "end"]
    );
    await hello(workers[0]);
    workers[0].send({ type: "error", message: "boom", final: false, fallback: false });
    assert.deepStrictEqual(
      workers[1].messages().map((m) => m.type),
      ["end"]
    );
    p.destroy();
  });

  it("feeds a restarted worker from the next whole record pushed", async () => {
    const { p, workers } = player();
    p.push(new Uint8Array([1]), false, false);
    await tick();
    workers[0].send({ type: "error", message: "boom", final: false, fallback: false });
    p.push(new Uint8Array([2]), true, false);
    p.push(new Uint8Array([3]), true, false);
    await tick();
    assert.deepStrictEqual(pushed(workers[1]), [[[[3], true, false]]]);
    p.destroy();
  });
});

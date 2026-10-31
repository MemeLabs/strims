// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import { normalizeRanges } from "./ranges";
import {
  BACK_BUFFER,
  MAX_APPEND_BYTES,
  MAX_QUEUED_BYTES,
  MediaSink,
  QUOTA_BACK_BUFFER,
  concat,
} from "./sink";

const AV = 'video/mp4; codecs="avc1.64001f,mp4a.40.2"';
const AV2 = 'video/mp4; codecs="avc1.640028,mp4a.40.2"';
const V = 'video/mp4; codecs="avc1.64001f"';

// A SourceBuffer that logs what it's asked and stays updating until the
// test finishes the update.
class FakeSourceBuffer extends EventTarget {
  updating = false;
  ranges: [number, number][] = [];
  quota = 0;

  constructor(private log: string[]) {
    super();
  }

  get buffered(): TimeRanges {
    const r = this.ranges;
    return { length: r.length, start: (i) => r[i][0], end: (i) => r[i][1] };
  }

  changeType(type: string): void {
    this.log.push(`changeType ${type}`);
  }

  appendBuffer(data: Uint8Array): void {
    if (this.quota > 0) {
      this.quota--;
      throw new DOMException("full", "QuotaExceededError");
    }
    this.log.push(`append ${data.join(",")}`);
    this.updating = true;
  }

  remove(start: number, end: number): void {
    this.log.push(`remove ${start} ${end}`);
    this.updating = true;
  }

  finish(): void {
    this.updating = false;
    this.dispatchEvent(new Event("updateend"));
  }
}

const sources: FakeMediaSource[] = [];

class FakeMediaSource extends EventTarget {
  static supported = (type: string) => type !== 'video/mp4; codecs="hev1"';
  log: string[] = [];
  duration = NaN;
  readyState: ReadyState = "closed";
  sb: FakeSourceBuffer | null = null;

  constructor() {
    super();
    sources.push(this);
  }

  static isTypeSupported(type: string): boolean {
    return FakeMediaSource.supported(type);
  }

  addSourceBuffer(type: string): FakeSourceBuffer {
    this.log.push(`addSourceBuffer ${type}`);
    this.sb = new FakeSourceBuffer(this.log);
    return this.sb;
  }

  endOfStream(): void {
    this.log.push("endOfStream");
    this.readyState = "ended";
  }

  open(): void {
    this.readyState = "open";
    this.dispatchEvent(new Event("sourceopen"));
  }
}

const bytes = (...b: number[]) => new Uint8Array(b);

const setup = () => {
  sources.length = 0;
  const attached: FakeMediaSource[] = [];
  const errors: string[] = [];
  const sink = new MediaSink(FakeMediaSource as unknown as typeof MediaSource, {
    attach: (ms) => attached.push(ms as unknown as FakeMediaSource),
    error: (e) => errors.push(e),
  });
  const ms = sources[0];
  ms.open();
  return { sink, ms, attached, errors };
};

describe("MediaSink", () => {
  it("makes the MediaSource's duration infinite", () => {
    const { ms } = setup();
    assert.strictEqual(ms.duration, Infinity);
  });

  it("appends the media queued while the SourceBuffer is busy as one buffer", () => {
    const { sink, ms } = setup();
    sink.init(bytes(1), AV, true);
    sink.media(bytes(2));
    sink.media(bytes(3));
    ms.sb.finish();
    ms.sb.finish();
    assert.deepStrictEqual(ms.log, [`addSourceBuffer ${AV}`, "append 1", "append 2,3"]);
  });

  it("appends at most MAX_APPEND_BYTES at once", () => {
    const { sink, ms } = setup();
    sink.init(bytes(1), AV, true);
    const half = new Uint8Array(MAX_APPEND_BYTES / 2 + 1);
    sink.media(half);
    sink.media(half);
    ms.sb.finish();
    ms.sb.finish();
    assert.strictEqual(ms.log.filter((l) => l.startsWith("append")).length, 3);
  });

  it("changes the type before every init after the first", () => {
    const { sink, ms } = setup();
    sink.init(bytes(1), AV, true);
    ms.sb.finish();
    sink.init(bytes(2), AV2, true);
    ms.sb.finish();
    sink.init(bytes(3), AV2, true);
    assert.deepStrictEqual(ms.log, [
      `addSourceBuffer ${AV}`,
      "append 1",
      `changeType ${AV2}`,
      "append 2",
      `changeType ${AV2}`,
      "append 3",
    ]);
  });

  it("puts an init it can't take on a new MediaSource, dropping what was queued", () => {
    const { sink, ms, attached } = setup();
    sink.init(bytes(1), AV, true);
    sink.media(bytes(2));
    sink.init(bytes(3), V, false);
    sink.media(bytes(4));
    assert.strictEqual(attached.length, 2);
    const next = attached[1];
    next.open();
    next.sb.finish();
    assert.deepStrictEqual(ms.log, [`addSourceBuffer ${AV}`, "append 1"]);
    assert.deepStrictEqual(next.log, [`addSourceBuffer ${V}`, "append 3", "append 4"]);
  });

  it("takes a first init on its MediaSource whatever its compatibility", () => {
    const { sink, attached } = setup();
    sink.init(bytes(1), AV, false);
    assert.strictEqual(attached.length, 1);
  });

  it("evicts media behind the back buffer in batches", () => {
    const { sink, ms } = setup();
    sink.init(bytes(1), AV, true);
    ms.sb.finish();
    ms.sb.ranges = [[0, 30]];
    sink.position(BACK_BUFFER + 1);
    assert.ok(!ms.log.some((l) => l.startsWith("remove")));
    sink.position(26);
    assert.ok(ms.log.includes(`remove 0 ${26 - BACK_BUFFER}`));
  });

  it("evicts and appends again when the quota refuses an append", () => {
    const { sink, ms } = setup();
    sink.init(bytes(1), AV, true);
    ms.sb.finish();
    ms.sb.ranges = [[0, 20]];
    sink.position(5);
    ms.sb.quota = 1;
    sink.media(bytes(2));
    assert.ok(ms.log.includes(`remove 0 ${5 - QUOTA_BACK_BUFFER}`));
    ms.sb.finish();
    assert.strictEqual(ms.log[ms.log.length - 1], "append 2");
  });

  it("fails when the quota refuses an append and nothing can go", () => {
    const { sink, ms, errors } = setup();
    sink.init(bytes(1), AV, true);
    ms.sb.finish();
    ms.sb.quota = 1;
    sink.media(bytes(2));
    assert.strictEqual(errors.length, 1);
  });

  it("fails for a type the browser can't play", () => {
    const { sink, errors } = setup();
    sink.init(bytes(1), 'video/mp4; codecs="hev1"', true);
    assert.strictEqual(errors.length, 1);
  });

  it("fails once more than MAX_QUEUED_BYTES wait", () => {
    const { sink, errors } = setup();
    sink.init(bytes(1), AV, true);
    for (let n = 0; n <= MAX_QUEUED_BYTES; n += 1 << 20) {
      sink.media(new Uint8Array(1 << 20));
    }
    assert.strictEqual(errors.length, 1);
  });

  it("fails when the SourceBuffer refuses an append", () => {
    const { sink, ms, errors } = setup();
    sink.init(bytes(1), AV, true);
    ms.sb.dispatchEvent(new Event("error"));
    assert.strictEqual(errors.length, 1);
    sink.media(bytes(2));
    assert.deepStrictEqual(ms.log, [`addSourceBuffer ${AV}`, "append 1"]);
  });

  it("ends the stream once what is queued is appended", () => {
    const { sink, ms } = setup();
    sink.init(bytes(1), AV, true);
    sink.media(bytes(2));
    sink.end();
    sink.end();
    assert.deepStrictEqual(ms.log, [`addSourceBuffer ${AV}`, "append 1"]);
    ms.sb.finish();
    ms.sb.finish();
    assert.deepStrictEqual(ms.log, [
      `addSourceBuffer ${AV}`,
      "append 1",
      "append 2",
      "endOfStream",
    ]);
  });

  it("ends no stream it appended nothing to", () => {
    const { sink, ms, errors } = setup();
    sink.end();
    assert.deepStrictEqual(ms.log, []);
    assert.deepStrictEqual(errors, []);
  });

  it("evicts nothing from an ended stream, and media after the end appends", () => {
    const { sink, ms } = setup();
    sink.init(bytes(1), AV, true);
    ms.sb.finish();
    sink.end();
    ms.sb.ranges = [[0, 30]];
    sink.position(26);
    assert.ok(!ms.log.some((l) => l.startsWith("remove")));
    sink.media(bytes(2));
    assert.strictEqual(ms.log[ms.log.length - 1], "append 2");
  });

  it("ends no stream when media follows the end it queued", () => {
    const { sink, ms } = setup();
    sink.init(bytes(1), AV, true);
    sink.end();
    sink.media(bytes(2));
    ms.sb.finish();
    assert.ok(!ms.log.includes("endOfStream"));
  });

  it("waits for its MediaSource to open", () => {
    sources.length = 0;
    const sink = new MediaSink(FakeMediaSource as unknown as typeof MediaSource, {
      attach: () => undefined,
      error: () => undefined,
    });
    sink.init(bytes(1), AV, true);
    assert.deepStrictEqual(sources[0].log, []);
    sources[0].open();
    assert.deepStrictEqual(sources[0].log, [`addSourceBuffer ${AV}`, "append 1"]);
  });
});

describe("concat", () => {
  it("joins buffers in order", () => {
    assert.deepStrictEqual([...concat([bytes(1, 2), bytes(), bytes(3)])], [1, 2, 3]);
  });
});

describe("normalizeRanges", () => {
  it("sorts ranges and merges the overlapping and touching ones", () => {
    assert.deepStrictEqual(
      normalizeRanges([
        [5, 7],
        [0, 2],
        [1, 3],
        [7, 8],
        [10, 10],
      ]),
      [
        [0, 3],
        [5, 8],
      ]
    );
  });
});

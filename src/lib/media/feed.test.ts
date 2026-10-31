// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import {
  BATCH_ACK,
  BATCH_DATA,
  BATCH_END,
  BatchWriter,
  Credit,
  DATA_DISCONTINUITY,
  DATA_RECORD_END,
  endBatch,
  parseBatch,
} from "./feed";

// A batch as the node writes it: its flags, then each message's flags and bytes.
const nodeBatch = (flags: number, ...messages: [number, number[]][]): ArrayBuffer => {
  const out: number[] = [BATCH_DATA, flags];
  for (const [f, bytes] of messages) {
    const len = bytes.length;
    out.push(f, len & 0xff, (len >> 8) & 0xff, (len >> 16) & 0xff, len >>> 24, ...bytes);
  }
  return new Uint8Array(out).buffer;
};

const plain = (batch: ReturnType<typeof parseBatch>) =>
  typeof batch === "string" || batch.kind === "end"
    ? batch
    : {
        ack: batch.ack,
        messages: batch.messages.map((m) => [Array.from(m.data), m.segmentEnd, m.discontinuity]),
      };

describe("feed batches", () => {
  it("parse into the node's Data messages, in order, as views of the batch", () => {
    const buffer = nodeBatch(
      BATCH_ACK,
      [DATA_RECORD_END, [1, 2, 3]],
      [0, [4]],
      [DATA_RECORD_END | DATA_DISCONTINUITY, []]
    );
    const batch = parseBatch(buffer);
    assert.deepStrictEqual(plain(batch), {
      ack: true,
      messages: [
        [[1, 2, 3], true, false],
        [[4], false, false],
        [[], true, true],
      ],
    });
    assert.ok(typeof batch !== "string" && batch.kind === "data");
    assert.strictEqual(batch.messages[0].data.buffer, buffer);
  });

  it("parse an end batch", () => {
    assert.deepStrictEqual(parseBatch(endBatch()), { kind: "end" });
    assert.deepStrictEqual(new Uint8Array(endBatch()), new Uint8Array([BATCH_END]));
  });

  it("that are cut short or of an unknown kind don't parse", () => {
    const whole = new Uint8Array(nodeBatch(0, [DATA_RECORD_END, [1, 2, 3]]));
    for (const bad of [
      whole.slice(0, whole.length - 1),
      whole.slice(0, 4),
      new Uint8Array([9, 0]),
      new Uint8Array([BATCH_END, 0]),
      new Uint8Array([]),
    ]) {
      assert.strictEqual(typeof parseBatch(bad.buffer), "string", Array.from(bad).join(","));
    }
  });

  it("the window writes parse back to its messages", () => {
    const w = new BatchWriter();
    assert.ok(w.empty);
    w.add({ data: new Uint8Array([1, 2]), segmentEnd: false, discontinuity: true });
    w.add({ data: new Uint8Array([3]), segmentEnd: true, discontinuity: false });
    assert.strictEqual(w.size, 2 + 5 + 2 + 5 + 1);
    const batch = w.take(false);
    assert.ok(w.empty);
    assert.strictEqual(w.size, 2);
    assert.deepStrictEqual(
      batch,
      nodeBatch(0, [DATA_DISCONTINUITY, [1, 2]], [DATA_RECORD_END, [3]])
    );
    assert.deepStrictEqual(plain(parseBatch(batch)), {
      ack: false,
      messages: [
        [[1, 2], false, true],
        [[3], true, false],
      ],
    });
    w.add({ data: new Uint8Array([4]), segmentEnd: true, discontinuity: false });
    assert.strictEqual(new Uint8Array(w.take(true))[1], BATCH_ACK);
  });
});

describe("feed credit", () => {
  it("closes at its batches and asks for acks every quarter and when it fills", () => {
    const c = new Credit(1000, 8);
    const asked: number[] = [];
    for (let i = 1; i <= 8; i++) {
      assert.ok(c.open, `open before batch ${i}`);
      if (c.post(1)) asked.push(i);
    }
    assert.ok(!c.open);
    assert.deepStrictEqual(asked, [2, 4, 6, 8]);
    c.ack(2);
    assert.ok(c.open);
    assert.deepStrictEqual(c.pending, [6, 6]);
  });

  it("closes at its bytes, and the batch that fills it asks", () => {
    const c = new Credit(1000, 100);
    assert.ok(!c.post(100));
    assert.ok(c.post(200), "a quarter of the bytes");
    assert.ok(c.post(1000), "the batch that fills the window");
    assert.ok(!c.open);
    c.ack(2);
    assert.ok(!c.open);
    c.ack(3);
    assert.ok(c.open);
    assert.deepStrictEqual(c.pending, [0, 0]);
  });

  it("counts acks past what was posted as all of it, and old acks as nothing", () => {
    const c = new Credit(1000, 100);
    c.post(10);
    c.post(20);
    c.ack(1);
    c.ack(0);
    assert.deepStrictEqual(c.pending, [20, 1]);
    c.ack(50);
    assert.deepStrictEqual(c.pending, [0, 0]);
    c.post(5);
    c.ack(3);
    assert.deepStrictEqual(c.pending, [0, 0]);
  });
});

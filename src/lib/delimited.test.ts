// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import { DelimitedDecoder, readDelimited } from "./delimited";

const uvarint = (n: number): number[] => {
  const out: number[] = [];
  while (n >= 0x80) {
    out.push((n & 0x7f) | 0x80);
    n = Math.floor(n / 0x80);
  }
  out.push(n);
  return out;
};

const frame = (body: number[]) => [...uvarint(body.length), ...body];

const bytes = (...frames: number[][]) => new Uint8Array(frames.flat());

const decodeIn = (stream: Uint8Array, chunkSize: number) => {
  const d = new DelimitedDecoder();
  const out: number[][] = [];
  for (let i = 0; i < stream.length; i += chunkSize) {
    for (const m of d.push(stream.subarray(i, i + chunkSize))) out.push(Array.from(m));
  }
  return { out, pending: d.pending };
};

const body = (n: number) => Array.from({ length: n }, (_, i) => i & 0xff);

describe("delimited decoder", () => {
  it("splits messages however the stream is chunked, empty ones included", () => {
    const messages = [[1, 2, 3], [], body(300), [9]];
    const stream = bytes(...messages.map(frame));
    for (const size of [1, 2, 3, 7, stream.length]) {
      const { out, pending } = decodeIn(stream, size);
      assert.deepEqual(out, messages, `chunks of ${size}`);
      assert.equal(pending, false);
    }
  });

  it("holds a partial message until its end arrives", () => {
    const d = new DelimitedDecoder();
    assert.deepEqual(d.push(new Uint8Array([3, 1])), []);
    assert.equal(d.pending, true);
    assert.deepEqual(Array.from(d.push(new Uint8Array([2, 3]))[0]), [1, 2, 3]);
    assert.equal(d.pending, false);
  });

  it("refuses a length over the bound, and a length too long to be one", () => {
    assert.throws(() => new DelimitedDecoder().push(new Uint8Array(uvarint(11)), 10));
    assert.throws(() => new DelimitedDecoder().push(new Uint8Array([0x80, 0x80, 0x80, 0x80, 1])));
  });
});

const response = (chunks: Uint8Array[], { hold = false, status = 200 } = {}) =>
  new Response(
    new ReadableStream<Uint8Array>({
      start: (controller) => {
        for (const c of chunks) controller.enqueue(c);
        if (!hold) controller.close();
      },
    }),
    { status }
  );

// A fetch answering `res()`, whose body, unlike a browser's, carries on
// after the request's signal aborts.
const fakeFetch =
  (res: () => Response) =>
  (_: RequestInfo | URL, init?: RequestInit): Promise<Response> =>
    init?.signal?.aborted ? Promise.reject(init.signal.reason) : Promise.resolve(res());

describe("reading a delimited stream", () => {
  it("hands over each message and resolves when the stream ends", async () => {
    const got: number[][] = [];
    await readDelimited("http://x/events", {
      signal: new AbortController().signal,
      idleTimeoutMs: 1000,
      onMessage: (m) => got.push(Array.from(m)),
      fetch: fakeFetch(() => response([bytes(frame([1]), frame([])), bytes(frame([2, 2]))])),
    });
    assert.deepEqual(got, [[1], [], [2, 2]]);
  });

  it("rejects a stream that ends inside a message, or answers another status", async () => {
    const read = (res: () => Response) =>
      readDelimited("http://x/events", {
        signal: new AbortController().signal,
        idleTimeoutMs: 1000,
        onMessage: () => undefined,
        fetch: fakeFetch(res),
      });
    await assert.rejects(read(() => response([new Uint8Array([3, 1])])));
    await assert.rejects(read(() => response([], { status: 503 })));
  });

  it("gives up on a stream that goes silent", async () => {
    const got: number[][] = [];
    await assert.rejects(
      readDelimited("http://x/events", {
        signal: new AbortController().signal,
        idleTimeoutMs: 50,
        onMessage: (m) => got.push(Array.from(m)),
        fetch: fakeFetch(() => response([bytes(frame([1]))], { hold: true })),
      }),
      /idle/
    );
    assert.deepEqual(got, [[1]]);
  });

  it("stops when its signal aborts", async () => {
    const aborter = new AbortController();
    const read = readDelimited("http://x/events", {
      signal: aborter.signal,
      idleTimeoutMs: 10_000,
      onMessage: () => aborter.abort(),
      fetch: fakeFetch(() => response([bytes(frame([1]))], { hold: true })),
    });
    await assert.rejects(read);
    assert.equal(aborter.signal.aborted, true);
  });

  it("closes the request and its body when a message or its handler throws", async () => {
    // A stream the server keeps open, as its pings do.
    const held = (chunk: number[]) => {
      const seen = { cancelled: false, signal: null as AbortSignal | null };
      const fetchImpl = (_url: string | URL | Request, init?: RequestInit) => {
        seen.signal = init?.signal ?? null;
        const body = new ReadableStream<Uint8Array>({
          start: (c) => c.enqueue(new Uint8Array(chunk)),
          cancel: () => {
            seen.cancelled = true;
          },
        });
        return Promise.resolve(new Response(body, { status: 200 }));
      };
      return { seen, fetch: fetchImpl as typeof fetch };
    };

    const thrown = held(frame([1]));
    await assert.rejects(
      readDelimited("http://x/events", {
        signal: new AbortController().signal,
        idleTimeoutMs: 10_000,
        onMessage: () => {
          throw new Error("bad message");
        },
        fetch: thrown.fetch,
      }),
      /bad message/
    );
    assert.deepEqual([thrown.seen.cancelled, thrown.seen.signal?.aborted], [true, true]);

    const tooLong = held([0xff, 0xff, 0xff, 0xff, 0x01]);
    await assert.rejects(
      readDelimited("http://x/events", {
        signal: new AbortController().signal,
        idleTimeoutMs: 10_000,
        onMessage: () => undefined,
        fetch: tooLong.fetch,
      }),
      /too long/
    );
    assert.deepEqual([tooLong.seen.cancelled, tooLong.seen.signal?.aborted], [true, true]);
  });
});

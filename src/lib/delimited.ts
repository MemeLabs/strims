// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

/** The longest message a delimited stream may hold. */
export const MAX_MESSAGE_BYTES = 16 * 1024 * 1024;

// A uvarint holding a length under MAX_MESSAGE_BYTES takes at most this many bytes.
const MAX_LENGTH_BYTES = 4;

/**
 * Splits a byte stream into messages, each a uvarint length then that many
 * bytes, across whatever chunks the stream arrives in.
 */
export class DelimitedDecoder {
  private buffered = new Uint8Array(0);

  /**
   * The whole messages `chunk` completes, in order; a partial one waits for
   * the next chunk. Throws on a length over `maxBytes`, after which the
   * decoder is unusable.
   */
  push(chunk: Uint8Array, maxBytes = MAX_MESSAGE_BYTES): Uint8Array[] {
    let buf = chunk;
    if (this.buffered.length > 0) {
      buf = new Uint8Array(this.buffered.length + chunk.length);
      buf.set(this.buffered);
      buf.set(chunk, this.buffered.length);
    }

    const messages: Uint8Array[] = [];
    let pos = 0;
    for (;;) {
      let length = 0;
      let shift = 0;
      let i = pos;
      let complete = false;
      while (i < buf.length) {
        const b = buf[i++];
        length += (b & 0x7f) * 2 ** shift;
        shift += 7;
        if ((b & 0x80) === 0) {
          complete = true;
          break;
        }
        if (i - pos >= MAX_LENGTH_BYTES) {
          throw new Error("delimited message too long");
        }
      }
      if (!complete) break;
      if (length > maxBytes) {
        throw new Error("delimited message too long");
      }
      if (buf.length - i < length) break;
      messages.push(buf.subarray(i, i + length));
      pos = i + length;
    }

    this.buffered = buf.slice(pos);
    return messages;
  }

  /** Whether part of a message is buffered. */
  get pending(): boolean {
    return this.buffered.length > 0;
  }
}

export interface ReadDelimitedOptions {
  signal: AbortSignal;
  /** Each message, as it completes. */
  onMessage: (message: Uint8Array) => void;
  /** Gives up once the stream sends nothing for this long. */
  idleTimeoutMs: number;
  fetch?: typeof fetch;
}

/**
 * Reads a delimited stream from `url` until it ends, fails, idles out or
 * `signal` aborts. Resolves once the stream ends; rejects when it can't be
 * read, its status isn't 200, or it idles out.
 */
export const readDelimited = async (
  url: string,
  { signal, onMessage, idleTimeoutMs, fetch: fetchImpl = fetch }: ReadDelimitedOptions
): Promise<void> => {
  const aborter = new AbortController();
  const abort = () => aborter.abort(signal.reason);
  if (signal.aborted) abort();
  signal.addEventListener("abort", abort);

  let idle = false;
  let timer: ReturnType<typeof setTimeout>;
  const renew = () => {
    clearTimeout(timer);
    timer = setTimeout(() => {
      idle = true;
      aborter.abort(new Error("stream idle"));
    }, idleTimeoutMs);
  };

  // An abort, ours or the caller's, ends the read in progress itself: a
  // body's read needn't fail when its request's signal aborts.
  let reader: ReadableStreamDefaultReader<Uint8Array> | null = null;
  const cancel = () => void reader?.cancel().catch(() => undefined);
  aborter.signal.addEventListener("abort", cancel, { once: true });
  const stopped = () =>
    idle
      ? new Error("stream idle")
      : signal.reason instanceof Error
        ? signal.reason
        : new Error("stream aborted");

  try {
    renew();
    const res = await fetchImpl(url, { signal: aborter.signal, cache: "no-store" });
    if (res.status !== 200 || !res.body) {
      throw new Error(`stream answered ${res.status}`);
    }
    reader = res.body.getReader();
    if (aborter.signal.aborted) cancel();
    const decoder = new DelimitedDecoder();
    for (;;) {
      const { done, value } = await reader.read();
      if (done || aborter.signal.aborted) break;
      renew();
      for (const message of decoder.push(value)) {
        onMessage(message);
        if (aborter.signal.aborted) break;
      }
    }
    if (aborter.signal.aborted) throw stopped();
    if (decoder.pending) {
      throw new Error("stream ended inside a message");
    }
  } catch (e) {
    throw aborter.signal.aborted ? stopped() : e;
  } finally {
    clearTimeout(timer);
    signal.removeEventListener("abort", abort);
    // However the read ended, its request and body end with it: a stream
    // the server keeps alive with pings would otherwise stay open.
    if (!aborter.signal.aborted) aborter.abort(new Error("stream closed"));
    aborter.signal.removeEventListener("abort", cancel);
  }
};

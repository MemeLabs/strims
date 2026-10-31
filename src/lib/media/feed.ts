// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// A unit-format stream's Data messages on their way to a player worker, in
// batches: from a wasm node's feed straight from the swarm (crates/strims-wasm
// feed module), or from the window for a stream it reads over the Egress RPC.
//
// A batch is one ArrayBuffer:
//
//   kind      u8   BATCH_DATA or BATCH_END
//   flags     u8   BATCH_DATA only; BATCH_ACK: the sender asks for an ack
//   messages  BATCH_DATA only, until the end of the buffer:
//     flags   u8   DATA_RECORD_END, DATA_DISCONTINUITY
//     len     u32  little-endian
//     bytes   len bytes
//
// Each message is an Egress Data message. An end batch comes last: the
// stream ended. The worker acks each batch that asks with how many batches
// it has taken, once their outputs are taken; a sender keeps its unacked
// batches within a window (Credit).

export const BATCH_DATA = 1;
export const BATCH_END = 2;
export const BATCH_ACK = 0x01;
export const DATA_RECORD_END = 0x01;
export const DATA_DISCONTINUITY = 0x02;

const BATCH_HEADER = 2;
const MESSAGE_HEADER = 5;

/** The most bytes of messages the window puts in one batch. */
export const MAX_BATCH_BYTES = 256 * 1024;

/** An Egress Data message. */
export interface DataMessage {
  data: Uint8Array<ArrayBuffer>;
  segmentEnd: boolean;
  discontinuity: boolean;
}

export type Batch = { kind: "data"; ack: boolean; messages: DataMessage[] } | { kind: "end" };

// A batch's messages, as views of `buffer`; a string says why it doesn't parse.
export const parseBatch = (buffer: ArrayBuffer): Batch | string => {
  const bytes = new Uint8Array(buffer);
  if (bytes[0] === BATCH_END && bytes.byteLength === 1) {
    return { kind: "end" };
  }
  if (bytes[0] !== BATCH_DATA || bytes.byteLength < BATCH_HEADER) {
    return `a batch of kind ${bytes[0]} and ${bytes.byteLength} bytes`;
  }
  const view = new DataView(buffer);
  const messages: DataMessage[] = [];
  let at = BATCH_HEADER;
  while (at < bytes.byteLength) {
    if (at + MESSAGE_HEADER > bytes.byteLength) {
      return "a message header cut short";
    }
    const flags = bytes[at];
    const len = view.getUint32(at + 1, true);
    const start = at + MESSAGE_HEADER;
    if (start + len > bytes.byteLength) {
      return "a message cut short";
    }
    messages.push({
      data: bytes.subarray(start, start + len),
      segmentEnd: (flags & DATA_RECORD_END) !== 0,
      discontinuity: (flags & DATA_DISCONTINUITY) !== 0,
    });
    at = start + len;
  }
  return { kind: "data", ack: (bytes[1] & BATCH_ACK) !== 0, messages };
};

// The end batch.
export const endBatch = (): ArrayBuffer => new Uint8Array([BATCH_END]).buffer;

// Gathers Data messages into a data batch.
export class BatchWriter {
  private messages: DataMessage[] = [];
  private bytes = BATCH_HEADER;

  // The batch's bytes so far.
  get size(): number {
    return this.bytes;
  }

  get empty(): boolean {
    return this.messages.length === 0;
  }

  // Adds a message, which the batch copies when it's taken.
  add(message: DataMessage): void {
    this.messages.push(message);
    this.bytes += MESSAGE_HEADER + message.data.byteLength;
  }

  // The batch, asking for an ack if `ack`; the writer starts another.
  take(ack: boolean): ArrayBuffer {
    const out = new Uint8Array(this.bytes);
    const view = new DataView(out.buffer);
    out[0] = BATCH_DATA;
    out[1] = ack ? BATCH_ACK : 0;
    let at = BATCH_HEADER;
    for (const { data, segmentEnd, discontinuity } of this.messages) {
      out[at] = (segmentEnd ? DATA_RECORD_END : 0) | (discontinuity ? DATA_DISCONTINUITY : 0);
      view.setUint32(at + 1, data.byteLength, true);
      out.set(data, at + MESSAGE_HEADER);
      at += MESSAGE_HEADER + data.byteLength;
    }
    this.messages = [];
    this.bytes = BATCH_HEADER;
    return out.buffer;
  }
}

/**
 * The batches a sender posted and the worker hasn't acked, within
 * `maxBytes` and `maxBatches`. A batch asks for an ack every quarter of
 * either, and when it fills the window, so a full window always gets one.
 */
export class Credit {
  private unacked: number[] = [];
  private bytes = 0;
  private posted = 0;
  private acked = 0;
  private unaskedBytes = 0;
  private unaskedBatches = 0;

  constructor(readonly maxBytes: number, readonly maxBatches: number) {}

  // Another batch may be posted.
  get open(): boolean {
    return this.bytes < this.maxBytes && this.unacked.length < this.maxBatches;
  }

  // Counts a batch of `bytes` as posted: whether it asks for an ack.
  post(bytes: number): boolean {
    this.unacked.push(bytes);
    this.bytes += bytes;
    this.posted++;
    this.unaskedBytes += bytes;
    this.unaskedBatches++;
    const ask =
      !this.open ||
      this.unaskedBytes >= this.maxBytes / 4 ||
      this.unaskedBatches >= this.maxBatches / 4;
    if (ask) {
      this.unaskedBytes = 0;
      this.unaskedBatches = 0;
    }
    return ask;
  }

  // The worker has taken `taken` batches, counted from the first.
  ack(taken: number): void {
    const n = Math.min(taken, this.posted);
    while (this.acked < n) {
      this.acked++;
      this.bytes -= this.unacked.shift() ?? 0;
    }
  }

  // Bytes and batches posted and not yet acked.
  get pending(): [number, number] {
    return [this.bytes, this.unacked.length];
  }
}

/**
 * The Data messages a window pushes a player worker, for a stream it reads
 * over the Egress RPC: those pushed in one task go as one batch, at most
 * MAX_BATCH_BYTES, within a Credit window of unacked batches. Past the
 * window, messages are dropped to the next whole record, which goes with a
 * discontinuity, so a worker that falls behind holds a bounded queue.
 */
export class WindowBatches {
  private batch = new BatchWriter();
  private queued = false;
  private credit: Credit;
  /** Messages are dropped until a whole record starts. */
  private skipping = false;
  /** The next message starts a whole record. */
  private atRecordStart = true;
  /** The next message goes with a discontinuity: those before it were dropped. */
  private dropDiscontinuity = false;
  /** Messages dropped for the worker falling behind. */
  dropped = 0;

  /** Posts each batch to the worker, asking for an ack if `ack`. */
  constructor(
    private readonly maxBytes: number,
    private readonly maxBatches: number,
    private post: (batch: ArrayBuffer) => void
  ) {
    this.credit = new Credit(maxBytes, maxBatches);
  }

  // Takes a Data message, which the batch copies when it's posted.
  push({ data, segmentEnd, discontinuity }: DataMessage): void {
    if (discontinuity) this.skipping = false;
    const skip = this.skipping;
    this.skipping &&= !segmentEnd;
    this.atRecordStart = segmentEnd;
    if (skip) {
      // The record in progress began before this worker, or was dropped.
      return;
    }
    if (!this.credit.open) {
      // The worker fell behind: the record goes, and the next whole one
      // tells the worker so.
      this.dropped++;
      this.skipping = !segmentEnd;
      this.dropDiscontinuity = true;
      return;
    }
    this.batch.add({
      data,
      segmentEnd,
      discontinuity: discontinuity || this.dropDiscontinuity,
    });
    this.dropDiscontinuity = false;
    if (this.batch.size >= MAX_BATCH_BYTES) {
      this.flush();
    } else if (!this.queued) {
      this.queued = true;
      queueMicrotask(this.flush);
    }
  }

  // Posts the messages batched so far.
  flush = (): void => {
    this.queued = false;
    if (this.batch.empty) return;
    const ask = this.credit.post(this.batch.size);
    this.post(this.batch.take(ask));
  };

  // The worker took `batches` batches, counted from the first.
  ack(batches: number): void {
    this.credit.ack(batches);
  }

  // A new worker, posted to with `post`: its window starts empty, and its
  // messages from the next whole record on.
  restart(post: (batch: ArrayBuffer) => void): void {
    this.post = post;
    this.credit = new Credit(this.maxBytes, this.maxBatches);
    this.batch = new BatchWriter();
    this.skipping = !this.atRecordStart;
    this.dropDiscontinuity = false;
  }
}

/** A stream to read: the Egress RPC's OpenStream request's fields. */
export interface StreamRequest {
  swarmUri: string;
  networkKeys: Uint8Array[];
  mimeType: string;
}

/** A stream's feed to a player worker, straight from a node's swarm. */
export interface Feed {
  readonly id: number;
  /** Closes the stream; the port gets nothing more. */
  close(): void;
}

export interface FeedOptions {
  /** The feed this one takes over from, which closes once this one's stream is open. */
  replaces?: Feed;
  /** The feed ended: its stream didn't open, failed or ended, or its port closed. */
  closed?: (message: string) => void;
}

/** Opens feeds on a node that can post to a player worker's port itself. */
export interface FeedOpener {
  /** Feeds `request`'s stream to `port`, which it transfers. */
  open(request: StreamRequest, port: MessagePort, options?: FeedOptions): Feed;
}

// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Range, timeRanges } from "./ranges";

/** Seconds of media kept behind the playback position. */
export const BACK_BUFFER = 10;
/** Media behind the back buffer is evicted once this many more seconds pile up. */
export const EVICT_SLACK = 5;
/** Seconds kept behind the playback position when the browser's quota is reached. */
export const QUOTA_BACK_BUFFER = 1;
/** Appends refused for quota in a row before the sink fails. */
export const MAX_QUOTA_RETRIES = 3;
/** The most bytes one append takes: queued media segments are appended together up to it. */
export const MAX_APPEND_BYTES = 4 * 1024 * 1024;
/** The most bytes queued for appending; past it the sink fails. */
export const MAX_QUEUED_BYTES = 64 * 1024 * 1024;

export interface MediaSourceConstructor {
  new (): MediaSource;
  isTypeSupported(type: string): boolean;
}

export interface SinkEvents {
  /** A MediaSource for the video: the first, and a new one after each rebuild. */
  attach(ms: MediaSource): void;
  /** The sink can't go on; it takes nothing more. */
  error(message: string): void;
}

type Bytes = Uint8Array<ArrayBuffer>;

type Op =
  | { type: "init"; data: Bytes; mime: string }
  | { type: "media"; data: Bytes }
  | { type: "remove"; end: number };

// Concatenates byte arrays into one.
export const concat = (parts: Bytes[]): Bytes => {
  if (parts.length === 1) {
    return parts[0];
  }
  const out = new Uint8Array(parts.reduce((n, b) => n + b.byteLength, 0));
  let at = 0;
  for (const b of parts) {
    out.set(b, at);
    at += b.byteLength;
  }
  return out;
};

/**
 * Appends a remuxed stream's init and media segments to one SourceBuffer of
 * a MediaSource or ManagedMediaSource, in a window or a worker.
 *
 * - Segments queue while the SourceBuffer is busy, and queued media
 *   segments are appended together, up to MAX_APPEND_BYTES.
 * - Every init after the first is preceded by `changeType` with its type;
 *   an init the SourceBuffer can't take (a different set of tracks) gets a
 *   new MediaSource, since Chromium adds no SourceBuffer to a MediaSource
 *   that has had an init.
 * - The duration is infinite: a live stream's buffered end grows without
 *   a `durationchange` per append.
 * - Media more than BACK_BUFFER seconds behind the playback position goes
 *   in batches of EVICT_SLACK seconds. An append refused for quota evicts
 *   down to QUOTA_BACK_BUFFER and is retried.
 *
 * Bounds: MAX_QUEUED_BYTES queued. An append error, an unsupported type,
 * a full queue or a quota that eviction can't meet fail the sink.
 */
export class MediaSink {
  private ms: MediaSource;
  private sb: SourceBuffer | null = null;
  private open = false;
  private ops: Op[] = [];
  private queuedBytes = 0;
  private currentTime = 0;
  /** An eviction is queued or running. */
  private evicting = false;
  /** The running operation is a removal. */
  private removing = false;
  private quotaRetries = 0;
  private done = false;

  constructor(
    private readonly MediaSourceType: MediaSourceConstructor,
    private readonly events: SinkEvents
  ) {
    this.rebuild();
  }

  init(data: Bytes, mime: string, compatible: boolean): void {
    if (this.done) return;
    if (!this.MediaSourceType.isTypeSupported(mime)) {
      this.fail(`the browser can't play ${mime}`);
      return;
    }
    const initialized = this.sb !== null || this.ops.some((op) => op.type === "init");
    if (!compatible && initialized) {
      this.rebuild();
    }
    this.enqueue({ type: "init", data, mime });
  }

  media(data: Bytes): void {
    if (this.done) return;
    this.enqueue({ type: "media", data });
  }

  // The video's playback position, which bounds eviction.
  position(currentTime: number): void {
    if (this.done) return;
    this.currentTime = currentTime;
    const [first] = this.buffered();
    if (!this.evicting && first && first[0] < currentTime - BACK_BUFFER - EVICT_SLACK) {
      this.evicting = true;
      this.enqueue({ type: "remove", end: currentTime - BACK_BUFFER });
    }
  }

  /** Stops appending; the owner detaches the MediaSource from the video. */
  close(): void {
    this.done = true;
    this.ops = [];
    this.queuedBytes = 0;
    this.detach();
  }

  private buffered(): Range[] {
    try {
      return this.sb ? timeRanges(this.sb.buffered) : [];
    } catch {
      return [];
    }
  }

  private rebuild(): void {
    this.detach();
    this.ops = [];
    this.queuedBytes = 0;
    this.evicting = false;
    this.removing = false;
    this.quotaRetries = 0;
    this.open = false;
    this.ms = new this.MediaSourceType();
    this.ms.addEventListener("sourceopen", this.onSourceOpen);
    this.events.attach(this.ms);
  }

  private detach(): void {
    this.ms?.removeEventListener("sourceopen", this.onSourceOpen);
    this.sb?.removeEventListener("updateend", this.onUpdateEnd);
    this.sb?.removeEventListener("error", this.onError);
    this.sb = null;
  }

  private onSourceOpen = () => {
    this.open = true;
    try {
      this.ms.duration = Infinity;
    } catch {
      // The duration only bounds seeking, which the player keeps within
      // the buffered range anyway.
    }
    this.pump();
  };

  private onUpdateEnd = () => {
    if (this.removing) {
      this.removing = false;
      this.evicting = false;
    }
    this.pump();
  };

  private onError = () => {
    this.fail("the SourceBuffer refused an append");
  };

  private fail(message: string): void {
    if (this.done) return;
    this.close();
    this.events.error(message);
  }

  private enqueue(op: Op): void {
    if (op.type !== "remove") {
      this.queuedBytes += op.data.byteLength;
      if (this.queuedBytes > MAX_QUEUED_BYTES) {
        this.fail("the SourceBuffer fell behind its stream");
        return;
      }
    }
    this.ops.push(op);
    this.pump();
  }

  /** Starts the next queued operation once the SourceBuffer is idle. */
  private pump(): void {
    while (!this.done && this.open && !this.sb?.updating && this.ops.length) {
      const op = this.ops.shift();
      if (op.type === "remove") {
        if (this.remove(0, op.end)) return;
        this.evicting = false;
        continue;
      }
      let data = op.data;
      let taken = 1;
      if (op.type === "media") {
        if (!this.sb) {
          // Media before any init has no SourceBuffer to go to.
          this.queuedBytes -= op.data.byteLength;
          continue;
        }
        const parts = [op.data];
        let bytes = op.data.byteLength;
        while (this.ops[0]?.type === "media") {
          const next = this.ops[0];
          if (bytes + next.data.byteLength > MAX_APPEND_BYTES) break;
          this.ops.shift();
          parts.push(next.data);
          bytes += next.data.byteLength;
          taken++;
        }
        data = concat(parts);
      }
      try {
        if (op.type === "init") {
          if (this.sb) {
            this.sb.changeType(op.mime);
          } else {
            this.sb = this.ms.addSourceBuffer(op.mime);
            this.sb.addEventListener("updateend", this.onUpdateEnd);
            this.sb.addEventListener("error", this.onError);
          }
        }
        this.sb.appendBuffer(data);
        this.queuedBytes -= data.byteLength;
        this.quotaRetries = 0;
      } catch (e) {
        if ((e as DOMException)?.name === "QuotaExceededError" && this.sb) {
          // The init's changeType already ran; it repeats harmlessly.
          this.ops.unshift(taken > 1 || op.type === "media" ? { type: "media", data } : op);
          if (++this.quotaRetries > MAX_QUOTA_RETRIES) {
            this.fail("the browser's media quota is full");
            return;
          }
          if (this.remove(0, this.currentTime - QUOTA_BACK_BUFFER)) return;
          this.fail("the browser's media quota is full");
          return;
        }
        this.fail(`append failed: ${String(e)}`);
        return;
      }
    }
  }

  // Removes buffered media in [start, end); false when there is none.
  private remove(start: number, end: number): boolean {
    const [first] = this.buffered();
    if (!this.sb || !first || first[0] >= end) return false;
    try {
      this.sb.remove(start, end);
      this.removing = true;
      return true;
    } catch (e) {
      this.fail(`evicting failed: ${String(e)}`);
      return true;
    }
  }
}

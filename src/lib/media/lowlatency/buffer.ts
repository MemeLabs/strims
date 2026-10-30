// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { DemuxedUnitOutput } from "./protocol";

/** The most payload bytes the jitter buffer holds. */
export const MAX_BUFFER_BYTES = 64 * 1024 * 1024;
/** The most seconds of a track's decode times the jitter buffer holds. */
export const MAX_BUFFER_SECONDS = 30;

type Unit = Omit<DemuxedUnitOutput, "type">;

/** A queue that drops from its head without moving the rest each time. */
class Queue<T> {
  private items: T[] = [];
  private head = 0;

  get length(): number {
    return this.items.length - this.head;
  }

  at(i: number): T | undefined {
    return this.items[this.head + i];
  }

  push(x: T): void {
    this.items.push(x);
  }

  shift(): T | undefined {
    const x = this.items[this.head];
    if (x === undefined) return undefined;
    this.items[this.head++] = undefined;
    if (this.head > 64 && this.head * 2 > this.items.length) {
      this.items = this.items.slice(this.head);
      this.head = 0;
    }
    return x;
  }
}

/**
 * The jitter buffer: each track's units not yet decoded, in decode order.
 *
 * Bounds: MAX_BUFFER_BYTES of payloads and MAX_BUFFER_SECONDS of each
 * track's decode times; past them, a video track drops its oldest GOP (to
 * its next keyframe) and an audio track its oldest unit.
 */
export class UnitBuffer {
  private tracks = new Map<number, { video: boolean; units: Queue<Unit> }>();
  public bytes = 0;
  /** Units dropped for the bounds. */
  public evicted = 0;

  push(u: Unit): void {
    let t = this.tracks.get(u.track);
    if (!t) {
      t = { video: u.kind === "video", units: new Queue() };
      this.tracks.set(u.track, t);
    }
    t.units.push(u);
    this.bytes += u.data.byteLength;
    for (const [track, q] of this.tracks) {
      while (
        q.units.length > 1 &&
        (this.bytes > MAX_BUFFER_BYTES ||
          q.units.at(q.units.length - 1).decodeTimestamp - q.units.at(0).decodeTimestamp >
            MAX_BUFFER_SECONDS * 1e6)
      ) {
        this.evicted += this.dropHead(track, q.video);
      }
    }
  }

  // The next unit of `track` to decode.
  peek(track: number): Unit | undefined {
    return this.tracks.get(track)?.units.at(0);
  }

  shift(track: number): Unit | undefined {
    const u = this.tracks.get(track)?.units.shift();
    if (u) this.bytes -= u.data.byteLength;
    return u;
  }

  // Units of `track` held.
  count(track: number): number {
    return this.tracks.get(track)?.units.length ?? 0;
  }

  // Drops `track`'s units before its latest keyframe that presents at or
  // before `us`, and answers whether it had one; holds everything without.
  seekKey(track: number, us: number): boolean {
    const q = this.tracks.get(track)?.units;
    if (!q) return false;
    let key = -1;
    for (let i = 0; i < q.length; i++) {
      const u = q.at(i);
      if (u.keyframe && u.timestamp <= us) key = i;
    }
    if (key < 0) return false;
    for (let i = 0; i < key; i++) this.shift(track);
    return true;
  }

  // Whether `track` holds a keyframe.
  hasKey(track: number): boolean {
    return this.firstKey(track) !== null;
  }

  // The presentation time of `track`'s first keyframe held, in microseconds.
  firstKey(track: number): number | null {
    const q = this.tracks.get(track)?.units;
    if (!q) return null;
    for (let i = 0; i < q.length; i++) {
      if (q.at(i).keyframe) return q.at(i).timestamp;
    }
    return null;
  }

  // Drops `track`'s units that end before `us`; answers how many.
  dropBefore(track: number, us: number): number {
    let n = 0;
    for (let u = this.peek(track); u && u.timestamp + u.duration < us; u = this.peek(track)) {
      this.shift(track);
      n++;
    }
    return n;
  }

  // Drops `track`'s units up to its next keyframe; answers how many.
  dropToKey(track: number): number {
    let n = 0;
    for (let u = this.peek(track); u && !(n > 0 && u.keyframe); u = this.peek(track)) {
      this.shift(track);
      n++;
    }
    return n;
  }

  clear(): void {
    this.tracks.clear();
    this.bytes = 0;
  }

  private dropHead(track: number, video: boolean): number {
    return video ? this.dropToKey(track) : this.shift(track) ? 1 : 0;
  }
}

// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import FenwickTree from "./FenwickTree";

class MessageSizeCache {
  private settled: boolean[] = [];
  private margins: number[] = [];
  private heights: number[] = [];
  private offsets: FenwickTree;
  private lastSet = 0;
  public onchange?: () => void;

  constructor(private size: number, private estimate: number) {
    this.size = size = this.roundSize(size);
    this.settled = new Array<boolean>(size).fill(false);
    this.margins = new Array<number>(size * 2).fill(0);
    this.heights = new Array<number>(size).fill(estimate);
    this.offsets = new FenwickTree(this.heights);
  }

  private roundSize(size: number) {
    return Math.pow(2, Math.ceil(Math.log2(size)));
  }

  public grow(size: number) {
    if (size > this.size) {
      size = this.roundSize(size);
      const d = size - this.size;
      this.settled.push(...new Array<boolean>(d).fill(false));
      this.margins.push(...new Array<number>(d * 2).fill(0));
      this.heights.push(...new Array<number>(d).fill(this.estimate));
      this.offsets.push(...new Array<number>(d).fill(this.estimate));
      this.size = size;
    }
  }

  public reset() {
    this.settled.fill(false);
    this.margins.fill(0);
    this.heights.fill(this.estimate);
    this.offsets = new FenwickTree(this.heights);
    this.lastSet = 0;
  }

  public unset(i: number) {
    if (i > 0 && i < this.size) {
      this.set(i, false, this.estimate, 0, 0);
    }
  }

  public prune(n: number) {
    n = Math.min(n, this.size);
    this.settled.copyWithin(0, n).fill(false, this.size - n);
    this.margins.copyWithin(0, n * 2).fill(0, (this.size - n) * 2);
    this.heights.copyWithin(0, n).fill(this.estimate, this.size - n);
    this.offsets = new FenwickTree(this.heights);
    this.lastSet -= n;
  }

  public remove(i: number, n: number = 1) {
    if (i > this.lastSet) {
      return;
    }
    this.settled.copyWithin(i, i + n, this.lastSet);
    this.settled.fill(false, this.lastSet - n, this.lastSet);
    this.margins.copyWithin(i * 2, (i + n) * 2, this.lastSet * 2);
    this.margins.fill(0, this.lastSet * 2 - n * 2, this.lastSet * 2);
    this.heights.copyWithin(i, i + n, this.lastSet);
    this.heights.fill(this.estimate, this.lastSet - n, this.lastSet);
    this.lastSet -= n;

    let offsetChanged = false;
    for (let j = i; j < this.lastSet + n; j++) {
      offsetChanged ||= this.syncOffset(j);
    }
    if (offsetChanged) {
      this.onchange?.();
    }
  }

  public insert(i: number, n: number = 1) {
    if (i > this.lastSet) {
      return;
    }
    this.settled.copyWithin(i + n, i);
    this.settled.fill(false, i, i + n);
    this.margins.copyWithin((i + n) * 2, i * 2);
    this.margins.fill(0, i * 2, (i + n) * 2);
    this.heights.copyWithin(i + n, i);
    this.heights.fill(this.estimate, i, i + n);
    this.lastSet += n;

    let offsetChanged = false;
    for (let j = i; j < this.lastSet; j++) {
      offsetChanged ||= this.syncOffset(j);
    }
    if (offsetChanged) {
      this.onchange?.();
    }
  }

  public set(i: number, settled: boolean, height: number, marginTop: number, marginBottom: number) {
    this.settled[i] = settled;
    this.margins[i * 2] = marginTop;
    this.margins[i * 2 + 1] = marginBottom;
    this.heights[i] = height;
    this.lastSet = Math.max(this.lastSet, i);

    const offsetChanged = this.syncOffset(i);
    const nextOffsetChange = i + 1 < this.size && this.syncOffset(i + 1);
    if (offsetChanged || nextOffsetChange) {
      this.onchange?.();
    }
  }

  private collapseMargins(a: number, b: number) {
    if (a < 0 && b < 0) return Math.min(a, b);
    if (a >= 0 && b >= 0) return Math.max(a, b);
    return a + b;
  }

  private syncOffset(i: number) {
    const margin = this.collapseMargins(this.margins[i * 2 - 1] ?? 0, this.margins[i * 2]);
    const height = this.heights[i] + margin;
    if (this.offsets.get(i) !== height) {
      this.offsets.set(i, height);
      return true;
    }
    return false;
  }

  public isSettled(i: number) {
    return this.settled[i];
  }

  public getOffset(i: number) {
    return this.offsets.prefixSum(i);
  }

  public findIndex(off: number) {
    return this.offsets.rankQuery(off);
  }
}

export default MessageSizeCache;

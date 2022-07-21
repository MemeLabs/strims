// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import FenwickTree from "./FenwickTree";

class MessageSizeCache {
  private settled: boolean[] = [];
  private margins: number[] = [];
  private heights: number[] = [];
  private offsets: FenwickTree;
  public onchange?: () => void;

  constructor(private size: number, private estimate: number) {
    size = this.minSize(size);
    this.settled = new Array<boolean>(size).fill(false);
    this.margins = new Array<number>(size * 2).fill(0);
    this.heights = new Array<number>(size).fill(estimate);
    this.offsets = new FenwickTree(this.heights);
  }

  private minSize(size: number) {
    return 1 << (32 - Math.clz32(size * 2));
  }

  public grow(size: number) {
    if (size > this.size) {
      size = this.minSize(size);
      const d = size - this.size;
      this.settled.push(...new Array<boolean>(d).fill(false));
      this.margins.push(...new Array<number>(d * 2).fill(0));
      this.heights.push(...new Array<number>(d).fill(this.estimate));
      this.offsets = new FenwickTree(this.heights);
      this.size = size;
    }
  }

  public reset() {
    this.settled.fill(false);
    this.margins.fill(0);
    this.heights.fill(this.estimate);
    this.offsets = new FenwickTree(this.heights);
  }

  public unset(i: number) {
    if (this.settled[i]) {
      this.settled[i] = false;
    }
  }

  public prune(n: number) {
    n = Math.min(n, this.size);
    this.settled.copyWithin(0, n).fill(false, this.size - n);
    this.margins.copyWithin(0, n * 2).fill(0, (this.size - n) * 2);
    this.heights.copyWithin(0, n).fill(this.estimate, this.size - n);
    this.offsets = new FenwickTree(this.heights);
  }

  public set(i: number, settled: boolean, height: number, marginTop: number, marginBottom: number) {
    this.settled[i] = settled;
    this.margins[i * 2] = marginTop;
    this.margins[i * 2 + 1] = marginBottom;
    this.heights[i] = height;

    let changed = false;
    // update i +/- 1 to resolve css effects from sibling selectors
    changed = i > 0 && this.syncOffset(i - 1);
    changed = this.syncOffset(i) || changed;
    changed = (i < this.size && this.syncOffset(i + 1)) || changed;

    if (changed) {
      this.onchange?.();
    }
  }

  private syncOffset(i: number) {
    const height = this.heights[i] + Math.max(this.margins[i * 2 - 1] ?? 0, this.margins[i * 2]);
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

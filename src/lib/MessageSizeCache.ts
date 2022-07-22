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
    this.set(i, false, this.estimate, 0, 0);
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

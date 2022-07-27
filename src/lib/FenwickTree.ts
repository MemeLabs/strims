// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// FenwickTree
// SEE: https://en.wikipedia.org/wiki/Fenwick_tree

class FenwickTree {
  values: number[] = [];
  tree: number[];
  size: number;

  constructor(values: number[]) {
    this.push(...values);
  }

  public push(...values: number[]) {
    this.values.push(...values);
    this.tree = new Array<number>(values.length + 1).fill(0);
    this.size = values.length;
    for (let i = 0; i < this.size; i++) {
      this.add(i, this.values[i]);
    }
  }

  private lsb(i: number) {
    return i & -i;
  }

  public add(i: number, d: number) {
    for (i++; i <= this.size; i += this.lsb(i)) {
      this.tree[i] += d;
    }
  }

  public set(i: number, v: number) {
    this.add(i, v - this.values[i]);
    this.values[i] = v;
  }

  public get(i: number) {
    return this.values[i];
  }

  public prefixSum(i: number) {
    let sum = this.tree[0];
    for (; i !== 0; i -= this.lsb(i)) {
      sum += this.tree[i];
    }
    return sum;
  }

  public rangeSum(i: number, j: number) {
    let sum = 0;
    for (; j > i; j -= this.lsb(j)) {
      sum += this.tree[j];
    }
    for (; i > j; i -= this.lsb(i)) {
      sum -= this.tree[i];
    }
    return sum;
  }

  public rankQuery(v: number) {
    let i = 0;
    let j = this.size;

    v -= this.tree[0];
    for (; j > 0; j >>= 1) {
      if (i + j <= this.size && this.tree[i + j] <= v) {
        v -= this.tree[i + j];
        i += j;
      }
    }
    return i;
  }
}

export default FenwickTree;

// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import FenwickTree from "./FenwickTree";

describe("fenwick tree", () => {
  it("set", () => {
    const tree = new FenwickTree(new Array<number>(16).fill(10));
    assert.equal(tree.get(9), 10);
    assert.equal(tree.prefixSum(10), 100);
    assert.equal(tree.rangeSum(0, 10), 100);
    tree.set(9, 20);
    assert.equal(tree.get(9), 20);
    assert.equal(tree.prefixSum(10), 110);
    assert.equal(tree.rangeSum(0, 10), 110);
    tree.set(9, 30);
    assert.equal(tree.get(9), 30);
    assert.equal(tree.prefixSum(10), 120);
    assert.equal(tree.rangeSum(0, 10), 120);
  });
  it("rankQuery", () => {
    const tree = new FenwickTree(new Array<number>(32).fill(10));
    assert.equal(tree.rankQuery(0), 0);
    assert.equal(tree.rankQuery(100), 10);
    assert.equal(tree.rankQuery(250), 25);
  });
});

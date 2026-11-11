// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import { newResults } from "./api";

const at = "2026-11-11T21:22:00+00:00";

describe("newResults", () => {
  it("offers rows of an older snapshot than the catalog's latest", () => {
    assert.strictEqual(newResults({ id: 321, at }, 320), true);
    assert.strictEqual(newResults({ id: 400, at }, 1), true);
  });

  it("offers nothing for rows of the latest snapshot", () => {
    assert.strictEqual(newResults({ id: 320, at }, 320), false);
  });

  it("offers nothing for rows newer than the latest the page read", () => {
    // The page's own fetch indexed after its last read of GET /console/index.
    assert.strictEqual(newResults({ id: 320, at }, 321), false);
  });

  it("offers nothing while either snapshot is unknown", () => {
    assert.strictEqual(newResults(null, 320), false);
    assert.strictEqual(newResults(undefined, 320), false);
    assert.strictEqual(newResults({ id: 321, at }, null), false);
    assert.strictEqual(newResults({ id: 321, at }, undefined), false);
    assert.strictEqual(newResults(null, null), false);
  });
});

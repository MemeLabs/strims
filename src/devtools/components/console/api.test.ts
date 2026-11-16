// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import { gotoOf, newResults, routeOf } from "./api";

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

describe("routeOf", () => {
  const page = "http://blinky.local:8086/devtools.html";

  it("is a console address's fragment", () => {
    assert.strictEqual(
      routeOf(`${page}#/console/tree?exp=e1&sel=%7B%7D`),
      "/console/tree?exp=e1&sel=%7B%7D"
    );
    assert.strictEqual(routeOf(`${page}#/console`), "/console");
    assert.strictEqual(routeOf(`${page}#/console?exp=e1`), "/console?exp=e1");
  });

  it("is none for an address outside the console", () => {
    assert.strictEqual(routeOf(`${page}#/watch/x`), null);
    assert.strictEqual(routeOf(`${page}#/consoles/x`), null);
    assert.strictEqual(routeOf(`${page}#console/tree`), null);
    assert.strictEqual(routeOf(page), null);
    assert.strictEqual(routeOf("https://evil.example/"), null);
  });
});

describe("gotoOf", () => {
  const url = "http://127.0.0.1:8086/devtools.html#/console/overhead?exp=e1";

  it("takes an address sent after the last it took, and goes to its route", () => {
    assert.deepStrictEqual(gotoOf(0, { seq: 3, url }), {
      took: 3,
      route: "/console/overhead?exp=e1",
    });
    assert.deepStrictEqual(gotoOf(3, { seq: 4, url }), {
      took: 4,
      route: "/console/overhead?exp=e1",
    });
  });

  it("does nothing with one it took already", () => {
    // The stream pushes it, and report answers give it again until the server hears it was taken.
    assert.strictEqual(gotoOf(3, { seq: 3, url }), null);
    assert.strictEqual(gotoOf(5, { seq: 3, url }), null);
  });

  it("takes a restarted server's first address after one it took from the server before", () => {
    // A seq is the goto's time in ms (server.py Tabs): the new server's pass the old one's.
    const before = Date.UTC(2026, 10, 16, 12, 0, 0);
    assert.deepStrictEqual(gotoOf(before, { seq: before + 60_000, url }), {
      took: before + 60_000,
      route: "/console/overhead?exp=e1",
    });
  });

  it("takes one outside the console without going anywhere", () => {
    assert.deepStrictEqual(gotoOf(0, { seq: 2, url: "http://h/devtools.html#/watch/x" }), {
      took: 2,
      route: null,
    });
  });
});

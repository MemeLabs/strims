// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import { nodesQuery } from "./selection";

describe("nodesQuery", () => {
  const sel = { exp: "s14d11", cell: "wan-residential-seed-step", arm: "profiles" };

  it("groups meter_samples only for a view with a node picker", () => {
    // meter_error's pickers; a view's without the node's (run_timeline's); every picker's.
    const q = nodesQuery(sel, new Set(["exp", "cell", "arm", "node"]));
    assert.ok(q?.sql.includes("FROM experiment_jobs e JOIN meter_samples m USING (k, src)"));
    assert.deepStrictEqual(q?.params, sel);
    assert.strictEqual(nodesQuery(sel, new Set(["exp", "cell", "arm", "seed"])), null);
    assert.deepStrictEqual(nodesQuery(sel, null), q);
  });

  it("lists the nodes meter_error pools: the seed and the honest viewers", () => {
    // meter_error.sql's: a node's seeds where it is hostile are left out.
    assert.ok(nodesQuery(sel, null)?.sql.includes("AND m.role <> 'hostile' "));
  });

  it("asks nothing until the experiment, cell and arm are picked", () => {
    for (const missing of ["exp", "cell", "arm"] as const) {
      assert.strictEqual(nodesQuery({ ...sel, [missing]: "" }, null), null, missing);
    }
  });
});

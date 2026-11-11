// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";
import { readFileSync } from "fs";
import path from "path";

import { ALIGNS, EPISODES, KINDS, PHASES, REASONS } from "./vocab";

const DOCUMENT = readFileSync(
  path.resolve(__dirname, "../../../../crates/strims-wan/vocab.toml"),
  "utf8"
);

// The names of a section's tables, `[section.name]`, in the document's order.
const headers = (section: string): string[] =>
  [...DOCUMENT.matchAll(new RegExp(`^\\[${section}\\.([a-z0-9_]+)\\]$`, "gm"))].map((m) => m[1]);

describe("vocab", () => {
  it("lists the document's phases, episodes and reasons", () => {
    assert.deepStrictEqual(
      PHASES.map((p) => p.name),
      headers("phases")
    );
    assert.deepStrictEqual(
      EPISODES.map((e) => e.name),
      headers("episodes")
    );
    assert.deepStrictEqual(
      REASONS.map((r) => r.name),
      headers("reasons")
    );
  });

  it("lists every kind of the document, and aligns on its derived and scenario kinds", () => {
    assert.deepStrictEqual(
      KINDS.map((k) => k.id),
      headers("kinds")
    );
    const of = (c: string) => KINDS.filter((k) => k.class === c).map((k) => k.id as string);
    assert.deepStrictEqual([...ALIGNS], [...of("derived"), ...of("scenario")]);
  });
});

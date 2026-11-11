// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";
import { readFileSync, readdirSync } from "fs";
import path from "path";

import { VOCABS, expandVocab } from "./domains";
import { ALIGNS, EPISODES, KINDS, PHASES, REASONS } from "./vocab";

type Spec = Record<string, unknown>;

const DOCUMENT = readFileSync(
  path.resolve(__dirname, "../../../../crates/strims-wan/vocab.toml"),
  "utf8"
);
const VIEWS = path.resolve(__dirname, "../../../../hack/console/views");

// The names of a section's tables, `[section.name]`, in the document's order.
const headers = (section: string): string[] =>
  [...DOCUMENT.matchAll(new RegExp(`^\\[${section}\\.([a-z0-9_]+)\\]$`, "gm"))].map((m) => m[1]);

const isSpec = (v: unknown): v is Spec => typeof v === "object" && v !== null && !Array.isArray(v);

const views = (): [string, Spec][] =>
  readdirSync(VIEWS)
    .filter((f) => f.endsWith(".vl.json"))
    .map((f) => [f, JSON.parse(readFileSync(path.join(VIEWS, f), "utf8")) as Spec]);

// Every value of a spec with its path and its key (an array's, for its items), outside its data and
// its vocab references' insides.
const values = function* (v: unknown, at = "", key = ""): Generator<[string, string, unknown]> {
  yield [at, key, v];
  if (Array.isArray(v)) for (const [i, x] of v.entries()) yield* values(x, `${at}[${i}]`, key);
  else if (isSpec(v) && !("vocab" in v))
    for (const [k, x] of Object.entries(v))
      if (k !== "data" && k !== "datum") yield* values(x, at ? `${at}.${k}` : k, k);
};

const colours = (names: readonly { name: string; colour: string }[], only: string[]) =>
  only.map((n) => names.find((x) => x.name === n)?.colour);

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

  it("lists every kind of the document, and aligns on its current derived and scenario kinds", () => {
    assert.deepStrictEqual(
      KINDS.map((k) => k.id),
      headers("kinds")
    );
    const of = (c: string) =>
      KINDS.filter((k) => k.class === c && !("replaced_by" in k)).map((k) => k.id as string);
    assert.deepStrictEqual([...ALIGNS], [...of("derived"), ...of("scenario")]);
    const retired = KINDS.filter((k) => "replaced_by" in k).map((k) => k.id as string);
    assert.ok(retired.includes("up_step") && !ALIGNS.some((a) => retired.includes(a)), retired.join());
  });

  it("gives each phase, episode and reason a colour of its own in its section", () => {
    for (const [v, names] of Object.entries(VOCABS)) {
      const cs = names.map((n) => n.colour);
      assert.ok(
        cs.every((c) => /^#[0-9a-f]{6}$/.test(c)),
        v
      );
      assert.strictEqual(new Set(cs).size, cs.length, v);
    }
  });
});

describe("expandVocab", () => {
  it("makes a scale's domain and range of a vocabulary's names, in the document's order", () => {
    const spec = {
      encoding: {
        color: {
          field: "phase",
          scale: { vocab: "phases", only: ["steady", "startup"], zero: false },
        },
      },
    };
    const before = JSON.stringify(spec);
    const out = expandVocab(spec);
    assert.strictEqual(JSON.stringify(spec), before, "the spec given is unchanged");
    assert.deepStrictEqual(out, {
      encoding: {
        color: {
          field: "phase",
          scale: {
            zero: false,
            domain: ["startup", "steady"],
            range: colours(PHASES, ["startup", "steady"]),
          },
        },
      },
    });
    const all = expandVocab({ scale: { vocab: "reasons" } }).scale as Spec;
    assert.deepStrictEqual(
      all.domain,
      REASONS.map((r) => r.name)
    );
    assert.deepStrictEqual(
      all.range,
      REASONS.map((r) => r.colour)
    );
  });

  it("takes a list of vocabularies, each's names in turn", () => {
    const out = expandVocab({
      scale: { vocab: ["phases", "episodes"], only: ["collapse", "after_up", "unhealthy"] },
    });
    assert.deepStrictEqual(out.scale, {
      domain: ["after_up", "unhealthy", "collapse"],
      range: [
        ...colours(PHASES, ["after_up"]),
        ...colours(EPISODES, ["unhealthy", "collapse"]),
      ],
    });
  });

  it("makes a sort of the names", () => {
    const out = expandVocab({
      vconcat: [
        { encoding: { y: { field: "phase", sort: { vocab: "phases" } } } },
        { encoding: { facet: { sort: { vocab: "episodes", only: ["collapse", "healthy"] } } } },
      ],
    });
    const [a, b] = (out.vconcat as Spec[]).map((p) => p.encoding as Record<string, Spec>);
    assert.deepStrictEqual(
      a.y.sort,
      PHASES.map((p) => p.name)
    );
    assert.deepStrictEqual(b.facet.sort, ["healthy", "collapse"]);
  });

  it("leaves a spec without references, and its data, as they were", () => {
    const spec = {
      data: { values: [{ vocab: "phases" }] },
      layer: [
        { mark: "rect", encoding: { x: { datum: { vocab: 1 } }, y: { sort: ["a"] } } },
        { encoding: { color: { scale: { domain: ["a"] } }, x: { sort: { field: "x" } } } },
      ],
    };
    assert.deepStrictEqual(expandVocab(spec), spec);
  });

  it("throws on a vocabulary or a name the document lacks, and on a reference out of place", () => {
    const throws = (spec: Spec, message: RegExp) =>
      assert.throws(() => expandVocab(spec), { message });
    throws({ scale: { vocab: "kinds" } }, /^scale: no vocabulary "kinds" \(phases, episodes, reasons\)$/);
    throws({ scale: { vocab: "toString" } }, /no vocabulary "toString"/);
    throws({ scale: { vocab: [] } }, /vocab is a vocabulary or a list of them/);
    throws(
      { vconcat: [{ encoding: { color: { scale: { vocab: "phases", only: ["departure"] } } } }] },
      /^vconcat\[0\]\.encoding\.color\.scale: "departure" is no name of phases$/
    );
    throws({ sort: { vocab: ["phases", "reasons"], only: ["open", "x"] } }, /"x" is no name of phases or reasons/);
    throws({ sort: { vocab: "phases", only: "steady" } }, /only is a list of names/);
    throws({ sort: { vocab: "phases", only: ["steady", "steady"] } }, /only lists a name twice/);
    throws({ sort: { vocab: "phases", order: "descending" } }, /a vocab sort takes no order/);
    throws({ scale: { vocab: "phases", range: ["#000000"] } }, /a vocab scale takes no range/);
    throws({ scale: { domain: { vocab: "phases" } } }, /^scale\.domain: a vocab reference stands/);
    throws({ vocab: "phases" }, /^spec: a vocab reference stands/);
  });
});

describe("the saved views", () => {
  // collapse_grid colours a run's state (healthy, ramp, collapse), which the document doesn't hold.
  const STATES = "collapse_grid.vl.json:spec.encoding.color.scale.domain";
  const names = new Map<string, string>();
  for (const [v, ns] of Object.entries(VOCABS)) for (const n of ns) names.set(n.name, v);

  it("name the document's phases, episodes and reasons only through vocab references", () => {
    let refs = 0;
    for (const [f, spec] of views()) {
      for (const [at, key, v] of values(spec)) {
        if (isSpec(v) && "vocab" in v) refs++;
        if (typeof v !== "string" || !names.has(v)) continue;
        // A field or a title is a column or a label, not a list of the vocabulary's.
        if (key === "field" || key === "title") continue;
        if (`${f}:${at.replace(/\[\d+\]$/, "")}` === STATES) continue;
        assert.fail(`${f}: ${at} spells ${names.get(v)}' ${JSON.stringify(v)}: use a vocab reference`);
      }
    }
    assert.ok(refs >= 10, "the views colour and sort by the vocabulary");
  });

  it("name only the document's names in their references", () => {
    for (const [f, spec] of views())
      for (const [at, , v] of values(spec)) {
        if (!isSpec(v) || !("vocab" in v)) continue;
        const vs = typeof v.vocab === "string" ? [v.vocab] : (v.vocab as string[]);
        for (const x of vs)
          assert.ok(Object.keys(VOCABS).includes(x), `${f}: ${at}: no vocabulary ${x}`);
        for (const n of (v.only as string[] | undefined) ?? [])
          assert.ok(vs.includes(names.get(n) ?? ""), `${f}: ${at}: ${n} is no name of ${vs.join(" or ")}`);
      }
    for (const [f, spec] of views()) assert.doesNotThrow(() => expandVocab(spec), f);
  });
});

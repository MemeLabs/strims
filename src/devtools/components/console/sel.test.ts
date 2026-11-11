// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";
import { readFileSync, readdirSync } from "fs";
import path from "path";

import type { SavedView } from "./api";
import { expandVocab } from "./domains";
import { brushName, hovered } from "./hover";
import {
  WORKBENCH,
  added,
  chartOf,
  checked,
  fromBrush,
  fromClick,
  fromMoment,
  highlighted,
  panels,
  parseAddress,
  pathOf,
  readSel,
  selJson,
  selText,
  timeField,
  viewQuery,
  workbenchQuery,
} from "./sel";
import type { Panel, Sel } from "./sel";
import { isKey, ordered, selectionOf, viewParams } from "./selection";
import { config } from "./theme";

type Spec = Record<string, unknown>;

const HACK = path.resolve(__dirname, "../../../../hack");
const VIEWS = path.join(HACK, "console", "views");
const TESTDATA = path.join(HACK, "simlab", "tests", "testdata");
const PANELS = JSON.parse(readFileSync(path.join(TESTDATA, "panels.json"), "utf8")) as Record<
  string,
  { id: string; path: string; time: string | null; x2: string | null; encoded: string[] }[]
>;
const LINKS = JSON.parse(readFileSync(path.join(TESTDATA, "links.json"), "utf8")) as {
  name: string;
  params: Record<string, string>;
  sel: Sel | null;
  url: string;
}[];
const STALLS = "Stalls at the playout delay picked, as long as picked or longer";
const LAG = "Lag across viewers (playout delays dashed)";

/** The saved views as GET /console/views gives them, read from their files. */
const saved = (): SavedView[] =>
  readdirSync(VIEWS)
    .filter((f) => f.endsWith(".sql"))
    .map((f) => f.slice(0, -4))
    .sort()
    .map((name) => {
      const meta = JSON.parse(readFileSync(path.join(VIEWS, `${name}.json`), "utf8")) as SavedView;
      let spec: Spec | null = null;
      try {
        spec = JSON.parse(readFileSync(path.join(VIEWS, `${name}.vl.json`), "utf8")) as Spec;
      } catch {
        spec = null;
      }
      return {
        ...meta,
        name,
        sql: readFileSync(path.join(VIEWS, `${name}.sql`), "utf8"),
        spec,
      };
    });

const VIEW_LIST = saved();
const view = (name: string): SavedView => {
  const v = VIEW_LIST.find((x) => x.name === name);
  assert.ok(v, name);
  return v;
};

const logger = (warnings: string[]) => {
  const l = {
    level: () => l,
    info: () => l,
    debug: () => l,
    warn: (...a: unknown[]) => (warnings.push(a.join(" ")), l),
    error: (...a: unknown[]) => (warnings.push(a.join(" ")), l),
  };
  return l;
};

/** A spec as VegaChart embeds it, compiled to Vega: its warnings, and the Vega spec. */
const compiled = async (spec: Spec): Promise<[string[], Spec]> => {
  const { compile } = await import("vega-lite");
  const warnings: string[] = [];
  const out = compile({ ...spec, data: { values: [] }, config } as never, {
    logger: logger(warnings),
  });
  return [warnings, out.spec as unknown as Spec];
};

/** A selection's spec as VegaChart draws it, and hover.ts's marks for it. */
const drawn = (spec: Spec, sel: Sel | null, brush = true): Spec => {
  const h = highlighted(spec, sel);
  return hovered(h.spec, brush ? { brush: true, band: h.band, moment: h.moment } : {});
};

/** Every mark of a compiled Vega spec, with the chain of groups it is in. */
const marks = function* (
  m: Spec,
  group?: { mark?: Spec }
): Generator<{ name?: string; marktype?: string; group?: { mark?: Spec } } & Spec> {
  for (const x of (m.marks as Spec[] | undefined) ?? []) {
    const mark = { ...x, marktype: x.type as string, group } as Spec & { group?: { mark?: Spec } };
    yield mark;
    if (x.type === "group") yield* marks(x, { mark });
  }
};

const query = (url: string): string => url.slice(url.indexOf("?", url.indexOf("#")) + 1);

describe("sel", () => {
  it("walks every saved view's panels as views.py does (testdata/panels.json)", () => {
    const names = VIEW_LIST.map((v) => v.name);
    assert.deepStrictEqual(names, Object.keys(PANELS).sort());
    for (const v of VIEW_LIST) {
      const got = panels(v.spec).map(({ id, path, time, x2, encoded }) => ({
        id,
        path,
        time,
        x2,
        encoded,
      }));
      assert.deepStrictEqual(got, PANELS[v.name], v.name);
    }
    // The walk is the same over a spec with its vocabulary expanded, as VegaChart walks it.
    const rt = view("run_timeline").spec as Spec;
    assert.deepStrictEqual(
      panels(expandVocab(rt)).map((p) => p.encoded),
      panels(rt).map((p) => p.encoded)
    );
  });

  it("reads each address of testdata/links.json as views.parse reads it, and writes it back byte for byte", () => {
    assert.ok(LINKS.length >= 10);
    for (const link of LINKS) {
      const got = parseAddress(link.url, VIEW_LIST);
      assert.strictEqual(got.name, link.name, link.url);
      assert.deepStrictEqual(got.params, link.params, link.url);
      assert.deepStrictEqual(got.sel, link.sel, link.url);
      let q: URLSearchParams;
      if (link.name === WORKBENCH) {
        q = workbenchQuery(link.params, link.sel, VIEW_LIST);
      } else {
        const v = view(link.name);
        // The params the page binds: as viewParams reads them from the selection the address holds.
        const [params] = viewParams(v, selectionOf(got.query));
        const [want] = viewParams(v, {
          ...selectionOf(new URLSearchParams()),
          ...Object.fromEntries(
            v.params.map((p) => [isKey(p.name) ? p.name : p.kind, link.params[p.name] ?? ""])
          ),
        });
        assert.deepStrictEqual(params, want, link.url);
        q = viewQuery(v, link.params, link.sel ? checked(panels(v.spec), link.sel).sel : null);
      }
      assert.strictEqual(q.toString(), query(link.url), link.url);
      if (link.sel) {
        const text = new URLSearchParams(query(link.url)).get("sel");
        assert.strictEqual(selJson(link.sel), text);
      }
    }
  });

  it("orders a query as the addresses do, and leaves out what's empty or a key's default", () => {
    const q = ordered(
      new URLSearchParams(
        "sel=%7B%7D&x=t&foo=1&seed=3007&exp=e&delay=1250&cell=&sql=SELECT+1&exp=f"
      )
    );
    assert.strictEqual(q.toString(), "sql=SELECT+1&exp=e&seed=3007&foo=1&x=t&sel=%7B%7D");
  });

  it("checks a selection as views.selection does, and words it as selection_text", () => {
    const rt = panels(view("run_timeline").spec);
    const grid = panels(view("collapse_grid").spec);
    assert.deepStrictEqual(checked(rt, { panel: "2", at: { viewer: 7, t_s: [38, 46] } }).sel, {
      panel: STALLS,
      at: { t_s: [38, 46], viewer: 7 },
    });
    assert.deepStrictEqual(checked(grid, { panel: "0", at: { seed: 3007, cell: "x" } }).sel, {
      at: { cell: "x", seed: 3007 },
    });
    assert.strictEqual(
      selText(rt, { panel: STALLS, at: { t_s: [20, 21], viewer: 3 } }),
      `${STALLS} · t_s 20–21 · viewer 3`
    );
    assert.strictEqual(
      selText(grid, { at: { cell: "star-5", seed: 2001 } }),
      'cell "star-5" · seed 2001'
    );
    for (const [sel, message] of [
      [
        { panel: STALLS, at: { value: 3 } },
        /encodes no column "value": its columns are t_s, t2_s, viewer/,
      ],
      [{ at: { source: "sim" } }, /the view encodes no column "source"/],
      [
        { panel: STALLS, at: { viewer: [1, 3] } },
        /a range is on panel ".*"'s time field, t_s: not viewer/,
      ],
      [{ panel: STALLS, at: { t_s: [46, 38] } }, /t_s's range is \[LO, HI\]/],
      [{ panel: "lag", at: { t_s: 1 } }, /has no panel "lag"/],
      [{ at: {} }, /names a field/],
      [{ at: { t_s: 1 }, where: 1 }, /a selection is/],
    ] as [unknown, RegExp][])
      assert.throws(() => checked(rt, sel), message);
    assert.deepStrictEqual(readSel(rt, "{"), {
      sel: null,
      panel: null,
      error: "the address's sel isn't JSON: {",
    });
    assert.strictEqual(readSel(rt, "").sel, null);
  });

  it("makes selections of only the panel's columns from a click on any saved view's marks, and of its time field from a brush", () => {
    let clicks = 0;
    let brushes = 0;
    for (const v of VIEW_LIST) {
      const ps = panels(v.spec);
      for (const p of ps) {
        // A datum with every field the spec names, and some it doesn't.
        const datum: Record<string, unknown> = { lane: "x", source: "sim", _x0: 0, track: "lag" };
        for (const f of JSON.stringify(v.spec).matchAll(/"field":"([^"]+)"/g)) datum[f[1]] = 1;
        for (const isPath of [false, true]) {
          const s = fromClick(ps, p, datum, { time: p.time ? 12.5 : null, path: isPath });
          if (s === null) continue;
          clicks++;
          assert.deepStrictEqual(checked(ps, s).sel, s, `${v.name} ${p.id}`);
          for (const f of Object.keys(s.at))
            assert.ok(p.encoded.includes(f), `${v.name} ${p.id}: ${f}`);
          if (p.time) assert.strictEqual(s.at[p.time], 12.5);
          assert.strictEqual(s.panel, ps.length > 1 ? p.id : undefined);
        }
        if (p.time) {
          brushes++;
          const s = fromBrush(ps, p, 10.123456, 20.987654);
          assert.deepStrictEqual(s, {
            ...(ps.length > 1 ? { panel: p.id } : {}),
            at: { [p.time]: [10, 21] },
          });
          assert.deepStrictEqual(checked(ps, s).sel, s);
        } else assert.strictEqual(fromBrush(ps, p, 1, 2), null);
      }
    }
    assert.ok(clicks > 30 && brushes >= 15, `${clicks} clicks, ${brushes} brushes`);
    const rt = panels(view("run_timeline").spec);
    const stalls = rt.find((p) => p.id === STALLS);
    // A line's datum is its first point: its group's fields alone, at the cursor's time.
    const lag = rt.find((p) => p.id === LAG);
    assert.deepStrictEqual(
      fromClick(rt, lag, { t_s: 1, value: 50, series: "p99" }, { time: 40, path: true }),
      {
        panel: LAG,
        at: { series: "p99", t_s: 40 },
      }
    );
    assert.deepStrictEqual(fromMoment(rt, 40), { at: { t_s: 40 } });
    assert.deepStrictEqual(fromBrush(rt, stalls, 46.04, 38.01), {
      panel: STALLS,
      at: { t_s: [38, 46] },
    });
    // Shift-click: a brushed range plus a viewer, its stall's own time and span left out.
    const brushed: Sel = { panel: LAG, at: { t_s: [38, 46] } };
    const stall = fromClick(
      rt,
      stalls,
      { t_s: 39.1, t2_s: 40.3, viewer: 7, value: 1200 },
      { time: 39.1 }
    );
    assert.deepStrictEqual(stall, { panel: STALLS, at: { t2_s: 40.3, t_s: 39.1, viewer: 7 } });
    assert.deepStrictEqual(added(rt, brushed, stall), {
      panel: STALLS,
      at: { t_s: [38, 46], viewer: 7 },
    });
    // A field the new panel doesn't encode goes.
    assert.deepStrictEqual(added(rt, { panel: LAG, at: { series: "p99", t_s: 40 } }, stall), {
      panel: STALLS,
      at: { t2_s: 40.3, t_s: 39.1, viewer: 7 },
    });
    // A heat cell adds its viewer to a range, not its lag.
    const heat = rt.find((p) => p.path === "1");
    const cell = fromClick(rt, heat, { t_s: 40, t2_s: 41, viewer: 3, value: 812.5 }, { time: 40 });
    assert.deepStrictEqual(cell, {
      panel: heat.id,
      at: { t2_s: 41, t_s: 40, value: 812.5, viewer: 3 },
    });
    assert.deepStrictEqual(added(rt, brushed, cell), {
      panel: heat.id,
      at: { t_s: [38, 46], viewer: 3 },
    });
    const grid = panels(view("collapse_grid").spec);
    assert.deepStrictEqual(
      fromClick(grid, grid[0], {
        arm: "aimd",
        arm_title: "aimd: 3",
        cell: "star-31-5",
        seed: 3007,
        state: "collapse",
        k: "x",
      }),
      { at: { arm: "aimd", cell: "star-31-5", seed: 3007, state: "collapse" } }
    );
  });

  it("finds a mark's panel by Vega-Lite's names for a concat's members", async () => {
    for (const v of VIEW_LIST.filter((x) => x.spec)) {
      const ps = panels(v.spec);
      const [, vg] = await compiled(drawn(expandVocab(v.spec), null));
      const found = new Set<string | null>();
      for (const m of marks(vg)) if (m.type !== "group") found.add(pathOf(m));
      const want = ps.length > 1 ? ps.map((p) => p.path) : [null];
      assert.deepStrictEqual([...found].sort(), [...want].sort(), v.name);
    }
  });

  it("compiles every saved spec with its brushes and each kind of highlight, and no selection and no time panel as before", async () => {
    let rings = 0;
    for (const v of VIEW_LIST.filter((x) => x.spec)) {
      const spec = expandVocab(v.spec);
      const ps = panels(spec);
      const time = timeField(ps);
      const before = hovered(spec);
      assert.strictEqual(JSON.stringify(drawn(spec, null, false)), JSON.stringify(before), v.name);
      if (!time) {
        const [w1, a] = await compiled(drawn(spec, null));
        const [w2, b] = await compiled(before);
        assert.deepStrictEqual([w1, w2], [[], []], v.name);
        assert.strictEqual(JSON.stringify(a), JSON.stringify(b), `${v.name}: nothing to brush`);
      }
      const sels: Sel[] = [];
      if (time) sels.push({ at: { [time]: [10, 20] } }, { at: { [time]: 15 } });
      for (const p of ps) {
        const at: Record<string, unknown> = {};
        for (const f of p.encoded) at[f] = f === p.time ? [10, 20] : "v";
        sels.push(checked(ps, { panel: p.id, at }).sel);
      }
      for (const sel of [null, ...sels]) {
        const out = drawn(spec, sel);
        const [warnings, vg] = await compiled(out);
        assert.deepStrictEqual(warnings, [], `${v.name} ${JSON.stringify(sel)}`);
        const text = JSON.stringify(out);
        const names = [...marks(vg)].map((m) => m.name ?? "");
        if (time) {
          for (const p of ps.filter((x) => x.time))
            assert.ok(names.includes(`${brushName(p.path)}_brush`), `${v.name} ${p.path}: a brush`);
          const t = sel?.at[time];
          if (Array.isArray(t))
            assert.ok(text.includes(`"value":{"x":[10,20]}`), `${v.name}: the band`);
          if (typeof t === "number")
            assert.ok(
              names.some((n) => /^moment_\d+_marks$/.test(n)),
              `${v.name}: the moment's rule`
            );
        } else assert.ok(!names.some((n) => n.startsWith("brush_")), v.name);
        rings += names.filter((n) => /^ring_\d+_marks$/.test(n)).length;
        // Values besides the time draw a ring on the panel's marks: a layer, or in a facet the
        // marks' own stroke.
        const values = Object.fromEntries(
          Object.entries(sel?.at ?? {}).filter(([f]) => f !== time)
        );
        if (sel && Object.keys(values).length > 0) {
          const timeOnly =
            Object.keys(values).length < Object.keys(sel.at).length
              ? drawn(spec, { at: { [time]: sel.at[time] } })
              : drawn(spec, null);
          assert.notStrictEqual(
            text,
            JSON.stringify(timeOnly),
            `${v.name} ${JSON.stringify(sel)}: a ring`
          );
        }
      }
    }
    assert.ok(rings > 20, `${rings} rings`);
  });

  it("rings the marks a selection names: a copy of the mark, its rows the selection's, stroked with no fill", () => {
    const rt = expandVocab(view("run_timeline").spec);
    const sel: Sel = { panel: STALLS, at: { t_s: [38, 46], viewer: 7 } };
    const h = highlighted(rt, sel);
    assert.deepStrictEqual(h.band, [38, 46]);
    assert.strictEqual(h.moment, undefined);
    const stalls = (h.spec.vconcat as Spec[])[2];
    const [mark, ring] = stalls.layer as Spec[];
    assert.deepStrictEqual(
      mark.encoding,
      ((view("run_timeline").spec as Spec).vconcat as Spec[])[2].encoding
    );
    assert.deepStrictEqual(ring.transform, [
      {
        filter:
          'datum["viewer"] === 7 && datum["t_s"] <= 46 && (isValid(datum["t2_s"]) ? datum["t2_s"] : datum["t_s"]) >= 38',
      },
    ]);
    assert.deepStrictEqual(ring.mark, {
      type: "rect",
      stroke: "#0b0b0b",
      strokeWidth: 2,
      opacity: 1,
      fillOpacity: 0,
    });
    assert.strictEqual((ring.encoding as Spec).tooltip, undefined);
    // A line's ring: its series whole, grouped as the line is, at a moment.
    const lag = highlighted(rt, { panel: LAG, at: { series: "p99", t_s: 40 } });
    assert.strictEqual(lag.moment, 40);
    const lagRing = ((lag.spec.vconcat as Spec[])[0].layer as Spec[]).find(
      (l) => l.name === "ring_0"
    );
    assert.deepStrictEqual(lagRing.transform, [{ filter: 'datum["series"] === "p99"' }]);
    assert.deepStrictEqual((lagRing.encoding as Spec).detail, [
      { field: "series", type: "nominal" },
    ]);
    // A moment alone draws no ring; a selection's other panels are as they were.
    assert.deepStrictEqual(highlighted(rt, { at: { t_s: 40 } }), { spec: rt, moment: 40 });
    // A stacked area keeps every row and hides those the selection doesn't name.
    const overhead = "Overhead the seed and the viewers wrote, by kind of message";
    const bytes = (
      (highlighted(rt, { panel: overhead, at: { series: "ack" } }).spec.vconcat as Spec[])[7]
        .layer as Spec[]
    )[1];
    assert.strictEqual(bytes.transform, undefined);
    assert.deepStrictEqual((bytes.encoding as Spec).strokeOpacity, {
      condition: { test: 'datum["series"] === "ack"', value: 1 },
      value: 0,
    });
    // A facet's marks take their ring in place.
    const grid = highlighted(expandVocab(view("collapse_grid").spec), {
      at: { arm: "aimd", cell: "c", seed: 1 },
    });
    const enc = (grid.spec.spec as Spec).encoding as Spec;
    assert.deepStrictEqual(enc.stroke, {
      condition: {
        test: 'datum["arm"] === "aimd" && datum["cell"] === "c" && datum["seed"] === 1',
        value: "#0b0b0b",
      },
      value: "#fcfcfb",
    });
  });

  it("takes the auto-chart's choices from a chart of one mark", () => {
    assert.deepStrictEqual(
      chartOf({
        mark: { type: "line" },
        encoding: {
          x: { field: "t" },
          y: { field: "v", scale: { type: "log" } },
          color: { field: "s" },
        },
      }),
      { mark: "line", x: "t", y: "v", color: "s", logy: "1" }
    );
    assert.deepStrictEqual(
      chartOf({ mark: "arc", encoding: { x: { field: "t" }, y: { field: "v" } } }),
      {
        x: "t",
        y: "v",
      }
    );
    assert.strictEqual(chartOf({ layer: [] }), null);
    assert.strictEqual(chartOf(undefined), null);
  });
});

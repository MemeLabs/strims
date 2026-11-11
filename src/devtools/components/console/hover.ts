// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * A saved view's hover cursor: a vertical rule across its time panels at the time hovered in any
 * of them, and each time series' values there as one tooltip, wherever over its plot the pointer
 * is (a tooltip on a line or an area fires only over its path, and shows its first point's row).
 *
 * A spec's `hover` (not a Vega-Lite key) names the time field on the outermost spec,
 * `"hover": {"x": "t_s"}`, and marks each panel that takes part: a unit or layer spec in its
 * concats, or the outermost spec itself.
 *
 * - A time series, `"hover": {"tooltip": [...], "transform": [...]}`: a rule over the panel's rows,
 *   after its transforms and these (a pivot of long-form rows into the fields the tooltip lists),
 *   whose tooltip (an encoding's list of fields, or a mark's tooltip) is the one at the pointer's
 *   nearest x.
 * - A track of marks with tooltips of their own (a heatmap's cells, spans, events), `"hover": {}`:
 *   hovering a mark moves the cursor to its x and keeps its tooltip, and the space between marks
 *   moves it to the nearest time any row has.
 *
 * Two point selections on x hold the time hovered: CURSOR, which the invisible rules set through
 * their nearest-x voronoi (laid over a time series' plot, and under a track's marks), and
 * CURSOR_MARKS, which any of a track's marks sets. Each panel draws the cursor from every row's x,
 * so whichever time one panel hovers, each has a rule there.
 */

type Spec = Record<string, unknown>;

const isSpec = (v: unknown): v is Spec => typeof v === "object" && v !== null && !Array.isArray(v);

/** The nearest-x selection the time series' rules (and the tracks' gaps) set. */
export const CURSOR = "cursor";
/** The selection a track's marks set, at the hovered mark's x. */
export const CURSOR_MARKS = "cursor_marks";

const CONCATS = ["vconcat", "hconcat", "concat"];
/** A unit spec's own keys, which go to its first layer when it becomes a layer spec. */
const UNIT_KEYS = new Set(["mark", "encoding", "params", "projection", "transform", "data"]);
const RULE = { type: "rule", color: "#52514e", strokeWidth: 1 };

const array = (v: unknown): unknown[] => (Array.isArray(v) ? v : v === undefined ? [] : [v]);

/**
 * `spec` without its `hover` keys, at any depth.
 *
 * @param spec - a spec, or any part of one
 * @returns a copy without them
 */
const stripped = (spec: unknown): unknown => {
  if (Array.isArray(spec)) return spec.map(stripped);
  if (!isSpec(spec)) return spec;
  const out: Spec = {};
  for (const [k, v] of Object.entries(spec)) {
    if (k === "hover") continue;
    out[k] = k === "data" || k === "datum" ? v : stripped(v);
  }
  return out;
};

/**
 * `spec` with its `hover` keys expanded into Vega-Lite layers and selections (the module's docs);
 * a spec without a time field on its outermost `hover` loses its `hover` keys alone.
 *
 * @param spec - a saved view's Vega-Lite spec
 * @returns the spec in Vega-Lite alone
 */
export const hovered = (spec: Spec): Spec => {
  const top = spec.hover;
  if (!isSpec(top) || typeof top.x !== "string") return stripped(spec) as Spec;
  const x = top.x;
  const xEnc = { field: x, type: "quantitative" };
  const at = `datum[${JSON.stringify(x)}]`;
  // Every row's x once: the cursor's rows, and the gaps' rules in a track of marks.
  const grid = [
    { filter: `isValid(${at}) && isFinite(${at})` },
    { aggregate: [{ op: "count", as: "_rows" }], groupby: [x] },
  ];
  const series: string[] = [];
  const marks: string[] = [];
  const cursors: Spec[] = [];
  let n = 0;

  const panel = (p: Spec): Spec => {
    const h = p.hover as Spec;
    const i = n++;
    const outer: Spec = {};
    let layers: unknown[];
    // The panel's rows: its data (else the outer spec's) and its transforms.
    let own: { data?: unknown; transform: unknown[] };
    if (Array.isArray(p.layer)) {
      own = { data: p.data, transform: array(p.transform) };
      for (const [k, v] of Object.entries(p))
        if (!["hover", "layer", "data", "transform"].includes(k)) outer[k] = v;
      layers = p.layer.map((c: unknown) =>
        isSpec(c) && c.data === undefined
          ? {
              ...c,
              ...(own.data !== undefined ? { data: own.data } : {}),
              ...(own.transform.length
                ? { transform: [...own.transform, ...array(c.transform)] }
                : {}),
            }
          : c
      );
    } else {
      const unit: Spec = {};
      for (const [k, v] of Object.entries(p)) {
        if (k === "hover") continue;
        if (UNIT_KEYS.has(k)) unit[k] = v;
        else outer[k] = v;
      }
      own = { data: p.data, transform: array(p.transform) };
      layers = [unit];
    }
    // Its transforms and encoding follow once the selections are known.
    const cursor: Spec = { name: `cursor_${i}`, mark: RULE };
    cursors.push(cursor);
    // The rules whose nearest-x voronoi takes the pointer: over a time series' plot, and under a
    // track's marks.
    const name = `hover_${i}`;
    series.push(name);
    if (h.tooltip !== undefined) {
      const tooltip = Array.isArray(h.tooltip) ? h.tooltip : undefined;
      layers.push(cursor, {
        name,
        ...(own.data !== undefined ? { data: own.data } : {}),
        transform: [...own.transform, ...array(h.transform)],
        mark: { type: "rule", opacity: 0, ...(tooltip ? {} : { tooltip: h.tooltip }) },
        encoding: { x: xEnc, ...(tooltip ? { tooltip } : {}) },
      });
    } else {
      // The track's first layer takes the marks' selection: its events are any of the panel's.
      const m = layers.findIndex(isSpec);
      if (m >= 0) {
        const first = layers[m] as Spec;
        const unit = typeof first.name === "string" ? first.name : `marks_${i}`;
        layers[m] = { ...first, name: unit };
        marks.push(unit);
      }
      layers = [
        {
          name,
          transform: [...grid],
          mark: { type: "rule", opacity: 0 },
          encoding: { x: xEnc },
        },
        ...layers,
        cursor,
      ];
    }
    return { ...outer, layer: layers };
  };

  const walk = (s: Spec): Spec => {
    if (isSpec(s.hover) && (s.mark !== undefined || Array.isArray(s.layer))) return panel(s);
    const out: Spec = { ...s };
    for (const k of CONCATS)
      if (Array.isArray(s[k])) out[k] = (s[k] as unknown[]).map((c) => (isSpec(c) ? walk(c) : c));
    return out;
  };

  const out = walk(spec);
  delete out.hover;
  if (series.length === 0) return stripped(out) as Spec;
  const on = { on: "pointerover", clear: "pointerout", toggle: false };
  const params: Spec[] = [
    {
      name: CURSOR,
      select: { type: "point", encodings: ["x"], nearest: true, ...on },
      views: series,
    },
  ];
  if (marks.length > 0)
    params.push({
      name: CURSOR_MARKS,
      select: { type: "point", fields: [x], ...on },
      views: marks,
    });
  const hit = params.map((q) => ({ param: q.name, empty: false }));
  const shown = hit.length > 1 ? { or: hit } : hit[0];
  // The hovered x, and the grid's ends unseen: a domain over no rows warns, and over the grid's ends
  // it doesn't move as the cursor does.
  const ends = `${at} === datum._x0 || ${at} === datum._x1`;
  for (const c of cursors) {
    c.transform = [
      ...grid,
      {
        joinaggregate: [
          { op: "min", field: x, as: "_x0" },
          { op: "max", field: x, as: "_x1" },
        ],
      },
      { filter: { or: [shown, ends] } },
    ];
    c.encoding = { x: xEnc, opacity: { condition: { test: shown, value: 1 }, value: 0 } };
  }
  out.params = [...array(out.params), ...params];
  return stripped(out) as Spec;
};

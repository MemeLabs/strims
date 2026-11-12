// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * A selection on a console graph, as its address holds it (hack/simlab/views.py's docs define the
 * address; this module reads and writes the same bytes):
 *
 *   sel={"panel": P, "at": {FIELD: VALUE | [LO, HI], ...}}
 *
 * P is a panel's id: its title where no other panel has it, else its index path through the
 * concats (`1`, `0.2`; `0` for a spec that is one panel), left out for a graph of one panel. Each
 * FIELD is a column of the view's rows the panel encodes (or with no panel, that any panel
 * encodes): a VALUE matches by equality, a range `[LO, HI]` is on the time field alone (the
 * outermost spec's `hover.x`, on a panel carrying `hover`: hover.ts's time panels).
 *
 * - The walk (panels) is views.panels': a panel's layers, its time field, the column its x2
 *   encodes beside the time field (spans, which a range overlaps), and the columns a selection
 *   may name (`encoded`: SELECTABLE's channels and the facet's fields, less the fields its
 *   transforms make).
 * - A click on a mark selects its values (fromClick), a drag on a time panel a range of its time
 *   field (fromBrush), a click on a time series' plot the moment the cursor is at (fromMoment),
 *   and shift adds a click's values to the selection (added).
 * - The highlight is drawn from the selection alone, whoever wrote it: a range is a band in every
 *   time panel (the brush's initial value, hover.ts), a moment a rule there, and other values a
 *   ring on the marks they name (highlighted: a layer copied from the mark, its stroke drawn and
 *   its fill not).
 */

import type { Json, SavedView } from "./api";
import { CHART, DEFAULTS, KEYS, SEL, STATEMENT, isKey, ordered } from "./selection";

type Spec = Record<string, unknown>;

const isSpec = (v: unknown): v is Spec => typeof v === "object" && v !== null && !Array.isArray(v);

/** A value a field matches by equality. */
export type Value = string | number | boolean | null;
/** A range of the time field, LO first. */
export type Range = [number, number];

/** A selection on a graph: its panel's id (none for a graph of one panel) and its fields. */
export interface Sel {
  panel?: string;
  at: Record<string, Value | Range>;
}

const CONCATS = ["vconcat", "hconcat", "concat"];
/**
 * The channels whose fields a selection may name: a mark's place and what tells it from its
 * neighbours (views.SELECTABLE).
 */
const SELECTABLE = [
  "x",
  "x2",
  "y",
  "y2",
  "xOffset",
  "yOffset",
  "color",
  "fill",
  "stroke",
  "strokeDash",
  "shape",
  "detail",
  "row",
  "column",
  "facet",
];
/** The channels that place a mark, which a path mark's first point gives (fromClick). */
const PLACES = new Set(["x", "x2", "y", "y2", "xOffset", "yOffset"]);
const COLOURS = ["color", "fill", "stroke"];
/** The channels that facet a unit. */
const FACETS = ["row", "column", "facet"];
/** Transforms whose rows are their groupby's: the columns past them. */
const GROUPED = ["aggregate", "pivot", "regression", "loess", "density", "quantile"];
/** Marks drawn as one path per group, whose click datum is the group's first point. */
const PATHS = new Set(["line", "area", "trail"]);
/** Marks Vega-Lite stacks: their ring keeps every row, so its stack is theirs. */
const STACKED = new Set(["bar", "area"]);
/** Marks a ring can't outline. */
const UNRINGED = new Set(["text", "image", "geoshape", "arc", "boxplot", "errorbar", "errorband"]);
/** Marks whose fill a ring leaves out by `filled`. */
const SYMBOLS = new Set(["point", "circle", "square"]);
/** Marks whose fill a ring leaves out by its opacity. */
const AREAS = new Set(["rect", "bar", "area"]);
const INK = "#0b0b0b";
/** The field a faceted mark's ring orders its marks by. */
const RINGED = "_ringed";

/** A layer of a panel: its mark, whether it draws data of its own, and its encoding. */
export interface Layer {
  mark: string | null;
  own: boolean;
  encoding: Spec;
}

/** A panel of a spec (views.Panel). */
export interface Panel {
  id: string;
  path: string;
  title: string | null;
  time: string | null;
  x2: string | null;
  layers: Layer[];
  facet: Spec;
  encoded: string[];
}

/** What a chain of transforms leaves of the view's columns: the fields it made, the columns past a groupby's. */
interface Rows {
  made: Set<string>;
  kept: Set<string> | null;
}

const column = (rows: Rows, f: unknown): f is string =>
  typeof f === "string" && !rows.made.has(f) && (rows.kept === null || rows.kept.has(f));

const names = (v: unknown): string[] =>
  typeof v === "string"
    ? [v]
    : Array.isArray(v)
      ? v.filter((x): x is string => typeof x === "string")
      : [];

const transforms = (s: Spec): Spec[] =>
  Array.isArray(s.transform) ? s.transform.filter(isSpec) : [];

const scan = (chain: Spec[], from: Rows = { made: new Set(), kept: null }): Rows => {
  let rows = from;
  for (const t of chain) {
    if ("filter" in t) continue;
    const made = new Set(names(t.as));
    for (const k of ["aggregate", "joinaggregate", "window"])
      if (Array.isArray(t[k]))
        for (const a of t[k] as unknown[])
          if (isSpec(a) && typeof a.as === "string") made.add(a.as);
    if ("fold" in t && !("as" in t)) ["key", "value"].forEach((k) => made.add(k));
    if ("lookup" in t && !("as" in t) && isSpec(t.from))
      names(t.from.fields).forEach((k) => made.add(k));
    let kept = rows.kept;
    if (GROUPED.some((k) => k in t)) {
      const groupby = new Set(names(t.groupby));
      kept = kept === null ? groupby : new Set([...kept].filter((k) => groupby.has(k)));
    }
    rows = { made: new Set([...rows.made, ...made]), kept };
  }
  return rows;
};

const markOf = (s: Spec): string | null =>
  typeof s.mark === "string"
    ? s.mark
    : isSpec(s.mark) && typeof s.mark.type === "string"
      ? s.mark.type
      : null;

const titleOf = (s: Spec): string | null => {
  let t = s.title;
  if (isSpec(t)) t = t.text;
  if (Array.isArray(t)) t = t.map(String).join(" ");
  return typeof t === "string" && t ? t : null;
};

/** A channel's fields that are a row's values: its definitions with a field and no aggregate. */
const fieldsOf = (defs: unknown): string[] =>
  (Array.isArray(defs) ? defs : [defs])
    .filter((d): d is Spec => isSpec(d) && typeof d.field === "string" && !d.aggregate)
    .map((d) => d.field as string);

const isPanel = (s: Spec): boolean =>
  s.mark !== undefined || Array.isArray(s.layer) || (isSpec(s.facet) && isSpec(s.spec));

/**
 * A spec's panels, in its concats' order, as views.panels walks them (the module's docs).
 *
 * @param spec - a saved view's spec
 * @returns its panels
 */
export const panels = (spec: unknown): Panel[] => {
  if (!isSpec(spec)) return [];
  const hover = spec.hover;
  const top = isSpec(hover) && typeof hover.x === "string" ? hover.x : null;
  const out: Panel[] = [];

  const panel = (s: Spec, path: string, outer: Spec[], isTop: boolean) => {
    let chain = [...outer, ...transforms(s)];
    let facet: Spec = {};
    let inner = s;
    if (isSpec(s.facet) && isSpec(s.spec)) {
      const f = s.facet;
      facet =
        "field" in f
          ? { facet: f }
          : Object.fromEntries(Object.entries(f).filter(([, v]) => isSpec(v)));
      inner = s.spec;
      chain = [...chain, ...transforms(inner)];
    }
    const rows = scan(chain);
    const own = (!isTop && "data" in s) || (inner !== s && "data" in inner);
    const layers: [Layer, Spec[]][] = [];
    const flat = (u: Spec, enc: Spec, own: boolean, extra: Spec[], root: boolean) => {
      const e = { ...enc, ...(isSpec(u.encoding) ? u.encoding : {}) };
      const mine = own || (!root && "data" in u);
      const tf = [...extra, ...(root ? [] : transforms(u))];
      if (Array.isArray(u.layer)) {
        for (const c of u.layer) if (isSpec(c)) flat(c, e, mine, tf, false);
      } else layers.push([{ mark: markOf(u), own: mine, encoding: e }, tf]);
    };
    flat(inner, {}, own, [], true);
    const time = top !== null && "hover" in s ? top : null;
    const encoded: string[] = [];
    let x2: string | null = null;
    for (const d of Object.values(facet))
      encoded.push(...fieldsOf(d).filter((f) => column(rows, f)));
    for (const [layer, tf] of layers) {
      if (layer.own) continue;
      const lrows = scan(tf, rows);
      for (const ch of SELECTABLE)
        encoded.push(...fieldsOf(layer.encoding[ch]).filter((f) => column(lrows, f)));
      const xs = fieldsOf(layer.encoding.x);
      const x2s = fieldsOf(layer.encoding.x2);
      if (
        time !== null &&
        x2 === null &&
        xs.length === 1 &&
        xs[0] === time &&
        x2s.length > 0 &&
        x2s[0] !== time &&
        column(lrows, x2s[0])
      )
        x2 = x2s[0];
    }
    out.push({
      id: "",
      path,
      title: titleOf(s),
      time,
      x2,
      layers: layers.map(([l]) => l),
      facet,
      encoded: [...new Set(encoded)],
    });
  };

  const walk = (s: Spec, path: string, outer: Spec[], isTop: boolean) => {
    if (isPanel(s)) {
      panel(s, path || "0", outer, isTop);
      return;
    }
    const chain = [...outer, ...transforms(s)];
    for (const k of CONCATS)
      if (Array.isArray(s[k]))
        (s[k] as unknown[]).forEach((c, i) => {
          if (isSpec(c)) walk(c, path ? `${path}.${i}` : String(i), chain, false);
        });
  };

  walk(spec, "", [], true);
  const titles = out.map((p) => p.title);
  const paths = new Set(out.map((p) => p.path));
  for (const p of out)
    p.id =
      p.title && titles.filter((t) => t === p.title).length === 1 && !paths.has(p.title)
        ? p.title
        : p.path;
  return out;
};

/**
 * The time field the panels share: the first time panel's.
 *
 * @param ps - a spec's panels
 * @returns its time field, or null for none
 */
export const timeField = (ps: Panel[]): string | null => ps.find((p) => p.time)?.time ?? null;

const isNumber = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v);
const isValue = (v: unknown): v is Value =>
  v === null || typeof v === "string" || typeof v === "boolean" || isNumber(v);
const isRange = (v: unknown): v is Range =>
  Array.isArray(v) && v.length === 2 && isNumber(v[0]) && isNumber(v[1]) && v[0] <= v[1];

/** Why a selection doesn't check against a graph. */
export class SelError extends Error {}

/**
 * `sel` checked against a graph's panels, as views.selection checks it.
 *
 * @param ps - the graph's panels
 * @param sel - a selection, as the address gave it
 * @param name - the graph's name, for the errors
 * @returns the selection as the address holds it (its panel's id, where the graph has more than
 *   one, then `at` with its keys sorted), and its panel, if it names one or the graph has one
 * @throws SelError for a field no panel (or not its panel) encodes, a range on a field other than
 *   the time field, or a panel the graph lacks
 */
export const checked = (
  ps: Panel[],
  sel: unknown,
  name = "the view"
): { sel: Sel; panel: Panel | null } => {
  if (!isSpec(sel) || Object.keys(sel).some((k) => k !== "panel" && k !== "at"))
    throw new SelError('a selection is {"panel"?: ID, "at": {FIELD: VALUE | [LO, HI], ...}}');
  if (ps.length === 0) throw new SelError(`${name} draws no graph to select on`);
  const at = sel.at;
  if (!isSpec(at) || Object.keys(at).length === 0)
    throw new SelError("a selection's at names a field or more: {FIELD: VALUE | [LO, HI], ...}");
  let panel: Panel | null = null;
  if ("panel" in sel) {
    const want = sel.panel;
    panel = ps.find((p) => p.id === want) ?? ps.find((p) => p.path === want) ?? null;
    if (!panel)
      throw new SelError(
        `${name} has no panel ${JSON.stringify(want)}: its panels are ${ps
          .map((p) => JSON.stringify(p.id))
          .join(", ")}`
      );
  } else if (ps.length === 1) panel = ps[0];
  const fields = panel ? panel.encoded : [...new Set(ps.flatMap((p) => p.encoded))];
  const time = panel ? panel.time : timeField(ps);
  const scope = panel && ps.length > 1 ? `panel ${JSON.stringify(panel.id)}` : name;
  for (const [f, v] of Object.entries(at)) {
    if (!fields.includes(f))
      throw new SelError(
        `${scope} encodes no column ${JSON.stringify(f)}: its columns are ${
          fields.join(", ") || "none"
        }`
      );
    if (Array.isArray(v)) {
      if (!isRange(v))
        throw new SelError(
          `${f}'s range is [LO, HI], two numbers, LO first: not ${JSON.stringify(v)}`
        );
      if (f !== time)
        throw new SelError(
          `a range is on ${scope}'s time field${
            time ? `, ${time}` : ", which it has none of"
          }: not ${f}`
        );
    } else if (!isValue(v))
      throw new SelError(`${f}'s value is a string, number, boolean or null, or a range [LO, HI]`);
  }
  const sorted = Object.fromEntries(
    Object.keys(at)
      .sort()
      .map((k) => [k, at[k] as Value | Range])
  );
  return {
    sel: panel && ps.length > 1 ? { panel: panel.id, at: sorted } : { at: sorted },
    panel,
  };
};

/**
 * A selection as the address writes it: compact JSON, `panel` then `at`, as JSON.stringify writes
 * it (views.js_json writes the same bytes).
 *
 * @param sel - a checked selection
 * @returns its text
 */
export const selJson = (sel: Sel): string =>
  JSON.stringify(sel.panel !== undefined ? { panel: sel.panel, at: sel.at } : { at: sel.at });

/** A selection read from an address: the checked one, or why it doesn't check. */
export interface Read {
  sel: Sel | null;
  panel: Panel | null;
  error: string | null;
}

/**
 * The address's selection text checked against a graph's panels.
 *
 * @param ps - the graph's panels
 * @param text - the address's `sel` ("" for none)
 * @param name - the graph's name, for the errors
 * @returns the selection, or the error that kept it out
 */
export const readSel = (ps: Panel[], text: string, name?: string): Read => {
  if (!text) return { sel: null, panel: null, error: null };
  let given: unknown;
  try {
    given = JSON.parse(text);
  } catch {
    return { sel: null, panel: null, error: `the address's sel isn't JSON: ${text}` };
  }
  try {
    return { ...checked(ps, given, name), error: null };
  } catch (e) {
    return { sel: null, panel: null, error: e instanceof Error ? e.message : String(e) };
  }
};

/**
 * A selection in a line, as views.selection_text words it: its panel's id, then each field's
 * value or range.
 *
 * @param ps - the graph's panels
 * @param sel - a selection
 * @returns its words
 */
export const selText = (ps: Panel[], sel: Sel): string => {
  const { sel: s, panel } = checked(ps, sel);
  const out = panel && s.panel !== undefined ? [panel.id] : [];
  for (const [f, v] of Object.entries(s.at))
    out.push(
      Array.isArray(v) ? `${f} ${String(v[0])}–${String(v[1])}` : `${f} ${JSON.stringify(v)}`
    );
  return out.join(" · ");
};

/**
 * The fields of a path mark's group: those it doesn't place by (its colour, detail, dash, the
 * facet's), which its first point's datum holds for the whole path.
 */
const groupFields = (p: Panel): string[] => {
  const out = Object.values(p.facet).flatMap(fieldsOf);
  for (const l of p.layers)
    if (!l.own)
      for (const ch of SELECTABLE) if (!PLACES.has(ch)) out.push(...fieldsOf(l.encoding[ch]));
  return [...new Set(out)].filter((f) => p.encoded.includes(f));
};

const canonical = (ps: Panel[], sel: Sel): Sel | null => {
  try {
    return checked(ps, sel).sel;
  } catch {
    return null;
  }
};

/**
 * The selection a click on a mark makes: its datum's values of the panel's columns. A path mark's
 * datum is its first point, so of it only the fields of its group; on a time panel the time is
 * the cursor's (hover.ts), where it gives one.
 *
 * @param ps - the graph's panels
 * @param panel - the panel clicked
 * @param datum - the mark's datum
 * @param opts - the cursor's time (null for none), and whether the mark is a path
 * @returns the selection, or null where the datum holds none of the panel's columns
 */
export const fromClick = (
  ps: Panel[],
  panel: Panel,
  datum: Record<string, unknown>,
  opts: { time?: number | null; path?: boolean } = {}
): Sel | null => {
  const at: Record<string, Value> = {};
  for (const f of opts.path ? groupFields(panel) : panel.encoded) {
    const v = datum[f];
    if (v !== undefined && isValue(v)) at[f] = v;
  }
  if (panel.time && isNumber(opts.time)) at[panel.time] = opts.time;
  if (Object.keys(at).length === 0) return null;
  return canonical(ps, { panel: panel.id, at });
};

/**
 * The selection of a moment on the time field: every track's rows at it.
 *
 * @param ps - the graph's panels
 * @param t - the moment
 * @returns the selection, or null for a graph with no time field
 */
export const fromMoment = (ps: Panel[], t: number): Sel | null => {
  const time = timeField(ps);
  return time && isNumber(t) ? canonical(ps, { at: { [time]: t } }) : null;
};

/**
 * `v` to the digits a range of `span` tells apart: its second significant digit's (a drag's ends
 * are as near as the pointer puts them, a pixel's width of the time field).
 *
 * @param v - a brush's end
 * @param span - the brush's width
 * @returns it rounded
 */
const rounded = (v: number, span: number): number => {
  const digits = span > 0 ? Math.min(12, Math.max(0, 1 - Math.floor(Math.log10(span)))) : 2;
  return Number(v.toFixed(digits));
};

/**
 * The selection a brush on a time panel makes: a range of its time field, its ends to the
 * digits its width tells apart.
 *
 * @param ps - the graph's panels
 * @param panel - the time panel brushed
 * @param lo - the brush's low end, in the time field's units
 * @param hi - its high end
 * @returns the selection, or null for a panel with no time field or a brush of no width
 */
export const fromBrush = (ps: Panel[], panel: Panel, lo: number, hi: number): Sel | null => {
  if (!panel.time || !isNumber(lo) || !isNumber(hi)) return null;
  const [a, b] = lo <= hi ? [lo, hi] : [hi, lo];
  const range: Range = [rounded(a, b - a), rounded(b, b - a)];
  if (!(range[0] < range[1])) return null;
  return canonical(ps, { panel: panel.id, at: { [panel.time]: range } });
};

/** The fields a panel's marks encode as quantities: what a mark measures, not which it is. */
const measures = (p: Panel): Set<string> => {
  const out = new Set<string>();
  for (const l of p.layers)
    if (!l.own)
      for (const ch of SELECTABLE) {
        const defs = l.encoding[ch];
        for (const d of Array.isArray(defs) ? (defs as unknown[]) : [defs])
          if (isSpec(d) && typeof d.field === "string" && d.type === "quantitative")
            out.add(d.field);
      }
  return out;
};

/**
 * Selection `add`'s values added to `old` (shift-click): its panel the one `add` names, its fields
 * `old`'s and `add`'s (`add`'s where both name one), but a range `old` holds on the time field,
 * which it keeps, adding which marks the click names in it (a viewer, a series) and not the mark's
 * own time, span or quantities; then only the fields its panel encodes.
 *
 * @param ps - the graph's panels
 * @param old - the selection
 * @param add - what a click selected
 * @returns the selection of both
 */
export const added = (ps: Panel[], old: Sel, add: Sel): Sel => {
  const id = add.panel ?? old.panel;
  const panel =
    id !== undefined ? (ps.find((p) => p.id === id) ?? null) : ps.length === 1 ? ps[0] : null;
  const time = panel ? panel.time : timeField(ps);
  const keep = time !== null && Array.isArray(old.at[time]);
  const measured = panel ? measures(panel) : new Set<string>();
  const at: Record<string, Value | Range> = { ...old.at };
  for (const [f, v] of Object.entries(add.at)) {
    if (keep && (f === time || (panel && f === panel.x2) || measured.has(f))) continue;
    at[f] = v;
  }
  if (panel) for (const f of Object.keys(at)) if (!panel.encoded.includes(f)) delete at[f];
  return canonical(ps, { ...(id !== undefined ? { panel: id } : {}), at }) ?? add;
};

// --- the highlight ----------------------------------------------------------------------------------

const expr = (v: unknown): string => JSON.stringify(v);
const field = (f: string): string => `datum[${expr(f)}]`;

/** What a selection draws besides its rings: the band of its range and the rule of its moment. */
export interface Highlight {
  spec: Spec;
  /** A range of the time field: a band in every time panel. */
  band?: Range;
  /** A moment: a rule in every time panel. */
  moment?: number;
}

/** The node at `path` of `spec`'s concats, replaced by `f` of it. */
const replaced = (spec: Spec, path: string, f: (s: Spec) => Spec): Spec => {
  if (isPanel(spec)) return path === "0" ? f(spec) : spec;
  const [i, ...rest] = path.split(".").map(Number);
  for (const k of CONCATS)
    if (Array.isArray(spec[k])) {
      const kids = [...(spec[k] as unknown[])];
      const c = kids[i];
      if (!isSpec(c)) return spec;
      kids[i] = rest.length === 0 && isPanel(c) ? f(c) : replaced(c, rest.join(".") || "0", f);
      return { ...spec, [k]: kids };
    }
  return spec;
};

/** A unit spec's own keys, which go to its first layer when it becomes a layer spec. */
const UNIT_KEYS = new Set(["mark", "encoding", "params", "projection", "name"]);

/** The test of a ring's rows: the selection's values, and its time where the mark has its own. */
const ringTest = (panel: Panel, sel: Sel, path: boolean, stacked: boolean): string | null => {
  const values = Object.entries(sel.at).filter(([f]) => f !== panel.time);
  if (values.length === 0) return null;
  const parts = values.map(([f, v]) => `${field(f)} === ${expr(v)}`);
  const t = panel.time ? sel.at[panel.time] : undefined;
  if (panel.time && t !== undefined && !(path && stacked)) {
    const x = field(panel.time);
    if (Array.isArray(t)) {
      const end =
        panel.x2 && !path ? `(isValid(${field(panel.x2)}) ? ${field(panel.x2)} : ${x})` : x;
      parts.push(`${x} <= ${expr(t[1])} && ${end} >= ${expr(t[0])}`);
    } else if (!path) parts.push(`${x} === ${expr(t)}`);
  }
  return parts.join(" && ");
};

/**
 * A ring on a layer's marks the selection names: a copy of the layer drawn as an outline, its
 * colours and opacities its own. Its rows are the selection's, but a stacked mark's, whose copy
 * keeps every row so that its stack is the layer's, and hides the others.
 */
const ring = (c: unknown, panel: Panel, sel: Sel, name: string): Spec | null => {
  if (!isSpec(c) || c.data !== undefined || Array.isArray(c.layer)) return null;
  const m = markOf(c);
  if (!m || UNRINGED.has(m)) return null;
  const path = PATHS.has(m);
  const stacked = STACKED.has(m);
  const test = ringTest(panel, sel, path, stacked);
  if (!test) return null;
  const enc: Spec = { ...(isSpec(c.encoding) ? c.encoding : {}) };
  for (const k of [
    "tooltip",
    "opacity",
    "fillOpacity",
    "strokeOpacity",
    "strokeWidth",
    "href",
    "url",
    "description",
  ])
    delete enc[k];
  if (!stacked) {
    const groups = COLOURS.flatMap((ch) =>
      (Array.isArray(enc[ch]) ? (enc[ch] as unknown[]) : [enc[ch]])
        .filter((d): d is Spec => isSpec(d) && typeof d.field === "string")
        .map((d) => ({ field: d.field, ...(d.type ? { type: d.type } : {}) }))
    );
    for (const ch of COLOURS) delete enc[ch];
    const detail: unknown[] = Array.isArray(enc.detail)
      ? (enc.detail as unknown[])
      : enc.detail
        ? [enc.detail]
        : [];
    if (path && groups.length > 0) enc.detail = [...detail, ...groups];
  } else enc.strokeOpacity = { condition: { test, value: 1 }, value: 0 };
  const given: Spec = isSpec(c.mark) ? { ...c.mark } : { type: m };
  const width = isNumber(given.strokeWidth) ? given.strokeWidth : 2;
  for (const k of [
    "color",
    "fill",
    "stroke",
    "opacity",
    "fillOpacity",
    "strokeOpacity",
    "strokeWidth",
    "filled",
    "point",
    "line",
    "tooltip",
    "cursor",
    "href",
  ])
    delete given[k];
  const mark: Spec = {
    ...given,
    stroke: INK,
    strokeWidth: path ? width + 2 : 2,
    opacity: 1,
    ...(SYMBOLS.has(m) ? { filled: false } : AREAS.has(m) ? { fillOpacity: 0 } : {}),
  };
  const tf = [...transforms(c), ...(stacked ? [] : [{ filter: test }])];
  return { name, mark, encoding: enc, ...(tf.length ? { transform: tf } : {}) };
};

/**
 * A faceted panel's marks with the selection's ringed in place: a filled mark's stroke drawn where
 * the selection names it, a stroked mark's (a line's, a rule's) stroke widened. A layer of its own
 * in a facet would warn of rows from another source where its cells' scales are their own, so a
 * facet takes no ring layer.
 */
const outlined = (c: unknown, panel: Panel, sel: Sel): unknown => {
  if (!isSpec(c) || c.data !== undefined) return c;
  if (Array.isArray(c.layer)) return { ...c, layer: c.layer.map((k) => outlined(k, panel, sel)) };
  const m = markOf(c);
  if (!m || UNRINGED.has(m)) return c;
  const mark: Spec = isSpec(c.mark) ? c.mark : { type: m };
  const path = PATHS.has(m);
  const test = ringTest(panel, sel, path, path);
  if (!test) return c;
  const enc: Spec = isSpec(c.encoding) ? c.encoding : {};
  if (isSpec(enc.strokeWidth)) return c;
  const filled = (AREAS.has(m) || SYMBOLS.has(m)) && mark.filled !== false;
  const width = isNumber(mark.strokeWidth) ? mark.strokeWidth : filled ? 1 : 2;
  const strokeWidth = { condition: { test, value: filled ? 2 : width + 2 }, value: width };
  if (!filled) return { ...c, encoding: { ...enc, strokeWidth } };
  const stroke = isSpec(enc.stroke)
    ? { ...enc.stroke, condition: { test, value: INK } }
    : { condition: { test, value: INK }, value: mark.stroke ?? null };
  // The marks it names whole, though their neighbours are faint.
  const opacity = isSpec(enc.opacity)
    ? { opacity: { ...enc.opacity, condition: { test, value: 1 } } }
    : {};
  // The marks it names drawn last, over their neighbours' edges; a stack's order is its own.
  if (STACKED.has(m) || enc.order !== undefined)
    return { ...c, encoding: { ...enc, stroke, strokeWidth, ...opacity } };
  return {
    ...c,
    transform: [...transforms(c), { calculate: `(${test}) ? 1 : 0`, as: RINGED }],
    encoding: {
      ...enc,
      stroke,
      strokeWidth,
      ...opacity,
      order: { field: RINGED, type: "quantitative" },
    },
  };
};

/** `node` (a panel) with rings on its layers' marks the selection names. */
const ringed = (node: Spec, panel: Panel, sel: Sel, next: () => string): Spec => {
  if (isSpec(node.facet) && isSpec(node.spec))
    return { ...node, spec: outlined(node.spec, panel, sel) };
  if (Array.isArray(node.layer)) {
    const kids = node.layer.map((c: unknown) =>
      isSpec(c) && Array.isArray(c.layer) && c.data === undefined ? ringed(c, panel, sel, next) : c
    );
    const rings = (node.layer as unknown[])
      .map((c) => ring(c, panel, sel, next()))
      .filter((r): r is Spec => r !== null);
    return { ...node, layer: [...kids, ...rings] };
  }
  if (isSpec(node.encoding) && FACETS.some((k) => k in (node.encoding as Spec)))
    return outlined(node, panel, sel) as Spec;
  const unit: Spec = {};
  const outer: Spec = {};
  for (const [k, v] of Object.entries(node)) (UNIT_KEYS.has(k) ? unit : outer)[k] = v;
  const r = ring(unit, panel, sel, next());
  return r ? { ...outer, layer: [unit, r] } : node;
};

/**
 * `spec` with selection `sel`'s rings drawn, and the band and the moment hover.ts draws in every
 * time panel (the module's docs). A spec with no selection is the spec given.
 *
 * @param spec - a saved view's spec, its vocabulary expanded
 * @param sel - a checked selection, or null
 * @returns the spec with its rings, and the band and moment
 */
export const highlighted = (spec: Spec, sel: Sel | null): Highlight => {
  if (!sel) return { spec };
  const ps = panels(spec);
  const time = timeField(ps);
  const t = time !== null ? sel.at[time] : undefined;
  const fields = Object.keys(sel.at);
  const targets =
    sel.panel !== undefined
      ? ps.filter((p) => p.id === sel.panel)
      : ps.length === 1
        ? ps
        : ps.filter((p) => fields.every((f) => p.encoded.includes(f)));
  let n = 0;
  const next = () => `ring_${n++}`;
  let out = spec;
  for (const p of targets) out = replaced(out, p.path, (node) => ringed(node, p, sel, next));
  return {
    spec: out,
    ...(Array.isArray(t) ? { band: t } : {}),
    ...(isNumber(t) ? { moment: t } : {}),
  };
};

/** The scenegraph's part of a Vega item a click names: its mark, that mark's group, and up. */
interface SceneMark {
  name?: string;
  marktype?: string;
  role?: string;
  group?: { mark?: SceneMark };
}

/**
 * The index path of the panel a scenegraph mark is in, from Vega-Lite's names for a concat's
 * members (`concat_0_concat_2_…`), on it or its groups: null outside any concat.
 *
 * @param mark - an item's mark
 * @returns its panel's path, or null
 */
export const pathOf = (mark: SceneMark | undefined): string | null => {
  for (let m = mark; m; m = m.group?.mark) {
    const prefix = /^((?:concat_\d+_)+)/.exec(m.name ?? "");
    if (prefix) return [...prefix[1].matchAll(/concat_(\d+)_/g)].map((x) => x[1]).join(".");
  }
  return null;
};

// --- addresses ----------------------------------------------------------------------------------

/** The workbench's route. */
export const WORKBENCH = "workbench";
/** The keys the workbench takes besides its statement's `$name`s. */
export const WORKBENCH_KEYS = [...STATEMENT, ...CHART, SEL];

/**
 * The `$name`s a statement takes, in the order it first names them, outside its strings and
 * comments.
 *
 * @param sql - a statement
 * @returns its parameters' names
 */
export const paramNames = (sql: string): string[] => {
  const bare = sql.replace(/--[^\n]*|\/\*[\s\S]*?\*\/|'(?:[^']|'')*'/g, " ");
  return [...new Set([...bare.matchAll(/\$([A-Za-z_]\w*)/g)].map((m) => m[1]))];
};

/** The workbench's `$name`s an address sets: the selection's keys first, in KEYS' order. */
const workbenchNames = (sql: string): string[] => {
  const names = paramNames(sql).filter((n) => !WORKBENCH_KEYS.includes(n));
  return [...KEYS.filter((k) => names.includes(k)), ...names.filter((n) => !isKey(n))];
};

/**
 * The key of the address a saved view's param is read from (views.url_key): its name when that is
 * a key of the selection, else its kind when that is one.
 */
const urlKey = (p: { name: string; kind: string }): string | null =>
  isKey(p.name) ? p.name : isKey(p.kind) ? p.kind : null;

/** The workbench's auto-chart choices, as the address holds them. */
export type Chart = Partial<Record<"mark" | "x" | "y" | "color" | "logy", string>>;

/**
 * The workbench's auto-chart of `chart`'s choices, as views.chart_spec draws it (its fields' types
 * left to the rows): null without x and y.
 */
export const chartSpec = (chart: Chart): Spec | null =>
  chart.x && chart.y
    ? {
        mark: { type: chart.mark || "auto", tooltip: true },
        encoding: {
          x: { field: chart.x },
          y: { field: chart.y, ...(chart.logy === "1" ? { scale: { type: "symlog" } } : {}) },
          ...(chart.color ? { color: { field: chart.color } } : {}),
        },
      }
    : null;

/** A console address's route, the params the page reads from it as text, and its selection. */
export interface Address {
  name: string;
  params: Record<string, string>;
  sel: unknown;
  query: URLSearchParams;
}

/**
 * Console address `url` read as views.parse reads it: its route, each param the page reads as
 * text (its key's default where the address leaves it out), and its selection as given.
 *
 * @param url - a console address
 * @param views - the saved views
 * @returns the address
 * @throws for an address of no view, or a selection that isn't JSON
 */
export const parseAddress = (
  url: string,
  views: Pick<SavedView, "name" | "params" | "sql">[]
): Address => {
  const hash = url.trim().split("#").slice(1).join("#");
  const at = hash.indexOf("?");
  const [path, qs] = at < 0 ? [hash, ""] : [hash.slice(0, at), hash.slice(at + 1)];
  if (path !== "/console" && !path.startsWith("/console/"))
    throw new Error(`not a console address (…/devtools.html#/console/VIEW?…): ${url}`);
  const name = path.slice("/console/".length).split("/")[0] || "experiments";
  const query = new URLSearchParams(qs);
  const text = query.get(SEL);
  let sel: unknown = null;
  if (text) {
    try {
      sel = JSON.parse(text);
    } catch {
      throw new Error(`the address's sel isn't JSON: ${text}`);
    }
  }
  const params: Record<string, string> = {};
  const read = (k: string) => query.get(k) ?? (isKey(k) ? DEFAULTS[k] : "");
  if (name === WORKBENCH) {
    for (const k of STATEMENT) if (query.get(k)) params[k] = query.get(k);
    const named = params.view ? views.find((v) => v.name === params.view) : undefined;
    if (params.view && !named)
      throw new Error(`${url}: the workbench's view ${params.view} is no saved view`);
    const statement = params.sql || named?.sql;
    const names = statement ? workbenchNames(statement) : KEYS.filter((k) => query.get(k));
    for (const n of names) if (read(n) !== "") params[n] = read(n);
    for (const k of CHART) if (query.get(k)) params[k] = query.get(k);
    return { name, params, sel, query };
  }
  const view = views.find((v) => v.name === name);
  if (!view) throw new Error(`${url} names no view ${name}`);
  for (const p of view.params) {
    const k = urlKey(p);
    if (k !== null && read(k) !== "") params[p.name] = read(k);
  }
  return { name, params, sel, query };
};

/**
 * A saved view's address query, as views.link writes it: each param under its key (urlKey), in
 * KEYS' order, none empty or its key's default, then the selection.
 *
 * @param view - the saved view
 * @param params - its params, as text
 * @param sel - a checked selection, or null
 * @returns the query
 */
export const viewQuery = (
  view: Pick<SavedView, "params">,
  params: Record<string, string>,
  sel: Sel | null = null
): URLSearchParams => {
  const q = new URLSearchParams();
  for (const p of view.params) {
    const k = urlKey(p);
    if (k !== null && params[p.name] !== undefined && !q.has(k)) q.set(k, params[p.name]);
  }
  if (sel) q.set(SEL, selJson(sel));
  return ordered(q);
};

/**
 * The workbench's address query, as views.link writes it: its view or statement, the statement's
 * `$name`s (the selection's keys first), the auto-chart's choices, then the selection.
 *
 * @param params - `view` or `sql`, the `$name`s' values and the chart's choices, as text
 * @param sel - a checked selection, or null
 * @param views - the saved views
 * @returns the query
 */
export const workbenchQuery = (
  params: Record<string, string>,
  sel: Sel | null,
  views: Pick<SavedView, "name" | "sql">[]
): URLSearchParams => {
  const statement = params.sql || views.find((v) => v.name === params.view)?.sql || "";
  const q = new URLSearchParams();
  if (params.view) q.set("view", params.view);
  else if (params.sql) q.set("sql", params.sql);
  for (const n of statement ? workbenchNames(statement) : KEYS)
    if (params[n] !== undefined) q.set(n, params[n]);
  for (const k of CHART) if (params[k]) q.set(k, params[k]);
  if (sel) q.set(SEL, selJson(sel));
  return ordered(q);
};

/**
 * A param's value as an address's text: a string as itself, a number as JavaScript writes it,
 * else none.
 *
 * @param v - a bound value
 * @returns its text, or null for a value an address can't hold
 */
export const textOf = (v: Json | undefined): string | null =>
  typeof v === "string" ? v : isNumber(v) ? String(v) : null;

/**
 * The auto-chart choices that draw `spec`, where it is one mark of x and y fields.
 *
 * @param spec - a chart's spec
 * @returns its choices, or null for a spec they can't draw
 */
export const chartOf = (spec: unknown): Chart | null => {
  if (!isSpec(spec) || !isSpec(spec.encoding)) return null;
  const mark = markOf(spec);
  const { x, y, color } = spec.encoding;
  if (
    !mark ||
    !isSpec(x) ||
    !isSpec(y) ||
    typeof x.field !== "string" ||
    typeof y.field !== "string"
  )
    return null;
  const scale = isSpec(y.scale) ? y.scale.type : undefined;
  return {
    ...(["line", "point", "bar", "area", "rect", "tick"].includes(mark) ? { mark } : {}),
    x: x.field,
    y: y.field,
    ...(isSpec(color) && typeof color.field === "string" ? { color: color.field } : {}),
    ...(scale === "symlog" || scale === "log" ? { logy: "1" } : {}),
  };
};

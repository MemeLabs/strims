// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import useResizeObserver from "@react-hook/resize-observer";
import React, { useEffect, useLayoutEffect, useRef, useState } from "react";
import embed from "vega-embed";
import type { Result, VisualizationSpec } from "vega-embed";

import type { Json } from "./api";
import { expandVocab } from "./domains";
import { CURSOR, CURSOR_MARKS, brushName, hovered } from "./hover";
import { added, fromBrush, fromClick, fromMoment, highlighted, panels, pathOf } from "./sel";
import type { Panel, Sel } from "./sel";
import { config } from "./theme";

type Spec = Record<string, unknown>;

const isSpec = (v: unknown): v is Spec => typeof v === "object" && v !== null && !Array.isArray(v);

/**
 * `spec` with each `"width": "container"` set to `width` px: a concatenated view's panels can't size
 * to their container themselves.
 *
 * @param spec - a Vega-Lite spec
 * @param width - the chart's width
 * @returns the spec with numeric widths
 */
const sized = (spec: Spec, width: number): Spec => {
  const out: Spec = {};
  for (const [k, v] of Object.entries(spec)) {
    if (k === "width" && v === "container") out[k] = width;
    else if (Array.isArray(v))
      out[k] = v.map((x) => (isSpec(x) ? sized(x, width) : (x as unknown)));
    else if (isSpec(v) && k !== "data" && k !== "datum") out[k] = sized(v, width);
    else out[k] = v;
  }
  return out;
};

/** The marks a selection draws (hover.ts's brushes and moments, sel.ts's rings), which take no pointer events. */
const DRAWN = /^(brush_[\d_]+_brush(_bg)?|moment_\d+_marks|ring_\d+_marks)$/;
/** The cursor's invisible rules and their voronoi: a click there is on the time, not a mark. */
const CURSOR_RULES = /^hover_\d+_(marks|voronoi)$/;
/** How far a press moves before it is a drag (a brush), not a click. */
const DRAG_PX = 4;

/** The compiled Vega spec with the selection's marks out of the way of the pointer. */
const quiet = (vg: Spec): Spec => {
  const walk = (m: Spec) => {
    for (const x of Array.isArray(m.marks) ? m.marks : [])
      if (isSpec(x)) {
        if (typeof x.name === "string" && DRAWN.test(x.name)) x.interactive = false;
        if (x.type === "group") walk(x);
      }
  };
  walk(vg);
  return vg;
};

/** A scenegraph item, as a Vega event handler gets it. */
interface Item {
  datum?: unknown;
  mark?: { name?: string; marktype?: string; role?: string; group?: { mark?: Item["mark"] } };
}

const editable = (t: EventTarget | null): boolean =>
  t instanceof HTMLElement &&
  (t.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(t.tagName));

export interface VegaChartProps {
  spec: Spec;
  rows: Record<string, Json>[];
  /** The selection the chart draws (sel.ts), checked against its panels. */
  sel?: Sel | null;
  /**
   * Takes the selection a click, a drag on a time panel, a shift-click or Escape makes (null:
   * none); without it the chart takes no selections and draws none.
   */
  onSel?: (sel: Sel | null) => void;
}

/**
 * A Vega-Lite chart of a query's rows: the rows are its data, whatever data the spec names, and its
 * vocabulary references (domains.ts), then its hover keys (hover.ts), are expanded. Given `onSel`,
 * it draws the selection (sel.ts's rings, hover.ts's band and moment) and makes one:
 *
 * - a click on a mark selects its values, on a time panel at the cursor's time, and a click on a
 *   time series' plot the moment the cursor is at;
 * - a drag on a time panel brushes a range of its time field, read once the drag ends;
 * - shift adds a click's values to the selection;
 * - Escape, or a click on the plot's background, clears it.
 *
 * A new selection draws the chart again, the last one shown until the new one is.
 */
const VegaChart: React.FC<VegaChartProps> = ({ spec, rows, sel = null, onSel }) => {
  const select = useRef(onSel);
  select.current = onSel;
  const current = useRef(sel);
  current.current = sel;
  const selectable = onSel !== undefined;
  const box = useRef<HTMLDivElement>(null);
  const target = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const selKey = selectable && sel ? JSON.stringify(sel) : "";

  useLayoutEffect(() => setWidth(box.current?.clientWidth ?? 0), []);
  useResizeObserver(box, (e) => setWidth(Math.round(e.contentRect.width)));

  useEffect(() => {
    if (!selectable || !sel) return;
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !editable(e.target)) select.current?.(null);
    };
    document.addEventListener("keydown", key);
    return () => document.removeEventListener("keydown", key);
  }, [selectable, selKey]);

  useEffect(() => {
    const host = target.current;
    if (!host || width === 0) return;
    let result: Result | null = null;
    let live = true;
    const undo: (() => void)[] = [];
    let expanded: Spec;
    let ps: Panel[];
    let lit: ReturnType<typeof highlighted>;
    try {
      expanded = expandVocab(spec);
      ps = selectable ? panels(expanded) : [];
      lit = selectable ? highlighted(expanded, current.current) : { spec: expanded };
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      return;
    }
    const vl = hovered(
      lit.spec,
      selectable ? { brush: true, band: lit.band, moment: lit.moment } : {}
    );
    const full = {
      $schema: "https://vega.github.io/schema/vega-lite/v6.json",
      ...sized(vl, Math.max(200, width - 180)),
      data: { values: rows },
      config: { ...config, ...(isSpec(spec.config) ? spec.config : {}) },
    };
    setError(null);
    // The new chart draws out of sight over the last, which goes once it has.
    const el = document.createElement("div");
    if (host.childElementCount > 0) el.classList.add("console__chart-next");
    host.appendChild(el);
    embed(el, full as VisualizationSpec, {
      actions: { export: true, source: false, compiled: false, editor: false },
      renderer: "svg",
      ...(selectable ? { patch: quiet as never } : {}),
    }).then(
      (r) => {
        if (!live) {
          r.finalize();
          el.remove();
          return;
        }
        result = r;
        for (const c of [...host.children]) if (c !== el) c.remove();
        el.classList.remove("console__chart-next");
        const view = r.view;
        if (!selectable) return;
        const byPath = new Map(ps.map((p) => [p.path, p]));
        const panelOf = (it: Item): Panel | undefined =>
          byPath.get(pathOf(it.mark) ?? "0") ?? (ps.length === 1 ? ps[0] : undefined);
        // The time the cursor is at: the mark hovered's, else the nearest row's.
        const cursorAt = (field: string): number | null => {
          for (const name of [CURSOR_MARKS, CURSOR]) {
            try {
              const v = (view.signal(name) as Record<string, unknown> | null)?.[field];
              const t: unknown = Array.isArray(v) ? (v as unknown[])[0] : v;
              if (typeof t === "number" && Number.isFinite(t)) return t;
            } catch {
              // No such selection in this chart.
            }
          }
          return null;
        };
        let down: { x: number; y: number; path: string | null } | null = null;
        let dragged = false;
        view.addEventListener("pointerdown", (event, item) => {
          const e = event as PointerEvent;
          const it = item as Item | null | undefined;
          down = { x: e.clientX, y: e.clientY, path: it?.mark ? pathOf(it.mark) ?? "0" : null };
          dragged = false;
        });
        const moved = (e: PointerEvent) => {
          if (down && Math.hypot(e.clientX - down.x, e.clientY - down.y) > DRAG_PX) dragged = true;
        };
        const up = () => {
          const press = down;
          down = null;
          if (!press || !dragged || press.path === null) return;
          const panel = byPath.get(press.path);
          if (!panel?.time) return;
          const time = panel.time;
          void view.runAsync().then(() => {
            if (!live) return;
            let range: unknown;
            try {
              range = (view.signal(brushName(panel.path)) as Record<string, unknown> | null)?.[
                time
              ];
            } catch {
              return;
            }
            if (!Array.isArray(range) || range.length !== 2) return;
            const s = fromBrush(ps, panel, Number(range[0]), Number(range[1]));
            if (s) select.current?.(s);
          });
        };
        window.addEventListener("pointermove", moved, true);
        window.addEventListener("pointerup", up, true);
        undo.push(() => {
          window.removeEventListener("pointermove", moved, true);
          window.removeEventListener("pointerup", up, true);
        });
        view.addEventListener("click", (event, item) => {
          if (dragged) {
            dragged = false;
            return;
          }
          const shift = (event as MouseEvent).shiftKey;
          const it = item as Item | null | undefined;
          const now = current.current;
          if (!it?.mark || it.mark.marktype === "group") {
            if (now) select.current?.(null);
            return;
          }
          if (it.mark.role !== "mark") return;
          const panel = panelOf(it);
          if (!panel) return;
          let next: Sel | null = null;
          if (CURSOR_RULES.test(it.mark.name ?? "")) {
            const t = panel.time ? cursorAt(panel.time) : null;
            next = t === null ? null : fromMoment(ps, t);
          } else if (isSpec(it.datum)) {
            next = fromClick(ps, panel, it.datum, {
              time: panel.time ? cursorAt(panel.time) : null,
              path: ["line", "area", "trail"].includes(it.mark.marktype ?? ""),
            });
          }
          if (next) select.current?.(shift && now ? added(ps, now, next) : next);
        });
      },
      (e: unknown) => {
        el.remove();
        if (live) setError(e instanceof Error ? e.message : String(e));
      }
    );
    return () => {
      live = false;
      undo.forEach((f) => f());
      result?.finalize();
    };
  }, [spec, rows, width, selectable, selKey]);

  return (
    <div className="console__chart" ref={box}>
      {error && <div className="console__error">chart: {error}</div>}
      <div className="console__chart-host" ref={target} />
    </div>
  );
};

export default VegaChart;

// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import useResizeObserver from "@react-hook/resize-observer";
import React, { useEffect, useLayoutEffect, useRef, useState } from "react";
import embed from "vega-embed";
import type { Result, VisualizationSpec } from "vega-embed";

import type { Json } from "./api";
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

export interface VegaChartProps {
  spec: Spec;
  rows: Record<string, Json>[];
  /** A click on a mark, with its datum. */
  onSelect?: (datum: Record<string, Json>) => void;
}

/** A Vega-Lite chart of a query's rows: the rows are its data, whatever data the spec names. */
const VegaChart: React.FC<VegaChartProps> = ({ spec, rows, onSelect }) => {
  const select = useRef(onSelect);
  select.current = onSelect;
  const box = useRef<HTMLDivElement>(null);
  const target = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  const [error, setError] = useState<string | null>(null);

  useLayoutEffect(() => setWidth(box.current?.clientWidth ?? 0), []);
  useResizeObserver(box, (e) => setWidth(Math.round(e.contentRect.width)));

  useEffect(() => {
    if (!target.current || width === 0) return;
    let result: Result | null = null;
    let live = true;
    const full = {
      $schema: "https://vega.github.io/schema/vega-lite/v6.json",
      ...sized(spec, Math.max(200, width - 140)),
      data: { values: rows },
      config: { ...config, ...(isSpec(spec.config) ? spec.config : {}) },
    };
    setError(null);
    embed(target.current, full as VisualizationSpec, {
      actions: { export: true, source: false, compiled: false, editor: false },
      renderer: "svg",
    }).then(
      (r) => {
        if (!live) {
          r.finalize();
          return;
        }
        result = r;
        r.view.addEventListener("click", (_, item) => {
          const datum = (item as { datum?: unknown } | null | undefined)?.datum;
          if (isSpec(datum)) select.current?.(datum as Record<string, Json>);
        });
      },
      (e: unknown) => {
        if (live) setError(e instanceof Error ? e.message : String(e));
      }
    );
    return () => {
      live = false;
      result?.finalize();
    };
  }, [spec, rows, width]);

  return (
    <div className="console__chart" ref={box}>
      {error && <div className="console__error">chart: {error}</div>}
      <div ref={target} />
    </div>
  );
};

export default VegaChart;

// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useEffect, useMemo, useState } from "react";
import { useLocation, useNavigate } from "react-router-dom";

import { rowObjects, runSql } from "./api";
import type { Param, QueryResult, SavedView } from "./api";
import { expandVocab } from "./domains";
import ResultTable from "./ResultTable";
import { WORKBENCH_KEYS, panels, paramNames, readSel, selJson, selText, textOf } from "./sel";
import type { Panel, Read, Sel } from "./sel";
import { carried, ordered, useSel, viewParams } from "./selection";
import type { Selection } from "./selection";
import SqlEditor from "./SqlEditor";
import VegaChart from "./VegaChart";

/** A query's state: running, failed, or its result. */
export interface Run {
  result: QueryResult | null;
  error: string | null;
  running: boolean;
}

/**
 * Runs `sql` with `params` whenever either changes (not at all while `skip`), aborting the last.
 *
 * @param sql - the statement
 * @param params - its parameters
 * @param skip - whether to hold off
 * @param nonce - a change runs it again
 * @returns the run's state
 */
export const useQuery = (
  sql: string,
  params: Record<string, Param>,
  skip = false,
  nonce = 0
): Run => {
  const [run, setRun] = useState<Run>({ result: null, error: null, running: false });
  const key = JSON.stringify(params);
  useEffect(() => {
    if (skip) return;
    const abort = new AbortController();
    setRun((r) => ({ ...r, running: true, error: null }));
    runSql(sql, JSON.parse(key) as Record<string, Param>, { signal: abort.signal }).then(
      (result) => setRun({ result, error: null, running: false }),
      (e: unknown) => {
        if (!abort.signal.aborted) {
          setRun({
            result: null,
            error: e instanceof Error ? e.message : String(e),
            running: false,
          });
        }
      }
    );
    return () => abort.abort();
  }, [sql, key, skip, nonce]);
  return run;
};

/**
 * A graph's panels and the address's selection on it.
 *
 * @param spec - the graph's spec (none: no graph)
 * @param name - the graph's name, for the errors
 * @returns its panels, the selection read and checked, and its setter
 */
export const useGraphSel = (
  spec: Record<string, unknown> | null | undefined,
  name: string
): { panels: Panel[]; read: Read; set: (sel: Sel | null) => void } => {
  const [text, setText] = useSel();
  const ps = useMemo(() => {
    try {
      return spec ? panels(expandVocab(spec)) : [];
    } catch {
      return [];
    }
  }, [spec]);
  const read = useMemo(() => readSel(ps, text, name), [ps, text, name]);
  return { panels: ps, read, set: (sel) => setText(sel ? selJson(sel) : null) };
};

export interface SelectionChipProps {
  panels: Panel[];
  read: Read;
  /** The selection's arm, else the address's (the run timeline's arm). */
  arm: string;
  onClear: () => void;
}

/**
 * The address's selection in words (as hack/simlab/views.py's selection_text words it), with a
 * button that clears it and, where its values name a cell and a seed, one that opens that run's
 * timeline: its arm the selection's, else the address's.
 */
export const SelectionChip: React.FC<SelectionChipProps> = ({ panels: ps, read, arm, onClear }) => {
  const navigate = useNavigate();
  const location = useLocation();
  if (!read.sel && !read.error) return null;
  const at = read.sel?.at ?? {};
  const run =
    at.cell !== undefined &&
    at.seed !== undefined &&
    !Array.isArray(at.cell) &&
    !Array.isArray(at.seed)
      ? {
          cell: String(at.cell),
          seed: String(at.seed),
          arm: typeof at.arm === "string" ? at.arm : arm,
        }
      : null;
  const openRun = () => {
    if (!run) return;
    const q = carried(location.search);
    q.set("cell", run.cell);
    q.set("seed", run.seed);
    if (run.arm) q.set("arm", run.arm);
    navigate(`/console/run_timeline?${ordered(q).toString()}`);
  };
  return (
    <div className={`console__chip${read.error ? " console__chip--bad" : ""}`} role="status">
      <span className="console__chip-label">{read.error ? "selection not drawn" : "selected"}</span>
      <span className="console__chip-text">{read.sel ? selText(ps, read.sel) : read.error}</span>
      {run && (
        <button
          onClick={openRun}
          title={`${run.cell} seed ${run.seed}${run.arm ? ` ${run.arm}` : ""}`}
        >
          Open run timeline
        </button>
      )}
      <button className="console__chip-clear" onClick={onClear} title="Clear the selection (Esc)">
        ×
      </button>
    </div>
  );
};

/**
 * The workbench's address of a statement: the selection's keys `carry` holds, then its view or
 * statement, its `$name`s' values, the auto-chart's choices and a selection.
 *
 * @param carry - the address the workbench opens from
 * @param statement - a saved view's name, or a statement
 * @param values - its `$name`s' values
 * @param extra - the auto-chart's choices, and the selection's text
 * @returns the workbench's route and query
 */
export const workbenchHref = (
  carry: string,
  statement: { view: string; sql: string } | { sql: string },
  values: Record<string, unknown>,
  extra: Record<string, string> = {}
): string => {
  const q = carried(carry);
  if ("view" in statement) q.set("view", statement.view);
  else q.set("sql", statement.sql);
  for (const n of paramNames(statement.sql)) {
    const v = textOf(values[n] as never);
    if (v !== null && !WORKBENCH_KEYS.includes(n)) q.set(n, v);
  }
  for (const [k, v] of Object.entries(extra)) if (v) q.set(k, v);
  return `/console/workbench?${ordered(q).toString()}`;
};

export interface ViewPanelProps {
  view: SavedView;
  sel: Selection;
  /** Shown above the chart. */
  children?: React.ReactNode;
  /** The rows' table, in place of the plain one. */
  renderTable?: (r: QueryResult) => React.ReactNode;
  /** A change runs the query again. */
  nonce?: number;
}

/**
 * A saved view over the selection: its chart, with the address's selection on it (a click, a
 * brush or shift-click sets it), its rows, and its SQL ("open in workbench").
 */
const ViewPanel: React.FC<ViewPanelProps> = ({ view, sel, children, renderTable, nonce }) => {
  const navigate = useNavigate();
  const location = useLocation();
  const [params, missing] = viewParams(view, sel);
  const run = useQuery(view.sql, params, missing.length > 0, nonce);
  const [show, setShow] = useState<"chart" | "table" | "sql">(view.spec ? "chart" : "table");
  const rows = useMemo(() => (run.result ? rowObjects(run.result) : []), [run.result]);
  const graph = useGraphSel(view.spec, view.name);

  const openInWorkbench = () =>
    navigate(
      workbenchHref(location.search, { view: view.name, sql: view.sql }, params, {
        sel: graph.read.sel ? selJson(graph.read.sel) : "",
      })
    );

  return (
    <section className="console__view">
      <header className="console__view-head">
        <div>
          <h2>{view.title}</h2>
          <p className="console__desc">{view.description}</p>
          {view.todo && <p className="console__todo">TODO: {view.todo}</p>}
        </div>
        <div className="console__tabs">
          {view.spec && (
            <button className={show === "chart" ? "active" : ""} onClick={() => setShow("chart")}>
              Chart
            </button>
          )}
          <button className={show === "table" ? "active" : ""} onClick={() => setShow("table")}>
            Rows
          </button>
          <button className={show === "sql" ? "active" : ""} onClick={() => setShow("sql")}>
            SQL
          </button>
          <button onClick={openInWorkbench}>Open in workbench</button>
        </div>
      </header>
      {children}
      {missing.length > 0 && <div className="console__hint">Pick {missing.join(", ")}.</div>}
      {run.error && <div className="console__error">{run.error}</div>}
      {run.running && <div className="console__hint">Running…</div>}
      {view.spec && (
        <SelectionChip
          panels={graph.panels}
          read={graph.read}
          arm={sel.arm}
          onClear={() => graph.set(null)}
        />
      )}
      {show === "sql" && (
        <SqlEditor value={view.sql} onChange={() => undefined} readOnly rows={24} />
      )}
      {run.result &&
        show === "table" &&
        (renderTable ? renderTable(run.result) : <ResultTable result={run.result} />)}
      {run.result && show === "chart" && view.spec && (
        <>
          {rows.length === 0 ? (
            <div className="console__hint">No rows for this selection.</div>
          ) : (
            <VegaChart spec={view.spec} rows={rows} sel={graph.read.sel} onSel={graph.set} />
          )}
          <div className="console__meta">
            {run.result.rows.length} rows{run.result.truncated ? " (truncated)" : ""} ·{" "}
            {run.result.elapsed_ms} ms
          </div>
        </>
      )}
    </section>
  );
};

export default ViewPanel;

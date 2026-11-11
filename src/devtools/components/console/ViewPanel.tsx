// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";

import { rowObjects, runSql } from "./api";
import type { Json, Param, QueryResult, SavedView } from "./api";
import ResultTable from "./ResultTable";
import { viewParams } from "./selection";
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

export interface ViewPanelProps {
  view: SavedView;
  sel: Selection;
  /** A click on a mark, with its datum. */
  onSelect?: (datum: Record<string, Json>) => void;
  /** Shown above the chart. */
  children?: React.ReactNode;
  /** The rows' table, in place of the plain one. */
  renderTable?: (r: QueryResult) => React.ReactNode;
  /** A change runs the query again. */
  nonce?: number;
}

/** A saved view over the selection: its chart, its rows, and its SQL ("open in workbench"). */
const ViewPanel: React.FC<ViewPanelProps> = ({
  view,
  sel,
  onSelect,
  children,
  renderTable,
  nonce,
}) => {
  const navigate = useNavigate();
  const [params, missing] = viewParams(view, sel);
  const run = useQuery(view.sql, params, missing.length > 0, nonce);
  const [show, setShow] = useState<"chart" | "table" | "sql">(view.spec ? "chart" : "table");
  const rows = useMemo(() => (run.result ? rowObjects(run.result) : []), [run.result]);

  const openInWorkbench = () =>
    navigate(
      `/console/workbench?${new URLSearchParams(window.location.hash.split("?")[1] ?? "")}`,
      {
        state: { sql: view.sql, params, spec: view.spec, title: view.title },
      }
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
            <VegaChart spec={view.spec} rows={rows} onSelect={onSelect} />
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

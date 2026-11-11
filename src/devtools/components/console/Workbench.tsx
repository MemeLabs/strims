// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useEffect, useMemo, useRef, useState } from "react";
import { useSearchParams } from "react-router-dom";

import { rowObjects, runSql, saveView } from "./api";
import type { Param, QueryResult, Schema, ViewParam } from "./api";
import ResultTable from "./ResultTable";
import { WORKBENCH_KEYS, paramNames } from "./sel";
import type { Chart } from "./sel";
import { CHART, SEL, isKey, ordered } from "./selection";
import type { Selection } from "./selection";
import SqlEditor from "./SqlEditor";
import VegaChart from "./VegaChart";
import { SelectionChip, useGraphSel } from "./ViewPanel";

const START = `-- An experiment's collapses by cell and arm. Ctrl+Enter runs.
SELECT e.cell, e.arm_label, count(*) AS runs, sum(v.collapsed::INT) AS collapsed
FROM experiment_jobs e JOIN verdicts v USING (k, src)
WHERE e.exp_id = $exp
GROUP BY ALL ORDER BY collapsed DESC, cell, arm_label
`;

/** The picker a parameter's name suggests. */
const kindOf = (name: string): string =>
  ["exp", "cell", "arm", "arm2", "seed"].includes(name)
    ? name
    : ["delay", "min_stall_ms", "t"].includes(name)
    ? "number"
    : "text";

const MARKS = ["line", "point", "bar", "area", "rect", "tick"];

interface AutoChart {
  mark: string;
  x: string;
  y: string;
  color: string;
  logY: boolean;
}

const NUMERIC = /^(TINYINT|SMALLINT|INTEGER|BIGINT|HUGEINT|U\w*INT|FLOAT|DOUBLE|DECIMAL)/;
const TEMPORAL = /^(DATE|TIMESTAMP)/;

const fieldType = (r: QueryResult, name: string): string => {
  const t = r.columns.find((c) => c.name === name)?.type ?? "";
  return NUMERIC.test(t) ? "quantitative" : TEMPORAL.test(t) ? "temporal" : "nominal";
};

/** A Vega-Lite spec of the picked columns. */
const autoSpec = (r: QueryResult, a: AutoChart): Record<string, unknown> => ({
  width: "container",
  height: 320,
  mark: { type: a.mark, tooltip: true, ...(a.mark === "line" ? { point: false } : {}) },
  encoding: {
    x: { field: a.x, type: fieldType(r, a.x) },
    y: {
      field: a.y,
      type: fieldType(r, a.y),
      ...(a.logY ? { scale: { type: "symlog" } } : {}),
    },
    ...(a.color ? { color: { field: a.color, type: fieldType(r, a.color) } } : {}),
  },
});

/** The mark the auto-chart draws its x with, where the address names none. */
const markFor = (r: QueryResult, x: string): string =>
  fieldType(r, x) === "quantitative" ? "line" : "bar";

const guess = (r: QueryResult): AutoChart => {
  const nums = r.columns.filter((c) => NUMERIC.test(c.type)).map((c) => c.name);
  const others = r.columns.filter((c) => !NUMERIC.test(c.type)).map((c) => c.name);
  const x = others[0] ?? nums[0] ?? "";
  const y = nums.find((n) => n !== x) ?? nums[0] ?? "";
  return { mark: markFor(r, x), x, y, color: others[1] ?? "", logY: false };
};

/** The auto-chart's choices the address holds: none without x and y. */
const chartIn = (q: URLSearchParams): Chart | null => {
  const x = q.get("x");
  const y = q.get("y");
  if (!x || !y) return null;
  return {
    mark: q.get("mark") ?? "",
    x,
    y,
    color: q.get("color") ?? "",
    logy: q.get("logy") === "1" ? "1" : "",
  };
};

/** `q` with the auto-chart's choices `c` (none: no auto-chart). */
const withChart = (q: URLSearchParams, c: AutoChart | null): URLSearchParams => {
  const next = new URLSearchParams(q);
  for (const k of CHART) next.delete(k);
  if (c) {
    if (c.mark) next.set("mark", c.mark);
    next.set("x", c.x);
    next.set("y", c.y);
    if (c.color) next.set("color", c.color);
    if (c.logY) next.set("logy", "1");
  }
  return ordered(next);
};

/** The address's query as the URL holds it now. */
const addressNow = (): URLSearchParams => {
  const hash = window.location.hash;
  return new URLSearchParams(hash.includes("?") ? hash.slice(hash.indexOf("?") + 1) : "");
};

/** A statement's values as the workbench binds them: text that looks like a number, the number. */
const bound = (names: string[], value: (n: string) => string): Record<string, Param> =>
  Object.fromEntries(
    names.map((n) => {
      const v = value(n);
      return [n, v !== "" && /^-?\d+(\.\d+)?$/.test(v) ? Number(v) : v];
    })
  );

export interface WorkbenchProps {
  schema: Schema | null;
  sel: Selection;
  onSaved: () => void;
}

const SchemaBrowser: React.FC<{ schema: Schema | null; onPick: (text: string) => void }> = ({
  schema,
  onPick,
}) => {
  const [filter, setFilter] = useState("");
  const [open, setOpen] = useState<string | null>(null);
  if (!schema) return <div className="console__hint">Loading the schema…</div>;
  const about = new Map(
    schema.table_catalog.map((t) => [String(t.name), String(t.description ?? "")])
  );
  const f = filter.toLowerCase();
  const tables = schema.tables.filter(
    (t) => !f || t.name.includes(f) || t.columns.some((c) => c.name.toLowerCase().includes(f))
  );
  return (
    <div className="console__schema">
      <input
        placeholder="Filter tables and columns"
        value={filter}
        onChange={(e) => setFilter(e.target.value)}
      />
      <ul>
        {tables.map((t) => {
          const id = `${t.schema}.${t.name}`;
          const name = t.schema === "main" ? t.name : id;
          return (
            <li key={id}>
              <div className="console__schema-table" title={about.get(t.name)}>
                <button onClick={() => setOpen(open === id ? null : id)}>
                  {open === id ? "▾" : "▸"}
                </button>
                <a onClick={() => onPick(name)}>{name}</a>
                <span className="console__type">{t.columns.length}</span>
              </div>
              {(open === id || (f && t.columns.some((c) => c.name.toLowerCase().includes(f)))) && (
                <ul>
                  {about.get(t.name) && <li className="console__desc">{about.get(t.name)}</li>}
                  {t.columns
                    .filter((c) => open === id || c.name.toLowerCase().includes(f))
                    .map((c) => (
                      <li key={c.name}>
                        <a onClick={() => onPick(c.name)}>{c.name}</a>{" "}
                        <span className="console__type">{c.type}</span>
                      </li>
                    ))}
                </ul>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
};

/**
 * The SQL workbench: a schema browser, an editor, the rows, an auto-chart and save-as-view. Its
 * state is its address (hack/simlab/views.py's docs): a saved view's statement and spec (`view`) or
 * a statement of its own (`sql`), its `$name`s' values, and the auto-chart's choices (`mark`, `x`,
 * `y`, `color`, `logy`), whose chart it draws where the address names x and y, else the view's. An
 * address with a statement runs it; Run writes the editor's statement, its values and the chart
 * into the address, which the tab reports (api.ts reportTab); a selection on the chart is the
 * address's, as a saved view's is.
 */
const Workbench: React.FC<WorkbenchProps> = ({ schema, sel, onSaved }) => {
  const [query, setQuery] = useSearchParams();
  const setter = useRef(setQuery);
  setter.current = setQuery;
  // Writes the address: `f` of it as it is now, which a write since this render has moved past
  // `query` (a run's chart follows its statement).
  const write = (f: (q: URLSearchParams) => URLSearchParams) =>
    setter.current(f(addressNow()), { replace: true });
  const viewName = query.get("view") ?? "";
  const named = schema?.views.find((v) => v.name === viewName) ?? null;
  const given = query.get("sql") || named?.sql || "";
  const [sql, setSql] = useState(given || START);
  const names = useMemo(() => paramNames(sql), [sql]);
  const [values, setValues] = useState<Record<string, string>>({});
  const [result, setResult] = useState<QueryResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  const [saveName, setSaveName] = useState("");
  const [saveTitle, setSaveTitle] = useState("");
  const [saveDesc, setSaveDesc] = useState("");
  const [saved, setSaved] = useState<string | null>(null);
  // The address's statement the editor last took, and the statement and values last run.
  const taken = useRef(given);
  const ran = useRef<string | null>(null);

  const abort = useRef<AbortController | null>(null);

  // The address's value of a `$name`: the selection's for its keys, else its own key's; a name the
  // workbench's own keys take has none but what's typed.
  const fromAddress = (n: string): string =>
    WORKBENCH_KEYS.includes(n) ? values[n] ?? "" : isKey(n) ? sel[n] : query.get(n) ?? "";
  const value = (n: string): string => values[n] ?? fromAddress(n);

  useEffect(() => {
    if (!given || given === taken.current) return;
    taken.current = given;
    setSql(given);
    setValues({});
  }, [given]);

  useEffect(() => {
    if (named && !saveTitle) setSaveTitle(named.title);
  }, [named]);

  const execute = (statement: string, params: Record<string, Param>) => {
    ran.current = JSON.stringify([statement, params]);
    abort.current?.abort();
    const control = new AbortController();
    abort.current = control;
    setRunning(true);
    setError(null);
    runSql(statement, params, { maxRows: 100_000, timeoutS: 120, signal: control.signal }).then(
      (r) => {
        setResult(r);
        setRunning(false);
        // A chart for the rows: the address's, while its columns are the rows', else the view's
        // spec, else a guess, which goes into the address.
        const q = addressNow();
        const c = chartIn(q);
        const columns = r.columns.map((col) => col.name);
        const fits = c && columns.includes(c.x) && columns.includes(c.y);
        if (fits || (!c && q.get("view") && named?.spec)) return;
        write((old) => withChart(old, guess(r)));
      },
      (e: unknown) => {
        if (control.signal.aborted) return;
        setError(e instanceof Error ? e.message : String(e));
        setRunning(false);
      }
    );
  };
  useEffect(() => () => abort.current?.abort(), []);

  // An address with a statement runs it, once for each statement and values it gives.
  const addressed = given ? JSON.stringify([given, bound(paramNames(given), fromAddress)]) : null;
  useEffect(() => {
    if (addressed === null || addressed === ran.current) return;
    const [statement, params] = JSON.parse(addressed) as [string, Record<string, Param>];
    execute(statement, params);
  }, [addressed]);

  const run = () => {
    const ins = names.filter((n) => !WORKBENCH_KEYS.includes(n));
    const params = bound(names, value);
    write((old) => {
      const next = new URLSearchParams(old);
      next.delete("view");
      next.delete("sql");
      if (named && sql === named.sql) next.set("view", named.name);
      else next.set("sql", sql);
      for (const n of paramNames(given)) if (!isKey(n) && !ins.includes(n)) next.delete(n);
      for (const n of ins) {
        const v = value(n);
        if (v === "") next.delete(n);
        else next.set(n, v);
      }
      return ordered(next);
    });
    taken.current = sql;
    // The values typed are the address's now.
    setValues((v) => Object.fromEntries(Object.entries(v).filter(([n]) => !ins.includes(n))));
    execute(sql, params);
  };

  const rows = useMemo(() => (result ? rowObjects(result) : []), [result]);
  const c = chartIn(query);
  const chart: AutoChart | null =
    c && result
      ? {
          mark: c.mark || markFor(result, c.x),
          x: c.x,
          y: c.y,
          color: c.color ?? "",
          logY: c.logy === "1",
        }
      : null;
  const chartKey = JSON.stringify(chart);
  const spec = useMemo(
    () => (chart && result ? autoSpec(result, chart) : !c && named ? named.spec : null),
    [result, chartKey, Boolean(c), named]
  );
  // A chart chosen anew takes no selection of the last one's.
  const setChart = (a: AutoChart | null) =>
    write((old) => {
      const next = withChart(old, a);
      next.delete(SEL);
      return next;
    });
  const graph = useGraphSel(spec, named && !chart ? named.name : "the workbench's chart");

  const save = () => {
    const viewParams: ViewParam[] = names.map((n) => {
      const kind = kindOf(n);
      const v = value(n);
      return { name: n, kind, ...(kind === "number" && v !== "" ? { default: Number(v) } : {}) };
    });
    saveView(saveName, {
      title: saveTitle || saveName,
      description: saveDesc,
      params: viewParams,
      sql,
      spec,
    }).then(
      (v) => {
        setSaved(`saved hack/console/views/${v.name}.sql`);
        onSaved();
      },
      (e: unknown) => setSaved(e instanceof Error ? e.message : String(e))
    );
  };

  return (
    <div className="console__workbench">
      <SchemaBrowser schema={schema} onPick={(t) => setSql((s) => s + t)} />
      <div className="console__work">
        <SqlEditor value={sql} onChange={setSql} onRun={run} rows={16} />
        <div className="console__bar">
          <button className="primary" onClick={run} disabled={running}>
            {running ? "Running…" : "Run"}
          </button>
          {names.map((n) => (
            <label key={n}>
              ${n}{" "}
              <input
                value={value(n)}
                onChange={(e) => setValues((v) => ({ ...v, [n]: e.target.value }))}
                size={Math.max(6, Math.min(40, value(n).length + 2))}
              />
            </label>
          ))}
        </div>
        {error && <div className="console__error">{error}</div>}
        {result && (chart || named?.spec) && (
          <div className="console__bar">
            {named?.spec && (
              <label>
                <input
                  type="checkbox"
                  checked={!chart}
                  onChange={(e) => setChart(e.target.checked ? null : guess(result))}
                />{" "}
                the view's spec
              </label>
            )}
            {chart &&
              (["mark", "x", "y", "color"] as const).map((k) => (
                <label key={k}>
                  {k}{" "}
                  <select
                    value={chart[k]}
                    onChange={(e) => setChart({ ...chart, [k]: e.target.value })}
                  >
                    {k === "color" && <option value="">none</option>}
                    {(k === "mark" ? MARKS : result.columns.map((col) => col.name)).map((o) => (
                      <option key={o}>{o}</option>
                    ))}
                  </select>
                </label>
              ))}
            {chart && (
              <label>
                <input
                  type="checkbox"
                  checked={chart.logY}
                  onChange={(e) => setChart({ ...chart, logY: e.target.checked })}
                />{" "}
                symlog y
              </label>
            )}
          </div>
        )}
        {result && spec && (
          <SelectionChip
            panels={graph.panels}
            read={graph.read}
            arm={sel.arm}
            onClear={() => graph.set(null)}
          />
        )}
        {result && spec && rows.length > 0 && (
          <VegaChart spec={spec} rows={rows} sel={graph.read.sel} onSel={graph.set} />
        )}
        {result && <ResultTable result={result} />}
        <details className="console__save">
          <summary>Save as view</summary>
          <div className="console__bar">
            <input
              placeholder="name (a-z, 0-9, _ -)"
              value={saveName}
              onChange={(e) => setSaveName(e.target.value)}
            />
            <input
              placeholder="title"
              value={saveTitle}
              onChange={(e) => setSaveTitle(e.target.value)}
            />
            <input
              className="wide"
              placeholder="description"
              value={saveDesc}
              onChange={(e) => setSaveDesc(e.target.value)}
            />
            <button onClick={save} disabled={!saveName}>
              Save
            </button>
            {saved && <span className="console__meta">{saved}</span>}
          </div>
        </details>
      </div>
    </div>
  );
};

export default Workbench;

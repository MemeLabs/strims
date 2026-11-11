// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useEffect, useMemo, useState } from "react";
import { useLocation } from "react-router-dom";

import { rowObjects, runSql, saveView } from "./api";
import type { Param, QueryResult, Schema, ViewParam } from "./api";
import ResultTable from "./ResultTable";
import type { Selection } from "./selection";
import SqlEditor from "./SqlEditor";
import VegaChart from "./VegaChart";

const START = `-- An experiment's collapses by cell and arm. Ctrl+Enter runs.
SELECT e.cell, e.arm_label, count(*) AS runs, sum(v.collapsed::INT) AS collapsed
FROM experiment_jobs e JOIN verdicts v USING (k, src)
WHERE e.exp_id = $exp
GROUP BY ALL ORDER BY collapsed DESC, ALL
`;

/** The `$name`s a statement takes, in order, outside its strings and comments. */
const paramNames = (sql: string): string[] => {
  const bare = sql.replace(/--[^\n]*|\/\*[\s\S]*?\*\/|'(?:[^']|'')*'/g, " ");
  return [...new Set([...bare.matchAll(/\$([A-Za-z_]\w*)/g)].map((m) => m[1]))];
};

/** The picker a parameter's name suggests. */
const kindOf = (name: string): string =>
  ["exp", "cell", "arm", "arm2", "seed"].includes(name)
    ? name
    : ["delay", "min_stall_ms", "t"].includes(name)
    ? "number"
    : "text";

interface WorkbenchState {
  sql?: string;
  params?: Record<string, Param>;
  spec?: Record<string, unknown> | null;
  title?: string;
}

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

const guess = (r: QueryResult): AutoChart => {
  const nums = r.columns.filter((c) => NUMERIC.test(c.type)).map((c) => c.name);
  const others = r.columns.filter((c) => !NUMERIC.test(c.type)).map((c) => c.name);
  const x = others[0] ?? nums[0] ?? "";
  const y = nums.find((n) => n !== x) ?? nums[0] ?? "";
  return {
    mark: fieldType(r, x) === "quantitative" ? "line" : "bar",
    x,
    y,
    color: others[1] ?? "",
    logY: false,
  };
};

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

/** The SQL workbench: a schema browser, an editor, the rows, an auto-chart and save-as-view. */
const Workbench: React.FC<WorkbenchProps> = ({ schema, sel, onSaved }) => {
  const location = useLocation();
  const given = (location.state ?? {}) as WorkbenchState;
  const [sql, setSql] = useState(given.sql ?? START);
  const names = useMemo(() => paramNames(sql), [sql]);
  const [values, setValues] = useState<Record<string, string>>({});
  const [result, setResult] = useState<QueryResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  const [chart, setChart] = useState<AutoChart | null>(null);
  const [useGivenSpec, setUseGivenSpec] = useState(Boolean(given.spec));
  const [saveName, setSaveName] = useState("");
  const [saveTitle, setSaveTitle] = useState(given.title ?? "");
  const [saveDesc, setSaveDesc] = useState("");
  const [saved, setSaved] = useState<string | null>(null);

  useEffect(() => {
    if (given.sql) setSql(given.sql);
    if (given.params) {
      setValues(Object.fromEntries(Object.entries(given.params).map(([k, v]) => [k, String(v)])));
    }
    setUseGivenSpec(Boolean(given.spec));
  }, [location.key]);

  const value = (n: string): string =>
    values[n] ?? (sel as unknown as Record<string, string>)[n] ?? "";

  const params = (): Record<string, Param> =>
    Object.fromEntries(
      names.map((n) => {
        const v = value(n);
        return [n, v !== "" && /^-?\d+(\.\d+)?$/.test(v) ? Number(v) : v];
      })
    );

  const run = () => {
    setRunning(true);
    setError(null);
    runSql(sql, params(), { maxRows: 100_000, timeoutS: 120 }).then(
      (r) => {
        setResult(r);
        setRunning(false);
        setChart((c) => (c && r.columns.some((col) => col.name === c.x) ? c : guess(r)));
      },
      (e: unknown) => {
        setError(e instanceof Error ? e.message : String(e));
        setRunning(false);
      }
    );
  };

  const rows = useMemo(() => (result ? rowObjects(result) : []), [result]);
  const spec =
    useGivenSpec && given.spec ? given.spec : result && chart ? autoSpec(result, chart) : null;

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
        {result && chart && (
          <div className="console__bar">
            {given.spec && (
              <label>
                <input
                  type="checkbox"
                  checked={useGivenSpec}
                  onChange={(e) => setUseGivenSpec(e.target.checked)}
                />{" "}
                the view's spec
              </label>
            )}
            {!useGivenSpec &&
              (["mark", "x", "y", "color"] as const).map((k) => (
                <label key={k}>
                  {k}{" "}
                  <select
                    value={chart[k]}
                    onChange={(e) => setChart({ ...chart, [k]: e.target.value })}
                  >
                    {k === "color" && <option value="">none</option>}
                    {(k === "mark" ? MARKS : result.columns.map((c) => c.name)).map((o) => (
                      <option key={o}>{o}</option>
                    ))}
                  </select>
                </label>
              ))}
            {!useGivenSpec && (
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
        {result && spec && rows.length > 0 && <VegaChart spec={spec} rows={rows} />}
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

// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "../styles/console.scss";

import React, { useCallback, useEffect, useState } from "react";
import { NavLink, Navigate, Route, Routes, useNavigate, useParams } from "react-router-dom";

import { getIndex, getSchema, startIndex } from "../components/console/api";
import type { Json, SavedView, Schema } from "../components/console/api";
import Ask from "../components/console/Ask";
import Experiments from "../components/console/Experiments";
import { useOptions, useSelection } from "../components/console/selection";
import type { Selection } from "../components/console/selection";
import ViewPanel from "../components/console/ViewPanel";
import Workbench from "../components/console/Workbench";
import Nav from "../components/Nav";

/** The views in the console's menu, in order; any other saved view follows them. */
const ORDER = [
  "experiments",
  "collapse_grid",
  "collapse_diff",
  "run_timeline",
  "run_overlay",
  "playback",
  "seed_bands",
  "p99_sources",
  "overhead",
  "tree",
  "scorecard",
];

const PICKERS: { key: keyof Selection; label: string }[] = [
  { key: "exp", label: "experiment" },
  { key: "cell", label: "cell" },
  { key: "arm", label: "arm" },
  { key: "arm2", label: "against" },
  { key: "seed", label: "seed" },
  { key: "delay", label: "playout delay" },
  { key: "min_stall_ms", label: "stalls from (ms)" },
  { key: "align", label: "align on" },
  { key: "t", label: "at (s)" },
];

const ALIGNS = [
  "start",
  "onset",
  "seed_down",
  "seed_up",
  "collapse",
  "depart",
  "up_step",
  "arrive",
];

interface PickersProps {
  schema: Schema | null;
  sel: Selection;
  setSel: (s: Partial<Selection>) => void;
  /** The selection keys the view takes; all for none. */
  uses: Set<string> | null;
}

const Pickers: React.FC<PickersProps> = ({ schema, sel, setSel, uses }) => {
  const options = useOptions(sel);
  const shown = PICKERS.filter((p) => !uses || uses.has(p.key));
  if (shown.length === 0) return null;
  const select = (key: keyof Selection, values: { value: string; label: string }[]) => (
    <select value={sel[key]} onChange={(e) => setSel({ [key]: e.target.value })}>
      <option value="">—</option>
      {values.map((v) => (
        <option key={v.value} value={v.value}>
          {v.label}
        </option>
      ))}
    </select>
  );
  const plain = (xs: string[]) => xs.map((x) => ({ value: x, label: x }));
  const control = (key: keyof Selection): React.ReactNode => {
    switch (key) {
      case "exp":
        return (
          <>
            <input
              list="console-experiments"
              value={sel.exp}
              size={Math.max(8, Math.min(50, sel.exp.length + 2))}
              onChange={(e) =>
                setSel({ exp: e.target.value, cell: "", arm: "", arm2: "", seed: "" })
              }
            />
            <datalist id="console-experiments">
              {(schema?.experiments ?? []).map((x) => (
                <option key={String(x.exp_id)} value={String(x.exp_id)}>
                  {[x.started_at, (x.tiers as Json[] | null)?.join(","), x.question]
                    .filter(Boolean)
                    .join(" · ")}
                </option>
              ))}
            </datalist>
          </>
        );
      case "cell":
        return select("cell", plain(options.cells));
      case "arm":
      case "arm2":
        return select(key, plain(options.arms));
      case "seed":
        return select(
          "seed",
          options.seeds.map((s) => ({
            value: String(s.seed),
            label: `${s.seed}${s.collapsed ? " (collapsed)" : ""}`,
          }))
        );
      case "delay":
        return select("delay", plain(["1250", "4250"]));
      case "align":
        return select("align", plain(ALIGNS));
      default:
        return (
          <input value={sel[key]} size={6} onChange={(e) => setSel({ [key]: e.target.value })} />
        );
    }
  };
  return (
    <div className="console__pickers">
      {shown.map((p) => (
        <label key={p.key}>
          <span>{p.label}</span>
          {control(p.key)}
        </label>
      ))}
    </div>
  );
};

const IndexStatus: React.FC<{ onIndexed: () => void }> = ({ onIndexed }) => {
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const read = useCallback(() => {
    getIndex().then(
      (s) => {
        const at = s.catalog_info.find((r) => r.key === "indexed_at")?.value;
        setText(s.running ? "indexing…" : `indexed ${String(at ?? "?")}`);
      },
      () =>
        setText(
          "console server unreachable (npm run console:build once, then python3 hack/console/server.py)"
        )
    );
  }, []);
  useEffect(() => {
    read();
    const t = setInterval(read, 30_000);
    return () => clearInterval(t);
  }, [read]);
  const go = () => {
    setBusy(true);
    startIndex().then(
      () => {
        setBusy(false);
        read();
        onIndexed();
      },
      () => setBusy(false)
    );
  };
  return (
    <div className="console__index">
      <span>{text}</span>
      <button onClick={go} disabled={busy}>
        {busy ? "Indexing…" : "Index now"}
      </button>
    </div>
  );
};

const viewUses = (v: SavedView): Set<string> => new Set(v.params.flatMap((p) => [p.name, p.kind]));

interface NamedViewProps {
  views: SavedView[];
  sel: Selection;
  setSel: (s: Partial<Selection>) => void;
}

const NamedView: React.FC<NamedViewProps> = ({ views, sel, setSel }) => {
  const { name } = useParams();
  const navigate = useNavigate();
  const view = views.find((v) => v.name === name);
  if (!view) return <div className="console__hint">No saved view {name}.</div>;
  if (view.name === "experiments") return <Experiments view={view} sel={sel} setSel={setSel} />;
  // A run picked in a grid opens its timeline.
  const onSelect = (d: Record<string, Json>) => {
    if (d.cell === undefined || d.seed === undefined) return;
    const arm = typeof d.arm === "string" ? d.arm : sel.arm;
    const next = new URLSearchParams(window.location.hash.split("?")[1] ?? "");
    next.set("cell", String(d.cell));
    next.set("seed", String(d.seed));
    if (arm) next.set("arm", arm);
    navigate(`/console/run_timeline?${next.toString()}`);
  };
  return <ViewPanel key={view.name} view={view} sel={sel} onSelect={onSelect} />;
};

/**
 * The sim-results console: saved views over the results catalog, a SQL workbench, and questions
 * in plain language that Claude answers in SQL.
 */
const Console: React.FC = () => {
  const [schema, setSchema] = useState<Schema | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [sel, setSel] = useSelection();
  const name = (useParams()["*"] ?? "").split("/")[0];
  const load = useCallback(() => {
    getSchema().then(setSchema, (e: unknown) =>
      setError(e instanceof Error ? e.message : String(e))
    );
  }, []);
  useEffect(load, [load]);

  const views = schema?.views ?? [];
  const ordered = [
    ...ORDER.map((n) => views.find((v) => v.name === n)).filter((v): v is SavedView => Boolean(v)),
    ...views.filter((v) => !ORDER.includes(v.name)),
  ];
  const current = views.find((v) => v.name === name);
  const search = window.location.hash.split("?")[1] ?? "";

  return (
    <div className="console">
      <Nav />
      <div className="console__frame">
        <nav className="console__menu">
          <h1>Sim results</h1>
          <NavLink to={`/console/ask?${search}`}>Ask</NavLink>
          <NavLink to={`/console/workbench?${search}`}>SQL workbench</NavLink>
          {ordered.map((v) => (
            <NavLink key={v.name} to={`/console/${v.name}?${search}`}>
              {v.title}
            </NavLink>
          ))}
          <IndexStatus onIndexed={load} />
        </nav>
        <main className="console__main">
          {error && <div className="console__error">{error}</div>}
          <Pickers
            schema={schema}
            sel={sel}
            setSel={setSel}
            uses={name === "workbench" ? null : current ? viewUses(current) : new Set()}
          />
          <Routes>
            <Route index element={<Navigate to="experiments" replace />} />
            <Route path="ask" element={<Ask />} />
            <Route
              path="workbench"
              element={<Workbench schema={schema} sel={sel} onSaved={load} />}
            />
            <Route path=":name" element={<NamedView views={views} sel={sel} setSel={setSel} />} />
          </Routes>
        </main>
      </div>
    </div>
  );
};

export default Console;

// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "../styles/console.scss";

import React, { useCallback, useEffect, useRef, useState } from "react";
import { NavLink, Navigate, Route, Routes, useLocation, useParams } from "react-router";

import Nav from "../components/Nav";
import Ask from "../components/console/Ask";
import Experiments from "../components/console/Experiments";
import ViewPanel from "../components/console/ViewPanel";
import Workbench from "../components/console/Workbench";
import {
  getIndex,
  getSchema,
  reportTab,
  startIndex,
  startTabReports,
} from "../components/console/api";
import type { IndexState, Json, SavedView, Schema, Snapshot } from "../components/console/api";
import { carried, useOptions, useSelection } from "../components/console/selection";
import type { Selection } from "../components/console/selection";
import { ALIGNS } from "../components/console/vocab";

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

/** A decimal number, as a number picker takes it. */
const NUMBER = /^-?(\d+\.?\d*|\.\d+)$/;

/** How long typing pauses before a number picker's text reaches the selection. */
const SETTLE_MS = 500;

interface NumberInputProps {
  /** The selection's value. */
  value: string;
  /** Sets the selection's value, with text that parses. */
  onCommit: (v: string) => void;
}

/**
 * A number picker. What's typed in it is its own, and reaches the selection once it parses, on
 * Enter, on blur or once typing pauses: an empty or partial number runs nothing, the view keeps
 * its last result, and a field left that way shows as invalid.
 */
const NumberInput: React.FC<NumberInputProps> = ({ value, onCommit }) => {
  const [text, setText] = useState(value);
  const [invalid, setInvalid] = useState(false);
  const sent = useRef(value);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  useEffect(() => {
    // The URL or another picker moved it; what this field sent is already its text.
    if (value === sent.current) return;
    sent.current = value;
    setText(value);
    setInvalid(false);
  }, [value]);
  useEffect(() => () => clearTimeout(timer.current), []);
  const commit = (t: string) => {
    clearTimeout(timer.current);
    const v = t.trim();
    const ok = NUMBER.test(v);
    setInvalid(!ok);
    if (ok && v !== sent.current) {
      sent.current = v;
      onCommit(v);
    }
  };
  return (
    <input
      value={text}
      size={6}
      inputMode="decimal"
      aria-invalid={invalid || undefined}
      title={invalid ? "a number" : undefined}
      onChange={(e) => {
        const t = e.target.value;
        setText(t);
        setInvalid(false);
        clearTimeout(timer.current);
        timer.current = setTimeout(() => commit(t), SETTLE_MS);
      }}
      onKeyDown={(e) => {
        if (e.key === "Enter") commit(text);
      }}
      onBlur={() => commit(text)}
    />
  );
};

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
  const plain = (xs: readonly string[]) => xs.map((x) => ({ value: x, label: x }));
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
      case "min_stall_ms":
      case "t":
        return <NumberInput value={sel[key]} onCommit={(v) => setSel({ [key]: v })} />;
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

/** How often the page reads the index's state: its "indexed" line and the catalog's snapshot. */
const INDEX_EVERY_MS = 30_000;

interface IndexStatusProps {
  /** GET /console/index's last answer; null before the first. */
  state: IndexState | null;
  /** Whether the last read of it failed. */
  unreachable: boolean;
  /** Runs once an index the button started has ended. */
  onIndexed: () => void;
}

const IndexStatus: React.FC<IndexStatusProps> = ({ state, unreachable, onIndexed }) => {
  const [busy, setBusy] = useState(false);
  const at = state?.catalog_info.find((r) => r.key === "indexed_at")?.value;
  const text = unreachable
    ? "console server unreachable (npm run console:build once, then python3 hack/console/server.py)"
    : state === null
      ? ""
      : state.running
        ? "indexing…"
        : `indexed ${String(at ?? "?")}`;
  const go = () => {
    setBusy(true);
    startIndex().then(
      () => {
        setBusy(false);
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
  /** The catalog's latest snapshot, as the page last read it. */
  latest: Snapshot | null;
}

const NamedView: React.FC<NamedViewProps> = ({ views, sel, setSel, latest }) => {
  const { name } = useParams();
  const view = views.find((v) => v.name === name);
  if (!view) return <div className="console__hint">No saved view {name}.</div>;
  if (view.name === "experiments") {
    return <Experiments view={view} sel={sel} setSel={setSel} latest={latest} />;
  }
  return <ViewPanel key={view.name} view={view} sel={sel} />;
};

/**
 * The sim-results console: saved views over the results catalog, a SQL workbench, and questions
 * in plain language that Claude answers in SQL.
 */
const Console: React.FC = () => {
  const [schema, setSchema] = useState<Schema | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [sel, setSel] = useSelection();
  const location = useLocation();
  const name = (useParams()["*"] ?? "").split("/")[0];
  // The tab tells the console server its address, for hack/console/ui.py.
  useEffect(startTabReports, []);
  useEffect(() => void reportTab(), [location.pathname, location.search]);
  const load = useCallback(() => {
    getSchema().then(setSchema, (e: unknown) =>
      setError(e instanceof Error ? e.message : String(e))
    );
  }, []);
  useEffect(load, [load]);
  // The index's state, read every 30 s: the menu's "indexed" line, and the snapshot the
  // experiments list compares its rows' with.
  const [index, setIndex] = useState<IndexState | null>(null);
  const [unreachable, setUnreachable] = useState(false);
  const readIndex = useCallback(() => {
    getIndex().then(
      (s) => {
        setIndex(s);
        setUnreachable(false);
      },
      () => setUnreachable(true)
    );
  }, []);
  useEffect(() => {
    readIndex();
    const t = setInterval(readIndex, INDEX_EVERY_MS);
    return () => clearInterval(t);
  }, [readIndex]);
  const indexed = useCallback(() => {
    readIndex();
    load();
  }, [readIndex, load]);

  const views = schema?.views ?? [];
  const ordered = [
    ...ORDER.map((n) => views.find((v) => v.name === n)).filter((v): v is SavedView => Boolean(v)),
    ...views.filter((v) => !ORDER.includes(v.name)),
  ];
  const current = views.find((v) => v.name === name);
  // Another page takes the selection's keys, not this one's selection on its graph.
  const search = carried(location.search).toString();

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
          <IndexStatus state={index} unreachable={unreachable} onIndexed={indexed} />
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
            <Route
              path=":name"
              element={
                <NamedView
                  views={views}
                  sel={sel}
                  setSel={setSel}
                  latest={index?.snapshot ?? null}
                />
              }
            />
          </Routes>
        </main>
      </div>
    </div>
  );
};

export default Console;

// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useEffect, useState } from "react";

import ResultTable from "./ResultTable";
import ViewPanel from "./ViewPanel";
import { approveRegeneration, fetchData, getRegeneration, newResults, regenerate } from "./api";
import type { QueryResult, Regeneration, SavedView, Snapshot } from "./api";
import type { Selection } from "./selection";

const split = (s: string): string[] =>
  s
    .split(",")
    .map((x) => x.trim())
    .filter(Boolean);

/** Seeds from "2001-2004", "2001,2002" or "2001". */
const seedRange = (s: string): number[] => {
  const m = /^\s*(\d+)\s*-\s*(\d+)\s*$/.exec(s);
  if (m) {
    const [a, b] = [Number(m[1]), Number(m[2])];
    return Array.from({ length: Math.max(0, b - a + 1) }, (_, i) => a + i);
  }
  return split(s).map(Number);
};

const FetchBox: React.FC<{ exp: string; sel: Selection; onDone: () => void }> = ({
  exp,
  sel,
  onDone,
}) => {
  const [reads, setReads] = useState(false);
  const [cells, setCells] = useState(sel.exp === exp ? sel.cell : "");
  const [arms, setArms] = useState(sel.exp === exp ? sel.arm : "");
  const [lines, setLines] = useState<string[]>([]);
  const [status, setStatus] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const go = () => {
    setBusy(true);
    setLines([]);
    setStatus(null);
    fetchData(
      { exp_id: exp, cells: split(cells), arms: split(arms), tables: reads ? ["reads"] : [] },
      (l) => l.line !== undefined && setLines((ls) => [...ls.slice(-200), l.line ?? ""])
    ).then(
      (last) => {
        setBusy(false);
        setStatus(
          `exit ${last.code ?? "?"}; S3 charged ${
            last.s3_usd === null || last.s3_usd === undefined ? "?" : `$${last.s3_usd.toFixed(4)}`
          }`
        );
        onDone();
      },
      (e: unknown) => {
        setBusy(false);
        setStatus(e instanceof Error ? e.message : String(e));
      }
    );
  };
  return (
    <div className="console__box">
      <h3>Fetch {exp} from S3</h3>
      <p className="console__desc">
        Copies what S3 still holds of the experiment into the cache and indexes it (simdb.py fetch).
        The rows come with every fetch; reads, the bulky table a timeline plays stalls from, only
        for the cells and arms named.
      </p>
      <div className="console__bar">
        <label>
          <input type="checkbox" checked={reads} onChange={(e) => setReads(e.target.checked)} />{" "}
          reads too
        </label>
        <label>
          cells <input value={cells} onChange={(e) => setCells(e.target.value)} placeholder="all" />
        </label>
        <label>
          arms <input value={arms} onChange={(e) => setArms(e.target.value)} placeholder="all" />
        </label>
        <button className="primary" onClick={go} disabled={busy}>
          {busy ? "Fetching…" : "Fetch"}
        </button>
        {status && <span className="console__meta">{status}</span>}
      </div>
      {lines.length > 0 && <pre className="console__log">{lines.join("\n")}</pre>}
    </div>
  );
};

const RegenerateBox: React.FC<{ exp: string; sel: Selection }> = ({ exp, sel }) => {
  const [cells, setCells] = useState(sel.exp === exp ? sel.cell : "");
  const [arms, setArms] = useState(sel.exp === exp ? sel.arm : "");
  const [seeds, setSeeds] = useState(sel.exp === exp ? sel.seed : "");
  const [quote, setQuote] = useState<Regeneration | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (quote?.state !== "running") return;
    const t = setInterval(() => {
      getRegeneration(quote.id).then(setQuote, () => undefined);
    }, 5000);
    return () => clearInterval(t);
  }, [quote?.id, quote?.state]);

  const ask = () => {
    setBusy(true);
    setError(null);
    setQuote(null);
    regenerate({
      exp_id: exp,
      cells: split(cells),
      arms: split(arms),
      seeds: seedRange(seeds),
    }).then(
      (r) => {
        setBusy(false);
        setQuote(r);
      },
      (e: unknown) => {
        setBusy(false);
        setError(e instanceof Error ? e.message : String(e));
      }
    );
  };
  const approve = () => {
    if (!quote) return;
    approveRegeneration(quote.id).then(setQuote, (e: unknown) =>
      setError(e instanceof Error ? e.message : String(e))
    );
  };
  return (
    <div className="console__box">
      <h3>Regenerate {exp} on the farm</h3>
      <p className="console__desc">
        Writes the suite's farm matrices for these cells, arms and seeds (suite.py farm --no-run)
        and quotes them (farm.py estimate). Nothing runs until you approve the quote.
      </p>
      <div className="console__bar">
        <label>
          cells <input value={cells} onChange={(e) => setCells(e.target.value)} placeholder="a,b" />
        </label>
        <label>
          arms <input value={arms} onChange={(e) => setArms(e.target.value)} placeholder="a,b" />
        </label>
        <label>
          seeds{" "}
          <input value={seeds} onChange={(e) => setSeeds(e.target.value)} placeholder="2001-2004" />
        </label>
        <button onClick={ask} disabled={busy}>
          {busy ? "Quoting…" : "Quote"}
        </button>
      </div>
      {error && <div className="console__error">{error}</div>}
      {quote && (
        <div>
          <div className="console__meta">
            {quote.id}: {quote.state}
            {quote.error ? `: ${quote.error}` : ""}
          </div>
          {quote.warnings.map((w) => (
            <div key={w} className="console__todo">
              {w}
            </div>
          ))}
          {quote.quote && <pre className="console__log">{quote.quote}</pre>}
          {quote.state === "quoted" && (
            <button className="danger" onClick={approve}>
              Approve: run {quote.matrices.length} matrices on the farm
            </button>
          )}
          {quote.log_tail && <pre className="console__log">{quote.log_tail.join("\n")}</pre>}
        </div>
      )}
    </div>
  );
};

/**
 * A time's hours and minutes, in this browser's zone.
 *
 * @param at - an ISO 8601 time
 * @returns its HH:MM, or `at` as it is where it isn't a time
 */
const clock = (at: string): string => {
  const d = new Date(at);
  if (Number.isNaN(d.getTime())) return at;
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
};

export interface ExperimentsProps {
  view: SavedView;
  sel: Selection;
  setSel: (s: Partial<Selection>) => void;
  /** The catalog's latest snapshot, as the page last read it (GET /console/index). */
  latest: Snapshot | null;
}

/**
 * The experiments, with fetch and regenerate actions. When the catalog's latest snapshot is newer
 * than the one its rows came from (an index since, by any process), a bar above the table says
 * so; the table loads again only when its Reload is clicked, or for its own reasons (a search, a
 * fetch), whose rows clear the bar.
 */
const Experiments: React.FC<ExperimentsProps> = ({ view, sel, setSel, latest }) => {
  const [action, setAction] = useState<{ exp: string; kind: "fetch" | "regenerate" } | null>(null);
  const [search, setSearch] = useState(sel.search);
  const [nonce, setNonce] = useState(0);
  const reload = () => setNonce((n) => n + 1);
  useEffect(() => {
    const t = setTimeout(() => setSel({ search }), 300);
    return () => clearTimeout(t);
  }, [search]);
  const table = (r: QueryResult) => {
    const idCol = r.columns.findIndex((c) => c.name === "exp_id");
    const kindCol = r.columns.findIndex((c) => c.name === "kind");
    return (
      <>
        {newResults(latest, r.snapshot) && (
          <div className="console__fresh" role="status">
            <span>New results are in the catalog (indexed {clock(latest.at)}).</span>
            <button className="primary" onClick={reload}>
              Reload
            </button>
          </div>
        )}
        <ResultTable
          result={r}
          limit={300}
          actions={(row) => {
            const exp = String(row[idCol]);
            return (
              <>
                <button onClick={() => setSel({ exp, cell: "", arm: "", arm2: "", seed: "" })}>
                  {sel.exp === exp ? "Selected" : "Select"}
                </button>
                <button onClick={() => setAction({ exp, kind: "fetch" })}>Fetch</button>
                {row[kindCol] === "farm" && (
                  <button onClick={() => setAction({ exp, kind: "regenerate" })}>
                    Regenerate…
                  </button>
                )}
              </>
            );
          }}
        />
      </>
    );
  };
  return (
    <ViewPanel view={view} sel={sel} renderTable={table} nonce={nonce}>
      <div className="console__bar">
        <input
          className="wide"
          placeholder="Search ids, questions, tiers and arms"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
      </div>
      {action?.kind === "fetch" && <FetchBox exp={action.exp} sel={sel} onDone={reload} />}
      {action?.kind === "regenerate" && <RegenerateBox exp={action.exp} sel={sel} />}
    </ViewPanel>
  );
};

export default Experiments;

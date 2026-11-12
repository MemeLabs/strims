// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useLocation, useNavigate } from "react-router";

import Markdown from "./Markdown";
import ResultTable from "./ResultTable";
import VegaChart from "./VegaChart";
import { workbenchHref } from "./ViewPanel";
import { approveRegeneration, askStream, getRegeneration, getSpend, rowObjects } from "./api";
import type { AskEvent, AskResult, Json, Param, Regeneration, Spend } from "./api";
import { chartOf } from "./sel";

/** The task budget a question gets unless the box says otherwise, and the least the API takes. */
const BUDGET = 200_000;
const MIN_BUDGET = 20_000;

interface Sql {
  sql: string;
  params: Record<string, Param>;
  purpose: string;
}

interface ToolStep {
  kind: "tool";
  id: string;
  name: string;
  input?: Record<string, Json>;
  sqls: Sql[];
  result?: AskResult;
  chart?: { title: string; spec: Record<string, unknown>; sql: string; result: AskResult };
  progress: string[];
  proposal?: { why: string; regeneration: Regeneration };
  outcome?: { is_error: boolean; preview: string; chars: number; elapsed_ms: number };
}

type Step =
  | { kind: "thinking" | "text"; key: string; text: string }
  | ToolStep
  | { kind: "fallback"; from: string; to: string; category: string | null }
  | { kind: "refusal"; category: string | null; explanation: string | null }
  | { kind: "error"; message: string };

type Usage = Extract<AskEvent, { type: "usage" }>;
type Done = Extract<AskEvent, { type: "done" }>;

interface Question {
  text: string;
  budget: number;
  steps: Step[];
  usage?: Usage;
  done?: Done;
  startedAt: number;
}

const usd = (x: number | undefined): string =>
  `$${(x ?? 0).toFixed(x !== undefined && x < 1 ? 4 : 2)}`;

const thousands = (n: number): string => n.toLocaleString("en-US");

/** `q` with `e` applied: deltas join their block, a tool's events its step. */
const applied = (q: Question, e: AskEvent): Question => {
  const steps = [...q.steps];
  const tool = (id: string, name = ""): [ToolStep, number] => {
    const i = steps.findIndex((s) => s.kind === "tool" && s.id === id);
    if (i >= 0) return [{ ...(steps[i] as ToolStep) }, i];
    steps.push({ kind: "tool", id, name, sqls: [], progress: [] });
    return [steps[steps.length - 1] as ToolStep, steps.length - 1];
  };
  const put = (s: ToolStep, i: number) => {
    steps[i] = s;
  };
  switch (e.type) {
    case "thinking":
    case "text": {
      const key = `${e.type}:${e.turn}:${e.block}`;
      const last = steps[steps.length - 1];
      if (last && (last.kind === "thinking" || last.kind === "text") && last.key === key) {
        steps[steps.length - 1] = { ...last, text: last.text + e.text };
      } else {
        steps.push({ kind: e.type, key, text: e.text });
      }
      break;
    }
    case "tool_start":
      tool(e.id, e.name);
      break;
    case "tool": {
      const [s, i] = tool(e.id, e.name);
      put({ ...s, name: e.name, input: e.input }, i);
      break;
    }
    case "sql": {
      const [s, i] = tool(e.id);
      put({ ...s, sqls: [...s.sqls, { sql: e.sql, params: e.params, purpose: e.purpose }] }, i);
      break;
    }
    case "result": {
      const [s, i] = tool(e.id);
      put({ ...s, result: e.result }, i);
      break;
    }
    case "chart": {
      const [s, i] = tool(e.id);
      put({ ...s, chart: { title: e.title, spec: e.spec, sql: e.sql, result: e.result } }, i);
      break;
    }
    case "progress": {
      const [s, i] = tool(e.id);
      put({ ...s, progress: [...s.progress.slice(-100), e.line] }, i);
      break;
    }
    case "proposal": {
      const [s, i] = tool(e.id);
      put({ ...s, proposal: { why: e.why, regeneration: e.regeneration } }, i);
      break;
    }
    case "tool_result": {
      const [s, i] = tool(e.id, e.name);
      put({ ...s, outcome: e }, i);
      break;
    }
    case "fallback":
      steps.push({ kind: "fallback", from: e.from, to: e.to, category: e.category });
      break;
    case "refusal":
      steps.push({ kind: "refusal", category: e.category, explanation: e.explanation });
      break;
    case "error":
      steps.push({ kind: "error", message: e.message });
      break;
    case "usage":
      return { ...q, steps, usage: e };
    case "done":
      return { ...q, steps, done: e };
    default:
      return q;
  }
  return { ...q, steps };
};

/** A proposed farm run's quote, which runs only when the Approve button is clicked. */
const QuoteCard: React.FC<{ why: string; given: Regeneration }> = ({ why, given }) => {
  const [r, setR] = useState(given);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    if (r.state !== "running") return;
    const t = setInterval(() => getRegeneration(r.id).then(setR, () => undefined), 5000);
    return () => clearInterval(t);
  }, [r.id, r.state]);
  const approve = () => {
    setBusy(true);
    approveRegeneration(r.id).then(
      (next) => {
        setBusy(false);
        setR(next);
      },
      (e: unknown) => {
        setBusy(false);
        setError(e instanceof Error ? e.message : String(e));
      }
    );
  };
  const est = r.estimate ?? {};
  const cost = typeof est.cost === "number" ? est.cost : null;
  const wall = typeof est.wall_s === "number" ? Math.round(est.wall_s / 60) : null;
  return (
    <div className="console__box console__quote">
      <h3>
        Proposed farm run of {r.exp_id}: {r.state}
      </h3>
      {why && <p className="console__desc">{why}</p>}
      <div className="console__meta">
        {r.cells.length} cells · arms {r.arms.join(", ")} · seeds {r.seeds[0]}
        {r.seeds.length > 1 ? `–${r.seeds[r.seeds.length - 1]}` : ""}
        {typeof est.jobs === "number" ? ` · ${thousands(est.jobs)} jobs` : ""}
        {wall !== null ? ` · ~${wall} min` : ""}
        {cost !== null ? ` · ~$${cost.toFixed(2)}` : ""} · quote {r.id}
      </div>
      {r.error && <div className="console__error">{r.error}</div>}
      {r.warnings.map((w) => (
        <div key={w} className="console__todo">
          {w}
        </div>
      ))}
      {r.quote && (
        <details>
          <summary>farm.py estimate</summary>
          <pre className="console__log">{r.quote}</pre>
        </details>
      )}
      {r.state === "quoted" && (
        <button className="danger" onClick={approve} disabled={busy}>
          Approve: run {r.matrices.length} matrices on the farm
          {cost !== null ? ` (~$${cost.toFixed(2)})` : ""}
        </button>
      )}
      {error && <div className="console__error">{error}</div>}
      {r.log_tail && <pre className="console__log">{r.log_tail.join("\n")}</pre>}
    </div>
  );
};

const ToolView: React.FC<{ step: ToolStep }> = ({ step }) => {
  const navigate = useNavigate();
  const location = useLocation();
  // The workbench's address of a statement and its values, and of its chart where the auto-chart
  // draws it (one mark of an x and a y field).
  const open = (sql: Sql, spec?: Record<string, unknown>) =>
    navigate(workbenchHref(location.search, { sql: sql.sql }, sql.params, chartOf(spec) ?? {}));
  const said = step.input?.purpose ?? step.input?.title;
  const purpose = typeof said === "string" ? said : "";
  const others = Object.entries(step.input ?? {}).filter(
    ([k]) => !["sql", "purpose", "title", "vega_lite"].includes(k)
  );
  const rows = useMemo(() => (step.chart ? rowObjects(step.chart.result) : []), [step.chart]);
  // A single view without a width of its own spans the panel.
  const spec = useMemo(() => {
    const given = step.chart?.spec ?? {};
    return ("mark" in given || "layer" in given) && !("width" in given)
      ? { width: "container", height: 320, ...given }
      : given;
  }, [step.chart]);
  const queried = step.sqls.length > 0 || step.result || step.chart;
  return (
    <div className={`console__step console__tool${step.outcome?.is_error ? " failed" : ""}`}>
      <div className="console__tool-head">
        <strong>{step.name}</strong>
        {purpose && <span> {purpose}</span>}
        {!purpose &&
          others.length > 0 &&
          others.map(([k, v]) => (
            <span key={k} className="console__meta">
              {" "}
              {k}={JSON.stringify(v)}
            </span>
          ))}
        <span className="console__meta">
          {step.outcome ? ` · ${step.outcome.elapsed_ms} ms` : " · running…"}
        </span>
      </div>
      {step.sqls.map((s, i) => (
        <div key={i} className="console__sql">
          <pre>{s.sql}</pre>
          <div className="console__bar">
            {Object.keys(s.params).length > 0 && (
              <span className="console__meta">
                {Object.entries(s.params)
                  .map(([k, v]) => `$${k} = ${JSON.stringify(v)}`)
                  .join(", ")}
              </span>
            )}
            <button
              onClick={() =>
                open(s, step.chart && s.sql === step.chart.sql ? step.chart.spec : undefined)
              }
            >
              Open in workbench
            </button>
          </div>
        </div>
      ))}
      {step.result && (
        <>
          <ResultTable result={step.result} limit={50} />
          {step.result.row_count !== undefined &&
            step.result.row_count > step.result.rows.length && (
              <div className="console__meta">
                {thousands(step.result.row_count)}
                {step.result.truncated ? "+" : ""} rows in all; Claude saw the first{" "}
                {step.result.rows.length}
              </div>
            )}
        </>
      )}
      {step.chart && (
        <div>
          {step.chart.title && <h4>{step.chart.title}</h4>}
          <VegaChart spec={spec} rows={rows} />
        </div>
      )}
      {step.progress.length > 0 && <pre className="console__log">{step.progress.join("\n")}</pre>}
      {step.proposal && <QuoteCard why={step.proposal.why} given={step.proposal.regeneration} />}
      {step.outcome?.is_error && <div className="console__error">{step.outcome.preview}</div>}
      {step.outcome && !step.outcome.is_error && !queried && !step.proposal && (
        <details>
          <summary className="console__meta">
            what came back ({thousands(step.outcome.chars)} characters)
          </summary>
          <pre className="console__log">{step.outcome.preview}</pre>
        </details>
      )}
    </div>
  );
};

const QuestionView: React.FC<{ q: Question; running: boolean }> = ({ q, running }) => {
  const u = q.usage;
  const d = q.done;
  const secs = d?.elapsed_ms !== undefined ? Math.round(d.elapsed_ms / 1000) : null;
  return (
    <div className="console__question">
      <div className="console__asked">{q.text}</div>
      {q.steps.map((s, i) => {
        switch (s.kind) {
          case "thinking":
            return (
              <details
                key={i}
                className="console__step console__thinking"
                open={running || undefined}
              >
                <summary>Thinking</summary>
                <Markdown text={s.text} />
              </details>
            );
          case "text":
            return (
              <div key={i} className="console__step">
                <Markdown text={s.text} />
              </div>
            );
          case "tool":
            return <ToolView key={s.id} step={s} />;
          case "fallback":
            return (
              <div key={i} className="console__step console__todo">
                {s.from} declined{s.category ? ` (${s.category})` : ""}; {s.to} answered from here
              </div>
            );
          case "refusal":
            return (
              <div key={i} className="console__step console__error">
                Refused{s.category ? ` (${s.category})` : ""}
                {s.explanation ? `: ${s.explanation}` : ""}
              </div>
            );
          case "error":
            return (
              <div key={i} className="console__step console__error">
                {s.message}
              </div>
            );
        }
      })}
      <div className="console__meta">
        {running ? "answering… " : ""}
        {usd(d?.usd ?? u?.question_usd)}
        {u ? ` · ${u.turn} turn${u.turn === 1 ? "" : "s"}` : ""}
        {u ? ` · ${thousands(u.budget.used)} of ${thousands(u.budget.total)} budget tokens` : ""}
        {u ? ` · ${thousands(u.tokens.cache_read)} cached input tokens read` : ""}
        {secs !== null ? ` · ${secs} s` : ""}
        {d && d.stop_reason !== "end_turn" ? ` · stopped: ${d.stop_reason}` : ""}
      </div>
    </div>
  );
};

/**
 * Questions in plain language, answered by Claude writing and running SQL over the catalog: each
 * statement, its rows, its charts and any proposed farm run as they happen, and what they cost.
 */
const Ask: React.FC = () => {
  const [questions, setQuestions] = useState<Question[]>([]);
  const [conversation, setConversation] = useState<string | null>(null);
  const [text, setText] = useState("");
  const [budget, setBudget] = useState(String(BUDGET));
  const [running, setRunning] = useState(false);
  const [spend, setSpend] = useState<Spend | null>(null);
  const abort = useRef<AbortController | null>(null);

  const readSpend = useCallback(() => {
    getSpend().then(setSpend, () => undefined);
  }, []);
  useEffect(readSpend, [readSpend]);
  useEffect(() => () => abort.current?.abort(), []);

  const n = Number(budget);
  const budgetOk = Number.isInteger(n) && n >= MIN_BUDGET;

  const update = (f: (q: Question) => Question) =>
    setQuestions((qs) => [...qs.slice(0, -1), f(qs[qs.length - 1])]);

  const ask = () => {
    const question = text.trim();
    if (!question || running || !budgetOk) return;
    const controller = new AbortController();
    abort.current = controller;
    setRunning(true);
    setText("");
    setQuestions((qs) => [...qs, { text: question, budget: n, steps: [], startedAt: Date.now() }]);
    askStream(
      { question, task_budget: n, ...(conversation ? { conversation_id: conversation } : {}) },
      (e) => {
        if (e.type === "start") setConversation(e.conversation_id);
        update((q) => applied(q, e));
      },
      controller.signal
    )
      .catch((e: unknown) => {
        const cancelled = controller.signal.aborted;
        update((q) =>
          applied(
            cancelled
              ? q
              : applied(q, { type: "error", message: e instanceof Error ? e.message : String(e) }),
            {
              type: "done",
              stop_reason: cancelled ? "cancelled" : "error",
              usd: q.usage?.question_usd,
              elapsed_ms: Date.now() - q.startedAt,
            }
          )
        );
      })
      .finally(() => {
        setRunning(false);
        abort.current = null;
        readSpend();
      });
  };

  const today = spend?.days.find((d) => d.day === new Date().toLocaleDateString("en-CA"));
  const conversationUsd = questions.reduce(
    (s, q) => s + (q.done?.usd ?? q.usage?.question_usd ?? 0),
    0
  );

  return (
    <div className="console__view console__ask">
      <div className="console__view-head">
        <div>
          <h2>Ask</h2>
          <p className="console__desc">
            Claude (claude-opus-5-5 at medium effort, with adaptive thinking and the server-side
            fallbacks on) answers by writing and running SQL over the catalog: every statement, its
            rows and charts show here, and a farm run it proposes runs only when you approve its
            quote. Spend is uncapped and counted per question.
          </p>
        </div>
        <div className="console__meta">
          this conversation {usd(conversationUsd)} · today {usd(today?.usd)} · all{" "}
          {usd(spend?.total_usd)}
        </div>
      </div>
      {questions.map((q, i) => (
        <QuestionView key={i} q={q} running={running && i === questions.length - 1} />
      ))}
      <div className="console__askbar">
        <textarea
          rows={3}
          value={text}
          placeholder={
            conversation
              ? "A follow-up question (Ctrl+Enter asks)"
              : "Ask about the sims' results, e.g. which cells are structural in sa0755? (Ctrl+Enter asks)"
          }
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
              e.preventDefault();
              ask();
            }
          }}
        />
        <div className="console__bar">
          <button className="primary" onClick={ask} disabled={running || !text.trim() || !budgetOk}>
            {conversation ? "Ask a follow-up" : "Ask"}
          </button>
          <label>
            task budget{" "}
            <input
              value={budget}
              size={8}
              onChange={(e) => setBudget(e.target.value.replace(/[^\d]/g, ""))}
              title={`tokens Claude paces this question to (at least ${thousands(MIN_BUDGET)})`}
            />{" "}
            tokens
          </label>
          {!budgetOk && <span className="console__error">at least {thousands(MIN_BUDGET)}</span>}
          {running && <button onClick={() => abort.current?.abort()}>Cancel</button>}
          {conversation && !running && (
            <button
              onClick={() => {
                setConversation(null);
                setQuestions([]);
              }}
            >
              New conversation
            </button>
          )}
        </div>
      </div>
    </div>
  );
};

export default Ask;

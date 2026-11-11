# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""Questions about the sims' results in plain language, answered by
Claude writing and running SQL over the catalog: the console's Ask panel.

  Asker(pool, schema, ...)
                 the conversations, their tools and the spend ledger
    .ask(question, conversation_id=None, task_budget=TASK_BUDGET,
         emit=None, cancel=None) -> Answer
                 one question, a new conversation's or a follow-up in one:
                 a manual tool loop of streamed turns, each event passed to
                 `emit` as a dict (the console's NDJSON lines, below), until
                 Claude ends its turn, `cancel` (a threading.Event) is set,
                 or the API refuses or fails
    .conversation(id) -> dict
                 a conversation's questions and their events, or None
  system_prompt(schema, saved) -> str
                 the system prompt: ask_prompt.md, then the catalog's
                 tables and columns, macros, metrics and event kinds
                 (catalog.schema's) and the saved views, sorted and free of
                 counts and times, so the same catalog gives the same bytes
  TOOLS          the tools' definitions
  priced(message, requested) -> [Iteration]
                 a response's usage as the ledger prices it: one entry per
                 `usage.iterations` entry (each fallback hop's), at its
                 model's PRICES
  spend(ledger=None, day=None) -> dict
                 what the questions cost, per day and per question, from
                 the ledger (target/console/spend.jsonl in the main
                 checkout)

Each request is `client.beta.messages.stream` with MODEL, adaptive thinking
shown summarized, EFFORT set explicitly, the server-side fallbacks'
default routing and a task budget (`output_config.task_budget`, BETAS),
the system prompt cached for an hour and the conversation's tail for five
minutes. The budget is advisory: Claude sees it count down over the
question's turns (a question starts a fresh one; tool results continue it)
and paces itself, and `remaining` is never sent, as the history is resent
whole. Nothing caps the turns or the spend; every response's usage is
priced and appended to the ledger, and the stream reports it.

The loop reads each response's stop_reason before it runs anything:
`tool_use` runs every tool call and returns their results in one user
message (a failure as `is_error`, which Claude can correct); `pause_turn`
sends the conversation again; `refusal` (the fallback chain declined too)
and `max_tokens` with a tool call pending drop that turn and run nothing;
anything else ends the question. History is append-only: each turn's
content goes back as the API gave it, but for what a fallback's boundary
leaves out (`echoed`), and a conversation keeps the system prompt and
tools it started with, which its thinking blocks are bound to.

The tools are simlab's functions: run_sql (the pool's guarded readers),
run_view (views.run), chart (its SQL's rows and a checked Vega-Lite spec),
score, arm_scores, structural and dynamics (simlab.scoring), fetch_data
(catalog.fetch, its S3 cost reported) and propose_farm_run
(regenerate.regenerate: suite.write_matrices and farm.quote). A proposal
is a quote and nothing more: only the console's Approve runs one, and
nothing here approves.

Events, each {type, ...}: start, thinking and text (deltas, by turn and
block), tool_start (a call's block began), tool (a call with its input),
sql (a statement a tool runs, with its params), result (a statement's
first rows), chart, progress (a fetch's lines), proposal (a quote),
tool_result (what went back to Claude, truncated, and is_error), fallback,
usage (a turn's tokens and cost, the question's and conversation's cost,
and the tokens counted against the task budget), refusal, error and done
{stop_reason, usd, tokens, turns, elapsed_ms}, always last.
"""

import concurrent.futures as cf
import dataclasses
import datetime
import json
import math
import re
import secrets
import threading
import time
from pathlib import Path

import anthropic

from . import catalog as catalog_module
from . import engine, regenerate, scoring, store_host, views
from . import farm as farm_module
from . import suite as suite_module

MODEL = "claude-opus-5-5"
MAX_TOKENS = 64_000
EFFORT = "medium"
FALLBACKS = "default"
BETAS = ("server-side-fallback-2026-07-01", "task-budgets-2026-03-13")
THINKING = {"type": "adaptive", "display": "summarized"}
# The task budget's tokens: the default, and the least the API takes.
TASK_BUDGET = 200_000
MIN_TASK_BUDGET = 20_000
MAX_TASK_BUDGET = 1_000_000
PROMPT = Path(__file__).resolve().parent / "ask_prompt.md"

# A statement's rows: how many run_sql counts, how many go back to Claude,
# how many a chart draws, and how long one may run.
SQL_ROWS = 10_000
SQL_SHOWN = 50
CHART_ROWS = 5_000
SQL_TIMEOUT_S = 60
# The most text a tool result holds, and what the console shows of one.
RESULT_CHARS = 30_000
PREVIEW_CHARS = 4_000
# Tool calls of one turn that run at once.
TOOL_WORKERS = 4


@dataclasses.dataclass(frozen=True)
class Price:
    """USD per million tokens."""
    input: float
    output: float
    cache_read: float
    cache_write_5m: float
    cache_write_1h: float


# A model's rates; one not listed is priced at the highest of each (a
# ceiling), as a fallback may route to a model this table predates.
PRICES = {
    "claude-opus-5-5": Price(4.00, 20.00, 0.20, 5.00, 8.00),
    "claude-opus-5": Price(5.00, 25.00, 0.50, 6.25, 10.00),
    "claude-opus-4-8": Price(5.00, 25.00, 0.50, 6.25, 10.00),
}
CEILING = Price(*(max(getattr(p, f.name) for p in PRICES.values()) for f in dataclasses.fields(Price)))
# The refusal categories billed when the refusal came before any output;
# a decline after some output is billed whatever its category.
BILLED_BEFORE_OUTPUT = frozenset({"bio", "frontier_llm", "reasoning_extraction"})


def ledger_path():
    """The spend ledger, target/console/spend.jsonl in the main checkout."""
    return store_host.main_checkout() / "target" / "console" / "spend.jsonl"


# --- the prompt -----------------------------------------------------------------------------------


def text_of(v):
    """A catalog value as the prompt writes it: a list or object as compact
    JSON with sorted keys."""
    if v is None:
        return ""
    if isinstance(v, (list, dict)):
        return json.dumps(v, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return str(v)


def clipped(s, n):
    s = " ".join(str(s).split())
    return s if len(s) <= n else s[:n - 1].rstrip() + "…"


MACRO_KINDS = ("macro", "table macro", "macros")
# A column's catalog note, at most this long.
NOTE_CHARS = 240


def schema_text(schema):
    """The catalog's part of the prompt: main's tables and views with
    their columns and the catalog's notes, raw's names, the macros, the
    scored metrics and the event kinds, each sorted by name."""
    about = {t["name"]: t for t in schema.get("table_catalog") or []}
    columns = collections_by(schema.get("metric_catalog") or [], "tbl")
    out = ["# The catalog", "",
           "Query the `main` schema's tables and views (the default). `raw` holds the sources they fold, which "
           "you rarely need: " + ", ".join(sorted(t["name"] for t in schema.get("tables") or []
                                                  if t["schema"] == "raw")) + ".", "",
           "## Tables and views", ""]
    for t in sorted((t for t in schema.get("tables") or [] if t["schema"] == "main"), key=lambda t: t["name"]):
        a = about.get(t["name"], {})
        out.append(f"### {t['name']}" + (f" ({a['kind']})" if a.get("kind") else ""))
        meta = [f"{k}: {clipped(a[k], 400)}" for k in ("grain", "keys", "joins") if a.get(k)]
        if meta:
            out.append("; ".join(meta))
        if a.get("description"):
            out.append(clipped(a["description"], 2000))
        cat = {m["name"]: m for m in columns.get(t["name"], [])}
        cols = []
        for c in t["columns"]:
            unit = (cat.get(c["name"]) or {}).get("unit")
            cols.append(f"{c['name']} {c['type']}" + (f" [{unit}]" if unit else ""))
        out.append("columns: " + ", ".join(cols))
        notes = {}
        for c in t["columns"]:
            d = (cat.get(c["name"]) or {}).get("description")
            if d:
                notes.setdefault(clipped(d, NOTE_CHARS), []).append(c["name"])
        for note, names in sorted(notes.items(), key=lambda kv: kv[1]):
            out.append(f"- {', '.join(names)}: {note}")
        out.append("")
    out += ["## Macros", ""]
    for m in sorted((t for t in schema.get("table_catalog") or [] if t.get("kind") in MACRO_KINDS),
                    key=lambda t: t["name"]):
        head = m["name"] + (m["keys"] if m.get("keys") and str(m["keys"]).startswith("(") else "")
        out.append(f"- `{head}` ({m['kind']}): {clipped(m.get('description') or '', 2000)}")
    out += ["", "## Scored metrics", "",
            "metric_catalog's tbl = 'suite' rows: what the scoring macros score, by name (better: the direction "
            "that is better; kind: ratio or diff; tie: the band within which an effect is a tie; source: the "
            "SQL over a run's row r and verdict v).", ""]
    for m in sorted(columns.get("suite", []), key=lambda m: m["name"]):
        fields = [f"{k} {text_of(m[k])}" for k in ("better", "kind", "tie", "guard", "unit") if m.get(k) is not None]
        line = f"- {m['name']}: " + ", ".join(fields)
        if m.get("source"):
            line += f"; source `{clipped(m['source'], 300)}`"
        if m.get("description"):
            line += f"; {clipped(m['description'], NOTE_CHARS)}"
        out.append(line)
    out += ["", "## Event kinds", "", "event_kinds: the `events` and `derived_events` kinds by sim.", ""]
    for e in sorted(schema.get("event_kinds") or [], key=lambda e: (str(e.get("sim")), str(e.get("kind")))):
        line = f"- {e.get('kind')} ({e.get('sim')}): {clipped(e.get('description') or '', NOTE_CHARS)}"
        if e.get("value_meaning"):
            line += f"; value: {clipped(e['value_meaning'], 120)}"
        out.append(line)
    return "\n".join(out) + "\n"


def collections_by(rows, key):
    out = {}
    for r in rows:
        out.setdefault(r.get(key), []).append(r)
    return out


def views_text(saved):
    """The saved views as examples: each one's name, title, description,
    params and SQL."""
    out = ["# The saved views", "",
           "The console's pages, each one statement whose `$name`s its params bind (run_view runs one). They "
           "are worked examples of the catalog's joins and of suite.py's formulas.", ""]
    for v in sorted(saved, key=lambda v: v.name):
        out.append(f"## {v.name}: {v.title}")
        if v.description:
            out.append(v.description)
        if v.params:
            out.append("params: " + ", ".join(
                f"{p.name} ({p.kind}" + (f", default {text_of(p.default) or repr(p.default)}" if p.default is not None
                                         else "") + ")" for p in v.params))
        sql = "\n".join(line for line in v.sql.splitlines() if not re.match(r"--\s*(Copyright|SPDX)", line))
        out += ["```sql", sql.strip("\n"), "```", ""]
    return "\n".join(out)


def system_prompt(schema, saved):
    """The system prompt for `schema` (catalog.schema's parts) and `saved`
    (views.saved's views): byte for byte the same for the same inputs."""
    glossary = re.sub(r"\A<!--.*?-->\s*", "", PROMPT.read_text(), flags=re.S)
    return glossary.rstrip("\n") + "\n\n" + schema_text(schema) + "\n" + views_text(saved)


# --- the tools ------------------------------------------------------------------------------------


def tool(name, description, properties):
    """A strict tool taking `properties`, every one required."""
    return {"name": name, "description": description, "strict": True,
            "input_schema": {"type": "object", "properties": properties, "required": list(properties),
                             "additionalProperties": False}}


STR = {"type": "string"}
NAMES = {"type": "array", "items": {"type": "string"}}

TOOLS = [
    tool("run_sql", "Runs one DuckDB SELECT (or DESCRIBE, SHOW, SUMMARIZE, or EXPLAIN of one) over the catalog, "
         "read-only. Returns its columns and types, its first 50 rows, how many rows it has (counted to 10,000) "
         "and its time. A statement that fails comes back as an error saying why: fix it and run it again.",
         {"sql": {**STR, "description": "The statement, its literals inline."},
          "purpose": {**STR, "description": "What it finds out, in a few words, for the console's list of "
                                            "queries."}}),
    tool("run_view", "Runs one of the console's saved views with its params, and returns its rows as run_sql does. "
         "A param left out takes the view's default.",
         {"name": {**STR, "description": "The view's name (the saved views list them)."},
          "params": {"type": "array", "description": "The params to bind, by name.",
                     "items": {"type": "object", "additionalProperties": False, "required": ["name", "value"],
                               "properties": {"name": STR,
                                              "value": {"anyOf": [STR, {"type": "number"}, {"type": "null"}]}}}}}),
    tool("chart", "Draws a Vega-Lite chart of one statement's rows in the console. The statement runs as run_sql's "
         "does (its first 5,000 rows are the chart's data); the spec names fields by the statement's column names "
         "and takes no data of its own (no `data`, `datasets` or URLs). Returns the rows drawn, or what's wrong "
         "with the spec.",
         {"sql": {**STR, "description": "The statement whose rows the chart draws."},
          "vega_lite": {**STR, "description": "The Vega-Lite spec, as JSON text."},
          "title": {**STR, "description": "The chart's title."}}),
    tool("score", "Scores arm_b of exp_b against arm_a of exp_a cell by cell, as `suite.py score exp_a:arm_a "
         "exp_b:arm_b` prints it: per cell, the collapsed runs of each with the Fisher p, and each metric that "
         "moved (+ better, - worse, ~ changed) with both medians and the ratio or difference's 95 % t-interval "
         "over the paired seeds; then the totals.",
         {"exp_a": STR, "arm_a": STR, "exp_b": STR, "arm_b": STR,
          "cells": {**NAMES, "description": "The cells to score; empty for every cell both ran."}}),
    tool("arm_scores", "Scores each arm of an experiment against its base, as suite.py's races rank them: collapsed "
         "runs (the base's beside), the geomean over cells of the lag p99 ratio and of wire bytes per useful byte, "
         "the guardrails broken, and each cell's line; best first.",
         {"exp": STR, "base": STR,
          "arms": {**NAMES, "description": "The arms to score; empty for every arm but the base."},
          "cells": {**NAMES, "description": "The cells; empty for every cell."},
          "rank": {"type": "string", "enum": ["lag", "overhead"],
                   "description": "Rank by lag (the default), or for overhead."}}),
    tool("structural", "The cells of an experiment where every arm collapsed on every seed (the structural "
         "macro): {cell, arms, runs}.", {"exp": STR}),
    tool("dynamics", "Each arm and cell of an experiment in the terms that tell a startup tail from a steady "
         "spike (the dynamics macro): runs, collapsed, lag p99 by phase, spike seconds past each playout delay, "
         "playback at each delay, the longest unhealthy spells.", {"exp": STR}),
    tool("fetch_data", "Copies what S3 still holds of an experiment's jobs into the catalog and indexes it: their "
         "farm flushes the catalog lacks, and with `tables` those sample tables (reads, forecast). Costs S3's "
         "charges, which it returns. Data S3 no longer keeps (7 days) comes back only from a farm rerun.",
         {"exp_id": STR,
          "cells": {**NAMES, "description": "The cells; empty for all."},
          "arms": {**NAMES, "description": "The arms; empty for all."},
          "tables": {**NAMES, "description": "Sample tables to fetch too (reads, forecast); empty for none."}}),
    tool("propose_farm_run", "Proposes running an experiment's cells, arms and seeds again on the farm: writes the "
         "suite's matrices and quotes them (farm.py estimate: jobs, wall time, cost). It starts nothing. The "
         "console shows the quote, and only the developer's Approve click there runs it; tell them what it would "
         "cost and why it's needed.",
         {"exp_id": STR, "cells": NAMES, "arms": NAMES,
          "seeds": {"type": "array", "items": {"type": "integer"}, "description": "Consecutive seeds."},
          "why": {**STR, "description": "What the rerun would answer, for the quote card."}}),
]


class ToolError(Exception):
    """A tool call that failed, as its is_error result says."""


@dataclasses.dataclass
class Call:
    """A tool call being run: its id and name, and `emit` for its events."""
    id: str
    name: str
    emit: object


def fitted(out, rows_key="rows"):
    """`out` as JSON text of at most RESULT_CHARS, its rows cut until it
    fits (rows_shown saying how many are left)."""
    text = json.dumps(out, ensure_ascii=False, default=str, allow_nan=True)
    rows = out.get(rows_key)
    if len(text) <= RESULT_CHARS or not isinstance(rows, list):
        return text[:RESULT_CHARS]
    n = len(rows)
    while n > 0 and len(text) > RESULT_CHARS:
        n //= 2
        text = json.dumps({**out, rows_key: rows[:n], "rows_shown": n}, ensure_ascii=False, default=str)
    return text[:RESULT_CHARS]


def walk(x):
    """Every dict in `x` (a parsed JSON value), outermost first."""
    if isinstance(x, dict):
        yield x
        for v in x.values():
            yield from walk(v)
    elif isinstance(x, list):
        for v in x:
            yield from walk(v)


VIEW_KEYS = ("mark", "layer", "concat", "hconcat", "vconcat", "facet", "repeat")
# The fields transforms make without an `as`.
MADE = {"fold": ["key", "value"], "density": ["value", "density"], "quantile": ["prob", "value"],
        "regression": [], "loess": []}


def spec_problems(spec, columns):
    """What's wrong with `spec` as a chart of rows with `columns`: data of
    its own, a URL, no view, or a field that is neither a column nor one a
    transform makes."""
    if not isinstance(spec, dict):
        return ["the spec isn't a JSON object"]
    out = []
    if not any(k in spec for k in VIEW_KEYS):
        out.append(f"the spec has no view: one of {', '.join(VIEW_KEYS)}")
    made, pivots = set(columns), False
    for d in walk(spec):
        for k in ("data", "datasets"):
            if k in d and not (k == "data" and d[k] is None):
                out.append(f"`{k}` given: the chart's data are its statement's rows")
        for k in ("url", "href"):
            if k in d:
                out.append(f"`{k}` given: a chart loads nothing")
        a = d.get("as")
        made.update([a] if isinstance(a, str) else [x for x in a if isinstance(x, str)] if isinstance(a, list) else [])
        for t, names in MADE.items():
            if t in d and "as" not in d:
                made.update(names)
        if "pivot" in d or "lookup" in d:
            pivots = True
    if not pivots:
        for d in walk(spec):
            for enc in (d.get("encoding"),) if isinstance(d.get("encoding"), dict) else ():
                for channel in walk(enc):
                    f = channel.get("field")
                    if isinstance(f, str) and f not in made and re.split(r"(?<!\\)[.\[]", f)[0] not in made:
                        out.append(f"field {f!r} is neither a column ({', '.join(columns)}) nor a transform's")
    return list(dict.fromkeys(out))


class Tools:
    """The tools over `pool` (engine.Pool of the catalog's readers): the
    saved views at `views_root`, `regenerations` (farm.Regenerations) and
    `suite` for proposals, `fetch` for fetch_data (catalog.fetch's
    signature, returning its result or a dict of it) and `scoring`."""

    def __init__(self, pool, views_root=views.VIEWS, regenerations=None, suite=suite_module, fetch=None,
                 scoring=scoring):
        self.pool = pool
        self._views_root = views_root
        self.regenerations = regenerations or farm_module.Regenerations()
        self.suite = suite
        self.fetch = fetch or catalog_module.fetch
        self.scoring = scoring

    @property
    def views_root(self):
        return root_of(self._views_root)

    def run(self, name, inp, call):
        """Tool `name` on `inp`: (text, is_error)."""
        fn = getattr(self, f"tool_{name}", None) if any(t["name"] == name for t in TOOLS) else None
        if fn is None:
            return f"no tool {name!r}", True
        if not isinstance(inp, dict):
            return "the tool's input isn't an object", True
        try:
            out = fn(inp, call)
        except ToolError as e:
            return str(e), True
        except engine.QueryError as e:
            return f"the query failed ({e.status}): {e}", True
        except Exception as e:
            return f"{type(e).__name__}: {e}", True
        return (out if isinstance(out, str) else fitted(out)), False

    def query(self, sql, params, max_rows, call, purpose=""):
        call.emit({"type": "sql", "id": call.id, "sql": sql, "params": params, "purpose": purpose})
        j = self.pool.query(sql, params, max_rows=max_rows, timeout_s=SQL_TIMEOUT_S, temporal_text=True).json()
        return j

    def rows_out(self, j, call):
        shown = j["rows"][:SQL_SHOWN]
        call.emit({"type": "result", "id": call.id,
                   "result": {"columns": j["columns"], "rows": shown, "row_count": len(j["rows"]),
                              "truncated": j["truncated"], "elapsed_ms": j["elapsed_ms"]}})
        return {"columns": j["columns"], "row_count": len(j["rows"]),
                "row_count_capped": j["truncated"], "rows": shown, "elapsed_ms": j["elapsed_ms"]}

    def tool_run_sql(self, inp, call):
        sql = inp.get("sql")
        if not isinstance(sql, str) or not sql.strip():
            raise ToolError("sql: the statement to run")
        return self.rows_out(self.query(sql, {}, SQL_ROWS, call, str(inp.get("purpose") or "")), call)

    def tool_run_view(self, inp, call):
        name = inp.get("name")
        try:
            view = views.get(name, self.views_root)
        except (KeyError, ValueError, TypeError):
            names = ", ".join(v.name for v in views.saved(self.views_root))
            raise ToolError(f"no saved view {name!r}: the views are {names}") from None
        given = {}
        for p in inp.get("params") or []:
            if isinstance(p, dict) and isinstance(p.get("name"), str):
                given[p["name"]] = p.get("value")
        params = {}
        for p in view.params:
            v = given.get(p.name)
            if v is None or v == "":
                v = p.default
            if v is not None:
                params[p.name] = views.value(p, v)
        missing = [p.name for p in view.params if p.name not in params]
        if missing:
            raise ToolError(f"view {view.name} needs {', '.join(missing)}")
        return self.rows_out(self.query(view.sql, params, SQL_ROWS, call, f"the {view.name} view"), call)

    def tool_chart(self, inp, call):
        try:
            spec = json.loads(inp.get("vega_lite") or "")
        except (TypeError, ValueError) as e:
            raise ToolError(f"vega_lite isn't JSON: {e}") from None
        sql = inp.get("sql")
        if not isinstance(sql, str) or not sql.strip():
            raise ToolError("sql: the statement whose rows the chart draws")
        if isinstance(spec, dict):
            spec = {k: v for k, v in spec.items() if k != "$schema"}
        early = [p for p in spec_problems(spec, []) if not p.startswith("field ")]
        if early:
            raise ToolError("; ".join(early))
        j = self.query(sql, {}, CHART_ROWS, call, f"chart: {inp.get('title') or ''}".strip())
        names = [c["name"] for c in j["columns"]]
        problems = spec_problems(spec, names)
        if problems:
            raise ToolError("; ".join(problems))
        if not j["rows"]:
            raise ToolError("the statement has no rows to draw")
        call.emit({"type": "chart", "id": call.id, "title": str(inp.get("title") or ""), "spec": spec, "sql": sql,
                   "result": j})
        return {"drawn": True, "rows": len(j["rows"]), "rows_capped": j["truncated"], "columns": names}

    def tool_score(self, inp, call):
        names(inp, "exp_a", "arm_a", "exp_b", "arm_b")
        card = self.scoring.score(inp["exp_a"], inp["arm_a"], inp["exp_b"], inp["arm_b"],
                                  cells=inp.get("cells") or None)
        if not card.cells:
            raise ToolError(f"no cells both {inp['exp_a']}:{inp['arm_a']} and {inp['exp_b']}:{inp['arm_b']} ran")
        return card.text()

    def tool_arm_scores(self, inp, call):
        names(inp, "exp", "base")
        got = self.scoring.arm_scores(inp["exp"], inp["base"], arms=inp.get("arms") or None,
                                      cells=inp.get("cells") or None, rank=inp.get("rank") or "lag")
        if not got:
            raise ToolError(f"{inp['exp']} has no arm but {inp['base']} to score")
        return "".join(s.text() for s in got)

    def tool_structural(self, inp, call):
        names(inp, "exp")
        return {"exp": inp["exp"], "rows": plain_rows(self.scoring.structural(inp["exp"]))}

    def tool_dynamics(self, inp, call):
        names(inp, "exp")
        rows = plain_rows(self.scoring.dynamics(inp["exp"]))
        if not rows:
            raise ToolError(f"{inp['exp']} has no runs the catalog holds a verdict of")
        return {"exp": inp["exp"], "rows": rows}

    def tool_fetch_data(self, inp, call):
        exp = inp.get("exp_id")
        cells, arms, tables = (list(inp.get(k) or []) for k in ("cells", "arms", "tables"))
        bad = regenerate.bad_name([exp, *cells, *arms, *tables])
        if bad is not None:
            raise ToolError(f"not a name: {json.dumps(bad)}")
        def progress(p):
            call.emit({"type": "progress", "id": call.id, "line": str(get(p, "message", "") or p)})
        try:
            r = self.fetch(exp, cells=cells or None, arms=arms or None, tables=tuple(tables), progress=progress)
        except SystemExit as e:
            raise ToolError(str(e)) from None
        r = dataclasses.asdict(r) if dataclasses.is_dataclass(r) else dict(r or {})
        return {"exp_id": exp, "s3_usd": r.get("s3_usd"), "pulled": len(r.get("pulled") or []),
                "gone": r.get("gone") or [], "note": "fetched and indexed: the catalog now holds it"}

    def tool_propose_farm_run(self, inp, call):
        exp = inp.get("exp_id")
        seeds = inp.get("seeds")
        if not isinstance(seeds, list) or not all(isinstance(s, int) and not isinstance(s, bool) for s in seeds):
            raise ToolError("seeds: a list of integers")
        cells, arms = list(inp.get("cells") or []), list(inp.get("arms") or [])
        bad = regenerate.bad_name([exp, *cells, *arms])
        if bad is not None:
            raise ToolError(f"not a name: {json.dumps(bad)}")
        try:
            r = regenerate.regenerate(self.pool, exp, cells, arms, seeds, self.regenerations, suite=self.suite)
        except farm_module.QuoteError as e:
            raise ToolError(str(e)) from None
        call.emit({"type": "proposal", "id": call.id, "why": str(inp.get("why") or ""), "regeneration": r})
        est = r.get("estimate") or {}
        return {"quote_id": r.get("id"), "state": r.get("state"), "error": r.get("error"),
                "jobs": est.get("jobs"), "cost_usd": est.get("cost"), "s3_cost_usd": est.get("s3_cost"),
                "wall_s": est.get("wall_s"), "warnings": r.get("warnings") or [],
                "quote": clipped(r.get("quote") or "", 2000),
                "approval": "nothing has started: the console shows this quote, and only the developer's Approve "
                            "click runs it"}


def root_of(views_root):
    """The views' directory: `views_root`, or what it returns when it is a
    function (the console's, which tests move)."""
    return Path(views_root() if callable(views_root) else views_root)


def names(inp, *keys):
    bad = regenerate.bad_name([inp.get(k) for k in keys])
    if bad is not None:
        raise ToolError(f"not a name: {json.dumps(bad)}")


def plain_rows(rows):
    """Rows of the CLI's JSON values with their non-finite numbers as
    strings, as JSON holds them."""
    def v(x):
        if isinstance(x, float) and not math.isfinite(x):
            return str(x)
        return x
    return [{k: v(x) for k, x in r.items()} for r in rows]


# --- spend ----------------------------------------------------------------------------------------


@dataclasses.dataclass
class Iteration:
    """One attempt of a response, as the ledger prices it: its type
    (message, fallback_message, compaction, advisor_message), the model
    that ran it, its tokens, whether it is billed and what it cost."""
    type: str
    model: str
    input_tokens: int = 0
    output_tokens: int = 0
    cache_read_input_tokens: int = 0
    cache_creation_input_tokens: int = 0
    cache_write_5m: int = 0
    cache_write_1h: int = 0
    billed: bool = True
    priced_as: str = ""
    usd: float = 0.0

    @property
    def input_total(self):
        return self.input_tokens + self.cache_read_input_tokens + self.cache_creation_input_tokens


def get(x, k, default=None):
    v = x.get(k, default) if isinstance(x, dict) else getattr(x, k, default)
    return default if v is None else v


def cost(it):
    """`it`'s USD at its model's rates (CEILING for a model PRICES lacks):
    cache writes at their TTL's rate, an unknown TTL's at the hour's."""
    p = PRICES.get(it.model)
    it.priced_as = it.model if p else "ceiling"
    p = p or CEILING
    if not it.billed:
        return 0.0
    return (it.input_tokens * p.input + it.output_tokens * p.output + it.cache_read_input_tokens * p.cache_read
            + it.cache_write_5m * p.cache_write_5m + it.cache_write_1h * p.cache_write_1h) / 1e6


def iteration(u, type, model):
    created = int(get(u, "cache_creation_input_tokens", 0))
    split = get(u, "cache_creation")
    w5, w1 = (int(get(split, "ephemeral_5m_input_tokens", 0)), int(get(split, "ephemeral_1h_input_tokens", 0))) \
        if split is not None else (0, created)
    if w5 + w1 < created:
        w1 += created - w5 - w1
    return Iteration(type=type, model=model, input_tokens=int(get(u, "input_tokens", 0)),
                     output_tokens=int(get(u, "output_tokens", 0)),
                     cache_read_input_tokens=int(get(u, "cache_read_input_tokens", 0)),
                     cache_creation_input_tokens=created, cache_write_5m=w5, cache_write_1h=w1)


def priced(message, requested=MODEL):
    """`message`'s usage, an Iteration per `usage.iterations` entry (the
    top-level usage when it has none), each priced. A hop a classifier
    declined before any output is billed only in BILLED_BEFORE_OUTPUT's
    categories: the k-th declined hop is the k-th `message` entry before a
    fallback's, its category the k-th fallback block's trigger; the last
    entry of a refused response is the refusal, its category
    stop_details'. Anything else is billed."""
    usage = get(message, "usage")
    served = get(message, "model") or requested
    entries = list(get(usage, "iterations", []) or [])
    content = get(message, "content", []) or []
    triggers = [get(get(b, "trigger"), "category") for b in content if get(b, "type") == "fallback"]
    out = []
    if not entries:
        out.append(iteration(usage, "message", served))
    else:
        for e in entries:
            out.append(iteration(e, get(e, "type", "message"), get(e, "model") or served))
    hops = [it for it in out if it.type == "message"] if any(it.type == "fallback_message" for it in out) else []
    for i, it in enumerate(hops):
        if i < len(triggers) and it.output_tokens == 0 and triggers[i] not in BILLED_BEFORE_OUTPUT:
            it.billed = False
    if get(message, "stop_reason") == "refusal" and out:
        last = out[-1]
        category = get(get(message, "stop_details"), "category")
        if last.output_tokens == 0 and category not in BILLED_BEFORE_OUTPUT:
            last.billed = False
    for it in out:
        it.usd = cost(it)
    return out


class Ledger:
    """target/console/spend.jsonl: a line per priced iteration."""

    def __init__(self, path=None):
        self._path = Path(path) if path else None
        self._lock = threading.Lock()

    @property
    def path(self):
        return self._path or ledger_path()

    def add(self, lines):
        if not lines:
            return
        text = "".join(json.dumps(line, sort_keys=True, separators=(",", ":")) + "\n" for line in lines)
        with self._lock:
            self.path.parent.mkdir(parents=True, exist_ok=True)
            with open(self.path, "a", encoding="utf-8") as f:
                f.write(text)


def spend(ledger=None, day=None):
    """What the questions cost, from the ledger: {days: [{day, usd,
    questions, requests}] (newest first), questions: [{day, conversation,
    question, text, usd, requests}] (`day`'s, or the latest 50), total_usd,
    ledger}."""
    path = Path(ledger) if ledger else ledger_path()
    days, questions, total = {}, {}, 0.0
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except FileNotFoundError:
        lines = []
    for line in lines:
        try:
            r = json.loads(line)
        except ValueError:
            continue
        usd = float(r.get("usd") or 0)
        total += usd
        d = days.setdefault(r.get("day"), {"day": r.get("day"), "usd": 0.0, "questions": set(), "requests": set()})
        key = (r.get("conversation"), r.get("question"))
        d["usd"] += usd
        d["questions"].add(key)
        d["requests"].add((*key, r.get("turn")))
        q = questions.setdefault(key, {"day": r.get("day"), "conversation": key[0], "question": key[1],
                                       "text": r.get("text") or "", "usd": 0.0, "requests": set()})
        q["usd"] += usd
        q["requests"].add(r.get("turn"))
    out_days = [{**d, "usd": round(d["usd"], 6), "questions": len(d["questions"]), "requests": len(d["requests"])}
                for d in sorted(days.values(), key=lambda d: str(d["day"]), reverse=True)]
    qs = [{**q, "usd": round(q["usd"], 6), "requests": len(q["requests"])} for q in questions.values()]
    qs = [q for q in qs if day is None or q["day"] == day]
    qs = qs[-50:] if day is None else qs
    return {"days": out_days, "questions": list(reversed(qs)), "total_usd": round(total, 6), "ledger": str(path)}


# --- the loop -------------------------------------------------------------------------------------


class Cancelled(Exception):
    pass


class Busy(Exception):
    """A question asked in a conversation answering another."""


@dataclasses.dataclass
class Answer:
    conversation_id: str
    stop_reason: str
    usd: float = 0.0
    turns: int = 0
    text: str = ""
    error: str = None


@dataclasses.dataclass
class Conversation:
    """A conversation: the system prompt and tools it started with (its
    thinking blocks are bound to them), its messages, and each question's
    events."""
    id: str
    system: list
    tools: list
    created_at: str
    messages: list = dataclasses.field(default_factory=list)
    questions: list = dataclasses.field(default_factory=list)
    usd: float = 0.0
    lock: threading.Lock = dataclasses.field(default_factory=threading.Lock, repr=False)


def echoed(content):
    """A turn's content as the next request sends it back: as given, but
    for the thinking, redacted thinking, connector text and tool calls
    before the last fallback block (a declined model's), and server tool
    calls before it without their result; the fallback blocks stay where
    they were."""
    blocks = list(content)
    last = max((i for i, b in enumerate(blocks) if get(b, "type") == "fallback"), default=None)
    if last is None:
        return blocks
    results = {get(b, "tool_use_id") for b in blocks if str(get(b, "type", "")).endswith("_tool_result")}
    out = []
    for i, b in enumerate(blocks):
        t = get(b, "type")
        if i < last and (t in ("thinking", "redacted_thinking", "connector_text", "tool_use")
                         or (t == "server_tool_use" and get(b, "id") not in results)):
            continue
        out.append(b)
    return out


def now():
    return datetime.datetime.now(datetime.timezone.utc).astimezone()


def error_text(e):
    """An SDK error as the console says it, most specific first. None of
    them carries the request's headers, so no key reaches the stream."""
    if isinstance(e, anthropic.AuthenticationError):
        return "the API refused the credentials (401): set ANTHROPIC_API_KEY where the console's server runs"
    if isinstance(e, anthropic.PermissionDeniedError):
        return f"the API denied the request (403): {e.message}"
    if isinstance(e, anthropic.NotFoundError):
        return f"the API found no such model or endpoint (404): {e.message}"
    if isinstance(e, anthropic.RateLimitError):
        after = e.response.headers.get("retry-after") if e.response is not None else None
        return "rate limited (429)" + (f": retry after {after} s" if after else "") + f": {e.message}"
    if isinstance(e, anthropic.BadRequestError):
        return f"the API refused the request (400): {e.message}"
    if isinstance(e, anthropic.APIStatusError):
        return f"the API answered {e.status_code}: {e.message}"
    if isinstance(e, anthropic.APITimeoutError):
        return "the API didn't answer in time"
    if isinstance(e, anthropic.APIConnectionError):
        return "couldn't reach the API (a network error)"
    return f"{type(e).__name__}: {e}"


class Asker:
    """Questions over the catalog's readers `pool`: `schema()` gives
    catalog.schema's parts (the console's cached copy), `client` the
    Anthropic client (anthropic.Anthropic() by default, made at the first
    question), `ledger` the spend ledger's path; `views_root`,
    `regenerations`, `suite`, `fetch` and `scoring` are the tools'
    (Tools): `views_root` may be a function returning the directory."""

    def __init__(self, pool, schema, client=None, ledger=None, views_root=views.VIEWS, regenerations=None,
                 suite=suite_module, fetch=None, scoring=scoring):
        self.schema = schema
        self._views_root = views_root
        self.tools = Tools(pool, views_root, regenerations, suite, fetch, scoring)
        self.ledger = Ledger(ledger)
        self._client = client
        self._conversations = {}
        self._lock = threading.Lock()

    def client(self):
        with self._lock:
            if self._client is None:
                self._client = anthropic.Anthropic()
            return self._client

    def busy(self, id):
        """Whether conversation `id` is answering a question."""
        c = self._conversations.get(id)
        return c is not None and c.lock.locked()

    def conversation(self, id):
        """A conversation's state as GET /console/ask/ID answers it."""
        c = self._conversations.get(id)
        if c is None:
            return None
        return {"id": c.id, "created_at": c.created_at, "usd": round(c.usd, 6),
                "questions": [{k: v for k, v in q.items()} for q in c.questions]}

    def start(self):
        """A new conversation, its system prompt and tools as of now."""
        saved = views.saved(root_of(self._views_root))
        system = [{"type": "text", "text": system_prompt(self.schema(), saved),
                   "cache_control": {"type": "ephemeral", "ttl": "1h"}}]
        c = Conversation(id=secrets.token_urlsafe(9), system=system, tools=[dict(t) for t in TOOLS],
                         created_at=now().isoformat(timespec="seconds"))
        with self._lock:
            self._conversations[c.id] = c
        return c

    def request(self, c, budget):
        """The request of a turn of `c`'s, with a task budget of `budget`."""
        return {"model": MODEL, "max_tokens": MAX_TOKENS, "betas": list(BETAS), "fallbacks": FALLBACKS,
                "thinking": dict(THINKING),
                "output_config": {"effort": EFFORT, "task_budget": {"type": "tokens", "total": budget}},
                "system": c.system, "tools": c.tools, "tool_choice": {"type": "auto"},
                "cache_control": {"type": "ephemeral"}, "messages": c.messages}

    def ask(self, question, conversation_id=None, task_budget=TASK_BUDGET, emit=None, cancel=None):
        """Answers `question` (the module docs): in conversation
        `conversation_id`, or a new one. KeyError for no such
        conversation, Busy for one answering a question, ValueError for a
        budget under MIN_TASK_BUDGET or past MAX_TASK_BUDGET."""
        if not MIN_TASK_BUDGET <= task_budget <= MAX_TASK_BUDGET:
            raise ValueError(f"task_budget: {MIN_TASK_BUDGET:,} to {MAX_TASK_BUDGET:,} tokens")
        if conversation_id is None:
            c = self.start()
        else:
            c = self._conversations.get(conversation_id)
            if c is None:
                raise KeyError(conversation_id)
        if not c.lock.acquire(blocking=False):
            raise Busy(c.id)
        try:
            return self._ask(c, question, task_budget, emit or (lambda e: None), cancel or threading.Event())
        finally:
            c.lock.release()

    def _ask(self, c, question, budget, emit, cancel):
        t0 = time.monotonic()
        record = {"question": question, "task_budget": budget, "asked_at": now().isoformat(timespec="seconds"),
                  "usd": 0.0, "stop_reason": None, "events": []}
        c.questions.append(record)
        n = len(c.questions)
        say_lock = threading.Lock()

        def say(event):
            with say_lock:
                if event["type"] not in ("thinking", "text"):
                    record["events"].append(event)
                emit(event)

        state = {"budget": budget, "usd": 0.0, "turns": 0,
                 "tokens": {"input": 0, "output": 0, "cache_read": 0, "cache_write": 0}, "budget_used": 0,
                 "last_in": None, "last_out": 0, "text": []}
        say({"type": "start", "conversation_id": c.id, "question": n, "model": MODEL, "effort": EFFORT,
             "fallbacks": FALLBACKS, "task_budget": budget})
        c.messages.append({"role": "user", "content": question})
        stop, error = None, None
        try:
            while True:
                if cancel.is_set():
                    raise Cancelled
                state["turns"] += 1
                message = self.turn(c, budget, say, cancel, state["turns"])
                self.account(c, n, question, message, state, say)
                stop = message.stop_reason
                content = echoed(message.content)
                if stop == "refusal":
                    d = message.stop_details
                    say({"type": "refusal", "category": get(d, "category"), "explanation": get(d, "explanation")})
                    break
                calls = [b for b in content if get(b, "type") == "tool_use"]
                if stop == "max_tokens" and calls:
                    say({"type": "error", "message": f"the turn reached max_tokens ({MAX_TOKENS:,}) inside a tool "
                                                     "call, which doesn't run: ask again, narrower"})
                    break
                c.messages.append({"role": "assistant", "content": content})
                state["text"] += [get(b, "text", "") for b in content if get(b, "type") == "text"]
                if stop == "pause_turn":
                    continue
                if stop == "tool_use" and calls:
                    results = self.run_tools(calls, say)
                    c.messages.append({"role": "user", "content": results})
                    continue
                break
        except Cancelled:
            stop = "cancelled"
        except anthropic.AnthropicError as e:
            stop, error = "error", error_text(e)
            say({"type": "error", "message": error})
        record["usd"] = round(state["usd"], 6)
        record["stop_reason"] = stop
        say({"type": "done", "stop_reason": stop, "usd": round(state["usd"], 6),
             "conversation_usd": round(c.usd, 6), "tokens": state["tokens"], "turns": state["turns"],
             "budget_used": state["budget_used"], "elapsed_ms": round((time.monotonic() - t0) * 1000)})
        return Answer(c.id, stop, round(state["usd"], 6), state["turns"], "\n\n".join(t for t in state["text"] if t),
                      error)

    def turn(self, c, budget, say, cancel, turn):
        """One streamed request: its deltas and blocks said as they come,
        and the final message."""
        with self.client().beta.messages.stream(**self.request(c, budget)) as stream:
            for ev in stream:
                if cancel.is_set():
                    raise Cancelled
                t = ev.type
                if t == "content_block_delta":
                    d = ev.delta
                    if d.type == "thinking_delta" and d.thinking:
                        say({"type": "thinking", "turn": turn, "block": ev.index, "text": d.thinking})
                    elif d.type == "text_delta" and d.text:
                        say({"type": "text", "turn": turn, "block": ev.index, "text": d.text})
                elif t == "content_block_start":
                    b = ev.content_block
                    if b.type == "fallback":
                        say({"type": "fallback", "turn": turn, "from": get(get(b, "from_"), "model"),
                             "to": get(get(b, "to"), "model"), "category": get(get(b, "trigger"), "category")})
                    elif b.type == "tool_use":
                        say({"type": "tool_start", "turn": turn, "block": ev.index, "id": b.id, "name": b.name})
            return stream.get_final_message()

    def account(self, c, n, question, message, state, say):
        """Prices `message`'s usage into the ledger and says it."""
        its = priced(message, MODEL)
        usd = sum(it.usd for it in its)
        state["usd"] += usd
        c.usd += usd
        tok = state["tokens"]
        for it in its:
            tok["input"] += it.input_tokens
            tok["output"] += it.output_tokens
            tok["cache_read"] += it.cache_read_input_tokens
            tok["cache_write"] += it.cache_creation_input_tokens
        # What counts against the budget: what Claude wrote, and what it
        # read that was new (the tool results: the input's growth past the
        # last turn's input and output).
        final = its[-1]
        grown = 0 if state["last_in"] is None else max(0, final.input_total - state["last_in"] - state["last_out"])
        state["budget_used"] += sum(it.output_tokens for it in its) + grown
        state["last_in"], state["last_out"] = final.input_total, final.output_tokens
        at = now()
        self.ledger.add([{"ts": at.isoformat(timespec="seconds"), "day": at.date().isoformat(), "conversation": c.id,
                          "question": n, "text": clipped(question, 200), "turn": state["turns"],
                          "request_id": getattr(message, "_request_id", None), "id": get(message, "id"),
                          **dataclasses.asdict(it), "usd": round(it.usd, 8)} for it in its])
        say({"type": "usage", "turn": state["turns"], "model": get(message, "model"),
             "iterations": [dataclasses.asdict(it) for it in its], "usd": round(usd, 6),
             "question_usd": round(state["usd"], 6), "conversation_usd": round(c.usd, 6),
             "budget": {"total": state["budget"], "used": state["budget_used"]}, "tokens": dict(tok)})

    def run_tools(self, calls, say):
        """Runs every call of a turn (at most TOOL_WORKERS at once): their
        results, in the calls' order, as one user message's content. A
        cancel waits for them, so the conversation keeps every call's
        result."""
        def one(b):
            call = Call(b.id, b.name, say)
            say({"type": "tool", "id": b.id, "name": b.name, "input": b.input})
            t0 = time.monotonic()
            text, is_error = self.tools.run(b.name, b.input, call)
            say({"type": "tool_result", "id": b.id, "name": b.name, "is_error": is_error,
                 "preview": text[:PREVIEW_CHARS], "chars": len(text),
                 "elapsed_ms": round((time.monotonic() - t0) * 1000)})
            out = {"type": "tool_result", "tool_use_id": b.id, "content": text}
            if is_error:
                out["is_error"] = True
            return out
        with cf.ThreadPoolExecutor(max_workers=min(TOOL_WORKERS, len(calls))) as pool:
            return list(pool.map(one, calls))

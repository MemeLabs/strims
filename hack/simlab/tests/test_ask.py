#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simlab.ask over a stand-in Anthropic client (no network): the loop's
stop reasons (tool_use, pause_turn, refusal, max_tokens with and without
a pending call, end_turn, an SDK error, a cancel); every tool's dispatch
and its failures as is_error; a proposal that runs nothing; the ledger's
prices, fallback hops and refusals among them, and spend's sums; the
system prompt byte for byte the same across calls, catalogs given in
another order and a conversation's turns; every request carrying the
task budget, the fallbacks, the effort, summarized thinking and strict
tools; history appended to, never edited, and what a fallback leaves out:

  python3 hack/simlab/tests/test_ask.py
"""

import copy
import json
import re
import sys
import tempfile
import threading
import types
import unittest
from pathlib import Path

HACK = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(HACK))

from simlab import bootstrap  # noqa: E402

if __name__ == "__main__":
    bootstrap.engine(__file__)

import anthropic  # noqa: E402
import httpx2  # noqa: E402
from anthropic.types.beta import BetaMessage  # noqa: E402

from simlab import ask, catalog, engine, farm, views  # noqa: E402

NS = types.SimpleNamespace


def usage(input=100, output=50, read=0, write5=0, write1=0, iterations=None):
    u = {"input_tokens": input, "output_tokens": output, "cache_read_input_tokens": read,
         "cache_creation_input_tokens": write5 + write1,
         "cache_creation": {"ephemeral_5m_input_tokens": write5, "ephemeral_1h_input_tokens": write1}}
    if iterations is not None:
        u["iterations"] = iterations
    return u


def msg(content, stop="end_turn", model=ask.MODEL, u=None, stop_details=None):
    """A response as the API sends one."""
    return {"id": "msg_" + str(id(content)), "type": "message", "role": "assistant", "model": model,
            "content": content, "stop_reason": stop, "stop_sequence": None, "stop_details": stop_details,
            "usage": u or usage()}


def text(t):
    return {"type": "text", "text": t}


def thinking(t):
    return {"type": "thinking", "thinking": t, "signature": "sig-" + t}


def call(id, name, **inp):
    return {"type": "tool_use", "id": id, "name": name, "input": inp}


class FakeStream:
    """client.beta.messages.stream's manager and stream: the message's
    blocks as their events, then the message."""

    def __init__(self, client, message, cut=None):
        self.client = client
        self.message = BetaMessage.model_validate(message)
        self.cut = cut
        self.closed = False

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.closed = True
        return False

    def __iter__(self):
        for i, b in enumerate(self.message.content):
            yield NS(type="content_block_start", index=i, content_block=b)
            if b.type == "thinking":
                yield NS(type="content_block_delta", index=i, delta=NS(type="thinking_delta", thinking=b.thinking))
            elif b.type == "text":
                for part in re.findall(r".{1,8}", b.text, re.S):
                    yield NS(type="content_block_delta", index=i, delta=NS(type="text_delta", text=part))
            if self.cut is not None and i == self.cut:
                self.client.cancel.set()
            yield NS(type="content_block_stop", index=i)

    def get_final_message(self):
        return self.message


class FakeClient:
    """Answers each request with the script's next message (a dict, or a
    function of the request returning one, or an exception to raise),
    keeping each request as JSON."""

    def __init__(self, script):
        self.script = list(script)
        self.requests = []
        self.streams = []
        self.cancel = threading.Event()
        self.beta = NS(messages=NS(stream=self.stream))

    def stream(self, **kw):
        self.requests.append(json.loads(json.dumps(kw, default=lambda o: o.to_dict())))
        item = self.script.pop(0)
        if isinstance(item, BaseException):
            raise item
        cut = None
        if isinstance(item, tuple):
            item, cut = item
        if callable(item):
            item = item(self.requests[-1])
        s = FakeStream(self, item, cut)
        self.streams.append(s)
        return s


def status_error(cls, code, message="slow down"):
    req = httpx2.Request("POST", "https://api.anthropic.com/v1/messages")
    return cls(message, response=httpx2.Response(code, request=req, headers={"retry-after": "7"}),
               body={"error": {"type": "rate_limit_error", "message": message}})


def make_lake(lake):
    (lake / "data").mkdir(parents=True)
    w = engine.writer(lake)
    try:
        w.execute("""
CREATE TABLE nums AS SELECT range AS i, range * 2 AS twice FROM range(200);
CREATE TABLE runs AS SELECT * FROM (VALUES ('calm', 'base', 1, 300.0), ('calm', 'new', 1, 240.0),
  ('storm', 'base', 1, 900.0), ('storm', 'new', 1, 950.0)) t(cell, arm, seed, lag_p99_ms);""")
    finally:
        w.close()


SCHEMA = {
    "tables": [{"schema": "main", "name": "runs", "columns": [{"name": "cell", "type": "VARCHAR"},
                                                              {"name": "lag_p99_ms", "type": "DOUBLE"}]},
               {"schema": "main", "name": "nums", "columns": [{"name": "i", "type": "BIGINT"}]},
               {"schema": "raw", "name": "runs", "columns": []}],
    "table_catalog": [{"name": "runs", "kind": "table", "grain": "a run", "keys": "cell, arm, seed", "joins": None,
                       "description": "each run's lag"},
                      {"name": "structural", "kind": "table macro", "grain": None, "keys": "(exp_name)", "joins": None,
                       "description": "the cells every arm collapsed on every seed"}],
    "metric_catalog": [{"name": "lag_p99_ms", "tbl": "runs", "type": "DOUBLE", "unit": "ms",
                        "description": "the honest viewers' p99 lag"},
                       {"name": "lag_p99_ms", "tbl": "suite", "type": None, "unit": "ms", "better": "lower",
                        "kind": "ratio", "tie": 0.05, "guard": "lo", "source": "r.lag_p99_ms",
                        "description": "lag p99"}],
    "event_kinds": [{"kind": "join", "sim": "ppspp", "description": "a viewer joins", "value_meaning": None},
                    {"kind": "collapse", "sim": "derived", "description": "a collapse began",
                     "value_meaning": "its length, s"}],
    "experiments": [{"exp_id": "e1"}],
    "catalog_info": [{"key": "indexed_at", "value": "2026-11-10T10:00:00+00:00"}],
}


class FakeScoring:
    """simlab.scoring's names the tools call, recording their calls."""

    def __init__(self):
        self.calls = []

    def score(self, exp_a, arm_a, exp_b, arm_b, cells=None):
        self.calls.append(("score", exp_a, arm_a, exp_b, arm_b, cells))
        if exp_a == "nope":
            raise RuntimeError("no experiment nope")
        return NS(cells=[{"name": "calm"}], text=lambda: "A = e1:base   B = e1:new\n\ncalm, 1 seeds\n")

    def arm_scores(self, exp, base, arms=None, cells=None, rank="lag"):
        self.calls.append(("arm_scores", exp, base, arms, cells, rank))
        return [NS(text=lambda: "\ne1:new against e1:base: collapsed 0 (1)\n")]

    def structural(self, exp):
        self.calls.append(("structural", exp))
        return [{"cell": "storm", "arms": 2, "runs": 2}]

    def dynamics(self, exp):
        self.calls.append(("dynamics", exp))
        return [{"cell": "calm", "arm_label": "new", "lag_p99_startup_ms": 900.0, "lag_p99_steady_ms": float("nan")}]


def setUpModule():
    global TMP, DIR, POOL
    TMP = tempfile.TemporaryDirectory(prefix="ask-test-")
    DIR = Path(TMP.name)
    make_lake(DIR / "lake")
    POOL = engine.Pool(DIR / "lake", DIR / "spill", size=2, guard=engine.Guard("1GB", 2, "1GiB"))
    root = DIR / "views"
    views.save("by_cell", views.View(name="by_cell", title="By cell", description="a cell's runs",
                                      params=[views.Param("cell", "cell", "calm")],
                                      sql="SELECT arm, lag_p99_ms FROM runs WHERE cell = $cell ORDER BY arm"), root)


def tearDownModule():
    POOL.close()
    TMP.cleanup()


def asker(script, schema=SCHEMA, **kw):
    client = FakeClient(script)
    a = ask.Asker(POOL, lambda: schema, client=client, ledger=DIR / f"spend-{id(client)}.jsonl",
                  views_root=DIR / "views", scoring=kw.pop("scoring", FakeScoring()), **kw)
    return a, client


def run(a, question, **kw):
    events = []
    answer = a.ask(question, emit=events.append, **kw)
    return answer, events


def kinds(events):
    return [e["type"] for e in events if e["type"] not in ("thinking", "text", "usage")]


class Loop(unittest.TestCase):
    def test_end_turn_answers_in_one_request(self):
        a, client = asker([msg([thinking("look"), text("Two cells.")])])
        answer, events = run(a, "how many cells?")
        self.assertEqual((answer.stop_reason, answer.turns, answer.text), ("end_turn", 1, "Two cells."))
        self.assertEqual(kinds(events), ["start", "done"])
        self.assertEqual("".join(e["text"] for e in events if e["type"] == "text"), "Two cells.")
        self.assertEqual([e["text"] for e in events if e["type"] == "thinking"], ["look"])
        c = a._conversations[answer.conversation_id]
        self.assertEqual([m["role"] for m in c.messages], ["user", "assistant"])

    def test_tool_use_runs_every_call_and_returns_them_in_one_message(self):
        a, client = asker([
            msg([thinking("query"), call("t1", "run_sql", sql="SELECT count(*) AS n FROM nums", purpose="count"),
                 call("t2", "run_sql", sql="SELECT * FROM nope", purpose="bad")], "tool_use"),
            msg([text("200 rows.")])])
        answer, events = run(a, "how many numbers?")
        self.assertEqual((answer.stop_reason, answer.turns), ("end_turn", 2))
        second = client.requests[1]["messages"]
        self.assertEqual([m["role"] for m in second], ["user", "assistant", "user"])
        results = second[2]["content"]
        self.assertEqual([r["tool_use_id"] for r in results], ["t1", "t2"])
        self.assertEqual(json.loads(results[0]["content"])["rows"], [[200]])
        self.assertNotIn("is_error", results[0])
        self.assertTrue(results[1]["is_error"])
        self.assertRegex(results[1]["content"], r"nope")
        self.assertEqual(kinds(events).count("tool_result"), 2)

    def test_pause_turn_sends_again_without_running_anything(self):
        a, client = asker([msg([thinking("a while"), text("Still")], "pause_turn"), msg([text(" here.")])])
        answer, events = run(a, "go on")
        self.assertEqual((answer.stop_reason, answer.turns), ("end_turn", 2))
        second = client.requests[1]["messages"]
        self.assertEqual([m["role"] for m in second], ["user", "assistant"])
        self.assertNotIn("tool", kinds(events))

    def test_a_refusal_runs_nothing_and_its_turn_is_dropped(self):
        a, client = asker([msg([text("partial"), call("t1", "run_sql", sql="SELECT 1", purpose="x")], "refusal",
                               stop_details={"type": "refusal", "category": "cyber", "explanation": "no"})])
        answer, events = run(a, "refuse me")
        self.assertEqual(answer.stop_reason, "refusal")
        self.assertNotIn("tool", kinds(events))
        self.assertEqual([e for e in events if e["type"] == "refusal"],
                         [{"type": "refusal", "category": "cyber", "explanation": "no"}])
        c = a._conversations[answer.conversation_id]
        self.assertEqual([m["role"] for m in c.messages], ["user"])

    def test_max_tokens_with_a_pending_call_doesnt_run_it(self):
        a, client = asker([msg([thinking("long"), call("t1", "run_sql", sql="SELECT 1", purpose="x")], "max_tokens")])
        answer, events = run(a, "too long")
        self.assertEqual(answer.stop_reason, "max_tokens")
        self.assertNotIn("tool", kinds(events))
        self.assertIn("error", kinds(events))
        self.assertEqual(len(client.requests), 1)
        self.assertEqual([m["role"] for m in a._conversations[answer.conversation_id].messages], ["user"])

    def test_max_tokens_in_text_keeps_the_cut_answer(self):
        a, client = asker([msg([text("cut off")], "max_tokens")])
        answer, events = run(a, "long answer")
        self.assertEqual((answer.stop_reason, answer.text), ("max_tokens", "cut off"))
        self.assertEqual([m["role"] for m in a._conversations[answer.conversation_id].messages],
                         ["user", "assistant"])

    def test_an_sdk_error_ends_the_question_with_its_text(self):
        a, client = asker([status_error(anthropic.RateLimitError, 429)])
        answer, events = run(a, "anything")
        self.assertEqual(answer.stop_reason, "error")
        err = [e for e in events if e["type"] == "error"][0]["message"]
        self.assertRegex(err, r"rate limited \(429\): retry after 7 s")
        self.assertEqual(events[-1]["type"], "done")
        a2, _ = asker([anthropic.APIConnectionError(request=httpx2.Request("POST", "https://api.anthropic.com"))])
        self.assertRegex(run(a2, "x")[1][-2]["message"], r"couldn't reach the API")
        a3, _ = asker([status_error(anthropic.AuthenticationError, 401, "invalid x-api-key")])
        self.assertRegex(run(a3, "x")[1][-2]["message"], r"refused the credentials \(401\)")

    def test_a_cancel_closes_the_stream_and_keeps_the_history_valid(self):
        a, client = asker([(msg([text("one two three four"), call("t1", "run_sql", sql="SELECT 1", purpose="x")],
                                "tool_use"), 0)])
        answer, events = run(a, "stop me", cancel=client.cancel)
        self.assertEqual(answer.stop_reason, "cancelled")
        self.assertTrue(client.streams[0].closed)
        self.assertNotIn("tool", kinds(events))
        self.assertEqual([m["role"] for m in a._conversations[answer.conversation_id].messages], ["user"])

    def test_a_follow_up_appends_and_echoes_the_history_unchanged(self):
        first = [msg([thinking("t1"), call("q1", "run_sql", sql="SELECT 1 AS one", purpose="one")], "tool_use"),
                 msg([thinking("t2"), text("One.")])]
        a, client = asker(first + [msg([text("Two.")])])
        answer, _ = run(a, "one?")
        _, events = run(a, "and two?", conversation_id=answer.conversation_id)
        r = client.requests
        self.assertEqual(len(r), 3)
        for before, after in zip(r, r[1:]):
            self.assertEqual(after["messages"][:len(before["messages"])], before["messages"])
            self.assertEqual((after["system"], after["tools"]), (before["system"], before["tools"]))
        self.assertEqual(r[2]["messages"][1]["content"],
                         [thinking("t1"), call("q1", "run_sql", sql="SELECT 1 AS one", purpose="one")])
        self.assertEqual(r[2]["messages"][-1], {"role": "user", "content": "and two?"})
        self.assertEqual(events[0]["question"], 2)
        with self.assertRaises(KeyError):
            a.ask("x", conversation_id="nope")

    def test_a_conversation_keeps_its_prompt_when_the_catalog_changes(self):
        schema = copy.deepcopy(SCHEMA)
        a, client = asker([msg([text("a")]), msg([text("b")])], schema=schema)
        answer, _ = run(a, "first")
        schema["tables"].append({"schema": "main", "name": "zzz", "columns": []})
        run(a, "second", conversation_id=answer.conversation_id)
        self.assertEqual(client.requests[0]["system"], client.requests[1]["system"])
        self.assertNotIn("### zzz", client.requests[1]["system"][0]["text"])

    def test_a_fallback_drops_the_declined_models_thinking_and_calls(self):
        content = [thinking("declined"), text("Let me"), call("x1", "run_sql", sql="SELECT 2", purpose="declined"),
                   {"type": "fallback", "from": {"model": ask.MODEL}, "to": {"model": "claude-opus-4-8"},
                    "trigger": {"type": "refusal", "category": "cyber"}},
                   thinking("served"), call("x2", "run_sql", sql="SELECT 3 AS three", purpose="served")]
        its = [{"type": "message", "model": ask.MODEL, **usage(1000, 5)},
               {"type": "fallback_message", "model": "claude-opus-4-8", **usage(1000, 40)}]
        a, client = asker([msg(content, "tool_use", model="claude-opus-4-8", u=usage(1000, 40, iterations=its)),
                           msg([text("Three.")], model="claude-opus-4-8")])
        answer, events = run(a, "three?")
        echoed = client.requests[1]["messages"][1]["content"]
        self.assertEqual([b["type"] for b in echoed], ["text", "fallback", "thinking", "tool_use"])
        self.assertEqual(echoed[1], content[3])
        self.assertEqual([e["id"] for e in events if e["type"] == "tool"], ["x2"])
        self.assertEqual([r["tool_use_id"] for r in client.requests[1]["messages"][2]["content"]], ["x2"])
        fb = [e for e in events if e["type"] == "fallback"]
        self.assertEqual((fb[0]["from"], fb[0]["to"], fb[0]["category"]), (ask.MODEL, "claude-opus-4-8", "cyber"))


class Requests(unittest.TestCase):
    def test_every_request_carries_the_budget_fallbacks_effort_and_thinking(self):
        a, client = asker([msg([call("t", "run_sql", sql="SELECT 1", purpose="x")], "tool_use"),
                           msg([text("a")], "pause_turn"), msg([text("b")]), msg([text("c")])])
        answer, _ = run(a, "first")
        run(a, "second", conversation_id=answer.conversation_id, task_budget=50_000)
        self.assertEqual(len(client.requests), 4)
        for i, r in enumerate(client.requests):
            with self.subTest(request=i):
                self.assertEqual(r["model"], "claude-opus-5-5")
                self.assertEqual(r["betas"], ["server-side-fallback-2026-07-01", "task-budgets-2026-03-13"])
                self.assertEqual(r["fallbacks"], "default")
                self.assertEqual(r["thinking"], {"type": "adaptive", "display": "summarized"})
                total = 200_000 if i < 3 else 50_000
                self.assertEqual(r["output_config"],
                                 {"effort": "medium", "task_budget": {"type": "tokens", "total": total}})
                self.assertNotIn("remaining", r["output_config"]["task_budget"])
                self.assertEqual(r["tool_choice"], {"type": "auto"})
                self.assertEqual(r["cache_control"], {"type": "ephemeral"})
                self.assertEqual(r["system"][0]["cache_control"], {"type": "ephemeral", "ttl": "1h"})
                self.assertEqual(r["max_tokens"], 64_000)
                for t in r["tools"]:
                    self.assertTrue(t["strict"])
                    self.assertFalse(t["input_schema"]["additionalProperties"])
                    self.assertEqual(sorted(t["input_schema"]["required"]), sorted(t["input_schema"]["properties"]))

    def test_a_budget_under_the_apis_least_is_refused(self):
        a, client = asker([])
        for bad in (19_999, 0, ask.MAX_TASK_BUDGET + 1):
            with self.assertRaises(ValueError):
                a.ask("x", task_budget=bad)
        self.assertEqual(client.requests, [])

    def test_the_tools_are_the_ones_named(self):
        self.assertEqual([t["name"] for t in ask.TOOLS],
                         ["run_sql", "run_view", "chart", "score", "arm_scores", "structural", "dynamics",
                          "fetch_data", "propose_farm_run"])
        for t in ask.TOOLS:
            self.assertTrue(hasattr(ask.Tools, f"tool_{t['name']}"), t["name"])


class Prompt(unittest.TestCase):
    def test_the_prompt_is_byte_for_byte_the_same(self):
        saved = views.saved(DIR / "views")
        p = ask.system_prompt(SCHEMA, saved)
        self.assertEqual(p, ask.system_prompt(copy.deepcopy(SCHEMA), views.saved(DIR / "views")))
        shuffled = {k: list(reversed(v)) for k, v in SCHEMA.items()}
        self.assertEqual(p, ask.system_prompt(shuffled, list(reversed(saved))))

    def test_the_prompt_holds_the_glossary_catalog_and_views_and_no_volatile_parts(self):
        p = ask.system_prompt(SCHEMA, views.saved(DIR / "views"))
        self.assertTrue(p.startswith("# Questions about the sims' results"))
        self.assertNotIn("SPDX", p)
        self.assertRegex(p, r"\*\*startup\s+exclusion\*\*")
        for needle in ("Structural cells", "Fisher exact p", "1250 ms", "4250 ms", "The oracle", "### runs (table)",
                       "columns: cell VARCHAR, lag_p99_ms DOUBLE [ms]",
                       "- lag_p99_ms: the honest viewers' p99 lag", "`structural(exp_name)` (table macro)",
                       "- lag_p99_ms: better lower, kind ratio, tie 0.05, guard lo, unit ms; source `r.lag_p99_ms`",
                       "- join (ppspp): a viewer joins", "## by_cell: By cell", "params: cell (cell, default calm)",
                       "SELECT arm, lag_p99_ms FROM runs WHERE cell = $cell"):
            self.assertTrue(needle in p, needle)
        # Nothing that changes as the catalog grows: counts, times, experiments.
        self.assertNotIn("2026-11-10", p)
        self.assertNotIn("indexed_at", p)
        self.assertNotIn("raw.runs", p.split("## Tables and views")[1])

    def test_the_real_views_build_a_prompt(self):
        p = ask.system_prompt(SCHEMA, views.saved())
        for v in views.saved():
            self.assertIn(f"## {v.name}: ", p)


class ToolsTest(unittest.TestCase):
    def setUp(self):
        self.events = []
        self.scoring = FakeScoring()
        self.fetched = []
        self.tools = ask.Tools(POOL, DIR / "views", farm.Regenerations(DIR / "regen"), scoring=self.scoring,
                               fetch=self.fetch)

    def fetch(self, exp, cells=None, arms=None, tables=(), progress=None):
        self.fetched.append((exp, cells, arms, tables))
        progress(catalog.Progress("pull", "pulling 2 flushes", 0, 2))
        if exp == "gone":
            raise SystemExit("experiment gone has no jobs: experiments lists them")
        return catalog.Fetched(exp, s3_usd=0.0123, ledger_lines=2, pulled=["f1", "f2"])

    def run_tool(self, tool, /, **inp):
        text, is_error = self.tools.run(tool, inp, ask.Call("c1", tool, self.events.append))
        return (text if is_error else json.loads(text) if text.startswith("{") else text), is_error

    def test_run_sql_gives_columns_rows_and_the_row_count(self):
        out, err = self.run_tool("run_sql", sql="SELECT i, twice FROM nums ORDER BY i", purpose="numbers")
        self.assertFalse(err)
        self.assertEqual(out["columns"], [{"name": "i", "type": "BIGINT"}, {"name": "twice", "type": "BIGINT"}])
        self.assertEqual((out["row_count"], out["row_count_capped"], len(out["rows"])), (200, False, 50))
        self.assertEqual(out["rows"][3], [3, 6])
        sql = [e for e in self.events if e["type"] == "sql"]
        self.assertEqual((sql[0]["sql"], sql[0]["purpose"]), ("SELECT i, twice FROM nums ORDER BY i", "numbers"))
        self.assertEqual([e["result"]["row_count"] for e in self.events if e["type"] == "result"], [200])

    def test_a_failed_or_refused_statement_is_an_error_claude_can_read(self):
        for sql, pattern in [("SELECT * FROM nope", r"failed \(400\).*nope"),
                             ("CREATE TABLE x AS SELECT 1", r"\(403\)"),
                             ("SELECT 1; SELECT 2", r"\(400\): a query is one statement"), ("", r"sql:")]:
            with self.subTest(sql=sql):
                out, err = self.run_tool("run_sql", sql=sql, purpose="")
                self.assertTrue(err)
                self.assertRegex(out, pattern)

    def test_an_unknown_tool_or_input_is_an_error(self):
        self.assertEqual(self.tools.run("drop_tables", {}, ask.Call("c", "x", self.events.append)),
                         ("no tool 'drop_tables'", True))
        self.assertTrue(self.tools.run("run_sql", "SELECT 1", ask.Call("c", "x", self.events.append))[1])

    def test_a_long_result_is_cut_to_fit(self):
        out, err = self.run_tool("run_sql", sql="SELECT repeat('x', 2000) AS s FROM range(50)", purpose="")
        self.assertFalse(err)
        self.assertLess(len(json.dumps(out)), ask.RESULT_CHARS + 1)
        self.assertLess(out["rows_shown"], 50)
        self.assertEqual(out["row_count"], 50)

    def test_run_view_binds_params_and_defaults(self):
        out, err = self.run_tool("run_view", name="by_cell", params=[{"name": "cell", "value": "storm"}])
        self.assertFalse(err)
        self.assertEqual(out["rows"], [["base", 900.0], ["new", 950.0]])
        out, _ = self.run_tool("run_view", name="by_cell", params=[])
        self.assertEqual(out["rows"], [["base", 300.0], ["new", 240.0]])
        self.assertEqual([e["params"] for e in self.events if e["type"] == "sql"],
                         [{"cell": "storm"}, {"cell": "calm"}])
        out, err = self.run_tool("run_view", name="nope", params=[])
        self.assertTrue(err)
        self.assertRegex(out, r"no saved view 'nope': the views are by_cell")

    def test_a_chart_draws_its_statements_rows_with_a_checked_spec(self):
        spec = {"mark": "bar", "encoding": {"x": {"field": "cell", "type": "nominal"},
                                            "y": {"field": "lag_p99_ms", "type": "quantitative"},
                                            "color": {"field": "arm"}}}
        sql = "SELECT cell, arm, lag_p99_ms FROM runs"
        out, err = self.run_tool("chart", sql=sql, vega_lite=json.dumps(spec), title="lag")
        self.assertFalse(err, out)
        self.assertEqual((out["rows"], out["columns"]), (4, ["cell", "arm", "lag_p99_ms"]))
        chart = [e for e in self.events if e["type"] == "chart"][0]
        self.assertEqual((chart["spec"], chart["sql"], len(chart["result"]["rows"])), (spec, sql, 4))
        made = {"transform": [{"calculate": "datum.lag_p99_ms / 1000", "as": "lag_s"}, {"fold": ["arm"]}],
                "mark": "line", "encoding": {"x": {"field": "lag_s"}, "y": {"field": "value"}}}
        self.assertFalse(self.run_tool("chart", sql=sql, vega_lite=json.dumps(made), title="")[1])
        for bad, pattern in [({**spec, "data": {"values": [{"a": 1}]}}, r"`data` given"),
                             ({**spec, "data": {"url": "http://x"}}, r"`data` given.*`url` given"),
                             ({"layer": [{"mark": "point", "data": {"url": "/etc/passwd"}}]}, r"`url` given"),
                             ({"mark": "bar", "encoding": {"x": {"field": "nope"}}}, r"field 'nope' is neither"),
                             ({"encoding": {}}, r"no view"), ([1, 2], r"isn't a JSON object")]:
            with self.subTest(spec=bad):
                out, err = self.run_tool("chart", sql=sql, vega_lite=json.dumps(bad), title="")
                self.assertTrue(err)
                self.assertRegex(out, pattern)
        out, err = self.run_tool("chart", sql=sql, vega_lite="{not json", title="")
        self.assertTrue(err)
        self.assertRegex(out, r"isn't JSON")
        self.assertEqual(len([e for e in self.events if e["type"] == "chart"]), 2)

    def test_the_scoring_tools_call_simlab_scoring(self):
        out, err = self.run_tool("score", exp_a="e1", arm_a="base", exp_b="e1", arm_b="new", cells=[])
        self.assertFalse(err)
        self.assertRegex(out, r"A = e1:base   B = e1:new")
        out, err = self.run_tool("arm_scores", exp="e1", base="base", arms=["new"], cells=[], rank="overhead")
        self.assertRegex(out, r"collapsed 0 \(1\)")
        out, err = self.run_tool("structural", exp="e1")
        self.assertEqual(out["rows"], [{"cell": "storm", "arms": 2, "runs": 2}])
        out, err = self.run_tool("dynamics", exp="e1")
        self.assertEqual(out["rows"][0]["lag_p99_steady_ms"], "nan")
        self.assertEqual(self.scoring.calls, [("score", "e1", "base", "e1", "new", None),
                                              ("arm_scores", "e1", "base", ["new"], None, "overhead"),
                                              ("structural", "e1"), ("dynamics", "e1")])
        out, err = self.run_tool("score", exp_a="nope", arm_a="a", exp_b="e1", arm_b="b", cells=[])
        self.assertTrue(err)
        self.assertRegex(out, r"RuntimeError: no experiment nope")
        out, err = self.run_tool("structural", exp="--drop")
        self.assertTrue(err)
        self.assertRegex(out, r"not a name")

    def test_fetch_data_fetches_and_reports_what_s3_charged(self):
        out, err = self.run_tool("fetch_data", exp_id="e1", cells=["calm"], arms=[], tables=["reads"])
        self.assertFalse(err)
        self.assertEqual((out["s3_usd"], out["pulled"]), (0.0123, 2))
        self.assertEqual(self.fetched, [("e1", ["calm"], None, ("reads",))])
        self.assertEqual([e["line"] for e in self.events if e["type"] == "progress"], ["pulling 2 flushes"])
        out, err = self.run_tool("fetch_data", exp_id="gone", cells=[], arms=[], tables=[])
        self.assertTrue(err)
        self.assertRegex(out, r"has no jobs")
        out, err = self.run_tool("fetch_data", exp_id="e1", cells=["a b"], arms=[], tables=[])
        self.assertTrue(err)

    def test_propose_farm_run_quotes_and_never_runs(self):
        got = []

        def regenerate(pool, exp, cells, arms, seeds, regenerations=None, suite=None):
            got.append((exp, cells, arms, seeds))
            return {"id": "q1", "state": "quoted", "quote": "16 jobs, ~$0.05", "warnings": [],
                    "estimate": {"jobs": 16, "cost": 0.05, "wall_s": 600, "s3_cost": 0.001}}

        def approve(*a, **k):
            raise AssertionError("a proposal approved a farm run")

        saved = (ask.regenerate.regenerate, farm.approve, farm.Regenerations.approve)
        ask.regenerate.regenerate, farm.approve, farm.Regenerations.approve = regenerate, approve, approve
        try:
            out, err = self.run_tool("propose_farm_run", exp_id="e1", cells=["calm"], arms=["new"], seeds=[1, 2],
                                     why="more seeds")
        finally:
            ask.regenerate.regenerate, farm.approve, farm.Regenerations.approve = saved
        self.assertFalse(err)
        self.assertEqual((out["quote_id"], out["state"], out["cost_usd"], out["jobs"]), ("q1", "quoted", 0.05, 16))
        self.assertRegex(out["approval"], r"only the developer's Approve")
        self.assertEqual(got, [("e1", ["calm"], ["new"], [1, 2])])
        proposal = [e for e in self.events if e["type"] == "proposal"][0]
        self.assertEqual((proposal["why"], proposal["regeneration"]["id"]), ("more seeds", "q1"))
        out, err = self.run_tool("propose_farm_run", exp_id="e1", cells=[], arms=[], seeds=["1"], why="")
        self.assertTrue(err)

    def test_nothing_in_ask_approves(self):
        src = (HACK / "simlab" / "ask.py").read_text()
        self.assertNotRegex(src, r"\bapprove\s*\(")
        self.assertNotIn("subprocess", src)


class Spend(unittest.TestCase):
    def test_a_turn_is_priced_at_its_models_rates(self):
        m = BetaMessage.model_validate(msg([text("x")], u=usage(input=1_000, output=2_000, read=100_000,
                                                                 write5=3_000, write1=40_000)))
        [it] = ask.priced(m)
        self.assertEqual((it.model, it.billed, it.priced_as), ("claude-opus-5-5", True, "claude-opus-5-5"))
        # 1000*4 + 2000*20 + 100000*0.2 + 3000*5 + 40000*8, per million.
        self.assertAlmostEqual(it.usd, (4_000 + 40_000 + 20_000 + 15_000 + 320_000) / 1e6)

    def test_each_fallback_hop_is_priced_and_a_pre_output_decline_billed_by_its_category(self):
        def hops(category, declined_output):
            its = [{"type": "message", "model": ask.MODEL, **usage(1_000, declined_output)},
                   {"type": "fallback_message", "model": "claude-opus-4-8", **usage(2_000, 100, read=10_000)}]
            content = [{"type": "fallback", "from": {"model": ask.MODEL}, "to": {"model": "claude-opus-4-8"},
                        "trigger": {"type": "refusal", "category": category}}, text("ok")]
            return ask.priced(BetaMessage.model_validate(msg(content, model="claude-opus-4-8",
                                                             u=usage(2_000, 100, iterations=its))))
        served = (2_000 * 5 + 100 * 25 + 10_000 * 0.5) / 1e6
        cyber = hops("cyber", 0)
        self.assertEqual([(i.type, i.model, i.billed) for i in cyber],
                         [("message", ask.MODEL, False), ("fallback_message", "claude-opus-4-8", True)])
        self.assertEqual(cyber[0].usd, 0.0)
        self.assertAlmostEqual(cyber[1].usd, served)
        bio = hops("bio", 0)
        self.assertTrue(bio[0].billed)
        self.assertAlmostEqual(bio[0].usd, (1_000 * 4) / 1e6)
        midstream = hops("cyber", 30)
        self.assertTrue(midstream[0].billed)
        self.assertAlmostEqual(midstream[0].usd, (1_000 * 4 + 30 * 20) / 1e6)

    def test_a_refusal_before_output_is_billed_only_in_its_billed_categories(self):
        for category, billed in [(None, False), ("cyber", False), ("general_harms", False), ("bio", True),
                                 ("frontier_llm", True), ("reasoning_extraction", True)]:
            with self.subTest(category=category):
                m = BetaMessage.model_validate(msg([], "refusal", u=usage(500, 0),
                                                   stop_details={"type": "refusal", "category": category}))
                [it] = ask.priced(m)
                self.assertEqual(it.billed, billed)
                self.assertAlmostEqual(it.usd, 500 * 4 / 1e6 if billed else 0.0)

    def test_an_unknown_model_is_priced_at_the_ceiling(self):
        m = BetaMessage.model_validate(msg([text("x")], model="claude-opus-9", u=usage(1_000_000, 1_000_000)))
        [it] = ask.priced(m)
        self.assertEqual(it.priced_as, "ceiling")
        self.assertAlmostEqual(it.usd, 5.0 + 25.0)
        self.assertEqual(ask.CEILING, ask.Price(5.0, 25.0, 0.5, 6.25, 10.0))

    def test_writes_without_their_split_are_priced_at_the_hours_rate(self):
        u = {"input_tokens": 0, "output_tokens": 0, "cache_read_input_tokens": 0, "cache_creation_input_tokens": 1_000}
        [it] = ask.priced(BetaMessage.model_validate(msg([text("x")], u=u)))
        self.assertEqual((it.cache_write_5m, it.cache_write_1h), (0, 1_000))
        self.assertAlmostEqual(it.usd, 1_000 * 8 / 1e6)

    def test_the_ledger_and_spend_add_up(self):
        its = [{"type": "message", "model": ask.MODEL, **usage(1_000, 0)},
               {"type": "fallback_message", "model": "claude-opus-4-8", **usage(1_000, 1_000)}]
        fb = [{"type": "fallback", "from": {"model": ask.MODEL}, "to": {"model": "claude-opus-4-8"},
               "trigger": {"type": "refusal", "category": "cyber"}}]
        a, client = asker([msg([call("t", "run_sql", sql="SELECT 1", purpose="")], "tool_use",
                               u=usage(10_000, 1_000, write1=20_000)),
                           msg(fb + [text("done")], model="claude-opus-4-8", u=usage(1_000, 1_000, iterations=its))])
        answer, events = run(a, "what does it cost?")
        first = (10_000 * 4 + 1_000 * 20 + 20_000 * 8) / 1e6
        second = (1_000 * 5 + 1_000 * 25) / 1e6
        self.assertAlmostEqual(answer.usd, round(first + second, 6))
        lines = [json.loads(line) for line in a.ledger.path.read_text().splitlines()]
        self.assertEqual([(r["turn"], r["type"], r["model"], r["billed"]) for r in lines],
                         [(1, "message", ask.MODEL, True), (2, "message", ask.MODEL, False),
                          (2, "fallback_message", "claude-opus-4-8", True)])
        self.assertEqual({r["conversation"] for r in lines}, {answer.conversation_id})
        self.assertAlmostEqual(sum(r["usd"] for r in lines), first + second)
        usages = [e for e in events if e["type"] == "usage"]
        self.assertEqual([round(u["usd"], 6) for u in usages], [round(first, 6), round(second, 6)])
        self.assertEqual(usages[-1]["budget"]["total"], 200_000)
        # What Claude wrote, and the second turn's new input past the first's input and output.
        self.assertEqual(usages[-1]["budget"]["used"], 1_000 + 1_000 + max(0, 1_000 - 30_000 - 1_000))
        done = events[-1]
        self.assertEqual((done["type"], done["turns"]), ("done", 2))
        self.assertAlmostEqual(done["usd"], round(first + second, 6))
        s = ask.spend(a.ledger.path)
        self.assertEqual(len(s["days"]), 1)
        self.assertEqual((s["days"][0]["questions"], s["days"][0]["requests"]), (1, 2))
        self.assertAlmostEqual(s["total_usd"], round(first + second, 6))
        self.assertEqual(s["questions"][0]["text"], "what does it cost?")
        self.assertEqual(ask.spend(DIR / "none.jsonl")["total_usd"], 0)
        self.assertEqual(ask.spend(a.ledger.path, day="1999-01-01")["questions"], [])


class Eval(unittest.TestCase):
    def test_an_answers_numbers_leave_out_the_digits_of_names(self):
        from simlab import ask_eval
        self.assertEqual(ask_eval.numbers("star-15-10-end-abrupt: 10 of 32 (p = 0.24), x1.016, 1,250 ms, s7ae7e"),
                         [10, 32, 0.24, 1.016, 1250])
        self.assertEqual(ask_eval.numbers("seeds 2001-2032; 439ms"), [2001, 2032, 439])

    def test_checks_pass_on_texts_numbers_and_phrases(self):
        from simlab import ask_eval
        answer = "The cells are seed-starved and seed-restart; main's p99 is 439 ms, which is not significant."
        for check, want in [({"text": "Seed-Starved"}, True), ({"text": "star-31-5"}, False),
                            ({"number": 439.5, "tol": 0.01}, True), ({"number": 450, "tol": 0.01}, False),
                            ({"number": 5, "tol": 0}, False), ({"any": ["not significant", "noise"]}, True),
                            ({"none": ["significant"]}, False)]:
            self.assertEqual(ask_eval.passes(check, answer), want, check)

    def test_every_question_has_reference_sql_and_checks(self):
        from simlab import ask_eval
        qs = ask_eval.load()
        self.assertGreaterEqual(len(qs), 12)
        self.assertEqual(len({q["id"] for q in qs}), len(qs))
        for q in qs:
            self.assertTrue(q["reference_sql"] and q["checks"] and q["answer"], q["id"])
            for c in q["checks"]:
                ask_eval.label(c)
                ask_eval.passes(c, q["answer"])
            # The reference answer passes its own checks.
            self.assertTrue(all(ask_eval.passes(c, q["answer"]) for c in q["checks"]), q["id"])


if __name__ == "__main__":
    unittest.main()

# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""Questions about the sims' results in plain language, answered by
Claude writing and running SQL over the catalog.

  ask(question, conversation_id=None, emit=None) -> Answer
                 a manual tool loop of streamed turns (at most MAX_TURNS,
                 then one last turn without tools), each event passed to
                 `emit` as a dict (the console's NDJSON stream: start,
                 thinking, text, tool, sql, chart, progress, proposal,
                 fallback, usage, done, refusal, error, ping)
  spend(day=None) -> dict
                 what the questions cost, from target/console/spend.jsonl

Its tools are simlab's functions: run_sql (engine, guarded), run_view
(views.run), scorecard (scoring.score), chart, fetch_data
(catalog.fetch) and propose_farm_run (suite.write_matrices, farm.quote:
only the console's Approve runs it).
"""

import dataclasses

MAX_TURNS = 24


@dataclasses.dataclass
class Answer:
    conversation_id: str
    stop_reason: str
    usd: float = 0.0
    turns: int = 0


def ask(question, conversation_id=None, emit=None):
    raise NotImplementedError("simlab.ask.ask: increment I10")


def spend(day=None):
    raise NotImplementedError("simlab.ask.spend: increment I10")

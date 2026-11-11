# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simdb.py's selections and queries over the cache, as results, and the
text simdb.py and analyze.py print of them.

A Selection picks runs as simdb.py's options do (--src, --cell, --arm,
--seed, --since, --until, --bin). `views` makes the views over it
(canonical sources, farm reuse, suite relabels, the derived tables); each
query returns an engine.Result (or its records), and its `*_text`
renderer prints what simdb.py printed, byte for byte.

  sources(patterns) -> [src]
  reused(srcs, stored) -> (sql | None, [src])
  source_ranks(srcs) -> {src: rank}
  views(sel, derived=True, fetch=True) -> Views
  sql(sel, query, reads=False, text=False) -> [engine.Result]
  pooled(sel, by="none") -> engine.Result
  aligned(sel, event, field="lag_p99", per="run", node=None, detail=None,
          pool=False, before=10, after=60, nth=1) -> engine.Result
  events(sel) -> engine.Result
  osc(sel) -> engine.Result
  rows(sel, fields=None) -> engine.Result
  forecast(sel) -> engine.Result
  lake(query, text=False) -> [engine.Result]
  pooled_text(result, by), aligned_text(result, ...), events_text(result),
  osc_text(result), rows_text(result, fields), forecast_text(result)
"""

import dataclasses


@dataclasses.dataclass
class Selection:
    src: list = None
    cell: list = None
    arm: list = None
    seed: list = None
    since: float = None
    until: float = None
    bin: int = 1000
    pull_mb: float = None


@dataclasses.dataclass
class Views:
    """The SQL that makes a selection's views, and the tables it has."""
    sql: str
    have: set = dataclasses.field(default_factory=set)


def _todo(name):
    raise NotImplementedError(f"simlab.query.{name}: increment I4")


def sources(patterns):
    _todo("sources")


def reused(srcs, stored):
    _todo("reused")


def source_ranks(srcs):
    _todo("source_ranks")


def views(sel, derived=True, fetch=True):
    _todo("views")


def sql(sel, query, reads=False, text=False):
    _todo("sql")


def pooled(sel, by="none"):
    _todo("pooled")


def aligned(sel, event, field="lag_p99", per="run", node=None, detail=None, pool=False, before=10, after=60, nth=1):
    _todo("aligned")


def events(sel):
    _todo("events")


def osc(sel):
    _todo("osc")


def rows(sel, fields=None):
    _todo("rows")


def forecast(sel):
    _todo("forecast")


def lake(query, text=False):
    _todo("lake")

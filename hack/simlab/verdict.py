# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The verdicts on the sims' runs, and the derived tables made from them.

analyze.py's verdict block moves here byte-identical (its constants,
lags, presence, startup, measures, verdict, collapsed, phases, episodes,
events, run_startup, run_metrics, feasible and VERDICT_VERSION, a hash of
their source and constants, which the move keeps), and analyze.py
re-exports every name. They are stdlib alone: analyze.py imports this
module under the plain python3.

  series_of(meta, seconds, viewer_seconds, stays=None) -> dict | None
                 simdb.series_of: a run's series from its tables
  load_series(pairs) -> {(src, k): (meta, series)}
                 simdb.load_series, over the cache's sources
  derive(sources, workers=None, progress=None) -> Derived
                 the derived tables (verdicts, verdict_seconds, episodes,
                 derived_events) of each source lacking them, judged in a
                 forkserver pool (concurrent.futures over
                 multiprocessing's "forkserver" context, the pure verdict
                 code preloaded, never duckdb), each worker with one
                 DuckDB connection of its own, recycled after 50 sources;
                 a source that fails comes back in `failed`, its verdicts
                 left for the next index
"""

import dataclasses


@dataclasses.dataclass
class Derived:
    """What a derive made: the sources it judged, and those it couldn't,
    {src: error}."""
    judged: list = dataclasses.field(default_factory=list)
    failed: dict = dataclasses.field(default_factory=dict)


def series_of(meta, seconds, viewer_seconds, stays=None):
    raise NotImplementedError("simlab.verdict.series_of: increment I2")


def load_series(pairs):
    raise NotImplementedError("simlab.verdict.load_series: increment I3")


def derive(sources, workers=None, progress=None):
    raise NotImplementedError("simlab.verdict.derive: increment I3")

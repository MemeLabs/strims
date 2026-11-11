# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The sims' results catalog (a DuckLake at simlab.lake_root()): its index,
fetches from S3, and what the console's schema shows.

hack/sim-db/catalog.py and simdb.py's index, fetch and lake commands
call these; the console's server runs index and fetch in a forkserver
child, reading their Progress events from a queue.

  index(workers=None, progress=None, wait=True) -> Indexed
                 brings the catalog up to date with the cache, the farm's
                 and the suite's indexes, costs, the S3 ledger and git,
                 under lake/index.lock (one writer: engine.writer)
  fetch(exp, cells=None, arms=None, tables=(), workers=None, limit_mb=None,
        progress=None) -> Fetched
                 an experiment's flushes copied from S3 into the cache
                 (its sample tables too, as `tables` names them), then
                 indexed; what S3 charged
  availability(exp=None) -> engine.Result
                 each job's tables: local, on S3 or expired, and their
                 bytes
  state() -> dict
                 the catalog's catalog_info, and whether it needs an index
  compact(keep_minutes=15)
                 merges the written tables' small files
  null_stats() -> engine.Result
                 each column's NULL share, by table
  schema(pool) -> dict
                 what GET /console/schema answers, assembled once for the
                 server and for ask's prompt: {tables, table_catalog,
                 metric_catalog, event_kinds, experiments, catalog_info};
                 the server adds the saved views (views.saved) at each
                 request

Progress events are {kind, message, done, total}: kind one of "scan",
"derive", "write", "pull", "index", "done".
"""

import dataclasses


@dataclasses.dataclass
class Progress:
    kind: str
    message: str
    done: int = 0
    total: int = 0


@dataclasses.dataclass
class Indexed:
    """What an index did: whether it changed the catalog, the sources it
    (re)read, and those whose verdicts failed (they wait for the next
    index)."""
    changed: bool = False
    sources: list = dataclasses.field(default_factory=list)
    failed: dict = dataclasses.field(default_factory=dict)
    elapsed_s: float = 0.0


@dataclasses.dataclass
class Fetched:
    """What a fetch did: the flushes it pulled, those gone from S3 (to
    regenerate), and what S3 charged (`ledger_lines` lines of
    target/sim-farm/s3.jsonl, priced)."""
    exp: str
    s3_usd: float = 0.0
    ledger_lines: int = 0
    pulled: list = dataclasses.field(default_factory=list)
    gone: list = dataclasses.field(default_factory=list)


def index(workers=None, progress=None, wait=True):
    raise NotImplementedError("simlab.catalog.index: increment I3")


def fetch(exp, cells=None, arms=None, tables=(), workers=None, limit_mb=None, progress=None):
    raise NotImplementedError("simlab.catalog.fetch: increment I3")


def availability(exp=None):
    raise NotImplementedError("simlab.catalog.availability: increment I3")


def state():
    raise NotImplementedError("simlab.catalog.state: increment I3")


def compact(keep_minutes=15):
    raise NotImplementedError("simlab.catalog.compact: increment I3")


def null_stats():
    raise NotImplementedError("simlab.catalog.null_stats: increment I3")


def schema(pool):
    raise NotImplementedError("simlab.catalog.schema: increment I3")

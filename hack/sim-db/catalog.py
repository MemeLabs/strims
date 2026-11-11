#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The sims' results catalog: every sim result the machine knows of, in one
DuckLake, so that any question is one SQL statement over it.

  hack/sim-db/simdb.py index [--workers N] [--quiet]
  hack/sim-db/simdb.py fetch --exp E [--cells C,...] [--arms A,...] [--tables reads,forecast]
  hack/sim-db/simdb.py lake QUERY [--json]

`index` brings the catalog up to date with the cache (simdb.py's: the
suite's and the farm's flushes), the suite's and the farm's indexes, the
farm's costs and S3 ledger, and git, doing only what changed since it
last ran, and makes the verdicts of every source that lacks them,
`--workers` at a time (0: in its own process). `fetch` copies what S3
still holds of an experiment's jobs into the cache (with `--tables` the
bulky sample tables of the jobs picked by `--cells` and `--arms`),
indexes them, and prints what S3 charged for it, from the farm's S3
ledger. `lake` runs a query over the catalog, read-only.

The catalog is simlab.catalog's (hack/simlab/catalog.py), whose docs say
what it holds, table by table; these are its commands, which print what
an index and a fetch do on stderr. This module's other names are
simlab.catalog's, for the scripts that load it.
"""

import argparse
import json
import sys
from pathlib import Path

HACK = Path(__file__).resolve().parents[1]
if str(HACK) not in sys.path:
    sys.path.insert(0, str(HACK))

from simlab import catalog  # noqa: E402


def printed(quiet=False):
    """A progress callback printing each event's line on stderr, or none."""
    return None if quiet else (lambda p: print(p.message, file=sys.stderr))


def index(workers=None, quiet=False, wait=True):
    """catalog.index, its progress on stderr unless `quiet`."""
    return catalog.index(workers, printed(quiet), wait)


def cmd_index(a):
    index(a.workers, quiet=a.quiet)


def cmd_fetch(a):
    split = lambda s: [x.strip() for x in (s or "").split(",") if x.strip()]  # noqa: E731
    got = catalog.fetch(a.exp, split(a.cells), split(a.arms), split(a.tables), a.workers, a.pull_mb, printed())
    print(f"{a.exp}: fetched; S3 charged ${got.s3_usd:.4f} ({got.ledger_lines} ledger lines)")


def cmd_lake(a):
    if not catalog.catalog_path().exists():
        sys.exit(f"no catalog at {catalog.catalog_path()}: run simdb.py index")
    out = catalog.lake(a.query + ("" if a.query.rstrip().endswith(";") else ";"), out="json" if a.json else "text")
    print(json.dumps(out) if a.json else out, end="" if not a.json else "\n")


def add_commands(sub):
    """simdb.py's index, fetch and lake commands."""
    raw = argparse.RawDescriptionHelpFormatter
    i = sub.add_parser("index", help="bring the catalog up to date (catalog.py)", description=__doc__,
                       formatter_class=raw)
    i.add_argument("--workers", type=int,
                   help="sources derived at once (default: half the cores, at most 8; 0: in this process)")
    i.add_argument("--quiet", action="store_true")
    f = sub.add_parser("fetch", help="copy an experiment's data from S3 into the cache, and index it")
    f.add_argument("--exp", required=True, help="an experiment's exp_id (a suite farm run's name)")
    f.add_argument("--cells", help="only these cells, comma-separated")
    f.add_argument("--arms", help="only these arms (arm_label), comma-separated")
    f.add_argument("--tables", help="sample tables too, comma-separated: reads, forecast, arrivals")
    f.add_argument("--workers", type=int)
    f.add_argument("--pull-mb", type=float, default=None, help="the most of the sample tables it pulls, MB")
    lk = sub.add_parser("lake", help="a query over the catalog, read-only", description=__doc__, formatter_class=raw)
    lk.add_argument("query")
    lk.add_argument("--json", action="store_true")
    return {"index": cmd_index, "fetch": cmd_fetch, "lake": cmd_lake}


def __getattr__(name):
    """simlab.catalog's names (lake, scoring_sql, compact, ...)."""
    return getattr(catalog, name)


if __name__ == "__main__":
    from simlab import bootstrap

    bootstrap.engine(__file__)
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    cmds = add_commands(ap.add_subparsers(dest="cmd", required=True))
    args = ap.parse_args()
    cmds[args.cmd](args)

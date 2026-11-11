#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The sims' results catalog: every sim result the machine knows of, in one
DuckLake, so that any question is one SQL statement over it.

  hack/sim-db/simdb.py describe [TOPIC]
  hack/sim-db/simdb.py lake QUERY [--param NAME=VALUE ...] [--json]
  hack/sim-db/simdb.py view [NAME] [--param NAME=VALUE ...] [--json] [--max-rows N]
  hack/sim-db/simdb.py index [--workers N] [--quiet]
  hack/sim-db/simdb.py fetch --exp E [--cells C,...] [--arms A,...] [--tables reads,forecast]

`describe` says what the catalog holds, a topic at a time
(simlab.describe): with no topic, the topics and every table, view,
macro and saved view; `glossary` the sims' terms (cells, arms, the
verdict, collapses, phases, playout delays, structural cells, how arms
compare); `tables` a line a table; a table's name its columns, types,
units and notes; `macros`, `metrics` and `events` the macros (suite.py's
scoring among them), the scored metrics and the event kinds; `views`
the saved views; a view's name its params and SQL.

`lake` runs a query over the whole catalog, read-only, its macros
loaded: a script of any number of statements, or with `--param` one
SELECT through the console's guarded readers (engine.Pool) whose
`$name`s the params bind, each value JSON's where it parses as JSON (7,
true, [1, 2], "007" for a string of digits) and the text else; a query
with `$name`s goes in single quotes, or the shell takes them for its
own:

  simdb.py lake 'SELECT * FROM structural($exp)' --param exp=sa0755

`view` lists the console's saved views (simlab.views) or, given a name,
runs one: each `--param` a string, as the console's pickers give it (a
seed's or number's its number), a param not given its default; at most
`--max-rows` rows. Every query prints DuckDB's table, or with `--json`
its rows as objects, integers and decimals as numbers.

`index` brings the catalog up to date with the cache (simdb.py's: the
suite's and the farm's flushes), the suite's and the farm's indexes, the
farm's costs and S3 ledger, and git, doing only what changed since it
last ran, and makes the verdicts of every source that lacks them,
`--workers` at a time (0: in its own process): local runs reach the
catalog with the next index. `fetch` copies what S3 still holds of an
experiment's jobs into the cache (with `--tables` the bulky sample
tables of the jobs picked by `--cells` and `--arms`), indexes them, and
prints what S3 charged for it, from the farm's S3 ledger.

The catalog is simlab.catalog's (hack/simlab/catalog.py), whose docs say
what it holds, table by table; these are its commands, which print what
an index and a fetch do on stderr. This module's other names are
simlab.catalog's, for the scripts that load it.
"""

import argparse
import json
import re
import sys
from pathlib import Path

HACK = Path(__file__).resolve().parents[1]
if str(HACK) not in sys.path:
    sys.path.insert(0, str(HACK))

from simlab import catalog, describe, engine, views  # noqa: E402

# A --param's name: a statement's `$name`.
PARAM = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
# The CLI's readers spill under the cache's tmp/<READERS>/.
READERS = "cli"


def printed(quiet=False):
    """A progress callback printing each event's line on stderr, or none."""
    return None if quiet else (lambda p: print(p.message, file=sys.stderr))


def index(workers=None, quiet=False, wait=True):
    """catalog.index, its progress on stderr unless `quiet`."""
    return catalog.index(workers, printed(quiet), wait)


def need_catalog():
    if not catalog.catalog_path().exists():
        sys.exit(f"no catalog at {catalog.catalog_path()}: run simdb.py index")


def param(text):
    """A --param's (name, text)."""
    name, eq, value = text.partition("=")
    if not eq or not PARAM.fullmatch(name):
        raise argparse.ArgumentTypeError(f"{text!r} is no NAME=VALUE")
    return name, value


def typed(value):
    """A lake --param's value: JSON's where it parses as JSON, else the text."""
    try:
        return json.loads(value)
    except ValueError:
        return value


def printed_result(r, as_json):
    print(json.dumps(r.records()) if as_json else r.table(), end="\n" if as_json else "")


def cmd_index(a):
    index(a.workers, quiet=a.quiet)


def cmd_fetch(a):
    split = lambda s: [x.strip() for x in (s or "").split(",") if x.strip()]  # noqa: E731
    got = catalog.fetch(a.exp, split(a.cells), split(a.arms), split(a.tables), a.workers, a.pull_mb, printed())
    print(f"{a.exp}: fetched; S3 charged ${got.s3_usd:.4f} ({got.ledger_lines} ledger lines)")


def cmd_lake(a):
    need_catalog()
    if a.param:
        pool = catalog.pool(READERS)
        try:
            r = pool.query(a.query, {k: typed(v) for k, v in a.param}, max_rows=None, timeout_s=None,
                           text=not a.json)
        except engine.QueryError as e:
            sys.exit(f"lake: {e}")
        finally:
            pool.close()
        printed_result(r, a.json)
        return
    out = catalog.lake(a.query + ("" if a.query.rstrip().endswith(";") else ";"), out="json" if a.json else "text")
    print(json.dumps(out) if a.json else out, end="" if not a.json else "\n")


def cmd_view(a):
    if a.name is None:
        saved = views.saved()
        if a.json:
            print(json.dumps([{k: v.json()[k] for k in ("name", "title", "description", "params")} for v in saved]))
        else:
            print(engine.render_table(["name", "title", "params"],
                                      [[v.name, v.title, describe.params_text(v.params)] for v in saved]), end="")
        return
    need_catalog()
    pool = catalog.pool(READERS)
    try:
        r = views.run(pool, a.name, dict(a.param or []), max_rows=a.max_rows, timeout_s=None, text=not a.json)
    except KeyError:
        sys.exit(f"view: no view {a.name!r}: {', '.join(v.name for v in views.saved())}")
    except engine.QueryError as e:
        v = views.get(a.name)
        sys.exit(f"view {a.name}: {e}" + (f" (its params: {describe.params_text(v.params)})" if v.params else ""))
    finally:
        pool.close()
    printed_result(r, a.json)
    if r.truncated:
        print(f"view {a.name}: its first {a.max_rows:,} rows; --max-rows for more", file=sys.stderr)


def cmd_describe(a):
    need_catalog()
    pool = catalog.pool(READERS)
    try:
        print(describe.describe(a.topic, pool), end="")
    except ValueError as e:
        sys.exit(f"describe: {e}")
    finally:
        pool.close()


def add_commands(sub):
    """simdb.py's describe, lake, view, index and fetch commands."""
    raw = argparse.RawDescriptionHelpFormatter
    d = sub.add_parser("describe", help="what the catalog holds, by topic: start here (catalog.py)",
                       description=__doc__, formatter_class=raw)
    d.add_argument("topic", nargs="?", help="glossary, tables, macros, metrics, events, views, or a table's, "
                                            "macro's or saved view's name (none: the topics and names)")
    lk = sub.add_parser("lake", help="a query over the whole catalog, read-only, its macros loaded (catalog.py)",
                        description=__doc__, formatter_class=raw)
    lk.add_argument("query")
    lk.add_argument("--param", action="append", type=param, metavar="NAME=VALUE",
                    help="bind $NAME: VALUE as JSON where it parses (7, true, [1,2], '\"007\"'), else as text; "
                         "QUERY is then one SELECT, in single quotes (the shell expands $NAME in double ones)")
    lk.add_argument("--json", action="store_true")
    v = sub.add_parser("view", help="the console's saved views: list them, or run one (catalog.py)",
                       description=__doc__, formatter_class=raw)
    v.add_argument("name", nargs="?", help="the view to run (none: list them)")
    v.add_argument("--param", action="append", type=param, metavar="NAME=VALUE",
                   help="the view's param NAME, as the console's pickers give it (a param not given takes its "
                        "default)")
    v.add_argument("--json", action="store_true")
    v.add_argument("--max-rows", type=int, default=10_000, help="the most rows it prints (default 10,000)")
    i = sub.add_parser("index", help="bring the catalog up to date (catalog.py)", description=__doc__,
                       formatter_class=raw)
    i.add_argument("--workers", type=int,
                   help="sources derived at once (default: half the cores, at most 8; 0: in this process)")
    i.add_argument("--quiet", action="store_true")
    f = sub.add_parser("fetch", help="copy an experiment's data from S3 into the cache, and index it")
    f.add_argument("--exp", required=True, help="an experiment's exp_id (a suite farm run's name)")
    f.add_argument("--cells", help="only these cells, comma-separated")
    f.add_argument("--arms", help="only these arms (arm_label), comma-separated")
    f.add_argument("--tables", help="sample tables too, comma-separated: reads, forecast, arrivals, asks")
    f.add_argument("--workers", type=int)
    f.add_argument("--pull-mb", type=float, default=None, help="the most of the sample tables it pulls, MB")
    return {"describe": cmd_describe, "lake": cmd_lake, "view": cmd_view, "index": cmd_index, "fetch": cmd_fetch}


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

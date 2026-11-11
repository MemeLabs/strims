# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The sims' results, in one library: their store, verdicts, catalog,
queries, scoring, saved views, farm quotes and plain-language questions,
behind the scripts that print them (hack/sim-db/simdb.py and catalog.py,
hack/sim-suite/suite.py, hack/capture/analyze.py) and the console's HTTP
server (hack/console).

  engine    DuckDB: the pinned release and extensions, the catalog's
            connections (guarded readers, the indexer's writer, a fold's
            store), the statement gate, parameters, timeouts, a pool of
            readers, and results as Python values, JSON and the CLI's
            tables
  store     the store's SQL (store_host's fold and export) run in the
            module, and pulls from S3
  store_host
            the store as the farm hosts run it: stdlib and the DuckDB CLI
  verdict   analyze.py's verdicts, and the derived tables made from them
  vocab     the sims' event vocabulary (crates/strims-wan/vocab.toml),
            and the Rust and TS files made from it
  catalog   the DuckLake catalog: index, fetch, availability, schema
  describe  the catalog in words, a topic at a time: simdb.py describe's
            text, and the catalog's part of the Ask prompt
  query     simdb.py's selections and queries, as results
  suite     suite.py's tiers, cells, metrics, local runs and farm matrices
  scoring   the catalog's scoring macros, as scorecards and arm scores
  views     the console's saved views
  farm      farm.py's quotes and approved runs
  ask       questions in plain language, answered in SQL

Each module's docstring says what it offers. `engine` and the modules
over it need the duckdb module, which hack/pyproject.toml pins (uv's venv,
hack/.venv); `bootstrap`, `verdict`'s verdict functions, `farm`, `vocab`
and this module are stdlib alone, so a script under the plain python3
imports them; `store_host` is self-contained too, as farm.py ships it
alone.
A script puts hack/ on sys.path and imports `simlab`; its `__main__`
block runs `bootstrap.engine` before anything needs the module.
"""

import importlib.util
import sys
from pathlib import Path

HACK = Path(__file__).resolve().parents[1]
ROOT = HACK.parent
SIMDB_PY = HACK / "sim-db" / "simdb.py"


def simdb():
    """hack/sim-db/simdb.py: the cache's selections and queries, and the
    store's names (store_host's), whose cache and SQL simlab shares. The
    running script's own module when that is simdb.py, else loaded once by
    path (as `simdb` in sys.modules)."""
    for name in ("simdb", "__main__"):
        m = sys.modules.get(name)
        f = getattr(m, "__file__", None)
        if f and Path(f).resolve() == SIMDB_PY:
            return m
    spec = importlib.util.spec_from_file_location("simdb", SIMDB_PY)
    m = importlib.util.module_from_spec(spec)
    sys.modules["simdb"] = m
    try:
        spec.loader.exec_module(m)
    except BaseException:
        del sys.modules["simdb"]
        raise
    return m


def cache_root():
    """The sims' results cache (simdb.cache_root): `SIM_DB_ROOT`, else the
    main checkout's target/sim-db."""
    return simdb().cache_root()


def lake_root():
    """The catalog's directory: catalog.sqlite (DuckLake's, in SQLite)
    and data/ (its Parquet files)."""
    return cache_root() / "lake"

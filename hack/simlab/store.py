# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simdb.py's store, run in the module, and its S3 pulls.

simdb.py stays the farm hosts' file: its fold and export build their SQL
scripts as they do and run them through simdb.duck, which runs the CLI
unless an executor is installed (simdb.EXECUTOR). `install` installs this
module's, so on the dev machine suite.py's local folds and exports run in
the module; the hosts, which ship simdb.py alone, keep the CLI.

  execute(script, db=None, out="json", threads=None, memory=None)
                 simdb.duck's contract over the module (engine.store for
                 `db`, an in-memory instance else): the last statement's
                 rows as the CLI's JSON gave them (engine.Result.values,
                 as dicts), its text (`out` "text": engine.render_table of
                 every shown statement) or None; simdb.DuckError on a
                 failure, its message DuckDB's
  install()      simdb.EXECUTOR = execute, on simlab.simdb()'s module
  pull(run, reads=False) -> Pull
                 simdb.pull: `aws s3 sync` of a farm run's flushes (and
                 sim-samples/ with `reads`) into the cache, its S3 cost in
                 the farm's ledger
  fetch_samples(plan, gets, got) -> Pull
                 simdb.fetch_samples: the sample tables' files of a
                 sample_plan, each checked by its sha256
"""

import dataclasses


@dataclasses.dataclass
class Pull:
    """What a pull did: the sources it copied, those gone from S3, and
    what S3 charged (the ledger's lines it wrote, priced)."""
    run: str
    pulled: list = dataclasses.field(default_factory=list)
    gone: list = dataclasses.field(default_factory=list)
    s3_usd: float = 0.0
    ledger_lines: int = 0


def execute(script, db=None, out="json", threads=None, memory=None):
    raise NotImplementedError("simlab.store.execute: increment I1")


def install():
    raise NotImplementedError("simlab.store.install: increment I1")


def pull(run, reads=False):
    raise NotImplementedError("simlab.store.pull: increment I4")


def fetch_samples(plan, gets, got):
    raise NotImplementedError("simlab.store.fetch_samples: increment I4")

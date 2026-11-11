# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""suite.py's tiers, cells, metrics, local runs and farm matrices.

  TIERS          {tier: [cell]}, each cell {name, sim, test, env, mem_gib,
                 why, seeds}
  METRICS        [(name, getter SQL, better, kind, tie)]: the scoring's
  cells(tier, only=None, env=None) -> [cell]
                 a tier's cells, `only` those named, `env` set in each
  run_cells(cells, runs, slots, mem_gib) -> (rows, failures)
                 local runs of each cell and seed under each binary, their
                 tables folded into the cache (store.install's executor)
  farm_rows(name, cells, knob) -> {arm: {(cell, seed): (k, src)}}
  write_matrices(cells, seeds, arms=None, sets=None, ref="HEAD", knob=None,
                 env=None, question=None, against=None, no_off=False)
                 -> Matrices
                 suite.py farm's matrices (target/sim-suite/farm/) and its
                 index, written as `farm --no-run` writes them, the index
                 kept apart (pending/) until the farm starts the run
"""

import dataclasses


@dataclasses.dataclass
class Matrix:
    """One farm matrix: its file, its cell and the env it runs."""
    path: str
    cell: str
    env: dict


@dataclasses.dataclass
class Matrices:
    """A farm run's matrices: its name (the run's EC2 tag, s + 5 hex), its
    matrices in cell order, and its index (farm-runs/<name>.json)."""
    name: str
    matrices: list
    index: dict


def cells(tier, only=None, env=None):
    raise NotImplementedError("simlab.suite.cells: increment I5a")


def run_cells(cells, runs, slots, mem_gib):
    raise NotImplementedError("simlab.suite.run_cells: increment I5a")


def farm_rows(name, cells, knob):
    raise NotImplementedError("simlab.suite.farm_rows: increment I5a")


def write_matrices(cells, seeds, arms=None, sets=None, ref="HEAD", knob=None, env=None, question=None,
                   against=None, no_off=False):
    raise NotImplementedError("simlab.suite.write_matrices: increment I5a")

# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""farm.py's quotes and approved runs, for the console: the farm runs
matrices (suite.write_matrices') only once a person approves a quote.

  quote(matrices, regenerations=None) -> Quote
                 `farm.py estimate MATRICES --json` (the main checkout's
                 farm.py) in a child: Run.estimate()'s dict (jobs,
                 have_rows, timed, work_slot_h, instances, slots,
                 slots_by_region, model_wall_s, model_cost, past_runs,
                 wall_s, cost, s3_cost), what it printed, and the error it
                 exited with; kept under a new id
  approve(id, regenerations=None) -> Approval
                 `farm.py run MATRICES` started detached (a new session,
                 its output in target/console/regenerate/<id>.log), once
                 for a quoted id: QuoteError (409) for an unknown id or a
                 second approve
  Regenerations(root)
                 the quotes and their runs, target/console/regenerate/
                 <id>.json, in the shape the Node server kept them

stdlib alone: farm.py runs as its own process, never in this one.
"""

import dataclasses
from pathlib import Path


class QuoteError(Exception):
    def __init__(self, message, status=409):
        super().__init__(message)
        self.status = status


@dataclasses.dataclass
class Quote:
    id: str
    matrices: list
    estimate: dict = None
    text: str = ""
    error: str = None


@dataclasses.dataclass
class Approval:
    id: str
    pid: int
    log: str


class Regenerations:
    def __init__(self, root):
        self.root = Path(root)

    def get(self, id):
        raise NotImplementedError("simlab.farm.Regenerations.get: increment I6")

    def save(self, quote):
        raise NotImplementedError("simlab.farm.Regenerations.save: increment I6")


def quote(matrices, regenerations=None):
    raise NotImplementedError("simlab.farm.quote: increment I6")


def approve(id, regenerations=None):
    raise NotImplementedError("simlab.farm.approve: increment I6")

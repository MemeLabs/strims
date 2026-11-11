# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The catalog's scoring macros (scoring.sql), called through the engine,
as scorecards and arm scores.

A side is an arm's runs, {cell, seed, k, src}: an experiment's arm as
experiment_jobs has it (arm_side), or suite.py's own. Each result prints
exactly what suite.py prints (`text`) and goes to the console as JSON
(`json`).

  score(exp_a, arm_a, exp_b, arm_b, cells=None, show_all=False,
        traces=True) -> Scorecard
                 per cell and metric, the paired geomean ratio or mean
                 difference with its t-interval and mark, the collapse
                 counts and their Fisher p, the traces
  arm_scores(exp, base, arms=None, rank="lag", lag_budget=None,
             lag_ceiling=None) -> [ArmScore]
                 each arm against `base`: its guardrails, totals and
                 rank key, ranked
  structural(exp) -> engine.Result
  dynamics(exp) -> engine.Result
"""

import dataclasses


@dataclasses.dataclass
class Scorecard:
    a_label: str
    b_label: str
    cells: list = dataclasses.field(default_factory=list)
    show_all: bool = False

    def text(self):
        raise NotImplementedError("simlab.scoring.Scorecard.text: increment I5b")

    def json(self):
        raise NotImplementedError("simlab.scoring.Scorecard.json: increment I5b")


@dataclasses.dataclass
class ArmScore:
    title: str
    breaks: list = dataclasses.field(default_factory=list)
    totals: dict = dataclasses.field(default_factory=dict)
    rank_key: tuple = ()

    def text(self):
        raise NotImplementedError("simlab.scoring.ArmScore.text: increment I5b")

    def json(self):
        raise NotImplementedError("simlab.scoring.ArmScore.json: increment I5b")


def score(exp_a, arm_a, exp_b, arm_b, cells=None, show_all=False, traces=True):
    raise NotImplementedError("simlab.scoring.score: increment I5b")


def arm_scores(exp, base, arms=None, rank="lag", lag_budget=None, lag_ceiling=None):
    raise NotImplementedError("simlab.scoring.arm_scores: increment I5b")


def structural(exp):
    raise NotImplementedError("simlab.scoring.structural: increment I5b")


def dynamics(exp):
    raise NotImplementedError("simlab.scoring.dynamics: increment I5b")

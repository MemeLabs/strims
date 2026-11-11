# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""An experiment's cells, arms and seeds run again on the farm: the suite's
matrices for them, written and quoted (simlab.farm). Nothing runs until a
person approves the quote (farm.approve).

  regenerate(pool, exp_id, cells, arms, seeds, regenerations=None)
                 -> dict
                 the regeneration as the console's API answers it: {id,
                 exp_id, cells, arms, seeds, state (quoted, or failed),
                 created_at, commands, matrices, quote, estimate,
                 warnings, error?}, saved in `regenerations` unless refused
                 before any matrix was written. farm.QuoteError 404 for no
                 such experiment.
  plan(exp, request) -> [(tier, cells, seeds, kwargs)], or a str
                 suite.write_matrices' calls for `request`'s cells, arms
                 and seeds of `exp`, one per tier, or why there are none:
                 only a farm run regenerates, its seeds consecutive, each
                 arm one of its --refs (the base first) or of its --sets
                 (`off` left to no_off)
  suite_command(tier, cells, seeds, kwargs) -> [str]
                 suite.py farm's invocation that writes the same matrices
  env_warnings(exp_id, matrices, env) -> [str]
                 a warning for each matrix whose env isn't the one its
                 cell ran with in the experiment

Each cell runs in the experiment's tier whose cell of that name has the
env it ran with, else in its first tier with the cell. A matrix's env
differs from the experiment's when the suite's cells changed since it ran;
the warnings say where.
"""

import json
import re
import secrets

from . import farm as farm_module
from . import suite as suite_module

# A name a command takes: an experiment, cell, arm or table.
NAME = re.compile(r"[A-Za-z0-9][A-Za-z0-9_.@:+=/-]*")


def bad_name(names):
    """The first of `names` that isn't a name, or None."""
    return next((n for n in names if not isinstance(n, str) or not NAME.fullmatch(n)), None)


def rows(pool, sql, params):
    j = pool.query(sql, params, max_rows=100_000, timeout_s=60).json()
    names = [c["name"] for c in j["columns"]]
    return [dict(zip(names, r)) for r in j["rows"]]


def experiment(pool, exp_id):
    """`exp_id`'s row of `experiments` (exp_id, kind, knob, sets, arms,
    base), with `tiers` ({cell: tier}) and `env` ({cell: env}), or None."""
    found = rows(pool, "SELECT exp_id, kind, knob, sets, arms, base FROM experiments WHERE exp_id = $exp",
                 {"exp": exp_id})
    if not found:
        return None
    cells = rows(pool,
                 "SELECT c.cell, c.env, (SELECT t.tier FROM raw.tier_cells t "
                 "WHERE t.cell = c.cell AND list_contains(x.tiers, t.tier) "
                 "ORDER BY (t.env = c.env) DESC, t.tier LIMIT 1) AS tier "
                 "FROM experiment_cells c JOIN experiments x USING (exp_id) WHERE c.exp_id = $exp",
                 {"exp": exp_id})
    return {**found[0], "tiers": {c["cell"]: c["tier"] for c in cells if c["tier"]},
            "env": {c["cell"]: c["env"] or {} for c in cells}}


def plan(exp, r):
    """suite.write_matrices' calls that regenerate `r` ({cells, arms,
    seeds}) of `exp` (experiment's), one per tier: (tier, cells, seeds,
    kwargs) in the order `r` first names a cell of each tier; or why there
    are none."""
    if exp["kind"] != "farm":
        return f"{exp['exp_id']} is a local invocation; only farm runs regenerate"
    if not r["cells"] or not r["arms"] or not r["seeds"]:
        return "cells, arms and seeds, one each at least"
    bad = bad_name(r["cells"] + r["arms"])
    if bad is not None:
        return f"not a name: {json.dumps(bad)}"
    seeds = sorted({int(s) for s in r["seeds"]})
    if seeds != list(range(seeds[0], seeds[0] + len(seeds))):
        return "a farm run's seeds are consecutive: give a range"
    missing = [c for c in r["cells"] if c not in exp["tiers"]]
    if missing:
        return f"no tier of {exp['exp_id']} has {', '.join(missing)}"
    arms = exp["arms"] or {}
    if exp["knob"]:
        sets = exp["sets"] or []
        other = [a for a in r["arms"] if a not in sets]
        if other:
            return f"{', '.join(other)}: not a set of {exp['exp_id']} ({', '.join(sets)})"
        chosen = [s for s in sets if s in r["arms"]]
        base = exp["base"] if exp["base"] in chosen else chosen[0]
        kwargs = {"sets": [s for s in chosen if s != "off"], "ref": arms.get("x"), "knob": exp["knob"],
                  "against": base, "no_off": "off" not in chosen}
    else:
        other = [a for a in r["arms"] if a not in arms]
        if other:
            return f"{', '.join(other)}: not an arm of {exp['exp_id']}"
        # The first arm is the base.
        order = sorted((a for a in arms if a in r["arms"]), key=lambda a: a != exp["base"])
        kwargs = {"arms": {a: arms[a] for a in order}}
    tiers = {}
    for c in r["cells"]:
        tiers.setdefault(exp["tiers"][c], []).append(c)
    return [(tier, cells, seeds, kwargs) for tier, cells in tiers.items()]


def suite_command(tier, cells, seeds, kw):
    """suite.py farm's invocation that writes the same matrices."""
    out = ["suite.py", "farm", "--tier", tier, "--only", ",".join(cells), "--seeds", str(len(seeds)),
           "--first-seed", str(seeds[0])]
    if "arms" in kw:
        out += ["--refs", ",".join(f"{a}={c}" for a, c in kw["arms"].items())]
    else:
        knob = kw["knob"]
        out += ["--knob", f"{knob.get('ppspp', '')},{knob.get('strims-sim', '')}", "--ref", str(kw["ref"]),
                "--sets", ";".join(kw["sets"])]
        if kw["no_off"]:
            out.append("--no-off")
        out += ["--against", kw["against"]]
    return out + ["--no-run"]


def env_warnings(exp_id, matrices, env):
    """A warning for each of `matrices` (suite.Matrix) whose env isn't the
    one its cell ran with in experiment `exp_id` (`env`: {cell: env})."""
    out = []
    for m in matrices:
        got, want = m.env or {}, env.get(m.cell) or {}
        keys = [k for k in dict.fromkeys([*got, *want]) if got.get(k) != want.get(k)]
        if keys:
            changes = ", ".join(f"{k} ({want.get(k, 'unset')} -> {got.get(k, 'unset')})" for k in keys)
            out.append(f"{m.cell}: env differs from {exp_id}'s at {changes}")
    return out


def regenerate(pool, exp_id, cells, arms, seeds, regenerations=None, suite=suite_module, farm=farm_module):
    """Writes the matrices that run `cells`, `arms` and `seeds` of
    experiment `exp_id` again and quotes them (the module docs). `suite`
    and `farm` are the modules it calls."""
    regs = regenerations or farm.Regenerations()
    exp = experiment(pool, exp_id)
    if exp is None:
        raise farm.QuoteError(f"no experiment {exp_id}", 404)
    r = {"exp_id": exp_id, "cells": list(cells), "arms": list(arms), "seeds": list(seeds)}
    record = {**r, "id": None, "state": "quoted", "created_at": farm.now(), "commands": [], "matrices": [],
              "quote": "", "warnings": []}

    def failed(error, save=True):
        record.update(id=record["id"] or secrets.token_urlsafe(9), state="failed", error=error)
        if save:
            regs.save(record)
        return record

    planned = plan(exp, r)
    if isinstance(planned, str):
        return failed(planned, save=False)
    matrices = []
    for tier, names, tier_seeds, kw in planned:
        record["commands"].append(suite_command(tier, names, tier_seeds, kw))
        try:
            written = suite.write_matrices(suite.cells(tier, only=names), tier_seeds, **kw)
        except (Exception, SystemExit) as e:
            return failed(f"the suite's matrices for {tier}: {e}")
        if len(written.matrices) != len(names):
            return failed(f"the suite's matrices for {tier}: {len(written.matrices)} of {len(names)}")
        matrices += written.matrices
    record["warnings"] = env_warnings(exp_id, matrices, exp["env"])
    q = farm.quote(matrices, regs)
    record.update(id=q.id, matrices=q.matrices, quote=q.text or "", estimate=q.estimate)
    if q.error:
        return failed(q.error)
    regs.save(record)
    return record

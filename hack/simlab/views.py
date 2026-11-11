# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The console's saved views (hack/console/views): each NAME.sql (one
SELECT over the catalog, its `$name` parameters the view's), NAME.vl.json
(a Vega-Lite spec of its rows, or none) and NAME.json ({title,
description, params: [{name, kind, default?, label?}], todo?}).

  saved(root=VIEWS) -> [View]      read from disk at each call, by name
  get(name, root=VIEWS) -> View    KeyError for none
  save(name, view, root=VIEWS)     ValueError for a bad name or param kind
  run(pool, name, params, root=VIEWS, max_rows=..., timeout_s=...)
                                   -> engine.Result: the view's SQL through
                                   the pool's gate, the params' defaults
                                   filling those not given

A name is NAME (^[a-z][a-z0-9_-]*$); a param's kind is one of KINDS.
View.json() is what GET /console/schema's `views` lists.
"""

import dataclasses
from pathlib import Path

VIEWS = Path(__file__).resolve().parents[1] / "console" / "views"
KINDS = ("exp", "cell", "arm", "arm2", "seed", "number", "search", "align")


@dataclasses.dataclass
class Param:
    name: str
    kind: str
    default: object = None
    label: str = None


@dataclasses.dataclass
class View:
    name: str
    title: str
    description: str
    params: list
    sql: str
    spec: dict = None
    todo: str = None

    def json(self):
        raise NotImplementedError("simlab.views.View.json: increment I7")


def saved(root=VIEWS):
    raise NotImplementedError("simlab.views.saved: increment I7")


def get(name, root=VIEWS):
    raise NotImplementedError("simlab.views.get: increment I7")


def save(name, view, root=VIEWS):
    raise NotImplementedError("simlab.views.save: increment I7")


def run(pool, name, params, root=VIEWS, max_rows=10_000, timeout_s=30.0):
    raise NotImplementedError("simlab.views.run: increment I7")

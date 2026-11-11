# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The console's saved views (hack/console/views): each NAME.sql (one
SELECT over the catalog, its `$name` parameters the view's), NAME.vl.json
(a Vega-Lite spec of its rows, or none) and NAME.json ({title,
description, params: [{name, kind, default?, label?, options?}],
todo?}).

  saved(root=VIEWS) -> [View]      every view, by file name, read from disk
                                   at each call
  get(name, root=VIEWS) -> View    KeyError for none
  save(name, view, root=VIEWS) -> View
                                   writes the view's files (its spec's
                                   removed when it has none) and returns it
                                   as read back: ValueError for a bad name
                                   or view (check)
  run(pool, name, params, root=VIEWS, max_rows=..., timeout_s=...)
                                   -> engine.Result: the view's SQL through
                                   the pool's gate, each param not given
                                   (or given "" or None) taking its
                                   default, and a seed's or number's
                                   string its number, as the UI's pickers
                                   give them (selection.ts viewParams)

A view's name is NAME; files of other names are
none. A param's name is one the UI finds in a statement (`$name`), its
kind one of KINDS (the UI's pickers, the Workbench's text and a choice
of its options), its default a value the console's API binds: null, a
string, number or boolean, or a list of strings and numbers. View.json()
is what GET /console/schema's `views` lists.
"""

import dataclasses
import json
import math
import os
import re
import threading
from pathlib import Path

from .engine import QueryError

VIEWS = Path(__file__).resolve().parents[1] / "console" / "views"
NAME = re.compile(r"[a-z][a-z0-9_-]{0,63}")
PARAM = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
KINDS = ("exp", "cell", "arm", "arm2", "seed", "number", "text", "search", "align", "choice")
# The kinds whose string value the UI sends as a number.
NUMBER_KINDS = ("seed", "number")
PARAM_KEYS = ("name", "kind", "default", "label", "options")


@dataclasses.dataclass
class Param:
    name: str
    kind: str
    default: object = None
    label: str = None
    options: list = None

    def json(self):
        return {k: getattr(self, k) for k in PARAM_KEYS if getattr(self, k) is not None}


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
        out = {"name": self.name, "title": self.title, "description": self.description,
               "params": [p.json() for p in self.params], "sql": self.sql, "spec": self.spec}
        if self.todo is not None:
            out["todo"] = self.todo
        return out


def is_value(v):
    """Whether the console's API binds `v`."""
    if v is None or isinstance(v, (str, bool)):
        return True
    if isinstance(v, (int, float)):
        return math.isfinite(v)
    return isinstance(v, list) and all(
        isinstance(x, str) or isinstance(x, (int, float)) and not isinstance(x, bool) and math.isfinite(x)
        for x in v)


def param(p):
    """`p` (a Param, or its JSON) checked: ValueError says what's wrong."""
    if isinstance(p, dict):
        extra = set(p) - set(PARAM_KEYS)
        if extra:
            raise ValueError(f"a param has {', '.join(sorted(extra))}: {', '.join(PARAM_KEYS)} only")
        p = Param(*(p.get(k) for k in PARAM_KEYS))
    elif not isinstance(p, Param):
        raise ValueError(f"a param is {{{', '.join(PARAM_KEYS)}}}, not {p!r}")
    if not isinstance(p.name, str) or not PARAM.fullmatch(p.name):
        raise ValueError(f"a param's name is a $name: {PARAM.pattern}, not {p.name!r}")
    if p.kind not in KINDS:
        raise ValueError(f"param {p.name}'s kind is one of {', '.join(KINDS)}, not {p.kind!r}")
    if not is_value(p.default):
        raise ValueError(f"param {p.name}'s default is a string, number, boolean or list, not {p.default!r}")
    if p.label is not None and not isinstance(p.label, str):
        raise ValueError(f"param {p.name}'s label is a string")
    if p.options is not None and not (isinstance(p.options, list) and all(isinstance(o, str) for o in p.options)):
        raise ValueError(f"param {p.name}'s options are a list of strings")
    return p


def check(name, view):
    """`view` saved as `name`, checked: its title the name and its
    description empty when none. ValueError says what's wrong."""
    if not isinstance(name, str) or not NAME.fullmatch(name):
        raise ValueError(f"a view's name is {NAME.pattern}, not {name!r}")
    if not isinstance(view, View):
        raise ValueError(f"a view is a View, not {type(view).__name__}")
    if not isinstance(view.sql, str) or not view.sql.strip():
        raise ValueError("a view's sql is one statement")
    for k in ("title", "description", "todo"):
        if getattr(view, k) is not None and not isinstance(getattr(view, k), str):
            raise ValueError(f"a view's {k} is a string")
    if view.spec is not None and not isinstance(view.spec, dict):
        raise ValueError("a view's spec is a Vega-Lite spec (an object), or none")
    if view.params is not None and not isinstance(view.params, list):
        raise ValueError("a view's params are a list")
    params = [param(p) for p in view.params or []]
    names = [p.name for p in params]
    if len(set(names)) < len(names):
        raise ValueError(f"a view's params are named once each: {', '.join(names)}")
    return View(name, name if view.title is None else view.title, view.description or "", params, view.sql,
                view.spec, view.todo)


def read(root, name):
    """The view `name` in `root`, or None."""
    if not NAME.fullmatch(name):
        return None

    def text(ext):
        try:
            return (root / f"{name}{ext}").read_text()
        except FileNotFoundError:
            return None

    def parsed(ext, default):
        t = text(ext)
        try:
            return default if t is None else json.loads(t)
        except ValueError as e:
            raise ValueError(f"{root / (name + ext)}: {e}") from None

    sql = text(".sql")
    if sql is None:
        return None
    meta = parsed(".json", {})
    params = (meta.get("params") or []) if isinstance(meta, dict) else None
    if not isinstance(params, list) or not all(isinstance(p, dict) for p in params):
        raise ValueError(f"{root / (name + '.json')}: {{title, description, params: [{{name, kind, ...}}]}}")
    params = [Param(*(p.get(k) for k in PARAM_KEYS)) for p in params]
    title = meta.get("title")
    return View(name, name if title is None else title, meta.get("description") or "", params, sql,
                parsed(".vl.json", None), meta.get("todo"))


def saved(root=VIEWS):
    """Every saved view in `root`, by file name."""
    root = Path(root)
    try:
        files = sorted(p.name for p in root.iterdir() if p.name.endswith(".sql"))
    except FileNotFoundError:
        return []
    views = (read(root, f[:-len(".sql")]) for f in files)
    return [v for v in views if v is not None]


def get(name, root=VIEWS):
    v = read(Path(root), name) if isinstance(name, str) else None
    if v is None:
        raise KeyError(f"no view {name!r}")
    return v


def write(path, text):
    tmp = path.with_name(f".{path.name}.{os.getpid()}.{threading.get_ident()}")
    tmp.write_text(text)
    os.replace(tmp, path)


def save(name, view, root=VIEWS):
    """Writes `view` as `name`'s files in `root` and returns it as read
    back."""
    v = check(name, view)
    root = Path(root)
    root.mkdir(parents=True, exist_ok=True)
    meta = {"title": v.title, "description": v.description, "params": [p.json() for p in v.params]}
    if v.todo is not None:
        meta["todo"] = v.todo
    write(root / f"{name}.sql", v.sql if v.sql.endswith("\n") else v.sql + "\n")
    write(root / f"{name}.json", json.dumps(meta, indent=2, ensure_ascii=False) + "\n")
    if v.spec is not None:
        write(root / f"{name}.vl.json", json.dumps(v.spec, indent=2, ensure_ascii=False) + "\n")
    else:
        (root / f"{name}.vl.json").unlink(missing_ok=True)
    return get(name, root)


def value(p, v):
    """`v` given for `p` as the UI sends it: a seed's or number's string
    as its number."""
    if p.kind not in NUMBER_KINDS or not isinstance(v, str):
        return v
    try:
        n = float(v)
    except ValueError:
        raise QueryError(f"${p.name} is a number, not {v!r}", 400) from None
    if not math.isfinite(n):
        raise QueryError(f"${p.name} is a number, not {v!r}", 400)
    return int(n) if n.is_integer() else n


def run(pool, name, params=None, root=VIEWS, max_rows=10_000, timeout_s=30.0):
    """View `name`'s rows over `pool` (engine.Pool): KeyError for no such
    view, QueryError for a param missing or the query failing."""
    view = get(name, root)
    given = params or {}
    values = {k: v for k, v in given.items() if k not in {p.name for p in view.params}}
    for p in view.params:
        v = given.get(p.name)
        if v is None or v == "":
            v = p.default
        if v is not None:
            values[p.name] = value(p, v)
    return pool.query(view.sql, values, max_rows=max_rows, timeout_s=timeout_s)

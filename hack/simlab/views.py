# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The console's saved views (hack/console/views): each NAME.sql (one
SELECT over the catalog, its `$name` parameters the view's), NAME.vl.json
(a Vega-Lite spec of its rows, or none; its `hover` keys, which the UI
expands, give time panels a shared hover cursor and tooltips:
src/devtools/components/console/hover.ts) and NAME.json ({title,
description, params: [{name, kind, default?, label?, options?}],
todo?}).

  saved(root=VIEWS) -> [View]      every view, by file name, read from disk
                                   at each call
  get(name, root=VIEWS) -> View    KeyError for none
  save(name, view, root=VIEWS) -> View
                                   writes the view's files (its spec's
                                   removed when it has none) and returns it
                                   as read back: ValueError for a bad name
                                   or view (checked)
  run(pool, name, params, root=VIEWS, max_rows=..., timeout_s=..., text=False, sel=None)
                                   -> engine.Result: the view's SQL through
                                   the pool's gate, each param not given
                                   (or given "" or None) taking its
                                   default, and a seed's or number's
                                   string its number, as the UI's pickers
                                   give them (selection.ts viewParams);
                                   with `text` its values DuckDB's strings
                                   (engine.run), as the CLI prints them;
                                   with `sel` the rows behind that
                                   selection (where); `name` the workbench,
                                   its statement (view_of)

A view's name is NAME, but for the page's routes (ROUTES); files of other
names are none. A param's name is one the UI finds in a statement
(`$name`), its kind one of KINDS (the UI's pickers, the Workbench's text
and a choice of its options), its default a value the console's API
binds: null, a string, number or boolean, or a list of strings and
numbers. View.json() is what GET /console/schema's `views` lists.

The console's addresses. The page is a HashRouter, a view at
BASE/devtools.html#/console/NAME?QUERY and the workbench at
#/console/workbench?QUERY, the query as URLSearchParams writes it
(application/x-www-form-urlencoded):

- A view's params: each under its name when that is a key of the page's
  selection (KEYS: useSelection reads those alone), else under its kind
  when that is one (selection.ts viewParams), in KEYS' order. A value
  the page's own default for its key (DEFAULTS) or "" is left out, as
  the page's setter leaves it; a param with neither key can't be set
  from an address.
- The workbench's: `view` (a saved view's statement and spec) or `sql`,
  then its statement's `$name`s' values, each under its own name (the
  selection's keys in KEYS' order, then the rest as the statement first
  names them; a name the workbench's own keys take, WORKBENCH_KEYS, can't
  be set), then the auto-chart's `mark`, `x`, `y`, `color` and `logy`
  (1 for a symlog y). Its chart is the auto-chart when the address names
  x and y, else the view's spec.
- `sel` last: a selection on the graph, compact JSON as JSON.stringify
  writes it, its keys `panel` (left out for a graph of one panel) then
  `at`, `at`'s keys sorted:

    {"panel": P, "at": {FIELD: VALUE | [LO, HI], ...}}

  P is a panel's id (panels: its title, else its index path). Each FIELD
  is a column of the view's rows the panel encodes (or with no panel,
  that any panel encodes): a VALUE matches by equality; a range, on the
  panel's time field alone, matches x BETWEEN LO AND HI, or where the
  panel encodes x2 on another column, the spans overlapping it (x <= HI
  AND x2 >= LO). A selection with a panel takes that panel's filter
  transforms too, those that translate to SQL (filter_sql).

  link(name, params, sel=None, base=None, root=VIEWS) -> str
                                   the address: ValueError for a param
                                   the view doesn't take or the address
                                   can't hold, a bad selection
                                   (selection), an unknown view
  parse(url, root=VIEWS) -> (name, params, sel)
                                   link's inverse: each param the page
                                   reads from the address, as text (its
                                   key's DEFAULTS where the address leaves
                                   it out), and the selection as given
  selection(view, sel) -> (sel, Panel | None)
                                   `sel` checked against the view's graph,
                                   as the address holds it
  where(view, sel) -> Where        the SQL condition of a selection, its
                                   values, its parts in words and the
                                   panel filters that don't translate
  selection_text(view, sel) -> str the selection in a line
  filter_sql(predicate) -> str     a filter transform's SQL: Untranslated
                                   for a form with none
  view_of(name, params, root=VIEWS) -> View
                                   the saved view, or the workbench's
                                   statement and chart as a View
  check(pool, name, params, sel=None, root=VIEWS) -> [str]
                                   what's wrong with an address over the
                                   catalog: its experiment, cell, arms and
                                   seed, its statement's columns, and
                                   whether its selection matches a row
  panels(spec) -> [Panel]          a spec's panels, as hover.ts walks them
  graph(view) -> dict              what the view's spec draws: each
                                   panel's id, title, marks, channels,
                                   time field, filters and colour legend,
                                   its vocabulary references expanded
                                   (expand_vocab, as domains.ts does)
  server_state() -> dict | None    the running console server's state file
                                   (state_path: {pid, port, urls})
  tabs(server_url=None) -> [dict]  the open console tabs' last reports
                                   ({id, url, focused, visible,
                                   last_seen}), the latest focused first,
                                   from the running server (else
                                   SIM_CONSOLE_URL's): NoServer when none
                                   answers
  tab(id=None, server_url=None) -> dict
                                   one of them, the first by default
  base() -> str                    the host links are on: SIM_CONSOLE_URL,
                                   else the origin of the latest focused
                                   tab's address, else the running
                                   server's first LAN URL (server_state),
                                   else its loopback URL, else BASE
"""

import dataclasses
import decimal
import ipaddress
import json
import math
import os
import re
import threading
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

from .engine import QueryError

VIEWS = Path(__file__).resolve().parents[1] / "console" / "views"
NAME = re.compile(r"[a-z][a-z0-9_-]{0,63}")
# The page's routes besides the saved views' (Console.tsx), which they
# take first.
WORKBENCH = "workbench"
ROUTES = ("ask", WORKBENCH)
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


def checked(name, view):
    """`view` saved as `name`, checked: its title the name and its
    description empty when none. ValueError says what's wrong."""
    if not isinstance(name, str) or not NAME.fullmatch(name):
        raise ValueError(f"a view's name is {NAME.pattern}, not {name!r}")
    if name in ROUTES:
        raise ValueError(f"{name} is the console's {name} page: a view needs another name")
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
    if not NAME.fullmatch(name) or name in ROUTES:
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
    v = checked(name, view)
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


def values_of(view, params):
    """The values `view`'s statement binds for `params`: a saved view's
    as its pickers give them (value), the workbench's as Workbench.tsx's
    params() does (a `$name` not given its key's DEFAULTS, else "", and
    text that looks like a number the number)."""
    given = params or {}
    if view.name == WORKBENCH:
        out = {}
        for p in view.params:
            v = given.get(p.name)
            v = DEFAULTS.get(p.name, "") if v is None else v
            if isinstance(v, str) and NUMERIC.fullmatch(v):
                v = float(v) if "." in v else int(v)
            out[p.name] = v
        return out
    values = {k: v for k, v in given.items() if k not in {p.name for p in view.params}}
    for p in view.params:
        v = given.get(p.name)
        if v is None or v == "":
            v = p.default
        if v is not None:
            values[p.name] = value(p, v)
    return values


def run(pool, name, params=None, root=VIEWS, max_rows=10_000, timeout_s=30.0, text=False, sel=None):
    """View `name`'s rows over `pool` (engine.Pool), with `text` as DuckDB's
    strings, and with `sel` those behind that selection (where; the panel
    filters that don't translate aren't applied, and where's `unapplied`
    names them): KeyError for no such view, ValueError for a bad selection
    or a workbench address with no statement, QueryError for a param
    missing or the query failing."""
    view = view_of(name, params, root)
    sql, values = view.sql, values_of(view, params)
    if sel is not None:
        w = where(view, sel)
        sql, values = w.wrap(sql), {**values, **w.params}
    return pool.query(sql, values, max_rows=max_rows, timeout_s=timeout_s, text=text)


# --- the console's addresses ----------------------------------------------------------------------

# The page's selection (src/devtools/components/console/selection.ts):
# the keys useSelection reads from the address's query, in its order, and
# the value each takes where the address leaves it out.
KEYS = ("exp", "cell", "arm", "arm2", "seed", "delay", "min_stall_ms", "align", "t", "search")
DEFAULTS = {"exp": "", "cell": "", "arm": "", "arm2": "", "seed": "", "delay": "1250", "min_stall_ms": "50",
            "align": "start", "t": "30", "search": ""}
BASE = "http://127.0.0.1:8086"
PAGE = "/devtools.html#/console/"
SEL = "sel"
# The workbench's auto-chart (Workbench.tsx): its choices in the address,
# and its marks.
CHART = ("mark", "x", "y", "color", "logy")
MARKS = ("line", "point", "bar", "area", "rect", "tick")
WORKBENCH_KEYS = ("view", "sql", *CHART, SEL)
# Text the workbench binds as a number (Workbench.tsx params()).
NUMERIC = re.compile(r"-?\d+(\.\d+)?")
# The picker a workbench `$name` suggests (Workbench.tsx kindOf).
WORKBENCH_KINDS = {"exp": "exp", "cell": "cell", "arm": "arm", "arm2": "arm2", "seed": "seed", "delay": "number",
                   "min_stall_ms": "number", "t": "number"}
CONCATS = ("vconcat", "hconcat", "concat")
# The channels whose fields a selection may name: a mark's place and
# what tells it from its neighbours (a tooltip's, a text's, a size's or
# an opacity's fields are no mark's identity).
SELECTABLE = ("x", "x2", "y", "y2", "xOffset", "yOffset", "color", "fill", "stroke", "strokeDash", "shape", "detail",
              "row", "column", "facet")
COLOURS = ("color", "fill", "stroke")
# Transforms whose rows are their groupby's: the columns past them.
GROUPED = ("aggregate", "pivot", "regression", "loess", "density", "quantile")
# The vocabularies a spec's references name (domains.ts VOCABS).
VOCAB_SECTIONS = ("phases", "episodes", "reasons")
SERVER = "python3 hack/console/server.py"
BUILD = "npm run console:build"
TAB_TIMEOUT_S = 5


def form(s):
    """`s` as URLSearchParams writes a key or value: the
    application/x-www-form-urlencoded byte serializer's, which leaves
    alphanumerics and *-._ alone and writes a space as +."""
    return urllib.parse.quote_plus(s, safe="*").replace("~", "%7E")


def js_number(x):
    """Number `x` as JavaScript's String(x) and JSON.stringify write it:
    shortest round-trip digits, fixed from 1e-7 up to 1e21, else
    d.ddde+N."""
    if isinstance(x, bool) or not isinstance(x, (int, float)):
        raise ValueError(f"not a number: {x!r}")
    if isinstance(x, int):
        return str(x)
    if not math.isfinite(x):
        raise ValueError(f"not a finite number: {x!r}")
    if x == 0:
        return "0"
    sign, digits, exp = decimal.Decimal(repr(x)).normalize().as_tuple()
    ds = "".join(map(str, digits))
    k, n = len(ds), exp + len(ds)
    if k <= n <= 21:
        out = ds + "0" * (n - k)
    elif 0 < n <= 21:
        out = ds[:n] + "." + ds[n:]
    elif -6 < n <= 0:
        out = "0." + "0" * -n + ds
    else:
        e = n - 1
        out = (ds if k == 1 else ds[0] + "." + ds[1:]) + ("e+" if e > 0 else "e-") + str(abs(e))
    return ("-" if sign else "") + out


def js_json(v):
    """`v` as JSON.stringify writes it, compact: its numbers js_number's,
    its objects' keys in their order."""
    if v is None:
        return "null"
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, (int, float)):
        return js_number(v)
    if isinstance(v, str):
        return json.dumps(v, ensure_ascii=False)
    if isinstance(v, (list, tuple)):
        return "[" + ",".join(js_json(x) for x in v) + "]"
    if isinstance(v, dict):
        return "{" + ",".join(f"{json.dumps(str(k), ensure_ascii=False)}:{js_json(x)}" for k, x in v.items()) + "}"
    raise ValueError(f"not JSON: {v!r}")


def text_of(v):
    """A param's value as the address's text: a string as itself, an
    integer or float as JavaScript writes it."""
    if v is None:
        return ""
    if isinstance(v, str):
        return v
    if isinstance(v, (int, float)) and not isinstance(v, bool):
        return js_number(v)
    raise ValueError(f"a param's value is text, not {v!r}")


def url_key(p):
    """The address's key `p` (a saved view's Param) is read from: its name
    when that is a key of the page's selection, else its kind when that is
    one, else None."""
    return p.name if p.name in KEYS else p.kind if p.kind in KEYS else None


def statement_params(sql):
    """The `$name`s a statement takes, in the order it first names them,
    outside its strings and comments (Workbench.tsx paramNames)."""
    bare = re.sub(r"--[^\n]*|/\*[\s\S]*?\*/|'(?:[^']|'')*'", " ", sql)
    return list(dict.fromkeys(re.findall(r"\$([A-Za-z_]\w*)", bare)))


def chart_spec(chart):
    """The workbench's auto-chart (Workbench.tsx autoSpec) of `chart`'s
    choices, the fields' types left to the rows: None without x and y."""
    if not chart.get("x") or not chart.get("y"):
        return None
    y = {"field": chart["y"], **({"scale": {"type": "symlog"}} if chart.get("logy") in (True, "1", "true") else {})}
    enc = {"x": {"field": chart["x"]}, "y": y}
    if chart.get("color"):
        enc["color"] = {"field": chart["color"]}
    return {"mark": {"type": chart.get("mark") or "auto", "tooltip": True}, "encoding": enc}


def view_of(name, params=None, root=VIEWS):
    """What an address of route `name` shows: the saved view (KeyError for
    none), or for the workbench its statement (`sql`, or `view`'s), its
    `$name`s its params (Workbench.tsx kindOf's kinds), and its chart
    (chart_spec's, else the view's spec)."""
    if name != WORKBENCH:
        if name in ROUTES:
            raise KeyError(f"{name} is the console's {name} page, no view")
        return get(name, root)
    given = params or {}
    named = get(given["view"], root) if given.get("view") else None
    sql = given.get("sql") or (named.sql if named else None)
    if not sql:
        raise ValueError("a workbench address with no statement (sql or view) runs the workbench's own first one")
    chart = {k: given[k] for k in CHART if given.get(k)}
    spec = chart_spec(chart) or (named.spec if named else None)
    names = [n for n in statement_params(sql) if n not in WORKBENCH_KEYS]
    return View(WORKBENCH, named.title if named else "SQL workbench", named.description if named else "",
                [Param(n, WORKBENCH_KINDS.get(n, "text")) for n in names], sql, spec)


def _view_query(view, params):
    names = {p.name for p in view.params}
    extra = sorted(set(params) - names)
    if extra:
        raise ValueError(f"{view.name} takes no {', '.join(extra)}: its params are "
                         f"{', '.join(p.name for p in view.params) or 'none'}")
    at = {}
    for p in view.params:
        if p.name not in params:
            continue
        v = text_of(params[p.name])
        if v != "" and p.kind in NUMBER_KINDS:
            try:
                value(p, v)
            except QueryError as e:
                raise ValueError(str(e)) from None
        key = url_key(p)
        if key is None:
            # Its default as text, where it has one; the page gives it that.
            textual = isinstance(p.default, (str, int, float)) and not isinstance(p.default, bool)
            if v != "" and not (textual and v == text_of(p.default)):
                raise ValueError(f"{view.name}'s {p.name} can't be set from an address: neither its name nor its "
                                 f"kind ({p.kind}) is a key of the page's selection ({', '.join(KEYS)})")
            continue
        if v == "":
            continue
        if at.get(key, v) != v:
            raise ValueError(f"{view.name}'s params read one key of the address, {key}: give them one value")
        at[key] = v
    return [(k, at[k]) for k in KEYS if k in at and at[k] != DEFAULTS[k]]


def _workbench_query(params, root):
    p = {k: v for k, v in params.items()}
    named, sql = text_of(p.pop("view", "")), text_of(p.pop("sql", ""))
    if named and sql:
        raise ValueError("the workbench takes sql or view, not both")
    try:
        statement = get(named, root).sql if named else sql or None
    except KeyError:
        raise ValueError(f"the workbench's view {named!r} is no saved view: "
                         f"{', '.join(v.name for v in saved(root))}") from None
    chart = {}
    for k in CHART:
        if k not in p:
            continue
        v = p.pop(k)
        if k == "logy":
            if v not in (True, False, "", "0", "1", "true", "false"):
                raise ValueError(f"logy is true or false, not {v!r}")
            if v in (True, "1", "true"):
                chart[k] = "1"
            continue
        v = text_of(v)
        if k == "mark" and v and v not in MARKS:
            raise ValueError(f"the workbench's marks are {', '.join(MARKS)}, not {v!r}")
        if v:
            chart[k] = v
    names = statement_params(statement) if statement is not None else list(KEYS)
    taken = sorted(n for n in p if n in WORKBENCH_KEYS)
    if taken:
        raise ValueError(f"${', $'.join(taken)} can't be set from an address: the workbench's "
                         f"{', '.join(WORKBENCH_KEYS)} take those keys")
    extra = sorted(set(p) - set(names))
    if extra:
        what = "its statement takes" if statement is not None else "with no statement it takes the selection's"
        raise ValueError(f"the workbench takes no {', '.join(extra)}: {what} "
                         f"{', '.join(names) or 'none'}")
    order = [k for k in KEYS if k in names] + [n for n in names if n not in KEYS]
    out = [("view", named)] if named else [("sql", sql)] if sql else []
    for n in order:
        if n not in p:
            continue
        v = text_of(p[n])
        if v != "" and v != DEFAULTS.get(n):
            out.append((n, v))
    return out + [(k, chart[k]) for k in CHART if k in chart]


def link(name, params=None, sel=None, base=None, root=VIEWS):
    """The address of view `name` (or the workbench) with `params` (text,
    as simdb.py view --param takes them) and selection `sel`, on `base`
    (base()'s by default): the module docs."""
    params = dict(params or {})
    if name == WORKBENCH:
        query = _workbench_query(params, root)
    else:
        try:
            view = view_of(name, None, root)
        except KeyError:
            raise ValueError(f"no view {name!r}: {', '.join(v.name for v in saved(root))}, or {WORKBENCH}") from None
        query = _view_query(view, params)
    if sel is not None:
        view = view_of(name, params, root) if name == WORKBENCH else view
        canon, _ = selection(view, sel)
        query.append((SEL, js_json(canon)))
    host = (base if base is not None else default_base()).rstrip("/")
    qs = "&".join(f"{form(k)}={form(v)}" for k, v in query)
    return f"{host}{PAGE}{name}" + (f"?{qs}" if qs else "")


def parse(url, root=VIEWS):
    """(name, params, sel) of console address `url` (link's inverse): its
    route, the params the page reads from it as text, and its selection
    as given (selection checks it). ValueError for an address of no view."""
    if not isinstance(url, str):
        raise ValueError("an address is text")
    fragment = urllib.parse.urlsplit(url.strip()).fragment
    path, _, qs = fragment.partition("?")
    if path != "/console" and not path.startswith("/console/"):
        raise ValueError(f"not a console address (…/devtools.html#/console/VIEW?…): {url}")
    name = path[len("/console/"):].split("/")[0] or "experiments"
    query = {}
    for k, v in urllib.parse.parse_qsl(qs, keep_blank_values=True):
        query.setdefault(k, v)
    sel = None
    if query.get(SEL):
        try:
            sel = json.loads(query[SEL])
        except ValueError:
            raise ValueError(f"the address's sel isn't JSON: {query[SEL]}") from None
    if name == WORKBENCH:
        params = {}
        for k in ("view", "sql"):
            if query.get(k):
                params[k] = query[k]
        try:
            statement = params.get("sql") or (get(params["view"], root).sql if params.get("view") else None)
        except KeyError:
            raise ValueError(f"{url}: the workbench's view {params['view']!r} is no saved view") from None
        if statement is None:
            names = [k for k in KEYS if query.get(k)]
        else:
            names = [n for n in statement_params(statement) if n not in WORKBENCH_KEYS]
            names = [k for k in KEYS if k in names] + [n for n in names if n not in KEYS]
        for n in names:
            v = query.get(n, DEFAULTS.get(n, ""))
            if v != "":
                params[n] = v
        params.update((k, query[k]) for k in CHART if query.get(k))
        return name, params, sel
    if name in ROUTES:
        raise ValueError(f"{url} is the console's {name} page, which shows no view")
    try:
        view = get(name, root)
    except KeyError:
        raise ValueError(f"{url} names no view {name!r}: {', '.join(v.name for v in saved(root))}") from None
    params = {}
    for p in view.params:
        key = url_key(p)
        if key is not None:
            v = query.get(key, DEFAULTS[key])
            if v != "":
                params[p.name] = v
    return name, params, sel


# --- panels ---------------------------------------------------------------------------------------


@dataclasses.dataclass
class Filter:
    """A filter transform: its predicate as the spec gives it (an
    expression, or a predicate object's JSON), its SQL (None where it
    doesn't translate, and why), and the layer it filters (None: the
    panel's every layer)."""
    text: str
    sql: str = None
    why: str = None
    layer: int = None


@dataclasses.dataclass
class Layer:
    """A mark of a panel: its type, whether it draws data of its own (none
    of the view's rows), its encoding (with the encoding it inherits) and
    its own filters."""
    mark: str
    own: bool
    encoding: dict
    filters: list


@dataclasses.dataclass
class Panel:
    """A panel of a spec: a unit, layer or facet spec, a member of a
    concat or the outermost spec. `id` is its title (a string, or
    title.text) where no other panel has it, else `path`, its index path
    through the concats (1, or 0.2 when nested; 0 for the outermost
    spec). `time` is the outermost spec's hover.x where the panel carries
    a `hover` key (hover.ts's time panels), and `x2` the column its x2
    encodes beside it. `encoded` are the columns of the view's rows its
    marks encode (SELECTABLE's channels, the facet's), less the fields its
    transforms make; `filters` its transforms' and its outer specs'."""
    id: str
    path: str
    title: str
    time: str
    x2: str
    layers: list
    facet: dict
    encoded: list
    filters: list


@dataclasses.dataclass(frozen=True)
class Rows:
    """What a chain of transforms leaves of the view's columns: the fields
    it made, and the columns past a groupby's (None: every one)."""
    made: frozenset = frozenset()
    kept: frozenset = None

    def column(self, f):
        return isinstance(f, str) and f not in self.made and (self.kept is None or f in self.kept)


def _names(v):
    if isinstance(v, str):
        return [v]
    return [x for x in v if isinstance(x, str)] if isinstance(v, list) else []


def _transforms(s):
    t = s.get("transform")
    return [x for x in t if isinstance(x, dict)] if isinstance(t, list) else []


def scan(transforms, rows=Rows(), layer=None):
    """(rows, filters): what `transforms` leave of the columns, from
    `rows`, and their filters, each translated where its fields are
    columns at its place."""
    filters = []
    for t in transforms:
        if "filter" in t:
            f = t["filter"]
            text = f if isinstance(f, str) else json.dumps(f, ensure_ascii=False)
            try:
                filters.append(Filter(text, filter_sql(f, rows.column), layer=layer))
            except Untranslated as e:
                filters.append(Filter(text, None, str(e), layer))
            continue
        made = set(_names(t.get("as")))
        for k in ("aggregate", "joinaggregate", "window"):
            if isinstance(t.get(k), list):
                made |= {a["as"] for a in t[k] if isinstance(a, dict) and isinstance(a.get("as"), str)}
        if "fold" in t and "as" not in t:
            made |= {"key", "value"}
        if "lookup" in t and "as" not in t and isinstance(t.get("from"), dict):
            made |= set(_names(t["from"].get("fields")))
        kept = rows.kept
        if any(k in t for k in GROUPED):
            groupby = frozenset(_names(t.get("groupby")))
            kept = groupby if kept is None else kept & groupby
        rows = Rows(rows.made | made, kept)
    return rows, filters


def _mark(s):
    m = s.get("mark")
    return m if isinstance(m, str) else m.get("type") if isinstance(m, dict) else None


def _title(s):
    t = s.get("title")
    if isinstance(t, dict):
        t = t.get("text")
    if isinstance(t, list):
        t = " ".join(str(x) for x in t)
    return t if isinstance(t, str) and t else None


def _fields(defs):
    """A channel's fields that are a row's values: its definitions (a
    list for detail and tooltip) with a field and no aggregate (a binned
    field's rows keep their values)."""
    defs = defs if isinstance(defs, list) else [defs]
    return [d["field"] for d in defs if isinstance(d, dict) and isinstance(d.get("field"), str)
            and not d.get("aggregate")]


def panels(spec):
    """Spec `spec`'s panels (Panel), in its concats' order."""
    if not isinstance(spec, dict):
        return []
    hover = spec.get("hover")
    top_time = hover.get("x") if isinstance(hover, dict) and isinstance(hover.get("x"), str) else None
    out = []

    def panel(s, path, outer, top):
        chain = outer + _transforms(s)
        facet, inner = {}, s
        if isinstance(s.get("facet"), dict) and isinstance(s.get("spec"), dict):
            f = s["facet"]
            facet = {"facet": f} if "field" in f else {k: v for k, v in f.items() if isinstance(v, dict)}
            inner = s["spec"]
            chain = chain + _transforms(inner)
        rows, filters = scan(chain)
        own = (not top and "data" in s) or (inner is not s and "data" in inner)
        layers = []

        def flat(u, enc, own, extra, root):
            e = {**enc, **(u.get("encoding") if isinstance(u.get("encoding"), dict) else {})}
            own = own or (not root and "data" in u)
            tf = extra + ([] if root else _transforms(u))
            if isinstance(u.get("layer"), list):
                for c in u["layer"]:
                    if isinstance(c, dict):
                        flat(c, e, own, tf, False)
            else:
                layers.append((Layer(_mark(u), own, e, []), tf))

        flat(inner, {}, own, [], True)
        time = top_time if "hover" in s else None
        encoded, x2, done = [], None, []
        for ch, d in facet.items():
            encoded += [f for f in _fields(d) if rows.column(f)]
        for i, (layer, tf) in enumerate(layers):
            lrows, lfilters = scan(tf, rows, i)
            layer.filters = lfilters
            done.append(layer)
            if layer.own:
                continue
            for ch in SELECTABLE:
                encoded += [f for f in _fields(layer.encoding.get(ch)) if lrows.column(f)]
            xs, x2s = _fields(layer.encoding.get("x")), _fields(layer.encoding.get("x2"))
            if time and x2 is None and xs == [time] and x2s and x2s[0] != time and lrows.column(x2s[0]):
                x2 = x2s[0]
        out.append(Panel(None, path, _title(s), time, x2, done, facet, list(dict.fromkeys(encoded)), filters))

    def walk(s, path, outer, top):
        if s.get("mark") is not None or isinstance(s.get("layer"), list) or (
                isinstance(s.get("facet"), dict) and isinstance(s.get("spec"), dict)):
            panel(s, path or "0", outer, top)
            return
        chain = outer + _transforms(s)
        for k in CONCATS:
            if isinstance(s.get(k), list):
                for i, c in enumerate(s[k]):
                    if isinstance(c, dict):
                        walk(c, f"{path}.{i}" if path else str(i), chain, False)

    walk(spec, "", [], True)
    titles = [p.title for p in out]
    for p in out:
        p.id = p.title if p.title and titles.count(p.title) == 1 and p.title not in {q.path for q in out} else p.path
    return out


def time_field(spec):
    """The outermost spec's hover.x, where any panel carries `hover`."""
    ps = panels(spec)
    return next((p.time for p in ps if p.time), None)


# --- filters as SQL ---------------------------------------------------------------------------------


class Untranslated(ValueError):
    """A filter whose form has no SQL here."""


TOKEN = re.compile(r"""\s*(?:(?P<op>===|!==|==|!=|<=|>=|&&|\|\||[!()<>\[\].])|(?P<num>\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)
                       |(?P<str>'(?:[^'\\]|\\.)*'|"(?:[^"\\]|\\.)*")|(?P<name>[A-Za-z_$][\w$]*))""", re.X)


ESCAPES = {"n": "\n", "t": "\t", "r": "\r", "b": "\b", "f": "\f", "v": "\v", "0": "\0"}


def _unquoted(s):
    """A JavaScript string literal's value."""
    out, i, body = [], 0, s[1:-1]
    while i < len(body):
        c = body[i]
        if c != "\\":
            out.append(c)
            i += 1
            continue
        e = body[i + 1]
        if e in "ux":
            n = 4 if e == "u" else 2
            digits = body[i + 2:i + 2 + n]
            if not re.fullmatch(r"[0-9A-Fa-f]{%d}" % n, digits):
                raise Untranslated(f"a string it can't read: {s}")
            out.append(chr(int(digits, 16)))
            i += 2 + n
            continue
        out.append(ESCAPES.get(e, e))
        i += 2
    return "".join(out)


def _tokens(text):
    out, i = [], 0
    while i < len(text):
        m = TOKEN.match(text, i)
        if not m or m.end() == i:
            if text[i:].strip() == "":
                break
            raise Untranslated(f"{text[i:].strip()[:20]!r} isn't an expression's part this translates")
        i = m.end()
        kind = m.lastgroup
        v = m.group(kind)
        out.append((kind, _unquoted(v) if kind == "str" else v))
    return out


def sql_name(f):
    return '"' + f.replace('"', '""') + '"'


def sql_literal(v):
    if v is None:
        return "NULL"
    if isinstance(v, bool):
        return "TRUE" if v else "FALSE"
    if isinstance(v, (int, float)):
        return js_number(v)
    return "'" + v.replace("'", "''") + "'"


def filter_sql(f, column=lambda name: True):
    """Filter predicate `f` as a SQL condition: an expression of `datum.F`
    (or datum['F']) and literals compared by === and !==, joined by &&, ||
    and !, in parentheses as needed, each field a column (`column(F)`).
    Untranslated says why another form doesn't: a predicate object, a
    function, another operator, a field a transform made."""
    if not isinstance(f, str):
        raise Untranslated("a predicate object, not an expression")
    toks = _tokens(f)
    i = 0

    def peek(*want):
        return i < len(toks) and toks[i][1] in want and toks[i][0] == "op"

    def take():
        nonlocal i
        if i >= len(toks):
            raise Untranslated("the expression ends early")
        i += 1
        return toks[i - 1]

    def operand():
        kind, v = take()
        if kind == "name" and v == "datum":
            k, op = take()
            if (k, op) == ("op", "."):
                k, name = take()
                if k != "name":
                    raise Untranslated("datum. takes a field's name")
            elif (k, op) == ("op", "["):
                k, name = take()
                if k != "str" or take() != ("op", "]"):
                    raise Untranslated("datum[...] takes a string")
            else:
                raise Untranslated("datum alone is no field")
            if peek(".", "[", "("):
                raise Untranslated(f"datum.{name}'s parts or calls")
            if not column(name):
                raise Untranslated(f"{name} is a field a transform makes, no column of the view's rows")
            return sql_name(name)
        if kind == "num":
            return v
        if kind == "str":
            return sql_literal(v)
        if kind == "name" and v in ("true", "false", "null"):
            return {"true": "TRUE", "false": "FALSE", "null": "NULL"}[v]
        if kind == "name":
            raise Untranslated(f"{v}: only datum's fields and literals translate")
        raise Untranslated(f"{v!r} where a field or a literal goes")

    def compare():
        if peek("("):
            take()
            out = either()
            if not peek(")"):
                raise Untranslated("a parenthesis left open")
            take()
            return out
        left = operand()
        if i < len(toks) and toks[i][0] == "op" and toks[i][1] in ("===", "!=="):
            op = take()[1]
            right = operand()
            return f"{left} {'IS NOT DISTINCT FROM' if op == '===' else 'IS DISTINCT FROM'} {right}"
        if i < len(toks) and toks[i][0] == "op" and toks[i][1] in ("==", "!=", "<", "<=", ">", ">="):
            raise Untranslated(f"{toks[i][1]}: only === and !== compare here (JavaScript's others coerce)")
        raise Untranslated("a field or a literal alone is no comparison (its truthiness is JavaScript's)")

    def negated():
        if peek("!"):
            take()
            return f"NOT ({negated()})"
        return compare()

    def both():
        out = [negated()]
        while peek("&&"):
            take()
            out.append(negated())
        return out[0] if len(out) == 1 else "(" + " AND ".join(out) + ")"

    def either():
        out = [both()]
        while peek("||"):
            take()
            out.append(both())
        return out[0] if len(out) == 1 else "(" + " OR ".join(out) + ")"

    out = either()
    if i < len(toks):
        raise Untranslated(f"{toks[i][1]!r} past the expression's end")
    return out


# --- selections -----------------------------------------------------------------------------------


def _scalar(v):
    return v is None or isinstance(v, (str, bool)) or (isinstance(v, (int, float)) and math.isfinite(v))


def _number(v):
    return isinstance(v, (int, float)) and not isinstance(v, bool) and math.isfinite(v)


def selection(view, sel):
    """`sel` checked against `view`'s graph (the module docs): (the
    selection as the address holds it, its Panel or None). ValueError
    says what's wrong: a field no panel (or not its panel) encodes, a
    range on a field other than the time field, a panel the graph lacks."""
    if not isinstance(sel, dict) or set(sel) - {"panel", "at"}:
        raise ValueError('a selection is {"panel"?: ID, "at": {FIELD: VALUE | [LO, HI], ...}}')
    ps = panels(view.spec)
    if not ps:
        raise ValueError(f"{view.name} draws no graph to select on")
    at = sel.get("at")
    if not isinstance(at, dict) or not at:
        raise ValueError("a selection's at names a field or more: {FIELD: VALUE | [LO, HI], ...}")
    panel = None
    if "panel" in sel:
        want = sel["panel"]
        panel = next((p for p in ps if p.id == want), None) or next((p for p in ps if p.path == want), None)
        if panel is None:
            raise ValueError(f"{view.name} has no panel {want!r}: its panels are "
                             + ", ".join(json.dumps(p.id, ensure_ascii=False) for p in ps))
    elif len(ps) == 1:
        panel = ps[0]
    if panel:
        fields, time = panel.encoded, panel.time
        scope = f"panel {json.dumps(panel.id, ensure_ascii=False)}" if len(ps) > 1 else view.name
    else:
        fields = list(dict.fromkeys(f for p in ps for f in p.encoded))
        time, scope = time_field(view.spec), view.name
    for f, v in at.items():
        if f not in fields:
            raise ValueError(f"{scope} encodes no column {f!r}: its columns are {', '.join(fields) or 'none'}")
        if isinstance(v, list):
            if len(v) != 2 or not all(_number(x) for x in v) or v[0] > v[1]:
                raise ValueError(f"{f}'s range is [LO, HI], two numbers, LO first: not {js_json(v)}")
            if f != time:
                raise ValueError(f"a range is on {scope}'s time field"
                                 + (f", {time}" if time else ", which it has none of") + f": not {f}")
        elif not _scalar(v):
            raise ValueError(f"{f}'s value is a string, number, boolean or null, or a range [LO, HI]")
    out = {"panel": panel.id} if panel and len(ps) > 1 else {}
    out["at"] = {k: at[k] for k in sorted(at)}
    return out, panel


@dataclasses.dataclass
class Where:
    """A selection's rows: `sql` the condition over the view's rows
    (bound from `params`), `parts` its parts in words, `unapplied` the
    panel filters that don't translate (with why), which it leaves out."""
    sel: dict
    panel: Panel
    sql: str
    params: dict
    parts: list
    unapplied: list

    def wrap(self, statement):
        """`statement`'s rows that the condition keeps."""
        body = statement.rstrip().rstrip(";")
        return f"SELECT * FROM (\n{body}\n) AS view_rows WHERE {self.sql}"


def where(view, sel):
    """Selection `sel`'s condition over `view`'s rows (Where): its panel's
    filters that translate, each value's equality, and its range's
    BETWEEN, or overlap where the panel encodes x2. ValueError for a bad
    selection (selection)."""
    sel, panel = selection(view, sel)
    taken = set(statement_params(view.sql))
    conds, params, parts, unapplied = [], {}, [], []

    def bind(v):
        n = len(params)
        name = f"_sel{n}"
        while name in taken:
            name = "_" + name
        params[name] = v
        return f"${name}"

    if panel:
        for f in panel.filters:
            if f.sql is None:
                unapplied.append(f"{f.text} ({f.why})")
            else:
                conds.append(f.sql)
                parts.append(f.text)
        # The panel's rows are its layers': those a layer's filters keep,
        # for any layer drawing the view's rows. A layer without filters,
        # or with one that doesn't translate, keeps them all.
        layered = [layer for layer in panel.layers if not layer.own]
        for layer in layered:
            unapplied += [f"{f.text} (layer {f.layer}: {f.why})" for f in layer.filters if f.sql is None]
        if layered and all(layer.filters and all(f.sql for f in layer.filters) for layer in layered):
            terms = [layer.filters for layer in layered]
            conds.append("(" + " OR ".join(" AND ".join(f.sql for f in t) for t in terms) + ")")
            parts.append(" || ".join(" && ".join(f.text for f in t) for t in terms))
    for f, v in sel["at"].items():
        col = sql_name(f)
        if isinstance(v, list):
            lo, hi = bind(v[0]), bind(v[1])
            if panel and panel.x2:
                conds.append(f"{col} <= {hi} AND coalesce({sql_name(panel.x2)}, {col}) >= {lo}")
                parts.append(f"{f} to {panel.x2} overlaps {js_number(v[0])} to {js_number(v[1])}")
            else:
                conds.append(f"{col} BETWEEN {lo} AND {hi}")
                parts.append(f"{f} from {js_number(v[0])} to {js_number(v[1])}")
        elif v is None:
            conds.append(f"{col} IS NULL")
            parts.append(f"{f} is null")
        else:
            conds.append(f"{col} = {bind(v)}")
            parts.append(f"{f} = {js_json(v)}")
    return Where(sel, panel, " AND ".join(conds) or "TRUE", params, parts, unapplied)


def selection_text(view, sel):
    """Selection `sel` in a line: its panel's id, then each field's value
    or range."""
    sel, panel = selection(view, sel)
    out = [panel.id] if panel and "panel" in sel else []
    for f, v in sel["at"].items():
        out.append(f"{f} {js_number(v[0])}–{js_number(v[1])}" if isinstance(v, list) else f"{f} {js_json(v)}")
    return " · ".join(out)


# --- checks over the catalog ----------------------------------------------------------------------


def check(pool, name, params=None, sel=None, root=VIEWS):
    """What's wrong with the address of `name`, `params` and `sel` over
    the catalog `pool` reads, one string a problem (none: it shows rows):
    a view or selection that doesn't check, an experiment the catalog
    lacks, a cell, arm or arm2 none of its jobs have, a seed that didn't
    run (in that cell and arm, where given), a workbench chart's column
    the statement lacks, a param missing or the query failing, and no
    rows (or none the selection matches)."""
    params = dict(params or {})
    try:
        link(name, params, sel, base=BASE, root=root)
        view = view_of(name, params, root)
    except KeyError as e:
        return [e.args[0]]
    except ValueError as e:
        return [str(e)]
    values = values_of(view, params)
    by_kind = {}
    for p in view.params:
        v = values.get(p.name)
        if p.kind in ("exp", "cell", "arm", "arm2", "seed") and v not in (None, ""):
            by_kind.setdefault(p.kind, v)
    problems = []

    def rows(sql, args):
        return pool.query(sql, args, max_rows=1000, timeout_s=60).values()

    try:
        exp = by_kind.get("exp")
        if exp is not None:
            if not rows("SELECT 1 FROM experiments WHERE exp_id = $exp", {"exp": exp}):
                return [f"no experiment {exp!r} in the catalog"]
            for kind, col in (("cell", "cell"), ("arm", "arm_label"), ("arm2", "arm_label")):
                v = by_kind.get(kind)
                if v is None:
                    continue
                have = [r[0] for r in rows(f"SELECT DISTINCT {col} FROM experiment_jobs WHERE exp_id = $exp "
                                           f"ORDER BY 1", {"exp": exp})]
                if v not in have:
                    problems.append(f"{exp} has no {kind} {v!r}: its {'arm' if col == 'arm_label' else kind}s are "
                                    f"{', '.join(map(str, have)) or 'none'}")
            if "seed" in by_kind and not problems:
                cond = " AND ".join(["exp_id = $exp"] + [f"{col} = ${k}" for k, col in (("cell", "cell"),
                                                                                       ("arm", "arm_label"))
                                                         if k in by_kind])
                args = {"exp": exp, **{k: by_kind[k] for k in ("cell", "arm") if k in by_kind}}
                ran = [r[0] for r in rows(f"SELECT DISTINCT seed FROM experiment_jobs WHERE {cond} ORDER BY 1", args)]
                if by_kind["seed"] not in ran:
                    where_ = " ".join(str(by_kind[k]) for k in ("cell", "arm") if k in by_kind)
                    problems.append(f"seed {by_kind['seed']} didn't run in {exp}" + (f" {where_}" if where_ else "")
                                    + f": its seeds are {', '.join(map(str, ran)) or 'none'}")
        if problems:
            return problems
        if name == WORKBENCH:
            got = pool.query(view.sql, values, max_rows=1, timeout_s=60)
            cols = got.names
            for k in ("x", "y", "color"):
                if params.get(k) and params[k] not in cols:
                    problems.append(f"the statement has no column {params[k]!r} for the chart's {k}: its columns "
                                    f"are {', '.join(cols)}")
            if problems:
                return problems
        got = run(pool, name, params, root=root, max_rows=1, timeout_s=60, sel=sel)
        if not got.rows:
            problems.append(f"the selection matches no row of {name}" if sel is not None
                            else f"{name} has no rows for these params")
    except QueryError as e:
        problems.append(f"{name}: {e}")
    return problems


# --- graphs ---------------------------------------------------------------------------------------


def vocabularies():
    """{vocabulary: [(name, colour)]}: the event vocabulary's phases,
    episodes and reasons in its order (vocab.py's document, which vocab.ts
    is made from)."""
    from . import vocab

    return {s: [(n, t["colour"]) for n, t in vocab.entries(s).items()] for s in VOCAB_SECTIONS}


def _named(ref, at, vocabs):
    names = [ref.get("vocab")] if isinstance(ref.get("vocab"), str) else ref.get("vocab")
    if not (isinstance(names, list) and names and all(isinstance(x, str) for x in names)):
        raise ValueError(f"{at}: vocab is a vocabulary or a list of them")
    found, seen = [], {}
    for v in names:
        if v not in vocabs:
            raise ValueError(f"{at}: no vocabulary {json.dumps(v)} ({', '.join(vocabs)})")
        for n, colour in vocabs[v]:
            if n in seen:
                raise ValueError(f"{at}: {json.dumps(n)} is a name of both {seen[n]} and {v}")
            seen[n] = v
            found.append((n, colour))
    only = ref.get("only")
    if only is None:
        return found
    if not (isinstance(only, list) and only and all(isinstance(x, str) for x in only)):
        raise ValueError(f"{at}: only is a list of names")
    for n in only:
        if n not in seen:
            raise ValueError(f"{at}: {json.dumps(n)} is no name of {' or '.join(names)}")
    if len(set(only)) != len(only):
        raise ValueError(f"{at}: only lists a name twice")
    return [x for x in found if x[0] in set(only)]


def expand_vocab(spec, vocabs=None):
    """`spec` with its vocabulary references expanded, as domains.ts's
    expandVocab expands them: a scale's `{"vocab": V, "only"?: [...]}` its
    domain and range (the names and their colours), a sort's the names.
    ValueError for a reference out of place, or to a vocabulary or name
    the document lacks."""
    vocabs = vocabularies() if vocabs is None else vocabs

    def walk(v, at):
        if isinstance(v, list):
            return [walk(x, f"{at}[{i}]") for i, x in enumerate(v)]
        if not isinstance(v, dict):
            return v
        if "vocab" in v:
            raise ValueError(f"{at or 'spec'}: a vocab reference stands for a scale or a sort")
        out = {}
        for k, x in v.items():
            here = f"{at}.{k}" if at else k
            if k in ("data", "datum"):
                out[k] = x
            elif k == "scale" and isinstance(x, dict) and "vocab" in x:
                rest = {a: b for a, b in x.items() if a not in ("vocab", "only")}
                for bad in ("domain", "range", "scheme"):
                    if bad in rest:
                        raise ValueError(f"{here}: a vocab scale takes no {bad}: the document gives it")
                names = _named(x, here, vocabs)
                if len({c for _, c in names}) != len(names):
                    raise ValueError(f"{here}: two of its names have one colour")
                out[k] = {**rest, "domain": [n for n, _ in names], "range": [c for _, c in names]}
            elif k == "sort" and isinstance(x, dict) and "vocab" in x:
                other = [a for a in x if a not in ("vocab", "only")]
                if other:
                    raise ValueError(f"{here}: a vocab sort takes no {', '.join(other)}")
                out[k] = [n for n, _ in _named(x, here, vocabs)]
            else:
                out[k] = walk(x, here)
        return out

    return walk(spec, "")


def _channel(ch, d):
    """A channel's field, and its type, title and scale type where it
    gives them (a title or scale of null: none shown, none)."""
    out = {"channel": ch, "field": d["field"]}
    if d.get("type"):
        out["type"] = d["type"]
    if "title" in d:
        out["title"] = d["title"]
    if "scale" in d:
        if d["scale"] is None:
            out["scale"] = None
        elif isinstance(d["scale"], dict) and d["scale"].get("type"):
            out["scale"] = d["scale"]["type"]
    return out


def _channels(enc):
    """An encoding's channels with a field, but its tooltip."""
    out = []
    for ch, d in enc.items():
        for x in (d if isinstance(d, list) else [d]):
            if ch != "tooltip" and isinstance(x, dict) and isinstance(x.get("field"), str):
                out.append(_channel(ch, x))
    return out


def _legend(p):
    for layer in p.layers:
        if layer.own:
            continue
        for ch in COLOURS:
            d = layer.encoding.get(ch)
            if isinstance(d, dict) and isinstance(d.get("field"), str):
                scale = d.get("scale") if isinstance(d.get("scale"), dict) else {}
                domain, colours = scale.get("domain"), scale.get("range")
                values = None
                if isinstance(domain, list):
                    colours = colours if isinstance(colours, list) else []
                    values = [[v, colours[i] if i < len(colours) else None] for i, v in enumerate(domain)]
                return {"channel": ch, "field": d["field"], "values": values}
    return None


def graph(view):
    """What `view`'s spec draws, as JSON's values: {view, title, time,
    panels: [{id, path, title, time, x2, marks: [{mark, own_data,
    channels: [{channel, field, type?, title?, scale?}], filters}],
    facet, encoded, filters, legend: {channel, field, values: [[value,
    colour]] or None for the theme's} or None}]}, a filter {text, sql,
    why?}; its vocabulary references expanded (expand_vocab)."""
    spec = expand_vocab(view.spec) if view.spec is not None else None
    ps = panels(spec)

    def filt(f):
        return {"text": f.text, "sql": f.sql, **({"why": f.why} if f.why else {})}

    return {"view": view.name, "title": view.title, "time": time_field(spec), "panels": [{
        "id": p.id, "path": p.path, "title": p.title, "time": p.time, "x2": p.x2,
        "marks": [{"mark": layer.mark, "own_data": layer.own, "channels": _channels(layer.encoding),
                   "filters": [filt(f) for f in layer.filters]} for layer in p.layers],
        "facet": _channels(p.facet), "encoded": p.encoded, "filters": [filt(f) for f in p.filters],
        "legend": _legend(p)} for p in ps]}


# --- the server and its tabs ----------------------------------------------------------------------


class NoServer(RuntimeError):
    """No console server answers: the message says how to start one."""


def state_path():
    """The running console server's state, target/console/server.json in
    the main checkout ({pid, port, urls}: its loopback URL, then those it
    gave for the network), which hack/console/server.py writes at its
    start and removes at its exit."""
    from . import store_host

    return store_host.main_checkout() / "target" / "console" / "server.json"


def _alive(pid):
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    except (OverflowError, ValueError, TypeError):
        return False
    return True


def server_state():
    """state_path()'s state while its process runs, else None."""
    try:
        state = json.loads(state_path().read_text())
    except (OSError, ValueError):
        return None
    if not isinstance(state, dict) or not isinstance(state.get("urls"), list) or not state["urls"]:
        return None
    return state if _alive(state.get("pid")) else None


def origin(url):
    """`url`'s scheme://host[:port]."""
    u = urllib.parse.urlsplit(url)
    return f"{u.scheme}://{u.netloc.rpartition('@')[2]}"


def server():
    """The console server tabs report to: the running one's loopback URL
    (server_state), else SIM_CONSOLE_URL. NoServer for neither."""
    state = server_state()
    if state:
        return origin(state["urls"][0])
    if os.environ.get("SIM_CONSOLE_URL"):
        return origin(os.environ["SIM_CONSOLE_URL"])
    raise NoServer(f"no console server is running: start one with {SERVER} (after {BUILD})")


def tabs(server_url=None):
    """The open console tabs' last reports (GET /console/tabs at
    `server_url`, server()'s by default): [{id, url, focused, visible,
    last_seen}], last_seen in seconds, the latest focused first."""
    at = (server_url or server()).rstrip("/")
    try:
        with urllib.request.urlopen(f"{at}/console/tabs", timeout=TAB_TIMEOUT_S) as r:
            return json.loads(r.read().decode("utf-8"))
    except urllib.error.HTTPError as e:
        raise NoServer(f"the console server at {at} answers {e.code} to GET /console/tabs: restart it ({SERVER}) "
                       "for the tab reports") from None
    except (OSError, ValueError) as e:
        raise NoServer(f"no console server answers at {at} ({getattr(e, 'reason', e)}): start one with {SERVER} "
                       f"(after {BUILD})") from None


def tab(id=None, server_url=None):
    """Tab `id`'s last report, or with none the latest focused tab's:
    LookupError for none."""
    got = tabs(server_url)
    if not got:
        raise LookupError("no console tab has reported its address: open the console in a browser "
                          f"({BASE}/ or the server's address)")
    if id is None:
        return got[0]
    for t in got:
        if t.get("id") == id:
            return t
    raise LookupError(f"no tab {id!r}: the tabs are {', '.join(t.get('id', '?') for t in got)}")


def base():
    """The host a link is on: SIM_CONSOLE_URL; else the origin of the
    latest focused tab's address, the host the user's browser reaches;
    else the running server's first LAN URL, else its loopback URL; else
    BASE."""
    if os.environ.get("SIM_CONSOLE_URL"):
        return os.environ["SIM_CONSOLE_URL"].rstrip("/")
    state = server_state()
    if not state:
        return BASE
    try:
        for t in tabs(origin(state["urls"][0])):
            if isinstance(t.get("url"), str) and urllib.parse.urlsplit(t["url"]).netloc:
                return origin(t["url"])
    except NoServer:
        pass
    urls = [u for u in state["urls"] if isinstance(u, str)]
    lan = [u for u in urls if not _loopback(urllib.parse.urlsplit(u).hostname)]
    return origin((lan or urls)[0]) if urls else BASE


def _loopback(host):
    if host in (None, "localhost"):
        return True
    try:
        return ipaddress.ip_address(host).is_loopback
    except ValueError:
        return False


default_base = base

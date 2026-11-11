#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The sim console's addresses from the shell (simlab.views): what a
console graph draws and what the user selected on it, from a console URL
or their open tab, the rows behind that selection, and links that open a
view with a value or a time range highlighted.

  python3 hack/console/ui.py describe [VIEW | URL | --tab [ID]]
  python3 hack/console/ui.py rows (URL | --tab [ID]) [--json] [--max-rows N]
  python3 hack/console/ui.py tabs
  python3 hack/console/ui.py link VIEW [--param NAME=VALUE ...] [--sel JSON] [--check] [--base URL]
  python3 hack/console/ui.py link workbench (--sql SQL | --view NAME) [--chart MARK,x=F,y=F,color=F,logy]
                                [--param NAME=VALUE ...] [--sel JSON] [--check] [--base URL]

`describe VIEW` says what a saved view's graph draws: each panel's id
(its title, else its index path), marks and channels, its time field
(the panels whose cursor and time ranges are shared), its filters, the
columns a selection may name, and its colour legend (the event
vocabulary's colours where it names a vocabulary). `describe URL`, or
`--tab` (the most recently focused console tab's address, or tab ID's),
says the view, its params, the selection in words, the commands that give
its rows, and its graph. With nothing, the views.

`rows` prints the rows behind an address's selection (the view's rows,
those its selection's panel filters and values keep), as `simdb.py view`
prints a view's: DuckDB's table, or with `--json` objects; what the
address is and any panel filter that doesn't translate to SQL (and so
isn't applied) go to stderr.

`tabs` lists the console tabs open in a browser, as the servers running
(each its own) last heard from them, the latest focused first.

`link` writes an address. Each `--param` is text, as `simdb.py view
--param` takes it; `--sel` the selection, JSON:
{"panel": ID, "at": {FIELD: VALUE | [LO, HI], ...}}, a range on the
panel's time field alone (simlab/views.py's docs); `--check` confirms it
over the catalog (the experiment, its cell, arms and seed, and a row the
selection matches). Its host is `--base`, else SIM_CONSOLE_URL, else the
host of the latest focused tab, else the first running server's (in port
order). For the workbench, `--chart` sets the auto-chart: a mark (line,
point, bar, area, rect, tick), x=, y=, color= and logy.

`describe VIEW`, `describe URL` and `link` without `--check` need neither
the server nor the catalog. `rows` and `--check` read the catalog
(simdb.py's, through the reader `simdb.py view` uses); `--tab` and `tabs`
ask the console servers running (python3 hack/console/server.py, after
npm run console:build).
"""

import argparse
import json
import re
import shlex
import sys
import textwrap
from pathlib import Path

HACK = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(HACK))

from simlab import bootstrap  # noqa: E402

if __name__ == "__main__":
    bootstrap.engine(__file__)

from simlab import engine, views  # noqa: E402

SIMDB = "python3 hack/sim-db/simdb.py"
UI = "python3 hack/console/ui.py"
# The catalog's readers this CLI spills under, as simdb.py's commands do.
READERS = "cli"
PARAM = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")


def param(text):
    """A --param's (name, text)."""
    name, eq, value = text.partition("=")
    if not eq or not PARAM.fullmatch(name):
        raise argparse.ArgumentTypeError(f"{text!r} is no NAME=VALUE")
    return name, value


def pool():
    """The catalog's reader, as simdb.py view opens it; exits when there's
    no catalog."""
    from simlab import catalog

    if not catalog.catalog_path().exists():
        sys.exit(f"no catalog at {catalog.catalog_path()}: run {SIMDB} index")
    return catalog.pool(READERS)


def address(a, cmd):
    """The address an invocation names: (url, the tab's report or None)."""
    if a.tab is not None:
        try:
            t = views.tab(a.tab or None)
        except views.NoServer as e:
            sys.exit(f"{cmd}: {e}")
        except LookupError as e:
            sys.exit(f"{cmd}: {e.args[0]}")
        return t["url"], t
    if not a.target:
        sys.exit(f"{cmd}: give a console URL or --tab")
    return a.target, None


def parsed(url, cmd):
    try:
        return views.parse(url)
    except ValueError as e:
        sys.exit(f"{cmd}: {e}")


# --- describe -------------------------------------------------------------------------------------


def channel_text(c):
    more = [c[k] for k in ("type",) if c.get(k)]
    if c.get("scale", ...) is None:
        more.append("no scale")
    elif c.get("scale"):
        more.append(f"{c['scale']} scale")
    if c.get("title"):
        more.append(json.dumps(c["title"], ensure_ascii=False))
    return f"{c['channel']} {c['field']}" + (f" ({', '.join(more)})" if more else "")


def filter_text(f):
    return f["text"] + ("" if f.get("sql") else f"  [no SQL: {f.get('why')}]")


def legend_text(lg):
    head = f"colours, {lg['channel']} {lg['field']}: "
    if lg["values"] is None:
        return head + "the theme's, by the rows' values"
    if all(c is None for _, c in lg["values"]):
        return head + ", ".join(str(v) for v, _ in lg["values"]) + " (the theme's colours, in that order)"
    return head + ", ".join(f"{v} {c}" for v, c in lg["values"])


def graph_lines(g):
    """`views.graph`'s description as lines."""
    if not g["panels"]:
        return [f"graph: none ({g['view']} is a table of its rows)"]
    n = len(g["panels"])
    out = [f"graph: {n} panel{'s' if n > 1 else ''}"
           + (f"; the time panels share the time field {g['time']} (its cursor, and a range's band)" if g["time"]
              else "")]
    for p in g["panels"]:
        head = f"  panel {json.dumps(p['id'], ensure_ascii=False)}" + (f" [{p['path']}]" if p["id"] != p["path"]
                                                                       else "")
        if p["time"]:
            head += f"  (time {p['time']}" + (f", spans to {p['x2']}" if p["x2"] else "") + ")"
        out.append(head)
        for m in p["marks"]:
            chans = " · ".join(channel_text(c) for c in m["channels"])
            out.append(f"    {m['mark']}: " + ("data of its own" + (": " if chans else "") if m["own_data"] else "")
                       + chans)
            out += [f"      filter: {filter_text(f)}" for f in m["filters"]]
        if p["facet"]:
            out.append("    facet: " + " · ".join(channel_text(c) for c in p["facet"]))
        out += [f"    filter: {filter_text(f)}" for f in p["filters"]]
        out.append(f"    selectable: {', '.join(p['encoded']) or 'none'}")
        if p["legend"]:
            out.append(f"    {legend_text(p['legend'])}")
    return out


def param_text(p):
    """A view's param: its name, kind and default, and the address's key
    where that isn't its name."""
    key = views.url_key(p)
    default = f", default {views.js_json(p.default)}" if p.default not in (None, "") else ""
    where = "" if key == p.name else f"; in the address as {key}" if key else "; not in the address"
    return f"{p.name} ({p.kind}{default}{where})"


def view_head(v):
    out = [f"{v.name}: {v.title}"]
    if v.description:
        out += textwrap.wrap(v.description, width=100, initial_indent="  ", subsequent_indent="  ")
    return out


def rows_commands(name, params, view, url):
    """The commands that give an address's rows: ui.py rows (its
    selection's) and simdb.py's (the view's)."""
    out = [f"{UI} rows {shlex.quote(url)}"]
    flags = "".join(f" --param {shlex.quote(f'{k}={v}')}" for k, v in params.items()
                    if k not in views.WORKBENCH_KEYS)
    if name != views.WORKBENCH:
        out.append(f"{SIMDB} view {name}{flags}")
    elif params.get("view") and not params.get("sql"):
        out.append(f"{SIMDB} view {params['view']}{flags}")
    else:
        out.append(f"{SIMDB} lake {shlex.quote(view.sql.strip())}{flags}")
    return out


def describe_address(url, tab=None):
    name, params, sel = parsed(url, "describe")
    try:
        view = views.view_of(name, params)
    except ValueError as e:
        print(f"{name}: {e}")
        return
    out = view_head(view)
    if tab:
        state = ", ".join(k for k in ("focused", "visible") if tab.get(k)) or "in the background"
        out.append(f"tab {tab['id']}: {state}, heard from {tab.get('last_seen')} s ago")
    out.append(f"address: {url}")
    out.append("params: " + (", ".join(f"{k}={v}" for k, v in params.items() if k != "sql") or "none"))
    if params.get("sql"):
        out += ["statement:", *textwrap.indent(params["sql"].strip(), "  ").splitlines()]
    if sel is None:
        out.append("selection: none")
    else:
        try:
            w = views.where(view, sel)
            out.append(f"selection: {views.selection_text(view, sel)}")
            out.append(f"  its rows: {' and '.join(w.parts)}")
            out += [f"  not applied (no SQL): {u}" for u in w.unapplied]
        except ValueError as e:
            out.append(f"selection: {views.js_json(sel)}, which this graph can't hold: {e}")
    out.append("rows:")
    out += [f"  {c}" for c in rows_commands(name, params, view, url)]
    out.append("")
    try:
        out += graph_lines(views.graph(view))
    except ValueError as e:
        out.append(f"graph: its spec doesn't expand: {e}")
    print("\n".join(out))


def cmd_describe(a):
    if a.tab is not None or (a.target and ("#" in a.target or "://" in a.target)):
        url, tab = address(a, "describe")
        return describe_address(url, tab)
    if a.target:
        if a.target == views.WORKBENCH:
            sys.exit("describe: the workbench's graph is its address's: describe a workbench URL")
        try:
            view = views.get(a.target)
        except KeyError:
            sys.exit(f"describe: no view {a.target!r}: {', '.join(v.name for v in views.saved())}")
        out = view_head(view)
        if view.params:
            out.append("params: " + ", ".join(param_text(p) for p in view.params))
        out.append("")
        out += graph_lines(views.graph(view))
        print("\n".join(out))
        return
    saved = views.saved()
    rows = [[v.name, v.title, str(len(views.panels(v.spec)))] for v in saved]
    print(engine.render_table(["view", "title", "panels"], rows), end="")
    print(f"{UI} describe VIEW for a view's graph; describe URL or --tab for an address")


# --- rows, tabs, link -----------------------------------------------------------------------------


def cmd_rows(a):
    url, _ = address(a, "rows")
    name, params, sel = parsed(url, "rows")
    try:
        view = views.view_of(name, params)
        w = views.where(view, sel) if sel is not None else None
    except (KeyError, ValueError) as e:
        sys.exit(f"rows: {e.args[0] if isinstance(e, KeyError) else e}")
    p = pool()
    try:
        r = views.run(p, name, params, max_rows=a.max_rows, timeout_s=None, text=not a.json, sel=sel)
    except (KeyError, ValueError) as e:
        sys.exit(f"rows: {e.args[0] if isinstance(e, KeyError) else e}")
    except engine.QueryError as e:
        sys.exit(f"rows: {name}: {e}")
    finally:
        p.close()
    shown = ", ".join(f"{k}={v}" for k, v in params.items() if k != "sql")
    print(f"rows: {name}" + (f" {shown}" if shown else "") + "; "
          + (f"selection {views.selection_text(view, sel)}: {' and '.join(w.parts)}" if w else "no selection"),
          file=sys.stderr)
    for u in w.unapplied if w else []:
        print(f"rows: not applied (no SQL), so its rows are among these: {u}", file=sys.stderr)
    print(json.dumps(r.records()) if a.json else r.table(), end="\n" if a.json else "")
    if r.truncated:
        print(f"rows: the first {a.max_rows:,}; --max-rows for more", file=sys.stderr)


def cmd_tabs(a):
    try:
        got = views.tabs()
    except views.NoServer as e:
        sys.exit(f"tabs: {e}")
    if not got:
        print("no console tab has reported its address: open the console in a browser")
        return
    rows = []
    for t in got:
        try:
            name = views.parse(t["url"])[0]
        except ValueError:
            name = "?"
        rows.append([t["id"], "yes" if t["focused"] else "no", "yes" if t["visible"] else "no",
                     f"{t['last_seen']:g}", name, t["url"]])
    print(engine.render_table(["tab", "focused", "visible", "seen (s ago)", "view", "address"], rows), end="")


def chart(text):
    """--chart's choices: a mark, x=, y=, color=, logy."""
    out = {}
    for item in (x.strip() for x in text.split(",")):
        if not item:
            continue
        k, eq, v = item.partition("=")
        if not eq:
            if item in views.MARKS:
                out["mark"] = item
            elif item == "logy":
                out["logy"] = "1"
            else:
                raise argparse.ArgumentTypeError(f"{item!r}: a mark ({', '.join(views.MARKS)}), logy, or "
                                                 "x=, y=, color=, mark=")
        elif k in views.CHART:
            out[k] = v
        else:
            raise argparse.ArgumentTypeError(f"{k!r}: the chart takes {', '.join(views.CHART)}")
    return out


def cmd_link(a):
    params = dict(a.param or [])
    if a.name == views.WORKBENCH:
        for k, v in (("sql", a.sql), ("view", a.view)):
            if v is not None:
                params[k] = v
        params.update(a.chart or {})
    elif a.sql is not None or a.view is not None or a.chart:
        sys.exit("link: --sql, --view and --chart are the workbench's")
    sel = None
    if a.sel is not None:
        try:
            sel = json.loads(a.sel)
        except ValueError:
            sys.exit(f"link: --sel isn't JSON: {a.sel}")
    try:
        url = views.link(a.name, params, sel, base=a.base)
    except ValueError as e:
        sys.exit(f"link: {e}")
    if a.check:
        p = pool()
        try:
            problems = views.check(p, a.name, params, sel)
        finally:
            p.close()
        if problems:
            for x in problems:
                print(f"link: {x}", file=sys.stderr)
            sys.exit(1)
    print(url)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    tab_help = "the most recently focused console tab's address, or tab ID's"
    d = sub.add_parser("describe", help="a view's graph, or an address's view, params, selection and graph")
    d.add_argument("target", nargs="?", help="a view's name or a console URL (none: the views)")
    d.add_argument("--tab", nargs="?", const="", metavar="ID", help=tab_help)
    r = sub.add_parser("rows", help="the rows behind an address's selection")
    r.add_argument("target", nargs="?", help="a console URL")
    r.add_argument("--tab", nargs="?", const="", metavar="ID", help=tab_help)
    r.add_argument("--json", action="store_true")
    r.add_argument("--max-rows", type=int, default=10_000, help="the most rows it prints (default 10,000)")
    sub.add_parser("tabs", help="the console tabs open in a browser, and their addresses")
    k = sub.add_parser("link", help="an address of a view or the workbench, with a selection")
    k.add_argument("name", help=f"a saved view, or {views.WORKBENCH}")
    k.add_argument("--param", action="append", type=param, metavar="NAME=VALUE",
                   help="the view's param NAME, as text (a seed's or number's its number)")
    k.add_argument("--sel", metavar="JSON", help='the selection: {"panel": ID, "at": {FIELD: VALUE | [LO, HI]}}')
    k.add_argument("--check", action="store_true", help="confirm it over the catalog first")
    k.add_argument("--base", metavar="URL", help="the host (default: SIM_CONSOLE_URL, else the latest tab's)")
    k.add_argument("--sql", help="the workbench's statement")
    k.add_argument("--view", help="the workbench's saved view (its statement and spec)")
    k.add_argument("--chart", type=chart, metavar="MARK,x=F,y=F,color=F,logy", help="the workbench's auto-chart")
    a = ap.parse_args(argv)
    {"describe": cmd_describe, "rows": cmd_rows, "tabs": cmd_tabs, "link": cmd_link}[a.cmd](a)


if __name__ == "__main__":
    main()

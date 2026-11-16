#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The sim console's addresses from the shell (simlab.views): what a
console graph draws and what the user selected on it, from a console URL
or their open tab, the rows behind that selection, links that open a
view with a value or a time range highlighted, and sending their tab to
one.

  python3 hack/console/ui.py describe [VIEW | URL | --tab [ID]]
  python3 hack/console/ui.py rows (URL | --tab [ID]) [--json] [--max-rows N]
  python3 hack/console/ui.py tabs
  python3 hack/console/ui.py link VIEW [--param NAME=VALUE ...] [--sel JSON] [--check] [--base URL]
  python3 hack/console/ui.py link workbench (--sql SQL | --view NAME) [--chart MARK,x=F,y=F,color=F,logy]
                                [--param NAME=VALUE ...] [--sel JSON] [--check] [--base URL]
  python3 hack/console/ui.py go [VIEW | URL] [--tab ID] [--param NAME=VALUE ...] [--sel JSON | --sel null]
                                [--sql SQL | --view NAME] [--chart ...] [--check] [--wait S]

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

`go` sends a console tab (the latest focused, or `--tab ID`) to an
address and waits for it to say it went there. The address is VIEW's (no
params), the URL's, or with neither the tab's own, and then link's flags
but `--base` change it (the tab stays on its own host): each `--param`
set (NAME= leaves it out), `--sel` in place of the selection (null drops
it), the workbench's `--sql`, `--view` and `--chart`. It prints the
address the tab then reports, and the one it left to stderr. The tab
moves within its own page, a history entry its Back undoes: a tab in
view at once, one in the background at its next report (every 30 s, a
minute where the browser throttles a hidden tab's timers) or when it's
shown. Past `--wait` seconds (default 60, 120 for a tab in the
background) the address is withdrawn and go fails, saying what the tab's
last report gives of why; a tab whose page predates go fails at once
(reload it on a current build).

`describe VIEW`, `describe URL` and `link` without `--check` need neither
the server nor the catalog. `rows` and `--check` read the catalog
(simdb.py's, through the reader `simdb.py view` uses); `--tab`, `tabs`
and `go` ask the console servers running (python3
hack/console/server.py, after pnpm run console:build).
"""

import argparse
import json
import re
import shlex
import sys
import textwrap
import urllib.parse
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
# go's longest wait for its tab (server.py's GOTO_WAIT_MAX_S).
MAX_WAIT_S = 120


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


def changes(a):
    """Whether an invocation's flags change an address."""
    return bool(a.param or a.chart) or any(x is not None for x in (a.sel, a.sql, a.view))


def changed(a, name, params, sel, cmd):
    """(params, sel) of view `name`'s address with an invocation's flags
    applied: each --param set, the workbench's --sql or --view (each in
    place of the other) and --chart, and --sel in place of the selection
    (JSON null drops it)."""
    params = dict(params)
    params.update(a.param or [])
    if name == views.WORKBENCH:
        if a.sql is not None and a.view is None:
            params.pop("view", None)
        if a.view is not None and a.sql is None:
            params.pop("sql", None)
        for k, v in (("sql", a.sql), ("view", a.view)):
            if v is not None:
                params[k] = v
        params.update(a.chart or {})
    elif a.sql is not None or a.view is not None or a.chart:
        sys.exit(f"{cmd}: --sql, --view and --chart are the workbench's")
    if a.sel is not None:
        try:
            sel = json.loads(a.sel)
        except ValueError:
            sys.exit(f"{cmd}: --sel isn't JSON: {a.sel}")
    return params, sel


def checked(name, params, sel, cmd):
    """Exits with what's wrong with an address over the catalog
    (views.check), when anything is."""
    p = pool()
    try:
        problems = views.check(p, name, params, sel)
    finally:
        p.close()
    if problems:
        for x in problems:
            print(f"{cmd}: {x}", file=sys.stderr)
        sys.exit(1)


def cmd_link(a):
    params, sel = changed(a, a.name, {}, None, "link")
    try:
        url = views.link(a.name, params, sel, base=a.base)
    except ValueError as e:
        sys.exit(f"link: {e}")
    if a.check:
        checked(a.name, params, sel, "link")
    print(url)


def route(url):
    """An address's route, its fragment: what a tab goes to."""
    return urllib.parse.urlsplit(url).fragment


def missed(t):
    """Why a tab whose last report is `t` (the server's listing of it)
    didn't go to an address in go's wait. A page that predates go is
    refused at once (409), so the likelier causes: the tab is in the
    background, it left the console's pages (which stops its reports), or
    its goto stream or reports failed."""
    if t and not t.get("visible"):
        return ("it's in the background, where the browser may hold its reports past the wait: bring it up and go "
                "again")
    seen = t.get("last_seen") if t else None
    if isinstance(seen, (int, float)) and seen > views.TAB_REPORT_S + views.TAB_TIMEOUT_S:
        return (f"it hasn't reported for {seen:g} s: it left the console's pages, which stops its reports, or its "
                "reports to the server fail")
    return ("it reports, but took nothing: its goto stream to the server fails (the browser's console says why), or "
            "it left the console's pages just now")


def cmd_go(a):
    if a.wait is not None and not 0 < a.wait <= MAX_WAIT_S:
        sys.exit(f"go: --wait is seconds, more than 0 and at most {MAX_WAIT_S}")
    try:
        t = views.tab(a.tab or None)
    except views.NoServer as e:
        sys.exit(f"go: {e}")
    except LookupError as e:
        sys.exit(f"go: {e.args[0]}")
    # The address it starts from: the URL's, VIEW's with no params, else the tab's own.
    if a.target and ("#" in a.target or "://" in a.target):
        name, params, sel = parsed(a.target, "go")
    elif a.target:
        name, params, sel = a.target, {}, None
    elif changes(a):
        name, params, sel = parsed(t["url"], "go")
    else:
        sys.exit("go: give a VIEW or a URL, or --param or --sel to change the tab's own address")
    params, sel = changed(a, name, params, sel, "go")
    try:
        url = views.link(name, params, sel, base=views.origin(t["url"]))
    except ValueError as e:
        sys.exit(f"go: {e}")
    if a.check:
        checked(name, params, sel, "go")
    wait = a.wait or (views.GOTO_WAIT_S if t.get("visible") else views.GOTO_HIDDEN_WAIT_S)
    if not t.get("visible"):
        print(f"go: tab {t['id']} is in the background: it goes there at its next report (every 30 s, a minute where "
              "the browser throttles a hidden tab's timers) or when it's shown", file=sys.stderr)
    try:
        r = views.goto(url, t["id"], wait_s=wait)
    except views.NoServer as e:
        sys.exit(f"go: {e}")
    except (LookupError, ValueError) as e:
        sys.exit(f"go: {e.args[0]}")
    state = r.get("state")
    if state == "taken":
        now = r["tab"]["url"]
        print(f"go: tab {t['id']} was at {t['url']}", file=sys.stderr)
        if route(now) != route(url):
            print(f"go: the tab shows {now}, not {url}", file=sys.stderr)
        print(now)
        return
    if state == "replaced":
        sys.exit(f"go: a later go to tab {t['id']} replaced this one")
    if state == "gone":
        sys.exit(f"go: tab {t['id']} went away (closed, or unheard from for 2 minutes)")
    sys.exit(f"go: tab {t['id']} didn't go there in {wait:g} s, so the address is withdrawn: " + missed(r.get("tab")))


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
    address_flags(k)
    k.add_argument("--base", metavar="URL", help="the host (default: SIM_CONSOLE_URL, else the latest tab's)")
    g = sub.add_parser("go", help="send a console tab to an address, and wait for it to go there")
    g.add_argument("target", nargs="?", help=f"a saved view, {views.WORKBENCH} or a console URL (none: the tab's "
                                             "own address)")
    g.add_argument("--tab", nargs="?", const="", metavar="ID", help="the tab (default: the most recently focused)")
    address_flags(g)
    g.add_argument("--wait", type=float, metavar="S",
                   help=f"how long it waits for the tab (default {views.GOTO_WAIT_S} s, {views.GOTO_HIDDEN_WAIT_S} for "
                        f"a tab in the background; at most {MAX_WAIT_S})")
    a = ap.parse_args(argv)
    {"describe": cmd_describe, "rows": cmd_rows, "tabs": cmd_tabs, "link": cmd_link, "go": cmd_go}[a.cmd](a)


def address_flags(p):
    """link's and go's flags for what an address holds."""
    p.add_argument("--param", action="append", type=param, metavar="NAME=VALUE",
                   help="the view's param NAME, as text (a seed's or number's its number; NAME= leaves it out)")
    p.add_argument("--sel", metavar="JSON", help='the selection: {"panel": ID, "at": {FIELD: VALUE | [LO, HI]}}, '
                                                 "or null for none")
    p.add_argument("--check", action="store_true", help="confirm it over the catalog first")
    p.add_argument("--sql", help="the workbench's statement")
    p.add_argument("--view", help="the workbench's saved view (its statement and spec)")
    p.add_argument("--chart", type=chart, metavar="MARK,x=F,y=F,color=F,logy", help="the workbench's auto-chart")


if __name__ == "__main__":
    main()

# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The catalog in words, a topic at a time: what `simdb.py describe`
prints, and the catalog's part of the console's Ask prompt
(simlab.ask.system_prompt), which the same functions write.

  describe(topic=None, pool=None, views_root=views.VIEWS) -> str
                 a topic's text: with none, the index of the topics and
                 of every table, view, macro and saved view; else one of
                 TOPICS, or a name, which gives each thing it names (a
                 table or view of the catalog, a macro, a saved view:
                 `events` is a view of the catalog and the event kinds'
                 topic, `experiments` a table and a saved view).
                 ValueError for anything else, naming the topics. `pool`
                 is an engine.Pool of the catalog's readers; glossary and
                 views read files alone
  glossary() -> str
                 glossary.md: the sims' terms and DuckDB's dialect
  glossary_section(title) -> str
                 the text of glossary.md's section `title`, its heading
                 left out: KeyError for none
  catalog_text(schema) -> str
                 the catalog (catalog.schema's parts): main's tables and
                 views with their columns and the catalog's notes, raw's
                 names, the macros, the scored metrics and the event kinds
  views_text(saved, intro, sql=True) -> str
                 the saved views (views.saved's) under a heading and
                 `intro`: each one's title, description, params and, with
                 `sql`, its SQL

Every text is sorted by name and free of counts and times, so the same
catalog and views give the same bytes: the Ask prompt is cached on them.
Nothing here names the Ask panel's tools, which are ask_prompt.md's and
ask.py's to describe.
"""

import difflib
import json
import re
import textwrap
from pathlib import Path

from . import catalog, views

GLOSSARY = Path(__file__).resolve().parent / "glossary.md"
# A markdown file's leading comment (its licence and what it is for).
COMMENT = re.compile(r"\A<!--.*?-->\s*", re.S)
TOPICS = ("glossary", "tables", "macros", "metrics", "events", "views")
MACRO_KINDS = ("macro", "table macro", "macros")
# A column's catalog note, at most this long; a grain in `tables`' lines.
NOTE_CHARS = 240
GRAIN_CHARS = 160
VIEWS_INTRO = ("The console's pages, each one SELECT whose `$name`s its params bind. `simdb.py view NAME --param "
               "name=value` runs one, and `simdb.py describe NAME` gives its SQL.")


def text_of(v):
    """A catalog value as the text writes it: a list or object as compact
    JSON with sorted keys."""
    if v is None:
        return ""
    if isinstance(v, (list, dict)):
        return json.dumps(v, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return str(v)


def clipped(s, n):
    """`s` on one line, at most `n` characters, an ellipsis ending it where
    it was cut."""
    s = " ".join(str(s).split())
    return s if len(s) <= n else s[:n - 1].rstrip() + "…"


def collections_by(rows, key):
    out = {}
    for r in rows:
        out.setdefault(r.get(key), []).append(r)
    return out


# --- the glossary ---------------------------------------------------------------------------------


def glossary():
    """glossary.md, its leading comment left out."""
    return COMMENT.sub("", GLOSSARY.read_text())


def glossary_section(title):
    """The text under glossary.md's `## title`, up to the next heading,
    without the blank lines around it."""
    parts = re.split(r"^## (.+)\n", glossary(), flags=re.M)
    sections = dict(zip(parts[1::2], parts[2::2]))
    if title not in sections:
        raise KeyError(f"glossary.md has no section {title!r}: {', '.join(sections)}")
    return sections[title].strip("\n")


# --- the catalog ----------------------------------------------------------------------------------


def main_tables(schema):
    return sorted((t for t in schema.get("tables") or [] if t["schema"] == "main"), key=lambda t: t["name"])


def about(schema):
    """table_catalog's rows, by name."""
    return {t["name"]: t for t in schema.get("table_catalog") or []}


def raw_line(schema):
    return ("Query the `main` schema's tables and views (the default). `raw` holds the sources they fold, which "
            "you rarely need: " + ", ".join(sorted(t["name"] for t in schema.get("tables") or []
                                                   if t["schema"] == "raw")) + ".")


def table_lines(t, a, columns):
    """Table `t`'s lines (catalog.schema's `tables` entry): its kind, grain,
    keys and joins, description, columns with their types and units, and
    the columns' notes (`a`, its table_catalog row; `columns`, its
    metric_catalog rows), then a blank line."""
    out = [f"### {t['name']}" + (f" ({a['kind']})" if a.get("kind") else "")]
    meta = [f"{k}: {clipped(a[k], 400)}" for k in ("grain", "keys", "joins") if a.get(k)]
    if meta:
        out.append("; ".join(meta))
    if a.get("description"):
        out.append(clipped(a["description"], 2000))
    cat = {m["name"]: m for m in columns}
    cols = []
    for c in t["columns"]:
        unit = (cat.get(c["name"]) or {}).get("unit")
        cols.append(f"{c['name']} {c['type']}" + (f" [{unit}]" if unit else ""))
    out.append("columns: " + ", ".join(cols))
    notes = {}
    for c in t["columns"]:
        d = (cat.get(c["name"]) or {}).get("description")
        if d:
            notes.setdefault(clipped(d, NOTE_CHARS), []).append(c["name"])
    for note, names in sorted(notes.items(), key=lambda kv: kv[1]):
        out.append(f"- {', '.join(names)}: {note}")
    out.append("")
    return out


def tables_lines(schema):
    """Every main table and view, table_lines' each."""
    a = about(schema)
    columns = collections_by(schema.get("metric_catalog") or [], "tbl")
    out = ["## Tables and views", ""]
    for t in main_tables(schema):
        out += table_lines(t, a.get(t["name"], {}), columns.get(t["name"], []))
    return out


def macros(schema):
    return sorted((t for t in schema.get("table_catalog") or [] if t.get("kind") in MACRO_KINDS),
                  key=lambda t: t["name"])


def macro_line(m):
    head = m["name"] + (m["keys"] if m.get("keys") and str(m["keys"]).startswith("(") else "")
    return f"- `{head}` ({m['kind']}): {clipped(m.get('description') or '', 2000)}"


def macros_lines(schema):
    return ["## Macros", ""] + [macro_line(m) for m in macros(schema)]


def metrics_lines(schema):
    """metric_catalog's scored metrics (tbl 'suite')."""
    out = ["## Scored metrics", "",
           "metric_catalog's tbl = 'suite' rows: what the scoring macros score, by name (better: the direction "
           "that is better; kind: ratio or diff; tie: the band within which an effect is a tie; source: the "
           "SQL over a run's row r and verdict v).", ""]
    for m in sorted(collections_by(schema.get("metric_catalog") or [], "tbl").get("suite", []),
                    key=lambda m: m["name"]):
        fields = [f"{k} {text_of(m[k])}" for k in ("better", "kind", "tie", "guard", "unit") if m.get(k) is not None]
        line = f"- {m['name']}: " + ", ".join(fields)
        if m.get("source"):
            line += f"; source `{clipped(m['source'], 300)}`"
        if m.get("description"):
            line += f"; {clipped(m['description'], NOTE_CHARS)}"
        out.append(line)
    return out


def events_lines(schema):
    out = ["## Event kinds", "", "event_kinds: the `events` and `derived_events` kinds by sim.", ""]
    for e in sorted(schema.get("event_kinds") or [], key=lambda e: (str(e.get("sim")), str(e.get("kind")))):
        line = f"- {e.get('kind')} ({e.get('sim')}): {clipped(e.get('description') or '', NOTE_CHARS)}"
        if e.get("value_meaning"):
            line += f"; value: {clipped(e['value_meaning'], 120)}"
        out.append(line)
    return out


def catalog_text(schema):
    """The catalog's tables and views, macros, scored metrics and event
    kinds, under one heading."""
    out = (["# The catalog", "", raw_line(schema), ""] + tables_lines(schema) + macros_lines(schema)
           + [""] + metrics_lines(schema) + [""] + events_lines(schema))
    return "\n".join(out) + "\n"


def tables_text(schema):
    """A line a main table or view: its name, kind and grain."""
    a = about(schema)
    rows = [(t["name"], a.get(t["name"], {}).get("kind") or "", clipped(a.get(t["name"], {}).get("grain") or "",
                                                                          GRAIN_CHARS))
            for t in main_tables(schema)]
    wide = [max([len(r[i]) for r in rows] + [0]) for i in range(2)]
    lines = [f"{n:<{wide[0]}}  {k:<{wide[1]}}  {g}".rstrip() for n, k, g in rows]
    return "\n".join(["# Tables and views", "", raw_line(schema), "", *lines]) + "\n"


# --- the saved views ------------------------------------------------------------------------------


def params_text(params):
    """A view's params as one line: each one's name, kind and default."""
    return ", ".join(f"{p.name} ({p.kind}" + (f", default {text_of(p.default) or repr(p.default)}"
                                              if p.default is not None else "") + ")" for p in params)


def view_head(v):
    """A saved view's title, description and params."""
    out = [f"## {v.name}: {v.title}"]
    if v.description:
        out.append(v.description)
    if v.params:
        out.append("params: " + params_text(v.params))
    return out


def view_lines(v):
    """A saved view's view_head, its SQL (licence lines left out) and a
    blank line."""
    sql = "\n".join(line for line in v.sql.splitlines() if not re.match(r"--\s*(Copyright|SPDX)", line))
    return view_head(v) + ["```sql", sql.strip("\n"), "```", ""]


def views_text(saved, intro, sql=True):
    out = ["# The saved views", "", intro, ""]
    for v in sorted(saved, key=lambda v: v.name):
        out += view_lines(v) if sql else view_head(v) + [""]
    return "\n".join(out)


# --- the topics -----------------------------------------------------------------------------------

TOPIC_DOCS = {
    "glossary": "the sims' terms: sims, cells, arms, experiments, the oracle, the verdict and collapses, phases, "
                "lag and playout delays, structural cells, comparing arms, collapse counts, startup tails and "
                "steady spikes, overhead, data that left S3; and DuckDB's dialect",
    "tables": "every table and view of the catalog, a line each: its kind and grain",
    "TABLE": "a table or view: its grain, keys, joins, columns (types, units) and the columns' notes",
    "macros": "the macros, suite.py's scoring (effects, collapse_test, arm_score, structural, dynamics, "
              "fisher_p, ...), with their parameters",
    "MACRO": "a macro's line of `macros`",
    "metrics": "the scored metrics: each one's better direction, kind, tie band, guard, unit and SQL",
    "events": "the event kinds of `events` and `derived_events`, by sim",
    "views": "the saved views (the console's pages): their titles, descriptions and params",
    "VIEW": "a saved view: its title, description, params and SQL",
}


def wrapped(label, names):
    return textwrap.fill(", ".join(names), width=100, initial_indent=f"{label}: ", subsequent_indent="  ",
                         break_on_hyphens=False)


def index_text(schema, saved):
    names = [t["name"] for t in main_tables(schema)]
    lines = ["# The sims' results catalog", "", "`simdb.py describe TOPIC` describes one of:", ""]
    lines += [textwrap.fill(v, width=100, initial_indent=f"  {k:<10}", subsequent_indent=" " * 12)
              for k, v in TOPIC_DOCS.items()]
    lines += ["", wrapped("Tables and views", names), wrapped("Macros", [m["name"] for m in macros(schema)]),
              wrapped("Saved views", sorted(v.name for v in saved))]
    return "\n".join(lines) + "\n"


def describe(topic=None, pool=None, views_root=views.VIEWS):
    """The text of `topic` (the module docs)."""
    saved = views.saved(views_root)
    if topic == "glossary":
        return glossary()
    if topic == "views":
        return views_text(saved, VIEWS_INTRO, sql=False)
    schema = catalog.schema(pool)
    if topic is None:
        return index_text(schema, saved)
    parts = []
    table = next((t for t in main_tables(schema) if t["name"] == topic), None)
    if table is not None:
        parts.append("\n".join(table_lines(table, about(schema).get(topic, {}),
                                           collections_by(schema.get("metric_catalog") or [], "tbl")
                                           .get(topic, []))))
    fixed = {"tables": lambda: tables_text(schema), "macros": lambda: "\n".join(macros_lines(schema)) + "\n",
             "metrics": lambda: "\n".join(metrics_lines(schema)) + "\n",
             "events": lambda: "\n".join(events_lines(schema)) + "\n"}
    if topic in fixed:
        parts.append(fixed[topic]())
    parts += [macro_line(m) + "\n" for m in macros(schema) if m["name"] == topic]
    parts += ["\n".join(view_lines(v)) for v in saved if v.name == topic]
    if parts:
        return "\n".join(parts)
    names = ([t["name"] for t in main_tables(schema)] + [m["name"] for m in macros(schema)]
             + [v.name for v in saved])
    near = difflib.get_close_matches(str(topic), names + list(TOPICS), n=3)
    raise ValueError(f"no topic {topic!r}" + (f" (did you mean {', '.join(near)}?)" if near else "") +
                     f": the topics are {', '.join(TOPICS)}, or the name of a table, view, macro or saved view "
                     "(describe with no topic lists them)")

# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The catalog in words, a topic at a time: what `simdb.py describe`
prints, and the catalog's part of the console's Ask prompt
(simlab.ask.system_prompt), which the same functions write.

  describe(topic=None, pool=None, views_root=views.VIEWS) -> str
                 a topic's text: with none, the index of the topics and
                 of every table, view, macro, saved view and event kind;
                 else one of TOPICS, or a name, which gives each thing it
                 names (a table or view of the catalog, a macro, a saved
                 view, an event kind: `events` is a view of the catalog
                 and the event kinds' topic, `experiments` a table and a
                 saved view). ValueError for anything else, naming the
                 topics. `pool` is an engine.Pool of the catalog's
                 readers; glossary, phases and views read files alone
  glossary() -> str
                 glossary.md: the sims' terms and DuckDB's dialect
  glossary_section(title) -> str
                 the text of glossary.md's section `title`, its heading
                 left out: KeyError for none
  catalog_text(schema) -> str
                 the catalog (catalog.schema's parts): main's tables and
                 views with their columns and the catalog's notes, raw's
                 names, the macros, the scored metrics and the event kinds
  phases_text() -> str
                 the vocabulary's phases, episodes and reasons, each with
                 its description
  views_text(saved, intro, sql=True) -> str
                 the saved views (views.saved's) under a heading and
                 `intro`: each one's title, description, params and, with
                 `sql`, its SQL

The event kinds are event_kinds' rows, which the catalog makes from the
vocabulary (crates/strims-wan/vocab.toml, simlab.vocab) and the kinds the
runs hold; the phases, episodes and reasons are the vocabulary's own.
Every text is sorted (by name, the event kinds by their class first) and
free of counts and times, but the `events` topic's and an event kind's
counts, so the same catalog and views give the same bytes: the Ask
prompt is cached on them. Nothing here names the Ask panel's tools,
which are ask_prompt.md's and ask.py's to describe.
"""

import difflib
import json
import re
import textwrap
from pathlib import Path

from . import catalog, views, vocab

GLOSSARY = Path(__file__).resolve().parent / "glossary.md"
# A markdown file's leading comment (its licence and what it is for).
COMMENT = re.compile(r"\A<!--.*?-->\s*", re.S)
TOPICS = ("glossary", "tables", "macros", "metrics", "events", "phases", "views")
MACRO_KINDS = ("macro", "table macro", "macros")
# A column's catalog note, at most this long; a grain in `tables`' lines;
# an event kind's description, note or column.
NOTE_CHARS = 240
GRAIN_CHARS = 160
KIND_CHARS = 600
# What the vocabulary's other sections name, as the catalog holds them.
SECTIONS = {"phases": "What each second of a run is: verdict_seconds' phase, and pooled's and spec's by phase.",
            "episodes": "A run's spells: episodes' kind.",
            "reasons": "Why a node's push parent of a stream changed: parents' reason."}
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


def kinds_by(schema):
    """event_kinds' rows by kind, each kind's by sim."""
    out = {}
    for e in sorted(schema.get("event_kinds") or [], key=lambda e: (str(e.get("kind")), str(e.get("sim")))):
        out.setdefault(str(e.get("kind")), []).append(e)
    return out


def seen_text(rows):
    """How many events of a kind the catalog holds, by sim, and the days
    the first and last ran."""
    out = []
    for r in rows:
        n = r.get("n") or 0
        days = (f" ({str(r['first_seen'])[:10]} to {str(r['last_seen'])[:10]})"
                if n and r.get("first_seen") and r.get("last_seen") else "")
        out.append(f"{r.get('sim')} {n:,}{days}")
    return ", ".join(out)


def kind_lines(kind, rows, counts=False, grouped=False):
    """A kind's lines, from its event_kinds rows (one a sim): its class
    (but `grouped`, under its class's heading), knob and sims, its
    description, what its columns hold, each sim's note and a retired
    kind's replacement; with `counts`, how many the catalog holds."""
    r = rows[0]
    sims = [str(x.get("sim")) for x in rows]
    if r.get("documented") is False:
        out = [f"- {kind} ({', '.join(sims)}): not in the vocabulary"]
    else:
        tags = [t for t in (None if grouped else r.get("class"), r.get("env_gate") and f"env_gate {r['env_gate']}")
                if t]
        if sims != [catalog.DERIVED_SIM]:
            tags.append(", ".join(sims))
        out = [f"- {kind}" + (f" ({'; '.join(tags)})" if tags else "")
               + f": {clipped(r.get('description') or '', KIND_CHARS)}"]
        cols = []
        for c, key in (("node", "node"), ("peer", "peer"), ("value", "value_meaning"), ("detail", "detail")):
            if r.get(key):
                text = r[key] + (f" ({catalog.either(r['details'])})" if c == "detail" and r.get("details") else "")
                cols.append(f"{c}: {clipped(text, KIND_CHARS)}")
        if cols:
            out.append("  - " + "; ".join(cols))
        out += [f"  - {x['sim']}: {clipped(x['notes'], KIND_CHARS)}" for x in rows if x.get("notes")]
        out += [f"  - {x['sim']}'s replaced by {x['replaced_by']}" for x in rows if x.get("replaced_by")]
    if counts:
        out.append(f"  - events: {seen_text(rows)}")
    return out


def events_lines(schema, counts=False):
    """event_kinds' kinds by class (vocab.CLASSES' order, then those the
    vocabulary lacks), each kind_lines'."""
    by = kinds_by(schema)
    out = ["## Event kinds", "",
           "event_kinds: the kinds of `events` (the sims') and `derived_events` (the verdict's, sim 'derived'), "
           "the vocabulary's (crates/strims-wan/vocab.toml), by class: each kind's knob (env_gate, where a sim "
           "writes it only under one) and the sims whose runs may hold it, what it records, what its node, peer, "
           "value and detail hold, and a sim's note where that sim's kind differs."]
    groups = [(c, vocab.CLASS_DOCS[c]) for c in vocab.CLASSES] + [(None, "Kinds the runs hold that the vocabulary "
                                                                         "lacks.")]
    for cls, doc in groups:
        names = [k for k, rows in by.items() if rows[0].get("class") == cls
                 or (cls is None and rows[0].get("class") not in vocab.CLASSES)]
        if not names:
            continue
        out += ["", f"### {cls or 'undocumented'}", doc, ""]
        for k in names:
            out += kind_lines(k, by[k], counts, grouped=True)
    return out


# The phase rule (verdict.timeline's), as the phases topic gives it.
PHASE_RULE = (
    "A second's phase comes from its run's scenario events: each event of a kind that anchors a phase opens a "
    "window, the stream's start the run's startup. A window is [t, t + horizon), or for a span [t, end + "
    "horizon) to its until event of the same node (the run's end without one); a level's phase (uplink_rate's, "
    "rate_limit's) is after_down or after_up by its value against its node's previous level. Events at t <= 0 "
    "set the run up and open none. Second k is in a window when its anchor <= k + 0.5 < its stop; of the windows "
    "holding it, the one whose latest event (a span's end once it came, else its anchor) is latest wins, a tie "
    "to the kind the vocabulary lists first; none holds a steady second. The run's phase (verdict_seconds) takes "
    "every window but joins'; a viewer's (viewer_phases) the run's and its own joins', and a viewer counts in the "
    "verdict's measures but for join's horizon after each of its joins. phase_windows lists each run's windows; lag_since(exp) gives the "
    "lag against the seconds since each window's anchor and span's end, the curves the horizons are read from.")


def anchor_text(kid, a, phase):
    """Anchor `kid` (its vocabulary table `a`) of `phase`: its id, then how
    it opens the phase's window and for how long."""
    how = []
    p = a["phase"]
    if isinstance(p, dict):
        when = [d for d, x in p.items() if x == phase]
        how.append(f"a step {when[0]}" if a.get("level") else ", ".join(when))
    if a.get("scope") == "viewer":
        how.append("its viewer's own")
    h = f"{a['horizon_s']:g} s"
    how.append(f"to its {a['until']}, then {h}" if "until" in a else h)
    return f"{kid} ({'; '.join(how)})"


def phases_text():
    """The vocabulary's phases, episodes and reasons (SECTIONS), each with
    its description, a phase with its anchors (the kinds that open its
    windows, and how), after the phase rule (PHASE_RULE)."""
    out = ["# Phases, episodes and reasons", "",
           "The vocabulary's (crates/strims-wan/vocab.toml) names of what the verdict and the sims' tables hold "
           "beside the event kinds."]
    anchors = vocab.anchors()
    for section, doc in SECTIONS.items():
        out += ["", f"## {section.capitalize()}", doc, ""]
        if section == "phases":
            out += [PHASE_RULE, ""]
        for name, v in vocab.entries(section).items():
            out.append(f"- {name}: {v['description']}")
            of = [anchor_text(kid, a, name) for kid, a in anchors.items()
                  if name in (a["phase"].values() if isinstance(a["phase"], dict) else [a["phase"]])]
            if section == "phases" and of:
                out.append(f"  - anchors: {', '.join(of)}")
    return "\n".join(out) + "\n"


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

# The console's own CLI, and the studies' reports over the catalog, which
# the index points at.
UI_DESCRIBE = "python3 hack/console/ui.py describe"
REPORTS = ("The studies' reports over the catalog (hack/simlab), each with its own selection (--exp, --cell, "
           "--arm, --seed, --name, --phases, --collapsed) and --section: `simdb.py forecast-report`, the arrival "
           "forecast study over the decision logs (forecast.py, regimes.py), and `simdb.py coverage-report`, seed "
           "coverage: how soon each chunk reaches a node but the seed, by which route, the seed's children of its "
           "stream and why late chunks were late (coverage.py). Each one's --help says more.")
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
    "events": "the event kinds of `events` and `derived_events`, the vocabulary's (crates/strims-wan/vocab.toml), "
              "by class: each one's sims, columns and notes, and how many the catalog holds",
    "KIND": "an event kind's line of `events`",
    "phases": "the verdict's phases, the episodes and the parents table's reasons, the vocabulary's",
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
    if kinds := list(kinds_by(schema)):
        lines.append(wrapped("Event kinds", kinds))
    lines += ["", textwrap.fill(f"The console's graphs and addresses (a view's panels, the selection in a console URL "
                                f"or the user's open tab, links to a view): {UI_DESCRIBE}", width=100,
                                subsequent_indent="  ", break_on_hyphens=False),
              "", textwrap.fill(REPORTS, width=100, subsequent_indent="  ", break_on_hyphens=False)]
    return "\n".join(lines) + "\n"


def describe(topic=None, pool=None, views_root=views.VIEWS):
    """The text of `topic` (the module docs)."""
    saved = views.saved(views_root)
    if topic == "glossary":
        return glossary()
    if topic == "phases":
        return phases_text()
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
             "events": lambda: "\n".join(events_lines(schema, counts=True)) + "\n"}
    if topic in fixed:
        parts.append(fixed[topic]())
    parts += [macro_line(m) + "\n" for m in macros(schema) if m["name"] == topic]
    parts += ["\n".join(view_lines(v)) for v in saved if v.name == topic]
    kinds = kinds_by(schema)
    if topic in kinds:
        parts.append("\n".join(kind_lines(topic, kinds[topic], counts=True)) + "\n")
    if parts:
        return "\n".join(parts)
    names = ([t["name"] for t in main_tables(schema)] + [m["name"] for m in macros(schema)]
             + [v.name for v in saved] + list(kinds))
    near = difflib.get_close_matches(str(topic), names + list(TOPICS), n=3)
    raise ValueError(f"no topic {topic!r}" + (f" (did you mean {', '.join(near)}?)" if near else "") +
                     f": the topics are {', '.join(TOPICS)}, or the name of a table, view, macro, saved view or "
                     "event kind (describe with no topic lists them)")

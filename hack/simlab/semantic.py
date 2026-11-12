# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The sims' metrics and dimensions, crates/strims-wan/metrics.toml (its
header says what it holds): each declared once, read stdlib alone, so
suite.py and the catalog take them under the plain python3, and checked
against what they name.

  load(path=None) -> dict      metrics.toml (METRICS by default), parsed
  scored(doc=None) -> [(name, sql, better, kind, tie)]
                               the scored metrics as suite.py's METRICS
                               holds them, a share's and a per_useful's
                               SQL compiled
  guardrails(doc=None) -> {name: guard}
                               arm_score's guardrails, in the order it
                               checks them
  shown(doc=None), host_metrics(doc=None) -> (name, ...)
                               the metrics a scorecard shows in every
                               cell, ties too; the host's numbers
  share(num, den), per_useful(*fields) -> str
                               the SQL a share and a per_useful are
  fields_read(sql) -> {(table, name)}
                               the fields an SQL reads of a job's row r,
                               verdict v and job j
  run_metric_names() -> [str]  the scalars verdict.run_metrics writes
  problems(doc=None) -> [str]  what's wrong with it: spec.check_metrics',
                               a scored metric's field undeclared (a row's
                               not in fields.toml, a verdict's not a
                               verdicts column or run metric, a job's not
                               a host number), the run metrics not
                               verdict.run_metrics', a macro not one of
                               the lab's SQL, a dimension's values of no
                               vocabulary section or fields.toml param
  restatements(paths=None) -> [str]
                               the lint: a lake metric's `restated`
                               patterns found in the lab's SQL (its
                               views' and simlab's, SQL_FILES) outside its
                               macro's own definition, one a line
  outputs() -> {path: text}    the files made from it, metrics.schema.json,
                               after problems finds none; vocab.py's write
                               and check make and hold them
"""

import ast
import inspect
import json
import re
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
METRICS = ROOT / "crates" / "strims-wan" / "metrics.toml"
SCHEMA = ROOT / "crates" / "strims-wan" / "metrics.schema.json"
SQL_DIR = Path(__file__).with_name("sql")
VIEWS_DIR = ROOT / "hack" / "console" / "views"
# A job's host's numbers, which a metric reads as j.<name>.
HOST_FIELDS = ("rss_mb", "wall_s")
FIELD = re.compile(r"\b([rvj])\.([a-z_][a-z0-9_]*)\b")
MACRO = re.compile(r"^CREATE OR REPLACE MACRO ([a-z_][a-z0-9_]*)\(", re.M)


class MetricsError(ValueError):
    """metrics.toml doesn't parse, or doesn't check."""


def load(path=None):
    """metrics.toml (METRICS by default), parsed."""
    try:
        return tomllib.loads(Path(path or METRICS).read_text())
    except tomllib.TOMLDecodeError as e:
        raise MetricsError(f"{Path(path or METRICS).name}: {e}") from None


def share(num, den):
    """`num` over `den`, none where `den` is 0."""
    return f"CASE WHEN {den} <> 0 THEN coalesce({num}, 0) / ({den}) END"


def per_useful(*fields):
    """The row's `fields` summed, per byte of the stream the honest viewers
    read (`useful_bytes`); none without all of them."""
    return f"CASE WHEN r.useful_bytes <> 0 THEN ({' + '.join(f'r.{f}' for f in fields)}) / r.useful_bytes END"


def sql_of(m):
    """A scored metric's SQL: its own, or its share's or per_useful's."""
    if m.get("sql") is not None:
        return m["sql"]
    if m.get("share") is not None:
        return share(m["share"]["num"], m["share"]["den"])
    return per_useful(*m["per_useful"])


def scored(doc=None):
    """The scored metrics, suite.py's METRICS: (name, its SQL, better,
    kind, tie), in metrics.toml's order."""
    doc = doc or load()
    return [(m["name"], sql_of(m), m.get("better"), m["score"]["kind"], m["score"]["tie"]) for m in doc["scored"]]


def guardrails(doc=None):
    """arm_score's guardrails, {name: guard}, in the order it checks them."""
    doc = doc or load()
    by = {m["name"]: m["score"].get("guard") for m in doc["scored"]}
    return {n: by[n] for n in doc["guardrails"]}


def shown(doc=None):
    doc = doc or load()
    return tuple(m["name"] for m in doc["scored"] if m["score"].get("shown"))


def host_metrics(doc=None):
    doc = doc or load()
    return tuple(m["name"] for m in doc["scored"] if m["score"].get("host"))


def fields_read(sql):
    """The fields `sql` reads, {(r, v or j, name)}."""
    return set(FIELD.findall(sql))


def run_metric_names():
    """The scalars verdict.run_metrics writes, in the order it first names
    them: its `out` dict's keys."""
    from . import verdict
    tree = ast.parse(inspect.getsource(verdict.run_metrics))
    out = []

    def add(node):
        if isinstance(node, ast.Constant) and isinstance(node.value, str) and node.value not in out:
            out.append(node.value)

    for node in ast.walk(tree):
        if isinstance(node, ast.Assign) and any(isinstance(t, ast.Name) and t.id == "out" for t in node.targets) \
                and isinstance(node.value, ast.Dict):
            for k in node.value.keys:
                add(k)
        if isinstance(node, ast.Subscript) and isinstance(node.value, ast.Name) and node.value.id == "out" \
                and isinstance(node.ctx, ast.Store):
            add(node.slice)
    return out


def macros():
    """The lab's SQL's macros, {name: file}."""
    return {m.group(1): p for p in sorted(SQL_DIR.glob("*.sql")) for m in MACRO.finditer(p.read_text())}


def problems(doc=None):
    """What's wrong with metrics.toml (`doc`, parsed, else the file), one a
    line: its models' (spec.check_metrics) and what it names."""
    from . import spec, verdict, vocab
    doc = doc or load()
    out = spec.check_metrics(doc)
    if out:
        return out
    rows = vocab.declared()["rows"]
    have = {"r": {n for n, (_, e) in rows.items() if not e.get("retired")},
            "v": set(verdict.VERDICT_COLS) | set(run_metric_names()), "j": set(HOST_FIELDS)}
    what = {"r": "fields.toml's current rows", "v": "the verdicts' columns or run metrics",
            "j": "the job's host numbers"}
    for m in doc["scored"]:
        for t, name in sorted(fields_read(sql_of(m))):
            if name not in have[t]:
                cur = vocab.resolve(name) if t == "r" and name in rows else None
                out.append(f"[[scored]] {m['name']}: {t}.{name} isn't one of {what[t]}"
                           + ("" if cur is None else " (retired, with no replacement)" if cur == name
                              else f" (retired: read {cur})"))
    names = run_metric_names()
    if sorted(doc["run_metrics"]) != sorted(names):
        out.append(f"[run_metrics]: {sorted(doc['run_metrics'])} aren't verdict.run_metrics' {sorted(names)}")
    have_macros = macros()
    for where, e in [("collapsed", doc["collapsed"]), *((f"lake.{n}", e) for n, e in doc["lake"].items()),
                     *((f"dimensions.{n}", e) for n, e in doc["dimensions"].items() if e.get("macro"))]:
        if e["macro"] not in have_macros:
            out.append(f"[{where}]: macro {e['macro']} isn't in {SQL_DIR.relative_to(ROOT)}")
    fields = vocab.fields()
    params = {p for section in [fields["rows"], fields["meta"], *fields["columns"].values()] for key in section
              for p in vocab.PARAM.findall(key)}
    for n, d in doc["dimensions"].items():
        v = d.get("values")
        if isinstance(v, str):
            kind, _, name = v.partition(":")
            if kind == "vocab" and name not in vocab.SECTIONS:
                out.append(f"[dimensions.{n}]: values {v}: no vocabulary section {name}")
            elif kind == "fields" and name not in params:
                out.append(f"[dimensions.{n}]: values {v}: no fields.toml param {name}")
            elif kind not in ("vocab", "fields"):
                out.append(f"[dimensions.{n}]: values {v}: neither vocab:<section> nor fields:<param>")
    return out


SQL_FILES = (VIEWS_DIR, SQL_DIR)


def definition_spans(text, macro):
    """Where macro `macro` is defined in `text`: [(start, end)], its
    CREATE through the statement's semicolon."""
    out = []
    for m in re.finditer(rf"^CREATE OR REPLACE MACRO {re.escape(macro)}\(", text, re.M):
        end = text.find(";\n", m.end())
        out.append((m.start(), len(text) if end < 0 else end))
    return out


def restatements(paths=None, doc=None):
    """Each lake metric's `restated` patterns found in the SQL files
    (`paths`, by default every .sql under SQL_FILES) outside its macro's own
    definition, one a line: path:line, the metric and the text."""
    doc = doc or load()
    files = paths if paths is not None else [p for d in SQL_FILES for p in sorted(Path(d).glob("*.sql"))]
    out = []
    for p in files:
        text = Path(p).read_text()
        for name, e in doc["lake"].items():
            own = definition_spans(text, e["macro"])
            for pat in e.get("restated", ()):
                for m in re.finditer(pat, text):
                    if any(a <= m.start() < b for a, b in own):
                        continue
                    line = text.count("\n", 0, m.start()) + 1
                    rel = Path(p).resolve().relative_to(ROOT) if Path(p).resolve().is_relative_to(ROOT) else p
                    out.append(f"{rel}:{line}: {m.group(0)!r} restates {name} ({e['macro']})")
    return out


def outputs():
    """The files made from metrics.toml, {path: text}: its JSON schema.
    vocab.VocabError where problems finds any."""
    from . import spec, vocab
    if found := problems():
        raise vocab.VocabError("\n".join(found))
    return {SCHEMA: json.dumps(spec.metrics_schema(), indent=2) + "\n"}

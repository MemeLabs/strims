# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The recorded names' document, crates/strims-wan/fields.toml, and the
metrics' and dimensions', crates/strims-wan/metrics.toml, as typed models
(pydantic): what each key may hold, and the cross-references a key alone
can't check. vocab.py and semantic.py read the documents with tomllib and
import this module only to check them or write what's made from them, so
the plain python3 still reads them.

  check(doc) -> [str]       what's wrong with a parsed fields.toml, one a line
  schema() -> dict          its JSON schema (vocab.schema.json, which an
                            editor's TOML support validates with)
  check_metrics(doc) -> [str]
                            what's wrong with a parsed metrics.toml, as its
                            models alone see it (semantic.problems checks
                            what it names)
  metrics_schema() -> dict  its JSON schema (metrics.schema.json)
"""

import re
from typing import Annotated, Literal

from pydantic import BaseModel, ConfigDict, Field, ValidationError, model_validator

from . import vocab

SIMS = ("strims-sim", "ppspp")
LINES = ("decl", "p4tf")
Sim = Literal["strims-sim", "ppspp"]
Line = Literal["decl", "p4tf"]
TYPES = ("int", "num", "text", "bool", "ints", "nums", "json")
Type = Literal["int", "num", "text", "bool", "ints", "nums", "json"]
NONES = ("empty", "-1", "0", "negative")
NAME = re.compile(r"[a-z][a-z0-9_]*\Z")
FAMILY = re.compile(r"(?:[a-z0-9_]|\{[a-z][a-z0-9_]*\})+\Z")
PARAM = re.compile(r"\{([a-z][a-z0-9_]*)\}")
VALUE = re.compile(r"^[a-z0-9_]*$")
# A table's columns may be one of a shared formatter's.
SHARED = ("seed_log",)

Values = list[Annotated[str, Field(pattern=VALUE.pattern)]] | dict[Sim | Line, list[str]]
Product = dict[str, Values]


class Entry(BaseModel):
    """A recorded name, or a family of them (an id with {param} parts)."""
    model_config = ConfigDict(extra="forbid")
    sims: list[Sim] = Field(description="The sims that write it; [] for a name only a line writes.")
    lines: list[Line] = Field([], description="The lines that write it where main doesn't; a family's values a "
                                              "line adds are keyed by the line in its params.")
    type: Type = Field(description="int, num, text, bool, ints (a list of integers), nums (a list of numbers), "
                                   "json (another JSON value).")
    unit: str | None = None
    none: Literal["empty", "-1", "0", "negative", "null"] | None = Field(
        None, description="How a sim writes none: an empty field, -1, 0, a negative time (before the start), or "
                          "JSON's null.")
    older_none: Literal["-1"] | None = Field(
        None, description="How older runs wrote none, which the catalog's view reads as none too.")
    params: Product | list[Product] | None = Field(
        None, description="A family's params: {param = values}, each a list, or a table of lists by sim (and "
                          "line); a list of such tables is their union.")
    code: dict[Sim, dict[str, list[str]]] = Field(
        {}, description="By sim, a param's values as the code's arrays (Rust expressions, concatenated): a "
                        "generated test holds them equal.")
    consts: dict[Sim, list[str]] = Field(
        {}, description="By sim, the params whose values the code loops over as the generated module's consts.")
    description: str = Field(min_length=1, description="What it holds: what it counts or measures, over whom and when.")
    notes: dict[Sim, str] = Field({}, description="A sim's own doc where it says more, or other.")


class Table(BaseModel):
    model_config = ConfigDict(extra="forbid")
    sims: list[Sim]
    lines: list[Line] = []
    grain: str | None = None
    keys: str | None = None
    joins: str | None = None
    knobs: list[str] = Field([], description="The settings that write it, without their sim's prefix "
                                             "(STRIMS_SIM_ or SIM_).")
    sample: bool = Field(False, description="A sample table (store_host.SAMPLE_TABLES): the farm uploads it apart.")
    zstd: bool = Field(False, description="Written zstd-compressed, <table>.csv.zst.")
    normalized: str | None = Field(None, description="What the catalog's view changes.")
    none: Literal["-1"] | None = Field(
        None, description="Every numeric column but the ids writes none as -1, declared or not: the view reads "
                          "-1 as none in each.")
    shared: Literal["seed_log"] | None = Field(
        None, description="Its columns are a shared formatter's, which both sims write through.")
    description: str | None = None
    notes: dict[Sim, str] = {}


class Document(BaseModel):
    model_config = ConfigDict(extra="forbid")
    rows: dict[str, Entry]
    meta: dict[str, Entry]
    tables: dict[str, Table]
    columns: dict[str, dict[str, Entry]]

    @model_validator(mode="after")
    def references(self):
        found = problems(self)
        if found:
            raise ValueError("\n".join(found))
        return self


def entry_problems(where, key, e, sims_of_table=None):
    out = []
    family = "{" in key
    if family:
        if not FAMILY.match(key):
            out.append(f"{where}: not a family id (lower snake case with {{param}} parts)")
        names = PARAM.findall(key)
        if len(set(names)) != len(names):
            out.append(f"{where}: a param named twice")
        if e.params is None:
            out.append(f"{where}: a family without params")
        else:
            prods = e.params if isinstance(e.params, list) else [e.params]
            for p in prods:
                if sorted(p) != sorted(names):
                    out.append(f"{where}: params {sorted(p)} are not its id's {sorted(names)}")
                for k, v in p.items():
                    if isinstance(v, dict):
                        for who, vs in v.items():
                            if who in SIMS and who not in e.sims:
                                out.append(f"{where}: {k}'s values for {who}, which doesn't write it")
                            if who in LINES and who not in e.lines:
                                out.append(f"{where}: {k}'s values for {who}, not one of its lines")
            for s in e.sims:
                if not products(e.params, s):
                    out.append(f"{where}: no values for {s}")
        for s, ps in e.consts.items():
            if s not in e.sims:
                out.append(f"{where}: consts for {s}, which doesn't write it")
            for k in ps:
                if k not in names:
                    out.append(f"{where}: consts of {k}, not a param")
                elif len(products(e.params, s)) != 1:
                    out.append(f"{where}: consts of a union of products")
                elif k in e.code.get(s, {}):
                    out.append(f"{where}: {k} has both consts and code")
        for s, c in e.code.items():
            if s not in e.sims:
                out.append(f"{where}: code for {s}, which doesn't write it")
            for k in c:
                if k not in names:
                    out.append(f"{where}: code for {k}, not a param")
    else:
        if not NAME.match(key):
            out.append(f"{where}: not a lower snake case name")
        if e.params is not None or e.code or e.consts:
            out.append(f"{where}: params, code or consts on a name")
    if not e.sims and not e.lines:
        out.append(f"{where}: no sims and no lines")
    if len(set(e.sims)) != len(e.sims) or len(set(e.lines)) != len(e.lines):
        out.append(f"{where}: a sim or line twice")
    for s in e.notes:
        if s not in e.sims and not e.lines:
            out.append(f"{where}: a note for {s}, which doesn't write it")
    if sims_of_table is not None:
        extra = set(e.sims) - set(sims_of_table)
        if extra:
            out.append(f"{where}: written by {sorted(extra)}, whose tables lack it")
    return out


def expand(key, params, who):
    """A family's names for `who`, in order; [key] for a name."""
    return vocab.expand(key, {"params": params}, who)


def products(params, who):
    return vocab.products(params, who)


def problems(doc):
    """A Document's cross-reference problems."""
    out = []
    for key, e in doc.rows.items():
        out += entry_problems(f"[rows.{key}]", key, e)
    for key, e in doc.meta.items():
        out += entry_problems(f"[meta.{key}]", key, e)
    for t, cols in doc.columns.items():
        if t not in doc.tables:
            out.append(f"[columns.{t}]: no [tables.{t}]")
            continue
        tab = doc.tables[t]
        for key, e in cols.items():
            out += entry_problems(f"[columns.{t}.{key}]", key, e, tab.sims)
        written = {s for e in cols.values() for s in e.sims}
        if set(tab.sims) != written:
            out.append(f"[tables.{t}]: sims {tab.sims} aren't its columns' {sorted(written)}")
    for t, tab in doc.tables.items():
        if t not in doc.columns:
            out.append(f"[tables.{t}]: no columns")
        if not tab.sims and not tab.lines:
            out.append(f"[tables.{t}]: no sims and no lines")
    # every name once in its section (a table's columns, the rows, the meta keys), for each sim and line
    for section, entries in [("rows", doc.rows), ("meta", doc.meta)] + [(f"columns.{t}", c)
                                                                         for t, c in doc.columns.items()]:
        for who in (*SIMS, *LINES):
            seen = {}
            for key, e in entries.items():
                if who not in e.sims and who not in e.lines and not (
                        who in LINES and isinstance(e.params, (dict, list)) and products(e.params, who)):
                    continue
                for n in expand(key, e.params, who):
                    if n in seen and seen[n] != key:
                        out.append(f"[{section}.{key}]: {n} is [{section}.{seen[n]}]'s too ({who})")
                    seen[n] = key
    return out


def check(raw):
    """What's wrong with a parsed fields.toml (`raw`), one a line."""
    try:
        Document.model_validate(raw)
    except ValidationError as e:
        out = []
        for err in e.errors():
            loc = ".".join(str(x) for x in err["loc"])
            msg = err["msg"]
            if err["type"] == "value_error":
                out += [m for m in str(err.get("ctx", {}).get("error", msg)).splitlines()]
            else:
                out.append(f"{loc}: {msg}")
        return out
    return []


def schema():
    """fields.toml's JSON schema."""
    s = Document.model_json_schema()
    s["$schema"] = "https://json-schema.org/draft/2020-12/schema"
    s["title"] = "crates/strims-wan/fields.toml"
    return s


# --- metrics.toml -------------------------------------------------------------------

Better = Literal["lower", "higher"]
GUARDS = ("worse", "lo", "hi")


class Score(BaseModel):
    """How the suite scores a metric."""
    model_config = ConfigDict(extra="forbid")
    kind: Literal["ratio", "diff"] = Field(description="ratio (geometric) or diff (absolute).")
    tie: int | float = Field(description="The band an effect is a tie within.")
    guard: Literal["worse", "lo", "hi"] | None = Field(
        None, description="How arm_score breaks an arm on it: worse (marked worse in a cell), lo (its ratio "
                          "interval past GUARD_RATIO), hi (its ratio interval under 1).")
    shown: bool = Field(False, description="A scorecard shows it in every cell, ties too.")
    host: bool = Field(False, description="A host's number: marked h and kept out of the scorecard's totals.")


class Share(BaseModel):
    model_config = ConfigDict(extra="forbid")
    num: str
    den: str


class Scored(BaseModel):
    """A metric the suite scores."""
    model_config = ConfigDict(extra="forbid")
    name: str = Field(pattern=r"^[a-z][a-z0-9_/]*$")
    sql: str | None = Field(None, description="Its value, SQL over a job's row r, verdict v and job j.")
    share: Share | None = Field(None, description="num over den, none where den is 0.")
    per_useful: list[str] | None = Field(
        None, description="The row's fields summed, per byte of the stream the honest viewers read.")
    better: Better | None = None
    score: Score
    unit: str | None = None
    description: str | None = Field(
        None, description="What it measures; one that reads one field takes the field's where it has none.")
    note: str | None = Field(None, description="How it scores, beside its description.")

    @model_validator(mode="after")
    def one_value(self):
        if sum(x is not None for x in (self.sql, self.share, self.per_useful)) != 1:
            raise ValueError(f"{self.name}: one of sql, share and per_useful")
        return self


class Collapsed(BaseModel):
    model_config = ConfigDict(extra="forbid")
    macro: str
    better: Better
    kind: str
    guard: str
    description: str = Field(min_length=1)


class RunMetric(BaseModel):
    model_config = ConfigDict(extra="forbid")
    unit: str | None = None
    description: str = Field(min_length=1)


class LakeMetric(BaseModel):
    model_config = ConfigDict(extra="forbid")
    macro: str
    unit: str | None = None
    description: str = Field(min_length=1)
    restated: list[str] = Field([], description="Patterns that would restate its definition outside its macro.")


class Dimension(BaseModel):
    model_config = ConfigDict(extra="forbid")
    description: str = Field(min_length=1)
    values: list[str | int | bool] | str | None = Field(
        None, description="Its values: a list, vocab:<section> (the vocabulary's names) or fields:<param> "
                          "(fields.toml's params of that name).")
    column: str | None = Field(None, description="The column that gives it, table.column.")
    macro: str | None = Field(None, description="The macro that gives it.")


class Metrics(BaseModel):
    model_config = ConfigDict(extra="forbid")
    guardrails: list[str] = Field(description="The guarded metrics, in the order arm_score checks them.")
    scored: list[Scored]
    collapsed: Collapsed
    run_metrics: dict[str, RunMetric]
    lake: dict[str, LakeMetric]
    dimensions: dict[str, Dimension]

    @model_validator(mode="after")
    def references(self):
        out = []
        names = [m.name for m in self.scored]
        out += [f"[[scored]] {n}: named twice" for n in sorted({n for n in names if names.count(n) > 1})]
        guarded = [m.name for m in self.scored if m.score.guard]
        if sorted(self.guardrails) != sorted(guarded) or len(set(self.guardrails)) != len(self.guardrails):
            out.append(f"guardrails {self.guardrails} aren't the guarded metrics {guarded}, each once")
        for n in [*self.run_metrics, *self.lake, *self.dimensions]:
            if not NAME.match(n):
                out.append(f"{n}: not a lower snake case name")
        for n, d in self.dimensions.items():
            if (d.column is None) == (d.macro is None) and not (isinstance(d.values, str)
                                                               and d.values.startswith("fields:")):
                out.append(f"[dimensions.{n}]: one of column and macro")
        if out:
            raise ValueError("\n".join(out))
        return self


def _errors(model, raw):
    try:
        model.model_validate(raw)
    except ValidationError as e:
        out = []
        for err in e.errors():
            loc = ".".join(str(x) for x in err["loc"])
            msg = err["msg"]
            if err["type"] == "value_error":
                out += [m for m in str(err.get("ctx", {}).get("error", msg)).splitlines()]
            else:
                out.append(f"{loc}: {msg}")
        return out
    return []


def check_metrics(raw):
    """What's wrong with a parsed metrics.toml (`raw`), one a line, as its
    models see it."""
    return _errors(Metrics, raw)


def metrics_schema():
    """metrics.toml's JSON schema."""
    s = Metrics.model_json_schema()
    s["$schema"] = "https://json-schema.org/draft/2020-12/schema"
    s["title"] = "crates/strims-wan/metrics.toml"
    return s

# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The recorded names' document, crates/strims-wan/fields.toml, as typed
models (pydantic): what each key may hold, and the cross-references a key
alone can't check. vocab.py reads the document with tomllib and imports
this module only to check it or write what's made from it, so the plain
python3 still reads the vocabulary.

  check(doc) -> [str]       what's wrong with a parsed document, one a line
  schema() -> dict          the document's JSON schema (vocab.schema.json,
                            which an editor's TOML support validates with)
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

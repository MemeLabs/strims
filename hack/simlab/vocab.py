# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The sims' event vocabulary, crates/strims-wan/vocab.toml (its header
says what it holds): loaded, checked and asked, stdlib alone, so
analyze.py and the scripts under the plain python3 or `python -I` read it
too; and the files made from it, committed beside their readers:

  crates/strims-wan/src/vocab.rs             the document, its kinds as Rust
                                             (strims_wan::vocab, the sims'
                                             Kind) and its other names
  src/devtools/components/console/vocab.ts   the console's lists

  python3 hack/simlab/vocab.py write         makes them
  python3 hack/simlab/vocab.py check         exits 1 while either differs

  load(path=None) -> dict      the document (DOCUMENT by default), checked:
                               VocabError names its problems
  problems(doc) -> [str]       what's wrong with a parsed document

The queries take the document (`doc`, load()'s by default) and list ids
in its order:

  kinds(cls=None, sim=None)    the current kinds: of class `cls`, written
                               by `sim`
  kind(kid) -> dict            a kind's table
  emitted()                    the current kinds a sim here writes (Rust's
                               Kind)
  unwritten()                  the current kinds no sim here writes: other
                               builds' runs hold them
  retired()                    the kinds with `replaced_by`
  derived(), scenario()        the current kinds of those classes
  aligns()                     what a curve aligns on: derived, then
                               scenario
  anchors()                    the current kinds that anchor a phase
                               (`phase`), by id: the derived `start` among
                               them
  rule_kinds()                 the kinds of the sims' events the phase rule
                               reads: the anchors a sim writes and the
                               kinds that end their spans (`until`)
  anchor_fields()              each anchor's ANCHOR_KEYS, by id: what
                               defines the phase rule
  current(kid, sim)            the current id of a kind `sim`'s runs name
                               `kid`: itself, or its replacement
  notes(kid, sim)              that sim's note on the kind, or None
  holders(kid)                 the sims whose runs may hold the kind
  scope(kid)                   whose seconds a scenario kind bears on: run
                               (its default) or viewer
  value_text(k)                what a kind's table says `value` holds, its
                               unit after it
  knobs()                      the current kinds a knob (`env_gate`) gates,
                               by knob
  phases(), episodes(), reasons()
                               those sections' names
  entries(section)             a section's tables, by name

  outputs() -> {path: text}    the generated files, as the document makes
                               them
  stale() -> [path]            those that differ on disk
  write()                      writes them
"""

import functools
import json
import re
import sys
import textwrap
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DOCUMENT = ROOT / "crates" / "strims-wan" / "vocab.toml"
RUST = ROOT / "crates" / "strims-wan" / "src" / "vocab.rs"
TS = ROOT / "src" / "devtools" / "components" / "console" / "vocab.ts"

SIMS = ("strims-sim", "ppspp")
CLASSES = ("scenario", "observation", "decision", "derived")
SECTIONS = ("kinds", "phases", "episodes", "reasons")
# A kind's columns, as the document describes them.
FIELDS = ("node", "peer", "value", "unit", "detail")
# A kind's keys, in the order the generated files give them. `phase`,
# `level`, `until`, `scope` and `horizon_s` make a kind a phase's anchor.
KIND_KEYS = ("class", "sims", "env_gate", "description", *FIELDS, "details", "notes", "replaced_by", "phase",
             "level", "until", "scope", "horizon_s")
SECTION_KEYS = {s: ("description", "colour") for s in ("phases", "episodes", "reasons")}
SCOPES = ("run", "viewer")
# A kind's keys that make it an anchor, and define the phase rule.
ANCHOR_KEYS = ("phase", "level", "until", "scope", "horizon_s")
# The phases no kind anchors: steady, every second no window holds, and
# TAIL, past the verdict's seconds (a read, a request or a chunk there is
# in no other phase).
STEADY, TAIL = "steady", "tail"
UNANCHORED = (STEADY, TAIL)
ID = re.compile(r"[a-z][a-z0-9_]*\Z")
COLOUR = re.compile(r"#[0-9a-f]{6}\Z")


class VocabError(ValueError):
    """A document with problems (`problems`), one a line."""


def parse(text):
    """`text` as a document, unchecked."""
    try:
        return tomllib.loads(text)
    except tomllib.TOMLDecodeError as e:
        raise VocabError(f"{DOCUMENT.name}: {e}") from None


@functools.cache
def _loaded(path):
    doc = parse(Path(path).read_text())
    if found := problems(doc):
        raise VocabError("\n".join(found))
    return doc


def load(path=None):
    """The document at `path` (DOCUMENT), checked; one parse a path."""
    return _loaded(str(path or DOCUMENT))


def _text(v):
    return isinstance(v, str) and v.strip() != ""


def problems(doc):
    """What's wrong with parsed document `doc`, one string a problem:
    sections and keys it doesn't define, ids that aren't lower snake case,
    a kind without a class or description, sims it doesn't know, a derived
    kind a sim writes, a reference (`replaced_by`, `until`, `phase`) to
    nothing current, a retired kind whose replacement a sim of its runs
    doesn't hold (`replaced_by` by sim names each of its sims, and only
    those), an anchor without a horizon or with a phase table that is
    neither {down, up} (a level) nor its `details`', anchor keys on a kind
    that anchors nothing, a phase but steady and tail that no kind
    anchors, and a phase, episode or reason without a colour of its own in
    its section (lower case #rrggbb)."""
    out = []
    for s in doc:
        if s not in SECTIONS:
            out.append(f"[{s}]: not a section ({', '.join(SECTIONS)})")
    for s in SECTIONS:
        if not isinstance(doc.get(s), dict) or not doc.get(s):
            out.append(f"[{s}]: missing or empty")
    if out:
        return out
    ks = doc["kinds"]
    live = {k for k, v in ks.items() if isinstance(v, dict) and "replaced_by" not in v}
    for s in SECTIONS:
        for name, v in doc[s].items():
            if not ID.match(name):
                out.append(f"[{s}.{name}]: not a lower snake case id")
            if not isinstance(v, dict):
                out.append(f"[{s}.{name}]: not a table")
                continue
            if not _text(v.get("description")):
                out.append(f"[{s}.{name}]: no description")
            keys = KIND_KEYS if s == "kinds" else SECTION_KEYS[s]
            for key in v:
                if key not in keys:
                    out.append(f"[{s}.{name}]: unknown key {key!r}")
    for kid, k in ks.items():
        if not isinstance(k, dict):
            continue
        where = f"[kinds.{kid}]"
        cls = k.get("class")
        if cls not in CLASSES:
            out.append(f"{where}: class {cls!r} is none of {', '.join(CLASSES)}")
        if cls == "derived":
            for key in ("sims", "env_gate", "notes", "replaced_by"):
                if key in k:
                    out.append(f"{where}: a derived kind takes no {key!r}: the verdict makes it")
        elif not isinstance(k.get("sims"), list):
            out.append(f"{where}: no sims list")
        else:
            sims = k["sims"]
            if any(s not in SIMS for s in sims) or len(set(sims)) != len(sims):
                out.append(f"{where}: sims {sims} are not distinct sims of {', '.join(SIMS)}")
        for key in (*FIELDS, "env_gate"):
            if key in k and not _text(k[key]):
                out.append(f"{where}: {key} is not text")
        if "details" in k:
            d = k["details"]
            if not isinstance(d, list) or not all(_text(x) for x in d) or len(set(d)) != len(d):
                out.append(f"{where}: details is not a list of distinct values")
        if "notes" in k:
            n = k["notes"]
            if not isinstance(n, dict) or any(s not in SIMS or not _text(t) for s, t in n.items()):
                out.append(f"{where}: notes is not a table of text by sim")
        if "replaced_by" in k:
            r = k["replaced_by"]
            sims = k.get("sims") if isinstance(k.get("sims"), list) else []
            by = r if isinstance(r, dict) else dict.fromkeys(sims, r)
            if isinstance(r, dict) and any(s not in SIMS for s in r):
                out.append(f"{where}: replaced_by names a sim it doesn't know")
            elif isinstance(r, dict) and set(r) != set(sims):
                out.append(f"{where}: replaced_by's sims {sorted(r)} are not its sims {sorted(sims)}")
            for t in dict.fromkeys(r.values() if isinstance(r, dict) else [r]):
                if t not in live:
                    out.append(f"{where}: replaced_by {t!r} is not a current kind")
            for s, t in by.items():
                if t in live and isinstance(ks[t], dict) and s in SIMS and s not in _holders(ks[t]):
                    out.append(f"{where}: replaced_by {t!r} for {s}, whose runs don't hold it")
        if "until" in k and k["until"] not in live:
            out.append(f"{where}: until {k['until']!r} is not a current kind")
        if "phase" in k:
            p = k["phase"]
            names = list(p.values()) if isinstance(p, dict) else [p]
            if isinstance(p, dict) and k.get("level") is True and set(p) != {"down", "up"}:
                out.append(f"{where}: a level's phase table is {{down, up}}")
            elif isinstance(p, dict) and k.get("level") is not True and set(p) != set(k.get("details") or ()):
                out.append(f"{where}: a phase table is {{down, up}}, with level = true, or by its details")
            elif not isinstance(p, dict) and k.get("level") is True:
                out.append(f"{where}: a level's phase is a table {{down, up}}")
            for name in names:
                if name not in doc["phases"]:
                    out.append(f"{where}: phase {name!r} is not a phase")
            if "horizon_s" not in k:
                out.append(f"{where}: an anchor takes a horizon_s")
        else:
            for key in ANCHOR_KEYS[1:]:
                if key in k and key != "scope":
                    out.append(f"{where}: {key} without a phase")
        if "level" in k and not isinstance(k["level"], bool):
            out.append(f"{where}: level is not a boolean")
        if "scope" in k and k["scope"] not in SCOPES:
            out.append(f"{where}: scope {k['scope']!r} is none of {', '.join(SCOPES)}")
        if "horizon_s" in k:
            h = k["horizon_s"]
            if isinstance(h, bool) or not isinstance(h, (int, float)) or h < 0:
                out.append(f"{where}: horizon_s is not a number of seconds")
    anchored = {n for k in ks.values() if isinstance(k, dict) and "replaced_by" not in k and "phase" in k
                for n in (k["phase"].values() if isinstance(k["phase"], dict) else [k["phase"]])}
    for name in doc["phases"]:
        if name not in anchored and name not in UNANCHORED:
            out.append(f"[phases.{name}]: no kind anchors it")
    for s in SECTION_KEYS:
        colours = {}
        for name, v in doc[s].items():
            if not isinstance(v, dict):
                continue
            c = v.get("colour")
            if c is None:
                out.append(f"[{s}.{name}]: no colour")
            elif not isinstance(c, str) or not COLOUR.match(c):
                out.append(f"[{s}.{name}]: colour {c!r} is not a lower case #rrggbb")
            elif c in colours:
                out.append(f"[{s}.{name}]: colour {c!r} is {colours[c]}'s too")
            else:
                colours[c] = name
    return out


# --- queries ---------------------------------------------------------------------


def _doc(doc):
    return load() if doc is None else doc


def kind(kid, doc=None):
    """Kind `kid`'s table (KeyError for none)."""
    return _doc(doc)["kinds"][kid]


def kinds(cls=None, sim=None, doc=None):
    """The current kinds' ids: of class `cls`, and written by `sim`, if
    given."""
    return [kid for kid, k in _doc(doc)["kinds"].items()
            if "replaced_by" not in k and (cls is None or k["class"] == cls)
            and (sim is None or sim in k.get("sims", ()))]


def emitted(doc=None):
    """The current kinds a sim here writes: the Rust `Kind`'s."""
    return [kid for kid in kinds(doc=doc) if kind(kid, doc).get("sims")]


def unwritten(doc=None):
    """The current kinds that are a sim's (not derived) but that no sim
    here writes."""
    return [kid for kid in kinds(doc=doc) if kind(kid, doc)["class"] != "derived" and not kind(kid, doc)["sims"]]


def retired(doc=None):
    """The retired kinds' ids (`replaced_by`)."""
    return [kid for kid, k in _doc(doc)["kinds"].items() if "replaced_by" in k]


def derived(doc=None):
    return kinds("derived", doc=doc)


def scenario(doc=None):
    return kinds("scenario", doc=doc)


def aligns(doc=None):
    """What a curve aligns on: the derived kinds, then the scenario ones."""
    return derived(doc) + scenario(doc)


def anchors(doc=None):
    """The current kinds that anchor a phase, {id: table}, in the
    document's order (its ties' order)."""
    return {kid: kind(kid, doc) for kid in kinds(doc=doc) if "phase" in kind(kid, doc)}


def rule_kinds(doc=None):
    """The current kinds of the sims' events the phase rule reads, in the
    document's order: the anchors a sim writes (not the derived `start`)
    and the kinds that end their spans (`until`)."""
    a = anchors(doc)
    want = {kid for kid, k in a.items() if k["class"] != "derived"} | {k["until"] for k in a.values() if "until" in k}
    return [kid for kid in kinds(doc=doc) if kid in want]


def anchor_fields(doc=None):
    """Each anchor's keys that define the phase rule (ANCHOR_KEYS), {id:
    {key: value}}, in the document's order."""
    return {kid: {key: k[key] for key in ANCHOR_KEYS if key in k} for kid, k in anchors(doc).items()}


def current(kid, sim, doc=None):
    """The current id of the kind `sim`'s runs name `kid`: `kid` itself
    while current, its replacement (for `sim`, where it is by sim) once
    retired, None for an id the document doesn't name or a sim a retired
    kind names no replacement for."""
    k = _doc(doc)["kinds"].get(kid)
    if k is None:
        return None
    r = k.get("replaced_by")
    if r is None:
        return kid
    return r.get(sim) if isinstance(r, dict) else r


def notes(kid, sim, doc=None):
    """`sim`'s note on kind `kid`, or None."""
    return kind(kid, doc).get("notes", {}).get(sim)


def holders(kid, doc=None):
    """The sims whose runs may hold kind `kid`, in SIMS' order: those that
    write it (a retired kind's, those that wrote it), or, for a kind only
    other builds write (no sims), every sim, since either sim's runs hold
    it from such a build with its knob on; none for a derived kind, which
    no sim writes."""
    return _holders(kind(kid, doc))


def _holders(k):
    if k.get("class") == "derived":
        return []
    sims = k.get("sims") or []
    return [s for s in SIMS if s in sims] if sims else list(SIMS)


def scope(kid, doc=None):
    """Whose seconds kind `kid` bears on: its `scope`, run (every viewer's)
    or viewer (its node's viewer's alone, as a join's); run for a scenario
    kind that names none; None for a kind of another class that names
    none."""
    k = kind(kid, doc)
    return k.get("scope") or ("run" if k["class"] == "scenario" else None)


def value_text(k):
    """What kind table `k` says `value` holds, its unit after a comma
    ("the limit, bytes/s"); None where it says nothing."""
    if "value" not in k:
        return None
    return k["value"] + (f", {k['unit']}" if "unit" in k else "")


def knobs(doc=None):
    """The current kinds a knob gates, {knob (`env_gate`): [id]}, in the
    document's order."""
    out = {}
    for kid in kinds(doc=doc):
        if "env_gate" in kind(kid, doc):
            out.setdefault(kind(kid, doc)["env_gate"], []).append(kid)
    return out


def phases(doc=None):
    return list(_doc(doc)["phases"])


def episodes(doc=None):
    return list(_doc(doc)["episodes"])


def reasons(doc=None):
    return list(_doc(doc)["reasons"])


def entries(section, doc=None):
    """Section `section`'s (phases, episodes or reasons) tables, {name:
    table}, in the document's order."""
    return dict(_doc(doc)[section])


# --- generated files -------------------------------------------------------------

HEADER = ("// Copyright 2026 Strims contributors\n"
          "// SPDX-License-Identifier: AGPL-3.0-only\n"
          "// @generated by hack/simlab/vocab.py from crates/strims-wan/vocab.toml: edit\n"
          "// the document, then run `python3 hack/simlab/vocab.py write`.\n")
SIM_VARIANTS = {"strims-sim": ("StrimsSim", "`crates/strims-sim`, the real stack's."),
                "ppspp": ("Ppspp", "The ppspp sim, `crates/strims-ppspp/tests/sim.rs`.")}
CLASS_DOCS = {
    "scenario": "Exogenous: what the scenario does to the run.",
    "observation": "The run's response.",
    "decision": "A node's scheduler decisions, from its event log.",
    "derived": "The verdict's own, made from the series (`derived_events`).",
}


def variant(kid):
    """Kind `kid`'s Rust variant: `parent_departed` is `ParentDeparted`."""
    return "".join(p.capitalize() for p in kid.split("_"))


def _markdown(text):
    """`text` for rustdoc: brackets and angle brackets outside code spans
    escaped, so none reads as a link or a tag."""
    parts = text.split("`")
    for i in range(0, len(parts), 2):
        parts[i] = re.sub(r"([\[\]<>])", r"\\\1", parts[i])
    return "`".join(parts)


def _doc_lines(text, indent, first="", rest=""):
    """`text` as `///` lines at `indent`, wrapped, the first prefixed with
    `first` and the others with `rest`."""
    pad = " " * indent + "/// "
    lines = textwrap.wrap(_markdown(text), 80 - len(pad), initial_indent=first, subsequent_indent=rest,
                          break_long_words=False, break_on_hyphens=False)
    return [pad + line for line in lines]


def _kind_doc(k):
    """A Rust variant's docs: its description, then its columns and its
    sims' notes as a list."""
    out = _doc_lines(k["description"], 4)
    items = []
    for f in ("node", "peer", "value", "detail"):
        if f not in k:
            continue
        text = value_text(k) if f == "value" else k[f]
        if f == "detail" and "details" in k:
            text += ": " + ", ".join(f"`{d}`" for d in k["details"])
        items.append(f"`{f}`: {text}")
    for sim, note in k.get("notes", {}).items():
        items.append(f"{sim}: {note}")
    if items:
        out.append("    ///")
        for item in items:
            out += _doc_lines(item, 4, "- ", "  ")
    return out


def _raw(text):
    """`text` as a Rust raw string literal."""
    n = 1
    while '"' + "#" * n in text:
        n += 1
    h = "#" * n
    return f'r{h}"{text}"{h}'


def _strs(names, indent):
    if not names:
        return "&[]"
    pad = " " * indent
    return "&[\n" + "".join(f'{pad}    "{n}",\n' for n in names) + f"{pad}]"


def rust(text, doc):
    """The Rust module of document `text` (parsed, `doc`)."""
    ks = doc["kinds"]
    em = emitted(doc)
    o = [HEADER, "\n"]
    o.append(
        "//! The sims' event vocabulary, [`DOCUMENT`] (`crates/strims-wan/vocab.toml`,\n"
        "//! whose header says what it holds): the kinds of the `events` tables, as\n"
        "//! [`Kind`] for those a sim here writes ([`Kind::written_by`]), and the\n"
        "//! others' names ([`UNWRITTEN`], [`RETIRED`], [`DERIVED`]); and the `parents`\n"
        "//! table's reasons ([`REASONS`]). `hack/simlab/vocab.py` reads the\n"
        "//! document, and makes this file and the console's lists from it.\n\n")
    o.append(f"/// The vocabulary, as `crates/strims-wan/vocab.toml` holds it.\npub const DOCUMENT: &str = {_raw(text)};\n\n")
    o.append("/// A sim that writes `events` tables.\n#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]\npub enum Sim {\n")
    for s in SIMS:
        v, d = SIM_VARIANTS[s]
        o.append(f"    /// {d}\n    {v},\n")
    o.append("}\n\nimpl Sim {\n")
    o.append(f"    pub const ALL: [Sim; {len(SIMS)}] = [{', '.join('Sim::' + SIM_VARIANTS[s][0] for s in SIMS)}];\n\n")
    o.append("    /// Its name, as the document's `sims` and the catalog's `sim` say it.\n")
    o.append("    pub fn name(self) -> &'static str {\n        match self {\n")
    for s in SIMS:
        o.append(f'            Sim::{SIM_VARIANTS[s][0]} => "{s}",\n')
    o.append("        }\n    }\n}\n\n")
    o.append("/// What a kind records.\n#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]\npub enum Class {\n")
    for c in CLASSES:
        o.append(f"    /// {CLASS_DOCS[c]}\n    {variant(c)},\n")
    o.append("}\n\nimpl Class {\n")
    o.append(f"    pub const ALL: [Class; {len(CLASSES)}] = [{', '.join('Class::' + variant(c) for c in CLASSES)}];\n\n")
    o.append("    /// Its name, as the document's `class` says it.\n")
    o.append("    pub fn name(self) -> &'static str {\n        match self {\n")
    for c in CLASSES:
        o.append(f'            Class::{variant(c)} => "{c}",\n')
    o.append("        }\n    }\n}\n\n")
    o.append("/// A kind of event a sim here writes, as its `events` table's `kind`\n"
             "/// ([`Kind::name`]) names it.\n")
    o.append("#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]\npub enum Kind {\n")
    for kid in em:
        o += [line + "\n" for line in _kind_doc(ks[kid])]
        o.append(f"    {variant(kid)},\n")
    o.append("}\n\nimpl Kind {\n")
    o.append(f"    /// Every kind, in the document's order.\n    pub const ALL: [Kind; {len(em)}] = [\n")
    o += [f"        Kind::{variant(kid)},\n" for kid in em]
    o.append("    ];\n\n")
    o.append("    /// Its id: the `kind` column's text.\n    pub fn name(self) -> &'static str {\n        match self {\n")
    o += [f'            Kind::{variant(kid)} => "{kid}",\n' for kid in em]
    o.append("        }\n    }\n\n")
    o.append("    /// The kind of id `name`, if a sim here writes one.\n")
    o.append("    pub fn from_name(name: &str) -> Option<Kind> {\n        match name {\n")
    o += [f'            "{kid}" => Some(Kind::{variant(kid)}),\n' for kid in em]
    o.append("            _ => None,\n        }\n    }\n\n")
    o.append("    /// What it records.\n    pub fn class(self) -> Class {\n        match self {\n")
    for c in CLASSES:
        of = [kid for kid in em if ks[kid]["class"] == c]
        if of:
            o.append("            " + "\n            | ".join(f"Kind::{variant(kid)}" for kid in of)
                     + f" => Class::{variant(c)},\n")
    o.append("        }\n    }\n\n")
    o.append("    /// Whether `sim` writes it.\n    pub fn written_by(self, sim: Sim) -> bool {\n        match sim {\n")
    for s in SIMS:
        of = [kid for kid in em if s in ks[kid]["sims"]]
        body = ("matches!(\n                self,\n                "
                + "\n                    | ".join(f"Kind::{variant(kid)}" for kid in of)
                + "\n            )") if of else "false"
        o.append(f"            Sim::{SIM_VARIANTS[s][0]} => {body},\n")
    o.append("        }\n    }\n}\n\n")
    o.append("/// The current kinds no sim here writes, which other builds' runs hold\n"
             "/// (their `env_gate` knob on).\n")
    o.append(f"pub const UNWRITTEN: &[&str] = {_strs(unwritten(doc), 0)};\n\n")
    o.append("/// The retired kinds: ids that older runs hold, each `replaced_by` a\n/// current one.\n")
    o.append(f"pub const RETIRED: &[&str] = {_strs(retired(doc), 0)};\n\n")
    o.append("/// The verdict's own kinds (`derived_events`), none of them a sim's.\n")
    o.append(f"pub const DERIVED: &[&str] = {_strs(derived(doc), 0)};\n\n")
    o.append("/// Why a node's push parent of a stream changed: the `parents` table's\n"
             "/// `reason`, `strims_ppspp::ParentWhy`'s names.\n")
    o.append(f"pub const REASONS: &[&str] = {_strs(reasons(doc), 0)};\n")
    return "".join(o)


def _ts(v):
    if isinstance(v, dict):
        return "{ " + ", ".join(f"{json.dumps(k)}: {_ts(x)}" for k, x in v.items()) + " }"
    if isinstance(v, list):
        return "[" + ", ".join(_ts(x) for x in v) + "]"
    return json.dumps(v, ensure_ascii=False)


def _ts_list(name, doc_comment, items):
    o = [f"/** {doc_comment} */\nexport const {name} = [\n"]
    for item in items:
        if isinstance(item, str):
            o.append(f"  {_ts(item)},\n")
            continue
        o.append("  {\n" + "".join(f"    {k}: {_ts(v)},\n" for k, v in item.items()) + "  },\n")
    o.append("] as const;\n")
    return "".join(o)


def typescript(doc):
    """The console's lists of document `doc`."""
    named = lambda s: [{"name": n, **v} for n, v in doc[s].items()]
    kinds_ = [{"id": kid, **{key: k[key] for key in KIND_KEYS if key in k}} for kid, k in doc["kinds"].items()]
    return "\n".join([
        HEADER,
        _ts_list("PHASES", "The verdict's phases: what each second of a run is.", named("phases")),
        _ts_list("EPISODES", "A run's spells.", named("episodes")),
        _ts_list("REASONS", "Why a node's push parent of a stream changed: the `parents` table's `reason`.",
                 named("reasons")),
        _ts_list("KINDS", "Every kind of event the `events` tables and `derived_events` hold, retired ones too.",
                 kinds_),
        _ts_list("ALIGNS", "The kinds a curve aligns on: the derived kinds, then the scenario ones.", aligns(doc)),
    ])


def outputs(path=None):
    """The generated files, {path: text}, as the document at `path`
    (DOCUMENT) makes them."""
    text = Path(path or DOCUMENT).read_text()
    doc = load(path)
    return {RUST: rust(text, doc), TS: typescript(doc)}


def stale(path=None):
    """The generated files whose text on disk isn't `outputs`'."""
    return [p for p, text in outputs(path).items() if not p.exists() or p.read_text() != text]


def write(path=None):
    """Writes the generated files that are stale; returns them."""
    out = []
    for p, text in outputs(path).items():
        if not p.exists() or p.read_text() != text:
            p.write_text(text)
            out.append(p)
    return out


def main(argv):
    if len(argv) != 1 or argv[0] not in ("write", "check"):
        print("usage: python3 hack/simlab/vocab.py write|check", file=sys.stderr)
        return 2
    try:
        if argv[0] == "write":
            for p in write():
                print(f"wrote {p.relative_to(ROOT)}")
            return 0
        bad = stale()
    except VocabError as e:
        print(f"vocab: {e}", file=sys.stderr)
        return 1
    for p in bad:
        print(f"{p.relative_to(ROOT)} is stale: run python3 hack/simlab/vocab.py write", file=sys.stderr)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

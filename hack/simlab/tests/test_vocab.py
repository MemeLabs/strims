#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simlab.vocab, stdlib alone: the generated files (vocab.rs, vocab.ts)
are the generator's output byte for byte; the document checks (ids
unique, references to what exists, a retired kind's replacement one its
sims' runs hold, derived kinds disjoint from the sims' kinds, an anchor's
phase, horizon and keys, every phase but steady and tail anchored, a
colour to each phase, episode and reason, one to a name of its section);
each
kind's `sims` the sims whose event writers name it; and the queries over
it. fields.toml: who writes each entry, its retired names (declared, in
no generated module), and a retired name's current one (resolve,
renames, aliases, canonical_row):

  python3 hack/simlab/tests/test_vocab.py
"""

import copy
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HACK = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(HACK))

from simlab import vocab  # noqa: E402

KIND = """
[kinds.{kid}]
class = "{cls}"
{sims}description = "a kind"
"""
PHASE = """
[phases.{name}]
description = "a phase"
colour = "#{i:06x}"
"""


def document(kinds=(("a", "scenario", ["ppspp"]),), phases=("steady",), extra=""):
    """A small document: `kinds` (id, class, sims or None), `phases` (each
    its own colour), one episode, one reason and one run state, then
    `extra`."""
    out = []
    for kid, cls, sims in kinds:
        s = "" if sims is None else "sims = [" + ", ".join(f'"{x}"' for x in sims) + "]\n"
        out.append(KIND.format(kid=kid, cls=cls, sims=s))
    out += [PHASE.format(name=p, i=i) for i, p in enumerate(phases)]
    out.append('\n[episodes.healthy]\ndescription = "a spell"\ncolour = "#000000"\n'
               '\n[reasons.open]\ndescription = "a reason"\ncolour = "#000000"\n'
               '\n[run_states.healthy]\ndescription = "a state"\ncolour = "#000000"\n')
    return "".join(out) + extra


def problems(text):
    return vocab.problems(vocab.parse(text))


def given(text, kid, line):
    """Document `text` with `line` in kind `kid`'s table."""
    head = f"[kinds.{kid}]\n"
    assert head in text
    return text.replace(head, head + line + "\n")


class Generated(unittest.TestCase):
    def test_the_generated_files_are_the_generators(self):
        out = vocab.outputs()
        self.assertEqual(set(out), {vocab.RUST, vocab.TS})
        for path, text in out.items():
            self.assertEqual(path.read_bytes(), text.encode(), f"{path} is stale: run vocab.py write")
        self.assertEqual(vocab.stale(), [])

    def test_check_passes_and_fails_by_the_files(self):
        script = HACK / "simlab" / "vocab.py"
        p = subprocess.run([sys.executable, "-I", str(script), "check"], capture_output=True, text=True)
        self.assertEqual((p.returncode, p.stderr), (0, ""))
        with tempfile.TemporaryDirectory() as d:
            doc = Path(d) / "vocab.toml"
            doc.write_text(vocab.DOCUMENT.read_text() + KIND.format(kid="zz_new", cls="observation",
                                                                    sims='sims = ["ppspp"]\n'))
            self.assertEqual(vocab.stale(doc), [vocab.RUST, vocab.TS])
            self.assertIn('Kind::ZzNew', vocab.outputs(doc)[vocab.RUST])
        p = subprocess.run([sys.executable, "-I", str(script), "neither"], capture_output=True, text=True)
        self.assertEqual(p.returncode, 2)

    def test_the_rust_embeds_the_document_as_written(self):
        rust = vocab.RUST.read_text()
        text = vocab.DOCUMENT.read_text()
        m = re.search(r'pub const DOCUMENT: &str = r(#+)"(.*?)"\1;', rust, re.S)
        self.assertEqual(m.group(2), text)
        self.assertIn('r##"say "#"##', vocab._raw('say "#'))


class Document(unittest.TestCase):
    def test_the_document_checks(self):
        doc = vocab.parse(vocab.DOCUMENT.read_text())
        self.assertEqual(vocab.problems(doc), [])
        self.assertIs(vocab.load(), vocab.load())

    def test_its_ids_are_unique(self):
        headers = re.findall(r"^\[kinds\.([a-z0-9_]+)\]$", vocab.DOCUMENT.read_text(), re.M)
        self.assertEqual(len(headers), len(set(headers)))
        self.assertEqual(headers, list(vocab.load()["kinds"]))
        with self.assertRaises(vocab.VocabError):
            vocab.parse(document(kinds=(("a", "scenario", ["ppspp"]), ("a", "observation", ["ppspp"]))))
        self.assertEqual(problems(document(kinds=(("A-b", "scenario", ["ppspp"]),))),
                         ["[kinds.A-b]: not a lower snake case id"])

    def test_derived_kinds_are_no_sims(self):
        doc = vocab.load()
        written = {kid for kid in vocab.kinds(doc=doc) if doc["kinds"][kid].get("sims")}
        self.assertFalse(written & set(vocab.derived(doc)))
        self.assertTrue(all("sims" not in doc["kinds"][kid] for kid in vocab.derived(doc)))
        self.assertEqual(problems(document(kinds=(("start", "derived", ["ppspp"]),))),
                         ["[kinds.start]: a derived kind takes no 'sims': the verdict makes it"])
        self.assertEqual(problems(document(kinds=(("a", "scenario", None),))), ["[kinds.a]: no sims list"])
        self.assertEqual(problems(document(kinds=(("a", "scenario", ["ppspp", "other"]),))),
                         ["[kinds.a]: sims ['ppspp', 'other'] are not distinct sims of strims-sim, ppspp"])

    def test_references_are_to_what_exists(self):
        good = document(kinds=(("a", "scenario", ["ppspp"]), ("b", "scenario", ["ppspp"])))
        self.assertEqual(problems(good), [])
        cases = {
            'replaced_by = "nope"': ["[kinds.b]: replaced_by 'nope' is not a current kind"],
            'replaced_by = { ppspp = "a", other = "a" }': ["[kinds.b]: replaced_by names a sim it doesn't know"],
            'until = "nope"\nphase = "steady"\nhorizon_s = 1': ["[kinds.b]: until 'nope' is not a current kind"],
            'phase = "nope"\nhorizon_s = 1': ["[kinds.b]: phase 'nope' is not a phase"],
            'phase = { down = "steady", up = "steady" }\nhorizon_s = 1': [
                "[kinds.b]: a phase table is {down, up}, with level = true, or by its details"],
            'scope = "world"': ["[kinds.b]: scope 'world' is none of run, viewer"],
            'phase = "steady"\nhorizon_s = -1': ["[kinds.b]: horizon_s is not a number of seconds"],
            'notes = { other = "x" }': ["[kinds.b]: notes is not a table of text by sim"],
            'details = ["x", "x"]': ["[kinds.b]: details is not a list of distinct values"],
            'nonsense = "x"': ["[kinds.b]: unknown key 'nonsense'"],
        }
        for line, want in cases.items():
            self.assertEqual(problems(given(good, "b", line)), want, line)
        # A retired kind's replacement is current: not itself retired.
        chain = document(kinds=(("a", "scenario", ["ppspp"]), ("b", "scenario", ["ppspp"]),
                                ("c", "scenario", ["ppspp"])))
        chain = given(given(chain, "b", 'replaced_by = "c"'), "c", 'replaced_by = "b"')
        self.assertEqual(problems(chain), ["[kinds.b]: replaced_by 'c' is not a current kind",
                                           "[kinds.c]: replaced_by 'b' is not a current kind"])
        self.assertEqual(problems(given(good, "b", 'phase = { down = "steady", up = "steady" }\nlevel = true\n'
                                                   'horizon_s = 1')), [])

    def test_an_anchor_takes_a_horizon_and_its_keys(self):
        good = document(kinds=(("a", "scenario", ["ppspp"]), ("b", "scenario", ["ppspp"])), phases=("steady", "x"))
        good = given(good, "a", 'phase = "x"\nhorizon_s = 10')
        self.assertEqual(problems(good), [])
        cases = {
            'phase = "steady"': ["[kinds.b]: an anchor takes a horizon_s"],
            'until = "a"': ["[kinds.b]: until without a phase"],
            "level = true": ["[kinds.b]: level without a phase"],
            "horizon_s = 5": ["[kinds.b]: horizon_s without a phase"],
            'phase = "x"\nlevel = true\nhorizon_s = 5': ["[kinds.b]: a level's phase is a table {down, up}"],
            'phase = { up = "x", side = "x" }\nlevel = true\nhorizon_s = 5': [
                "[kinds.b]: a level's phase table is {down, up}"],
            # By detail: its keys the kind's details.
            'details = ["p", "q"]\nphase = { p = "x", q = "steady" }\nhorizon_s = 5': [],
            'details = ["p", "q"]\nphase = { p = "x" }\nhorizon_s = 5': [
                "[kinds.b]: a phase table is {down, up}, with level = true, or by its details"],
            # A scope alone is no anchor's (lag_since reads it).
            'scope = "viewer"': [],
        }
        for line, want in cases.items():
            self.assertEqual(problems(given(good, "b", line)), want, line)
        # Every phase but steady and tail is some current kind's.
        three = document(kinds=(("a", "scenario", ["ppspp"]),), phases=("steady", "tail", "x"))
        self.assertEqual(problems(three), ["[phases.x]: no kind anchors it"])
        self.assertEqual(problems(given(three, "a", 'phase = "x"\nhorizon_s = 1')), [])

    def test_a_replacement_by_sim_names_its_sims_and_what_their_runs_hold(self):
        both = document(kinds=(("a", "scenario", ["strims-sim", "ppspp"]), ("b", "scenario", ["ppspp"]),
                               ("c", "scenario", ["strims-sim"])))
        self.assertEqual(problems(given(both, "a", 'replaced_by = { strims-sim = "c", ppspp = "b" }')), [])
        cases = {
            'replaced_by = { ppspp = "b" }': ["[kinds.a]: replaced_by's sims ['ppspp'] are not its sims "
                                             "['ppspp', 'strims-sim']"],
            'replaced_by = { strims-sim = "b", ppspp = "b" }': [
                "[kinds.a]: replaced_by 'b' for strims-sim, whose runs don't hold it"],
            'replaced_by = "c"': ["[kinds.a]: replaced_by 'c' for ppspp, whose runs don't hold it"],
        }
        for line, want in cases.items():
            self.assertEqual(problems(given(both, "a", line)), want, line)
        # A kind only other builds write (no sims) is either sim's to hold.
        other = document(kinds=(("a", "scenario", ["ppspp"]), ("b", "decision", [])))
        self.assertEqual(problems(given(other, "a", 'replaced_by = "b"')), [])

    def test_sections_and_colours(self):
        self.assertEqual(problems(document() + '\n[colours.x]\ndescription = "x"\n'),
                         ["[colours]: not a section (kinds, phases, episodes, reasons, run_states)"])
        two = document(phases=("steady", "tail"))
        self.assertEqual(problems(two), [])
        # A colour is one to a name of its section, whichever section; another section's may be the same.
        self.assertEqual(problems(two.replace('colour = "#000001"', 'colour = "#000000"')),
                         ["[phases.tail]: colour '#000000' is steady's too"])
        two_reasons = two + '\n[reasons.shed]\ndescription = "a reason"\ncolour = "#000000"\n'
        self.assertEqual(problems(two_reasons), ["[reasons.shed]: colour '#000000' is open's too"])
        self.assertEqual(problems(two.replace('colour = "#000001"', 'colour = "#ABCDEF"')),
                         ["[phases.tail]: colour '#ABCDEF' is not a lower case #rrggbb"])
        self.assertEqual(problems(document().replace('description = "a spell"\ncolour = "#000000"\n',
                                                     'description = "a spell"\n')),
                         ["[episodes.healthy]: no colour"])
        self.assertEqual(problems(document().replace('description = "a reason"', "")),
                         ["[reasons.open]: no description"])


# Each sim's event writers: the sources that name its kinds (`Kind::X`).
WRITERS = {"strims-sim": [HACK.parent / "crates" / "strims-sim" / "src" / "tables.rs"],
           "ppspp": [HACK.parent / "crates" / "strims-ppspp" / "tests" / "sim.rs"]}


def written(sim):
    """The kinds `sim`'s event writers name, by id."""
    variants = {vocab.variant(kid): kid for kid in vocab.emitted()}
    names = {m for f in WRITERS[sim] for m in re.findall(r"\bKind::([A-Z][A-Za-z0-9]*)", f.read_text())}
    return {variants.get(n, n) for n in names}


class Queries(unittest.TestCase):
    def test_the_sims_kinds(self):
        doc = vocab.load()
        strims = ["partition", "repair", "hold", "release", "crash", "restart", "latency", "pause_reading",
                  "resume_reading", "reset_writer", "rate_limit", "connect", "depart", "arrive", "parent_departed",
                  "parent_noticed", "uplink_rate", "competing_start", "competing_stop", "join", "first_read",
                  "caught_up", "copies"]
        ppspp = ["join", "first_read", "caught_up", "leave", "depart", "parent_departed", "relink", "uplink_rate",
                 "latency", "reconnect", "incident"]
        self.assertEqual(sorted(vocab.kinds(sim="strims-sim", doc=doc)), sorted(strims))
        self.assertEqual(sorted(vocab.kinds(sim="ppspp", doc=doc)), sorted(ppspp))
        self.assertEqual(set(vocab.emitted(doc)), set(strims) | set(ppspp))
        self.assertEqual(vocab.notes("parent_lost", "ppspp", doc)[:22], "Timed at the departure")
        self.assertIsNone(vocab.notes("arrive", "ppspp", doc))

    def test_each_kinds_sims_are_the_sims_that_write_it(self):
        for sim in vocab.SIMS:
            self.assertEqual(written(sim), set(vocab.kinds(sim=sim)), sim)

    def test_the_retired_ids_map_by_sim(self):
        self.assertEqual(vocab.retired(), ["first_frame", "up_step", "uplink", "parent_lost"])
        got = {kid: {sim: vocab.current(kid, sim) for sim in vocab.holders(kid)} for kid in vocab.retired()}
        self.assertEqual(got, {
            "first_frame": {"strims-sim": "first_read"},
            "up_step": {"strims-sim": "uplink_rate"},
            "uplink": {"strims-sim": "rate_limit", "ppspp": "uplink_rate"},
            "parent_lost": {"strims-sim": "parent_noticed", "ppspp": "parent_departed"},
        })
        for kid, by in got.items():
            for sim, now in by.items():
                self.assertIn(sim, vocab.kind(now)["sims"], f"{kid}: {sim}'s runs write {now}")
                # The meaning it had stays with it.
                self.assertEqual(vocab.kind(kid)["class"], vocab.kind(now)["class"], kid)

    def test_the_kinds_no_sim_here_writes(self):
        decisions = ["parent_choice", "parent_open", "parent_switch", "prune", "shed_by", "window_collapse",
                     "timeout_loss", "read_skip"]
        self.assertEqual(vocab.kinds("decision"), decisions)
        self.assertEqual(vocab.unwritten(), ["parent_channel_closed", *decisions])
        self.assertTrue(all(vocab.kind(k)["env_gate"] == "SIM_SPEC" for k in vocab.unwritten()))

    def test_names_of_the_other_sections(self):
        self.assertEqual(vocab.derived(), ["start", "seed_down", "seed_up", "present_join", "present_leave", "onset",
                                           "recovered", "collapse"])
        self.assertEqual(vocab.aligns(), vocab.derived() + vocab.scenario())
        self.assertEqual(vocab.phases(), ["startup", "after_down", "after_up", "steady", "after_departure",
                                          "after_disruption", "tail"])
        self.assertEqual(vocab.episodes(), ["healthy", "unhealthy", "collapse"])
        self.assertEqual(vocab.reasons(), ["open", "switch", "lost", "prune", "shed"])
        # An entry code writes by name is the vocabulary's, else it fails.
        self.assertEqual((vocab.named("phases", "startup"), vocab.named("reasons", "switch")), ("startup", "switch"))
        with self.assertRaisesRegex(KeyError, "the vocabulary's reasons have no 'graft'"):
            vocab.named("reasons", "graft")
        # The anchors in the document's order (its ties'): join first of
        # the scenario's, so a viewer's join holds the second a crash's or
        # a leave's span ends in; the start's last, a derived kind's.
        self.assertEqual(list(vocab.anchors()), [
            "join", "partition", "hold", "crash", "latency", "pause_reading", "reset_writer", "rate_limit",
            "connect", "depart", "uplink_rate", "competing_start", "leave", "reconnect", "incident", "start"])
        self.assertTrue(all(vocab.kind(k)["class"] == "scenario" for k in list(vocab.anchors())[:-1]))
        # The kinds the rule reads of the sims' events: the anchors but
        # the start, and the kinds that end their spans.
        self.assertEqual(vocab.rule_kinds(), [
            "join", "partition", "repair", "hold", "release", "crash", "restart", "latency", "pause_reading",
            "resume_reading", "reset_writer", "rate_limit", "connect", "depart", "uplink_rate", "competing_start",
            "competing_stop", "leave", "reconnect", "incident"])
        self.assertEqual(vocab.anchor_fields()["rate_limit"], {
            "phase": {"down": "after_down", "up": "after_up"}, "level": True, "horizon_s": 60})
        self.assertEqual(vocab.anchor_fields()["leave"], {"phase": "after_departure", "until": "join",
                                                          "horizon_s": 20})
        # No observation anchors: the run's response never sets a phase.
        self.assertFalse({k for k in vocab.anchors() if vocab.kind(k)["class"] not in ("scenario", "derived")})

    def test_current_follows_replaced_by(self):
        text = document(kinds=(("a", "scenario", ["strims-sim", "ppspp"]), ("b", "scenario", ["ppspp"]),
                               ("c", "scenario", ["strims-sim"])))
        text = given(given(text, "c", 'replaced_by = { strims-sim = "a" }'), "b", 'replaced_by = "a"')
        doc = vocab.parse(text)
        self.assertEqual(vocab.problems(doc), [])
        self.assertEqual([vocab.current(k, "ppspp", doc) for k in ("a", "b", "c", "z")], ["a", "a", None, None])
        self.assertEqual(vocab.current("c", "strims-sim", doc), "a")
        self.assertEqual(vocab.retired(doc), ["b", "c"])
        self.assertEqual(vocab.kinds(doc=doc), ["a"])

    def test_who_holds_a_kind_its_value_and_its_knob(self):
        self.assertEqual(vocab.holders("latency"), ["strims-sim", "ppspp"])
        self.assertEqual(vocab.holders("leave"), ["ppspp"])
        self.assertEqual(vocab.holders("parent_choice"), list(vocab.SIMS))
        self.assertEqual(vocab.holders("collapse"), [])
        self.assertEqual(vocab.value_text(vocab.kind("rate_limit")), "the cap, bytes/s")
        self.assertEqual(vocab.value_text(vocab.kind("uplink")), "the limit, bytes/s")
        self.assertEqual(vocab.value_text(vocab.kind("copies")), "its copies")
        self.assertIsNone(vocab.value_text(vocab.kind("join")))
        self.assertEqual(vocab.knobs(), {"SIM_SPEC": vocab.unwritten()})
        self.assertEqual(list(vocab.entries("reasons")), vocab.reasons())
        self.assertTrue(vocab.entries("phases")["steady"]["description"])

    def test_scope_is_run_for_a_scenario_kind_that_names_none(self):
        text = given(document(kinds=(("a", "scenario", ["ppspp"]), ("b", "scenario", ["ppspp"]),
                                     ("c", "observation", ["ppspp"]))), "b", 'scope = "viewer"')
        doc = vocab.parse(text)
        self.assertEqual(vocab.problems(doc), [])
        self.assertEqual([vocab.scope(k, doc) for k in ("a", "b", "c")], ["run", "viewer", None])
        self.assertEqual((vocab.scope("join"), vocab.scope("latency")), ("viewer", "run"))


class Fields(unittest.TestCase):
    def test_every_entry_says_what_it_is_and_who_writes_it(self):
        doc = vocab.fields()
        entries = [(f"{s}.{k}", e) for s in ("rows", "meta") for k, e in doc[s].items()]
        entries += [(f"columns.{t}.{k}", e) for t, cols in doc["columns"].items() for k, e in cols.items()]
        self.assertTrue(entries)
        for name, e in entries:
            with self.subTest(name):
                self.assertTrue(e.get("description"))
                self.assertLessEqual(set(e["sims"]), set(vocab.SIMS))
                self.assertLessEqual(set(e.get("lines", ())), set(vocab.LINES))
                self.assertLessEqual(set(e.get("retired", ())), set(vocab.SIMS))
                self.assertTrue(e["sims"] or e.get("lines") or e.get("writers") or e.get("retired"),
                                "an entry no sim writes names the lines or the lab's writers that do, or the sims "
                                "whose older runs hold it")

    def test_a_familys_names_are_its_params_by_sim(self):
        key = "stalls_per_min_{delay}{pop}"
        e = vocab.fields()["rows"][key]
        self.assertEqual(vocab.expand(key, e, "strims-sim"), [
            "stalls_per_min_1250", "stalls_per_min_1250_honest", "stalls_per_min_1250_dishonest",
            "stalls_per_min_4250", "stalls_per_min_4250_honest", "stalls_per_min_4250_dishonest"])
        self.assertEqual(vocab.expand(key, e, "ppspp"), ["stalls_per_min_1250", "stalls_per_min_1250_honest",
                                                         "stalls_per_min_4250", "stalls_per_min_4250_honest"])

    def renamed_doc(self):
        """fields.toml with two renames: lag_old_ms, ppspp's older
        lag_p99_ms, and share_old, each sim's older share by another name."""
        doc = copy.deepcopy(vocab.fields())
        doc["rows"]["lag_old_ms"] = {"sims": [], "retired": ["ppspp"], "replaced_by": "lag_p99_ms", "rename": True,
                                     "type": "int", "unit": "ms", "description": "x"}
        doc["rows"]["share_old"] = {"sims": [], "retired": ["strims-sim", "ppspp"],
                                    "replaced_by": {"strims-sim": "seed_upload_share", "ppspp": "seed_chunk_share"},
                                    "rename": True, "type": "num", "description": "y"}
        return doc

    def test_the_retired_names_are_declared(self):
        decl = vocab.declared()
        for n in ("startup_p50_ms", "startup_p99_ms", "steady_p50_ms", "steady_p99_ms", "up_filler"):
            self.assertEqual(decl["rows"][n][1]["retired"], ["strims-sim"], n)
        for n, cur in (("capacity", "capacity_bps"), ("delivered", "delivered_bps"), ("rate", "rate_bps")):
            self.assertIn(n, decl["columns"]["seed_decisions"])
            self.assertEqual(vocab.resolve(n, "ppspp", "columns.seed_decisions"), cur)
        # Older runs hold them, the generated modules none.
        outputs = vocab.field_outputs()
        self.assertIn('RowInt::LagP99Ms => "lag_p99_ms",', "".join(outputs.values()))
        for path, text in outputs.items():
            for n in ("startup_p50_ms", "startup_p99_ms", "steady_p50_ms", "steady_p99_ms", "up_filler"):
                self.assertNotIn(f'=> "{n}",', text, path)
            for n in ("capacity", "delivered", "rate"):
                self.assertNotIn(f"DecisionCol::{vocab.camel(n)} =>", text, path)
        self.assertIn("DecisionCol::CapacityBps =>", "".join(outputs.values()))

    def test_resolve_follows_replaced_by(self):
        doc = self.renamed_doc()
        self.assertEqual(vocab.resolve("lag_old_ms", "ppspp", doc=doc), "lag_p99_ms")
        self.assertEqual(vocab.resolve("lag_old_ms", doc=doc), "lag_p99_ms")
        self.assertEqual(vocab.resolve("share_old", "strims-sim", doc=doc), "seed_upload_share")
        self.assertEqual(vocab.resolve("share_old", "ppspp", doc=doc), "seed_chunk_share")
        # By sim, each another name: none for every sim.
        self.assertEqual(vocab.resolve("share_old", doc=doc), "share_old")
        # A current name, one retired with no replacement, one no entry holds.
        for n in ("lag_p99_ms", "startup_p99_ms", "no_such_name"):
            self.assertEqual(vocab.resolve(n, "strims-sim", doc=doc), n)

    def test_renames_and_aliases(self):
        doc = self.renamed_doc()
        self.assertEqual([r for r in vocab.renames(doc=doc) if r[0] in ("lag_old_ms", "share_old")],
                         [("lag_old_ms", "lag_p99_ms", ["ppspp"]), ("share_old", "seed_upload_share", ["strims-sim"]),
                          ("share_old", "seed_chunk_share", ["ppspp"])])
        self.assertEqual(vocab.renames(doc=doc)[:-3], vocab.renames())
        self.assertEqual(vocab.aliases("lag_p99_ms", doc=doc), ["lag_p99_ms", "lag_old_ms"])
        self.assertEqual(vocab.aliases("lag_old_ms", doc=doc), ["lag_p99_ms", "lag_old_ms"])
        self.assertEqual(vocab.aliases("seed_chunk_share", doc=doc), [*vocab.aliases("seed_chunk_share"), "share_old"])
        self.assertEqual(vocab.aliases("delivery", doc=doc), ["delivery"])
        # A retired name with a replacement but no rename (a column's) is no alias.
        self.assertEqual(vocab.renames("columns.seed_decisions"), [])

    def test_a_canonical_row_holds_its_renamed_fields_under_their_current_names(self):
        doc = self.renamed_doc()
        row = {"lag_old_ms": 5, "share_old": 0.25, "delivery": 1.0}
        self.assertEqual(vocab.canonical_row(row, "ppspp", doc=doc),
                         {"delivery": 1.0, "lag_p99_ms": 5, "seed_chunk_share": 0.25})
        # strims-sim's lag_old_ms is no rename: it stays as written.
        self.assertEqual(vocab.canonical_row(row, "strims-sim", doc=doc),
                         {"lag_old_ms": 5, "delivery": 1.0, "seed_upload_share": 0.25})
        # A row holding both keeps the current name's value.
        self.assertEqual(vocab.canonical_row({"lag_old_ms": 5, "lag_p99_ms": 7}, "ppspp", doc=doc),
                         {"lag_p99_ms": 7})
        self.assertEqual(row, {"lag_old_ms": 5, "share_old": 0.25, "delivery": 1.0})


if __name__ == "__main__":
    unittest.main()

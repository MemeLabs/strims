#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simlab.vocab, stdlib alone: the generated files (vocab.rs, vocab.ts)
are the generator's output byte for byte; the document checks (ids
unique, references to what exists, derived kinds disjoint from the sims'
kinds, a colour to each phase, episode and reason, one to a name of its
section); and the queries over it:

  python3 hack/simlab/tests/test_vocab.py
"""

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
    its own colour), one episode and one reason, then `extra`."""
    out = []
    for kid, cls, sims in kinds:
        s = "" if sims is None else "sims = [" + ", ".join(f'"{x}"' for x in sims) + "]\n"
        out.append(KIND.format(kid=kid, cls=cls, sims=s))
    out += [PHASE.format(name=p, i=i) for i, p in enumerate(phases)]
    out.append('\n[episodes.healthy]\ndescription = "a spell"\ncolour = "#000000"\n'
               '\n[reasons.open]\ndescription = "a reason"\ncolour = "#000000"\n')
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
            'until = "nope"': ["[kinds.b]: until 'nope' is not a current kind"],
            'phase = "nope"': ["[kinds.b]: phase 'nope' is not a phase"],
            'phase = { down = "steady", up = "steady" }': ["[kinds.b]: a phase table is {down, up}, with level = true"],
            'scope = "world"': ["[kinds.b]: scope 'world' is none of run, viewer"],
            "horizon_s = -1": ["[kinds.b]: horizon_s is not a number of seconds"],
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
        self.assertEqual(problems(given(good, "b", 'phase = { down = "steady", up = "steady" }\nlevel = true')), [])

    def test_sections_and_colours(self):
        self.assertEqual(problems(document() + '\n[colours.x]\ndescription = "x"\n'),
                         ["[colours]: not a section (kinds, phases, episodes, reasons)"])
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


class Queries(unittest.TestCase):
    def test_the_sims_kinds(self):
        doc = vocab.load()
        strims = ["partition", "repair", "hold", "release", "crash", "restart", "latency", "pause_reading",
                  "resume_reading", "reset_writer", "uplink", "connect", "depart", "arrive", "parent_lost",
                  "up_step", "competing_start", "competing_stop", "join", "first_frame", "caught_up", "copies"]
        ppspp = ["join", "first_read", "caught_up", "leave", "depart", "parent_lost", "relink", "uplink", "latency",
                 "reconnect", "incident"]
        self.assertEqual(sorted(vocab.kinds(sim="strims-sim", doc=doc)), sorted(strims))
        self.assertEqual(sorted(vocab.kinds(sim="ppspp", doc=doc)), sorted(ppspp))
        self.assertEqual(set(vocab.emitted(doc)), set(strims) | set(ppspp))
        self.assertEqual(vocab.notes("parent_lost", "ppspp", doc)[:22], "Timed at the departure")
        self.assertIsNone(vocab.notes("arrive", "ppspp", doc))

    def test_the_kinds_no_sim_here_writes(self):
        decisions = ["parent_choice", "parent_open", "parent_switch", "prune", "shed_by", "window_collapse",
                     "timeout_loss", "read_skip"]
        self.assertEqual(vocab.kinds("decision"), decisions)
        self.assertEqual(vocab.unwritten(), ["parent_channel_closed", *decisions])
        self.assertTrue(all(vocab.kind(k)["with"] == "SIM_SPEC" for k in vocab.unwritten()))
        self.assertEqual(vocab.retired(), [])

    def test_names_of_the_other_sections(self):
        self.assertEqual(vocab.derived(), ["start", "seed_down", "seed_up", "present_join", "present_leave", "onset",
                                           "recovered", "collapse"])
        self.assertEqual(vocab.aligns(), vocab.derived() + vocab.scenario())
        self.assertEqual(vocab.phases(), ["startup", "after_down", "after_up", "steady", "tail"])
        self.assertEqual(vocab.episodes(), ["healthy", "unhealthy", "collapse"])
        self.assertEqual(vocab.reasons(), ["open", "switch", "lost", "prune", "shed"])
        self.assertEqual(vocab.anchors(), {})

    def test_current_follows_replaced_by(self):
        text = document(kinds=(("a", "scenario", ["ppspp"]), ("b", "scenario", ["ppspp"]),
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
        self.assertEqual(vocab.value_text(vocab.kind("uplink")), "the limit, bytes/s")
        self.assertEqual(vocab.value_text(vocab.kind("copies")), "its copies")
        self.assertIsNone(vocab.value_text(vocab.kind("join")))
        self.assertEqual(vocab.knobs(), {"SIM_SPEC": vocab.unwritten()})
        self.assertEqual(list(vocab.entries("reasons")), vocab.reasons())
        self.assertTrue(vocab.entries("phases")["steady"]["description"])


if __name__ == "__main__":
    unittest.main()

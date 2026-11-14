#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""check.py's verdicts over a compare's results and a storybook index, on
results and indexes made up here: --fail-on-error's (head_failures),
--summary's Markdown (summary_markdown), --strict's catalog verdict
(catalog_failed) and the flags' checks. No browser or build runs.

    python3 hack/storybook/report_test.py"""

import argparse
import contextlib
import io
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import check  # noqa: E402


def shot(status="ok", error=None):
    return {"status": status, "error": error, "errors": [error] if error else [], "png": None}


def result(kind, route, mode="dark", **shots):
    r = {"mode": mode, "route": f"/storybook/{route}", "kind": kind, "time": 1.0,
         "shots": shots or {"base": shot(), "head": shot()}}
    if kind == "changed":
        r["ratio"] = 0.01
    return r


def args(**kw):
    a = {"leaks": False, "compare": "main", "no_scope": False, "scope": None, "holes": []}
    return argparse.Namespace(**(a | kw))


def builds():
    return check.Build("head", "", "a" * 40, "frames"), check.Build("base", "", "b" * 40, "frames")


def summary(results, modes=("dark",), **kw):
    return check.summary_markdown(results, builds(), list(modes), Path("/r/index.html"), 12.0, args(**kw))


class HeadFailures(unittest.TestCase):
    def test_a_head_failure_counts_and_a_base_one_doesnt(self):
        head = result("failing", "a/A/Head", base=shot(), head=shot("FAIL", "boom"))
        base = result("failing", "a/A/Base", base=shot("FAIL", "old"), head=shot())
        self.assertEqual(check.head_failures([head, base]), [head])

    def test_an_added_story_failing_counts_and_a_removed_one_doesnt(self):
        added = result("failing", "a/A/New", head=shot("FAIL", "boom"))
        removed = result("failing", "a/A/Gone", base=shot("FAIL", "old"))
        self.assertEqual(check.head_failures([added, removed]), [added])

    def test_changes_additions_removals_and_identical_stories_dont(self):
        rs = [result("changed", "a/A/C"), result("added", "a/A/N", head=shot()),
              result("removed", "a/A/R", base=shot()), {"mode": "dark", "route": "/storybook/a/A/I",
                                                        "kind": "identical", "time": 0.0}]
        self.assertEqual(check.head_failures(rs), [])


class Summary(unittest.TestCase):
    def test_counts_by_mode(self):
        rs = [result("changed", "a/A/One"), result("unchanged", "a/A/Two"),
              result("unchanged", "a/A/One", mode="portrait"), result("unchanged", "a/A/Two", mode="portrait")]
        md = summary(rs, modes=("dark", "portrait"))
        self.assertIn("| mode | changed | failing | added | removed | skipped | unchanged | identical |", md)
        self.assertIn("| dark | 1 | 0 | 0 | 0 | 0 | 1 | 0 |", md)
        self.assertIn("| portrait | 0 | 0 | 0 | 0 | 0 | 2 | 0 |", md)
        self.assertIn("head `aaaaaaaaaaaa` vs base `bbbbbbbbbbbb` (main)", md)
        self.assertIn("2 stories in 2 mode(s) in 12 s", md)
        self.assertIn("Report: `/r/index.html`", md)

    def test_changed_by_share_most_first_and_capped(self):
        rs = []
        for i in range(55):
            r = result("changed", f"a/A/S{i:02}")
            r["ratio"] = i / 1000
            rs.append(r)
        md = summary(rs)
        self.assertIn("**Changed, by the share of pixels** (55):", md)
        lines = [line for line in md.splitlines() if line.startswith("- dark")]
        self.assertEqual(lines[0], "- dark · `a/A/S54` 5.40%")
        self.assertEqual(len(lines), 50)
        self.assertIn("- … and 5 more", md)

    def test_failures_split_by_build_with_their_first_error(self):
        rs = [result("failing", "a/A/In%20head", base=shot(), head=shot("FAIL", "TypeError: `x` is undefined\nat y")),
              result("failing", "a/A/Base", base=shot("FAIL", "old error"), head=shot())]
        md = summary(rs)
        self.assertIn("**Failing in the head build** (1):\n- dark · `a/A/In head`: `TypeError: 'x' is undefined`", md)
        self.assertIn("**Failing in the base build only** (1):\n- dark · `a/A/Base`: `old error`", md)

    def test_nothing_to_list_lists_nothing(self):
        md = summary([result("unchanged", "a/A/One")])
        self.assertNotIn("**", md)

    def test_scope_and_holes(self):
        scope = {"why": None, "identical": {"/storybook/b/B/X", "/storybook/b/B/Y"},
                 "changed": {"css-loader!./src/a.scss": {"/storybook/a/A/One"}}}
        md = summary([result("changed", "a/A/One")], scope=scope, holes=[("dark", "/storybook/a/A/One", "x.js")])
        self.assertIn("2 identical by the builds' manifests, not captured", md)
        self.assertIn("Inputs that differ, by the stories each reaches: `./src/a.scss` (1)", md)
        self.assertIn("**Holes in a manifest** (1):\n- dark · `a/A/One`: `x.js`", md)

    def test_unscoped(self):
        md = summary([result("unchanged", "a/A/One")], scope={"why": "the base has no manifest",
                                                              "identical": set(), "changed": {}})
        self.assertIn("not scoped (the base has no manifest): every story captured", md)

    def test_leaks(self):
        self.assertIn("### Storybook: in place vs fresh, `aaaaaaaaaaaa`", summary([], leaks=True))


class Catalog(unittest.TestCase):
    INDEX = {"files": [{"path": "components/A.tsx", "status": "covered"},
                       {"path": "components/B.tsx", "status": "missing"}],
             "stale": []}

    def verdict(self, index, strict=True, files=()):
        with contextlib.redirect_stdout(io.StringIO()) as out:
            failed = check.catalog_failed(index, argparse.Namespace(strict=strict, files=list(files)))
        return failed, out.getvalue()

    def test_a_missing_file_fails_strict_only(self):
        self.assertEqual(self.verdict(self.INDEX), (True, "strict: the catalog has missing or stale files\n"))
        self.assertEqual(self.verdict(self.INDEX, strict=False), (False, ""))

    def test_files_limit_what_counts(self):
        self.assertFalse(self.verdict(self.INDEX, files=["pages/"])[0])
        self.assertTrue(self.verdict(self.INDEX, files=["components/B"])[0])

    def test_a_stale_entry_fails(self):
        index = {"files": [], "stale": [{"path": "pages/C.tsx", "module": "pages/C", "kind": "missing"}]}
        self.assertTrue(self.verdict(index)[0])


class Flags(unittest.TestCase):
    def test_summary_and_fail_on_error_need_a_compare(self):
        for flag in (["--summary", "x.md"], ["--fail-on-error"]):
            with self.subTest(flag=flag), contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit) as e:
                    check.main(["--port", "1", *flag])
                self.assertEqual(e.exception.code, 2)


if __name__ == "__main__":
    unittest.main()

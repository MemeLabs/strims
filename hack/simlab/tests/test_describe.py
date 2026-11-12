#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simlab.describe over a catalog of its own: every topic's text (the
index, the glossary, the tables, a table, the macros and one, the metrics,
the event kinds by class with their counts and one kind, the phases,
episodes and reasons, the saved views and one), the Ask prompt's kinds
without counts, a name naming a table and a view giving both, an unknown
topic refused with the topics named, and no text
naming the Ask panel's tools, over the console's own views too; the Ask
prompt taking the glossary's sections where ask_prompt.md names them:

  python3 hack/simlab/tests/test_describe.py
"""

import re
import sys
import tempfile
import unittest
from pathlib import Path

HACK = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(HACK))

from simlab import bootstrap  # noqa: E402

if __name__ == "__main__":
    bootstrap.engine(__file__)

from simlab import ask, catalog, describe, engine, views, vocab  # noqa: E402
from simlab.views import Param, View  # noqa: E402

# The Ask panel's tools named by no catalog object or plain word.
ASK_TOOLS = [t["name"] for t in ask.TOOLS if "_" in t["name"]]


def make_lake(lake):
    """A catalog holding the tables catalog.schema reads, two of the sims'
    and a raw one."""
    (lake / "data").mkdir(parents=True)
    w = engine.writer(lake)
    try:
        w.execute("""
CREATE SCHEMA raw;
CREATE TABLE raw.runs (k VARCHAR);
CREATE TABLE runs (cell VARCHAR, arm VARCHAR, seed BIGINT, lag_p99_ms DOUBLE);
CREATE TABLE events (k VARCHAR, t_us BIGINT, kind VARCHAR);
CREATE TABLE experiments (exp_id VARCHAR, kind VARCHAR, started_at VARCHAR, tiers VARCHAR[], arm_labels VARCHAR[],
                          base VARCHAR, question VARCHAR, cells BIGINT, jobs BIGINT);
INSERT INTO experiments VALUES ('e1', 'farm', '2026-11-10T10:00:00+00:00', ['peers'], ['a', 'off'], 'off', 'a?', 1, 2);
CREATE TABLE table_catalog (name VARCHAR, kind VARCHAR, grain VARCHAR, keys VARCHAR, joins VARCHAR,
                            description VARCHAR);
INSERT INTO table_catalog VALUES
  ('runs', 'table', 'a run', 'cell, arm, seed', NULL, 'each run''s lag'),
  ('events', 'view', 'an event', 'k, t_us', 'viewers USING (k, src, node)', 'the sims'' timed events'),
  ('experiments', 'table', 'an experiment', 'exp_id', NULL, 'every experiment'),
  ('structural', 'table macro', NULL, '(exp_name)', NULL, 'the cells every arm collapsed on every seed'),
  ('fisher_p', 'macro', NULL, '(a, b, c, d)', NULL, 'the two-sided Fisher exact p');
CREATE TABLE metric_catalog (name VARCHAR, tbl VARCHAR, type VARCHAR, unit VARCHAR, description VARCHAR,
                             better VARCHAR, kind VARCHAR, tie DOUBLE, guard VARCHAR, source VARCHAR,
                             first_seen VARCHAR, last_seen VARCHAR);
INSERT INTO metric_catalog VALUES
  ('lag_p99_ms', 'runs', 'DOUBLE', 'ms', 'the honest viewers'' p99 lag', NULL, NULL, NULL, NULL, NULL, 'x', 'y'),
  ('t_us', 'events', 'BIGINT', 'us', NULL, NULL, NULL, NULL, NULL, NULL, 'x', 'y'),
  ('lag_p99_ms', 'suite', NULL, 'ms', 'lag p99', 'lower', 'ratio', 0.05, 'lo', 'r.lag_p99_ms', 'x', 'y');
CREATE TABLE event_kinds (kind VARCHAR, sim VARCHAR, class VARCHAR, description VARCHAR, value_meaning VARCHAR,
                          node VARCHAR, peer VARCHAR, detail VARCHAR, details VARCHAR[], notes VARCHAR,
                          env_gate VARCHAR, replaced_by VARCHAR, documented BOOLEAN, n BIGINT,
                          first_seen TIMESTAMPTZ, last_seen TIMESTAMPTZ);
INSERT INTO event_kinds VALUES
  ('join', 'ppspp', 'scenario', 'A viewer joins.', NULL, 'the viewer', NULL, NULL, NULL, 'Each viewer''s.', NULL,
   NULL, true, 12, '2026-11-08 12:00:00+00', '2026-11-10 12:00:00+00'),
  ('join', 'strims-sim', 'scenario', 'A viewer joins.', NULL, 'the viewer', NULL, NULL, NULL, NULL, NULL, NULL, true,
   0, NULL, NULL),
  ('parent_lost', 'ppspp', 'observation', 'A parent departed.', NULL, 'the viewer', 'the parent', NULL, NULL, NULL,
   NULL, 'parent_departed', true, 4, '2026-11-08 12:00:00+00', '2026-11-08 12:00:00+00'),
  ('parent_lost', 'strims-sim', 'observation', 'A parent departed.', NULL, 'the viewer', 'the parent', NULL, NULL,
   NULL, NULL, 'parent_noticed', true, 0, NULL, NULL),
  ('prune', 'ppspp', 'decision', 'The node pruned the parent.', 'the stream', 'the node', 'the parent', 'the rule',
   ['silent', 'lossy'], NULL, 'SIM_SPEC', NULL, true, 0, NULL, NULL),
  ('collapse', 'derived', 'derived', 'A collapse began.', 'its length, s', NULL, NULL, NULL, NULL, NULL, NULL, NULL,
   true, 3, '2026-11-09 12:00:00+00', '2026-11-09 12:00:00+00'),
  ('old_kind', 'ppspp', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, false, 5,
   '2026-11-09 12:00:00+00', '2026-11-10 12:00:00+00');
CREATE TABLE catalog_info (key VARCHAR, value VARCHAR);
INSERT INTO catalog_info VALUES ('indexed_at', '2026-11-10T10:00:00+00:00');""")
    finally:
        w.close()


# The fixture's event kinds, as the events topic gives them.
KINDS_WITH_COUNTS = """### scenario
Exogenous: what the scenario does to the run.

- join (ppspp, strims-sim): A viewer joins.
  - node: the viewer
  - ppspp: Each viewer's.
  - events: ppspp 12 (2026-11-08 to 2026-11-10), strims-sim 0

### observation
The run's response.

- parent_lost (ppspp, strims-sim): A parent departed.
  - node: the viewer; peer: the parent
  - ppspp's replaced by parent_departed
  - strims-sim's replaced by parent_noticed
  - events: ppspp 4 (2026-11-08 to 2026-11-08), strims-sim 0

### decision
A node's scheduler decisions, from its event log.

- prune (env_gate SIM_SPEC; ppspp): The node pruned the parent.
  - node: the node; peer: the parent; value: the stream; detail: the rule (silent or lossy)
  - events: ppspp 0

### derived
The verdict's own, made from the series (`derived_events`).

- collapse: A collapse began.
  - value: its length, s
  - events: derived 3 (2026-11-09 to 2026-11-09)

### undocumented
Kinds the runs hold that the vocabulary lacks.

- old_kind (ppspp): not in the vocabulary
  - events: ppspp 5 (2026-11-09 to 2026-11-10)
"""


def setUpModule():
    global TMP, DIR, POOL, ROOT
    TMP = tempfile.TemporaryDirectory(prefix="describe-test-")
    DIR = Path(TMP.name)
    make_lake(DIR / "lake")
    POOL = engine.Pool(DIR / "lake", DIR / "spill", size=1, guard=engine.Guard("1GB", 1, "1GiB"))
    ROOT = DIR / "views"
    views.save("by_cell", View("by_cell", "By cell", "a cell's runs", [Param("cell", "cell", "calm")],
                               "-- Copyright 2026 Strims contributors\n-- SPDX-License-Identifier: AGPL-3.0-only\n"
                               "SELECT arm, lag_p99_ms FROM runs WHERE cell = $cell ORDER BY arm"), ROOT)
    views.save("experiments", View("experiments", "Experiments", "", [], "SELECT * FROM experiments"), ROOT)


def tearDownModule():
    POOL.close()
    TMP.cleanup()


def text(topic=None, root=None):
    return describe.describe(topic, POOL, ROOT if root is None else root)


class Topics(unittest.TestCase):
    def test_the_index_names_the_topics_and_every_name(self):
        t = text()
        for name in (*describe.TOPICS, "TABLE", "MACRO", "VIEW"):
            self.assertRegex(t, rf"(?m)^  {name} ", name)
        self.assertIn("Tables and views: catalog_info, event_kinds, events, experiments, metric_catalog, runs,\n"
                      "  table_catalog\n", t)
        self.assertIn("Macros: fisher_p, structural", t)
        self.assertIn("Saved views: by_cell, experiments", t)
        self.assertNotIn("raw", t.split("Tables and views:")[1])
        self.assertIn("\n\nThe console's graphs and addresses (a view's panels, the selection in a console URL or the "
                      "user's\n  open tab, links to a view): python3 hack/console/ui.py describe\n\n", t)
        # Then the studies' reports, each by its command.
        reports = t.split("python3 hack/console/ui.py describe\n\n")[1]
        self.assertTrue(reports.startswith("The studies' reports over the catalog (hack/simlab)"), reports)
        self.assertEqual(" ".join(reports.split()), describe.REPORTS)
        for cmd in ("forecast-report", "coverage-report"):
            self.assertIn(f"`simdb.py {cmd}`", describe.REPORTS)

    def test_the_glossary(self):
        t = text("glossary")
        self.assertEqual(t, describe.glossary())
        self.assertTrue(t.startswith("# The sims' results: the terms\n\n## The sims and their results\n\n"))
        self.assertNotIn("SPDX", t)
        self.assertRegex(t, r"\*\*startup\s+exclusion\*\*")
        for needle in ("**Structural cells.**", "Fisher exact p", "1250 ms", "4250 ms", "## Writing the SQL",
                       "DuckDB's dialect"):
            self.assertIn(needle, t)

    def test_tables_and_a_table(self):
        t = text("tables")
        self.assertRegex(t, r"(?m)^runs +table +a run$")
        self.assertRegex(t, r"(?m)^events +view +an event$")
        self.assertIn("`raw` holds the sources they fold, which you rarely need: runs.", t)
        self.assertEqual(text("runs"), "### runs (table)\ngrain: a run; keys: cell, arm, seed\neach run's lag\n"
                                       "columns: cell VARCHAR, arm VARCHAR, seed BIGINT, lag_p99_ms DOUBLE [ms]\n"
                                       "- lag_p99_ms: the honest viewers' p99 lag\n")

    def test_macros_metrics_and_events(self):
        m = text("macros")
        self.assertEqual(m, "## Macros\n\n- `fisher_p(a, b, c, d)` (macro): the two-sided Fisher exact p\n"
                            "- `structural(exp_name)` (table macro): the cells every arm collapsed on every seed\n")
        self.assertEqual(text("structural"), m.splitlines()[-1] + "\n")
        self.assertIn("- lag_p99_ms: better lower, kind ratio, tie 0.05, guard lo, unit ms; source `r.lag_p99_ms`; "
                      "lag p99\n", text("metrics"))
        # `events` is a view of the catalog and the event kinds' topic.
        e = text("events")
        self.assertTrue(e.startswith("### events (view)\ngrain: an event; keys: k, t_us; joins: viewers USING "
                                     "(k, src, node)\n"), e)
        self.assertIn("columns: k VARCHAR, t_us BIGINT [us], kind VARCHAR\n\n## Event kinds\n", e)
        self.assertTrue(e.endswith(KINDS_WITH_COUNTS), e)

    def test_the_event_kinds_by_class_their_counts_only_in_the_topic(self):
        self.assertEqual(text("events").split("\n\n## Event kinds\n\n")[1].split("\n\n", 1)[1], KINDS_WITH_COUNTS)
        prompt = describe.catalog_text(catalog.schema(POOL))
        self.assertTrue(prompt.endswith(re.sub(r"(?m)^  - events: .*\n", "", KINDS_WITH_COUNTS)), prompt)
        self.assertNotIn("events: ppspp 12", prompt)

    def test_an_event_kind(self):
        self.assertEqual(text("join"), "- join (scenario; ppspp, strims-sim): A viewer joins.\n  - node: the viewer\n"
                                       "  - ppspp: Each viewer's.\n"
                                       "  - events: ppspp 12 (2026-11-08 to 2026-11-10), strims-sim 0\n")
        self.assertTrue(text("prune").startswith("- prune (decision; env_gate SIM_SPEC; ppspp): The node pruned"))
        self.assertEqual(text("old_kind"), "- old_kind (ppspp): not in the vocabulary\n"
                                           "  - events: ppspp 5 (2026-11-09 to 2026-11-10)\n")
        self.assertIn("Event kinds: collapse, join, old_kind, parent_lost, prune\n", text())
        with self.assertRaisesRegex(ValueError, r"^no topic 'jion' \(did you mean join\?\)"):
            text("jion")

    def test_the_phases_episodes_and_reasons(self):
        t = describe.describe("phases")
        self.assertEqual(text("phases"), t)
        self.assertTrue(t.startswith("# Phases, episodes and reasons\n\n"), t)
        for section in describe.SECTIONS:
            names = list(vocab.entries(section))
            self.assertEqual(names, getattr(vocab, section)())
            block = t.split(f"## {section.capitalize()}\n", 1)[1].split("\n\n## ")[0].rstrip("\n") + "\n"
            self.assertEqual(re.findall(r"(?m)^- ([a-z_]+): ", block), names)
            for name, v in vocab.entries(section).items():
                self.assertIn(f"- {name}: {v['description']}\n", block)

    def test_the_views_and_a_view(self):
        v = text("views")
        self.assertIn("## by_cell: By cell\na cell's runs\nparams: cell (cell, default calm)\n", v)
        self.assertNotIn("```sql", v)
        self.assertEqual(text("by_cell"), "## by_cell: By cell\na cell's runs\nparams: cell (cell, default calm)\n"
                                          "```sql\nSELECT arm, lag_p99_ms FROM runs WHERE cell = $cell ORDER BY arm"
                                          "\n```\n")
        # A table and a saved view of one name: both, the table first.
        both = text("experiments")
        self.assertRegex(both, r"(?s)\A### experiments \(table\)\n.*\n\n## experiments: Experiments\n```sql\n"
                               r"SELECT \* FROM experiments\n```\n\Z")

    def test_an_unknown_topic_names_the_topics(self):
        with self.assertRaises(ValueError) as e:
            text("run")
        msg = str(e.exception)
        self.assertIn("no topic 'run' (did you mean runs", msg)
        for t in describe.TOPICS:
            self.assertIn(t, msg)
        with self.assertRaisesRegex(ValueError, r"^no topic 'zzz': the topics are glossary, tables"):
            text("zzz")

    def test_no_text_names_an_ask_tool(self):
        self.assertEqual(ASK_TOOLS, ["run_sql", "run_view", "arm_scores", "fetch_data", "propose_farm_run"])
        real = views.saved()
        self.assertTrue(real)
        outputs = {(t, "fixture"): text(t) for t in [None, *describe.TOPICS, "runs", "events", "experiments",
                                                     "structural", "fisher_p", "by_cell"]}
        outputs |= {(t, "views"): text(t, views.VIEWS) for t in [None, "views", *(v.name for v in real)]}
        for key, out in outputs.items():
            for tool in ASK_TOOLS:
                self.assertNotRegex(out, rf"\b{tool}\b", key)
            self.assertNotIn("`chart`", out, key)
            self.assertNotIn("Ask", out, key)


class Prompt(unittest.TestCase):
    def test_the_prompt_takes_the_glossarys_sections_where_it_names_them(self):
        p = ask.prompt_text()
        self.assertNotIn("<!--", p)
        self.assertIn("## The sims and their results\n\n" + describe.glossary_section("The sims and their results")
                      + "\n\n## Writing the SQL\n\n", p)
        sql = p.split("## Writing the SQL\n\n", 1)[1]
        bullets = re.findall(r"(?m)^- (.{0,16})", sql)
        self.assertEqual(bullets, ["One statement pe", "DuckDB's dialect", "A chart's spec t"])
        self.assertIn(describe.glossary_section("Writing the SQL") + "\n- A chart's spec", sql)

    def test_a_section_the_glossary_lacks(self):
        with self.assertRaisesRegex(KeyError, "no section 'Nope'"):
            describe.glossary_section("Nope")

    def test_the_catalog_and_views_texts_are_the_prompts(self):
        schema = catalog.schema(POOL)
        saved = views.saved(ROOT)
        p = ask.system_prompt(schema, saved)
        self.assertIn("\n\n" + describe.catalog_text(schema) + "\n# The saved views\n\n" + ask.VIEWS_INTRO + "\n\n", p)
        self.assertTrue(p.endswith("\n".join(describe.view_lines(views.get("experiments", ROOT)))))
        for t in ("runs", "events"):
            self.assertIn(text(t).split("\n\n## Event kinds")[0], p)


if __name__ == "__main__":
    unittest.main()

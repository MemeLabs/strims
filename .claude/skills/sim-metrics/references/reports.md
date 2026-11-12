# An analysis module and its report command

Main's example: 56cee325 (`hack/simlab/forecast.py`, `hack/simlab/regimes.py`
and `simdb.py forecast-report`).

1. **The module, `hack/simlab/<name>.py`.**
   - Its module docstring pins the definitions: what a row of the study
     is, what each section computes, and the terms.
   - It reads the catalog through `simlab.engine`, never the cache's
     files.
   - It selects jobs as the reports before it do (`forecast.Selection`:
     experiments, cells and arm labels as globs, seeds, phases, verdicts)
     and shares their selection flags rather than adding its own.
   - It keeps the selection's grain, (exp, cell, arm, k): a job can sit
     in several experiments and cells (the farm reuses rows), so a report
     that drops exp or cell counts a job once where it scored twice, or
     says why it may.
   - Its temporary tables carry a prefix of its own (forecast's `fs_sel`
     and the rest), so two reports can share a connection.
   - One name per definition: a quantity has one name in the SQL, the
     Python and the printed text.
   - Vocabulary names (phases, kinds, reasons) come from the vocabulary
     (`simlab.vocab`) or `query.TAIL`, never literals in the Python or
     its SQL.
   - It offers `SECTIONS`, `report(sel, sections)` and `report_text`,
     and its results are JSON-able, so the command's `--json` and the
     console can both take them.
2. **A definition the console may show too** is a scoring.sql macro the
   module calls, written over a `jobs` list (the `*_of(jobs)` form, as
   `viewer_lag_of` and `effects_of` are) so that a view calls it with its
   own jobs rather than copying it: `derived.md`.
3. **`hack/simlab/__init__.py`:** a line in the module list.
4. **`hack/sim-db/simdb.py`:** a usage line in its docstring and a
   paragraph saying what the report reads and prints, the choices from
   the module (as `from simlab.forecast import SECTIONS` gives
   forecast-report's), a thin `cmd_<name>_report` that parses and
   prints, and its subparser.
5. **Reachable from the skills and the index:**
   - add the report to sim-lab's row of reports
     (`.claude/skills/sim-lab/SKILL.md`), naming the question it answers
     and its command; test_skills checks that the command exists;
   - name it in describe's index (`describe.index_text`) where the index
     lists the reports.
6. **Tests:** `hack/simlab/tests/test_<name>.py` over a fixture catalog
   of its own (golden_cli's job dirs and the tables the study reads,
   folded, exported and indexed, as `test_forecast.py` builds its own),
   with jobs of both sims (a ppspp sim job among them), each section's
   numbers worked by hand, the command's text and its `--json`.
7. **Goldens:** `golden.py check`; the cli set's help moves with the new
   command: re-capture it and say why.
8. **Validation on runs** goes through sim-lab, under its rules.

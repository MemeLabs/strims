# A saved console view

Main's examples: b0705011 (the console and its first saved views),
c23b84dd (phases, episodes and reasons named and coloured by the
vocabulary), 8959d6c8 (selections on the graphs, kept in the address).

1. **`hack/console/views/NAME.sql`:**
   - a header comment saying what the view shows and what each part of
     its result is (`playback.sql`'s, for one);
   - `$name` params, bound by the console's pickers;
   - the jobs selected through `experiment_jobs`, and the tables joined
     `USING (k, src)`;
   - a viewer-second's phase from `viewer_phases`, and a second past the
     verdict's seconds through the lab's one rule for it, never a rule
     of the view's own (no `coalesce(phase, ...)` in a view);
   - scores from the scoring macros (`effects`, `collapse_test`,
     `structural`, `dynamics`), called over the view's jobs where the
     macro takes a `jobs` list (`viewer_lag_of`), never a copy of one;
   - a METRICS quantity read from the rows, `job_metrics` or
     `metric_defs`, never derived again in the view or claimed equal in
     a comment.
   A metric a report prints too is a scoring.sql macro both call
   (`derived.md`).
2. **`NAME.json`:** its title, description and params, each param's
   kind one of `views.KINDS` (`views.checked` checks them).
3. **`NAME.vl.json`**, the Vega-Lite spec:
   - phases, episodes and reasons only through `{"vocab": ...}` scale
     references, which take their names and colours from the
     vocabulary: c23b84dd found a literal `after_down` colour and a
     phase no rule makes;
   - categorical states shown beside the vocabulary's terms get a
     vocabulary section of their own, rather than an exceptions list in
     the view;
   - a time panel names its time field in `hover.x`, which the cursor,
     brushes and selections read.
4. **Optional:**
   - a place in the page's `ORDER` (`src/devtools/pages/Console.tsx`);
   - a new selection key goes in `selection.ts`' `KEYS` and `DEFAULTS`,
     `views.KEYS` and `views.DEFAULTS`, and Console.tsx's `PICKERS`
     together: test_views checks that they agree.
5. **Checks:**
   - `python3 hack/simlab/tests/test_views.py --write-panels`, then
     `test_views.py`: the panels file the TS tests read
     (`hack/simlab/tests/testdata/panels.json`) is views.py's walk of
     every view;
   - `pnpm run test:js`: `sel.test.ts` (each panel's selections),
     `hover.test.ts` (every saved spec compiles) and `vocab.test.ts`
     (`checks.md` on mocha under load);
   - `simdb.py view NAME --param exp=...` over the real catalog;
   - `python3 hack/console/ui.py describe NAME`, and
     `ui.py link NAME --param exp=... --check`;
   - `pnpm run console:build`, then the user's eye in the browser. The
     server reads the views at each request, so it needs no change.
6. **Commit** the view's three files and `panels.json`, and any TS file,
   each by name.

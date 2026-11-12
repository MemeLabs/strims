# A new sim table or log, or a column on one

Main's examples: 6e2d479a (`stalls` and `parents` in both sims) with
3ed18cb8 (their lab side); 6262342c (the request logs in both sims)
with 051596ac (their lab side); e7dfc404 (a departed host's last logs).

## Rust: both sims, one commit

Main gets it first where main has the mechanism, and the lines merge
main (SKILL.md's order of work).

1. **The ppspp crate.** The scheduler's measurement logs live in
   `crates/strims-ppspp/src/scheduler_log.rs`, whose module docs list
   them. A record is written beside the decision it describes, kept
   outside decision state, and read by nothing that decides.
   - The log is on only under its setting (`Runner::set_request_log`
     and the forecast log's).
   - It is bounded per swarm between the driver's takes
     (`FORECAST_CAP`), and counts the records it drops.
   - Work done only for the log runs only while logging, and is pure.
   - The driver takes it through the runner (`Runner::take_held`,
     `Runner::take_requests`), which keeps it past the swarm's
     schedulers.
   - A module the lines share stays world-agnostic: callers pass in
     their world's reading (a STATE or a HAVE), and the variants are a
     superset, so a line's merge of main resolves to one file.
2. **node-core.** A setting in `NodeConfig` (`NodeConfig::request_log`)
   and the records' fields in `NodeMetrics`
   (`crates/strims-node-core/src/event.rs`), so that
   `NodeConfig::last_metrics` hands a node's last records over when it
   departs or crashes. Without it a departed host's last logs were lost
   (e7dfc404). A log delivered through a sink of its own (as
   `NodeConfig::parent_log`'s `ParentSink` is) carries its lost count to
   both sims too.
3. **strims-sim.**
   - A field on `Scenario` (`Scenario::request_logs`) and its knob,
     `STRIMS_SIM_<X>`, read in `crates/strims-sim/src/sweep.rs`, whose
     module docs list the knobs.
   - The records collected in `crates/strims-sim/src/world.rs` as the
     nodes hand them over.
   - The table built with `Table::new` in
     `crates/strims-sim/src/tables.rs`, named in `ZSTD` if it is large.
     A table spills to its file as it fills (67ceca95), so write a large
     log's rows as they are taken rather than holding them to the end.
   - Times in true ms from the stream's start; hosts numbered as
     `events` numbers them; a peer's incarnation as `arrivals` counts
     it (`incarnations` and `incarnation` in tables.rs).
   - A `<log>_lost` key in `series_meta.json`, beside `held_lost` and
     the others, written whenever the knob is on, alike on every line,
     so that 0 says the log was on and lost nothing.
4. **The ppspp sim** (`crates/strims-ppspp/tests/sim.rs`): `Sim::keep_*`
   (`Sim::keep_requests`), the table in `Sim::write_tables` through
   `Table::typed`, the same `ZSTD` entry (test_catalog's `Registries`
   holds the two lists alike), the `series_meta.json` key, and the knob,
   `SIM_<X>`, read with `knob::<u32>` where the job starts.
5. **Columns.**
   - Keys: a log stamped once per pass needs an order column, and a
     wide log whose records repeat every natural key (several of one
     kind at one ms) needs an ordinal column, so its key is unique.
   - "None" and "never" are an empty field, NULL in the lake, never -1
     or 0. List every column that writes a sentinel ("0 until measured"
     is one) and write it empty, or have the view nullify it (step 12),
     with a test of each. When `held_ms` went from -1 to empty,
     simdb.py's censored count read `held_ms < 0` and had to change
     (06c539f2).
   - A unit suffix in each column's name (`catalog.UNITS` reads `_ms`,
     `_us`, `_s`, `_bps` and the rest).
   - A column named for a quantile holds that quantile, on every line
     that writes the table (`predicted_p90_ms` is the 90th percentile
     wherever it is written).
   - A quantity the sims know (a stream count, a host's role) is
     recorded, never inferred in the lab from other columns.
   - Where another line writes a table of the same name, keep a
     column's name where it means the same and give a new meaning a new
     name. The catalog reads every file's schema as it is written, and
     a column a file lacks is NULL.
6. **Docs**, which the catalog reads:
   - A row in each sim's doc table: strims-sim's tables module docs
     (`crates/strims-sim/src/tables.rs`) and the ppspp sim's
     `Sim::write_tables` docs, in the form
     `` | `name` | grain, with `STRIMS_SIM_X=1` | `col` (meaning), ... | ``
     (the ppspp sim's grain names `SIM_X=1`). `catalog.doc_tables` reads
     the rows, and `catalog.column_docs` a column's meaning from the
     parenthesized text right after its backticked name, or the text
     after `name`: when no parenthesis follows. The grain names the
     knob, which test_catalog holds to `query.TABLE_KNOBS`.
   - One parenthetical per column. In `` `a` and `b` (...) `` only `b`
     is documented, and a bare name documents nothing: the catalog then
     falls back to the first doc sentence naming the column
     (`catalog.row_docs`), which is often another column's.
   - A wide log with per-kind columns documents each column once, with
     its meanings by kind inside that column's own parentheses. Better,
     the sims' doc rows come from the shared formatter's column list,
     and the formatter covers the whole row (head columns, types and
     docs), so the two sims' rows can't drift.
   - The `series_meta.json` paragraph and the paragraph listing the
     logs, in both files.
7. **Truth that needs later holds** goes in the lab's view, not the sim:
   the sims log what a decision was made on, and the view joins what
   became of it (e18b801c, `asks` joined to `arrivals`).
8. **Tests:**
   - the log's unit tests in the ppspp crate;
   - a semantic test of every new table in each sim (strims-sim's
     `request_logs_say_what_they_are` in
     `crates/strims-sim/tests/capture.rs`, the ppspp sim's
     `sim_request_logs_say_what_they_are`), with departures where hosts
     leave (`a_departures_logs_keep_every_hold`): a departure behaviour
     gets its test in each sim;
   - the knob turned on in `a_run_with_tables_keeps_its_trace`
     (capture.rs) and `sim_tables_change_no_trace` (sim.rs), so that
     every nextest run checks the trace stays the same, with a scenario
     that takes the path the change touches (departures and crashes for
     what `NodeConfig::last_metrics` hands over).
9. **Neutrality and size:** `checks.md`.
10. **The cli golden:** a doc edit in a file of `catalog.DOC_FILES`
    moves the catalog's outputs. Re-capture in the same commit: e7dfc404
    landed with the golden failing and cf9fb1bc re-captured it after.

A mechanism that exists only on a branch keeps its log's docs in that
branch's sims (SKILL.md's rules); main registers the table (below) and
shows its columns undocumented.

## Python: on main, a commit of its own, `-- hack/`

Main's examples: 9af57af6, 7973c432, b16bf6a3, 051596ac.

11. **`store_host.TABLES`**, and `store_host.SAMPLE_TABLES` for a bulky
    table: a sample table changes the farm's uploads and what a pull
    fetches (b16bf6a3). The farm's hosts run this file (SKILL.md's farm
    rule).
12. **The catalog:**
    - `catalog.STANDARD_KEYS`, once the key is unique on real data:
      count the duplicates over the catalog first, with the key's
      columns in place of these:

          python3 hack/sim-db/simdb.py lake 'SELECT count(*) AS n FROM (SELECT k, node, peer, seq, t_ms FROM NAME GROUP BY ALL HAVING count(*) > 1)'

    - `catalog.STANDARD_JOINS`, each join checked on real data the same
      way;
    - `catalog.NORMALIZED` for what the view changes, and a
      `catalog.view_sql` case where it nullifies a sentinel or joins
      truth (a `query.<t>_view` with its `query.LOGGED` name, as `asks`
      and `haves` have);
    - a view whose truth joins another sample table (`asks` and `haves`
      read `arrivals`) needs that table too: fetch and pull bring it
      along, or the view refuses. It never reads a table it lacks as
      "never held".
13. **`query.TABLE_KNOBS`**, the knob a run writes the table under; the
    hints of `query.need` and simdb.py's help are made from it.
14. **Docs:** the joins paragraph of catalog.py's module docs, and
    simdb.py's `SQL_DOC` where the table joins the viewers. simdb.py's
    help and `simdb.py fetch`'s help list `store_host.TABLES` and
    `store_host.SAMPLE_TABLES` themselves.
15. **Tests:**
    - test_store's `FoldExport` fixture writes the new table, plain and
      `.zst`; its `assertTrue(rows, t)` fails without it.
    - test_query: a pull, and the view's columns and truth;
    - test_catalog's `Tables`: the lake view, its nullified sentinels and
      its table_catalog row; its `Registries` fails until the registries
      above hold the table.
16. **Goldens:** `golden.py check`. The cli set moves in help, sql-help
    and the catalog's fingerprints: re-capture it, then run the full
    check.
17. **Land** with no farm run live; then each line merges main.
18. **After the first runs**, index from main's checkout (an index from
    a branch's checkout writes that branch's docs, keys and views into
    the catalog every session reads): `simdb.py index`, then
    `simdb.py describe NAME`. Read every column's doc: each must be the
    column's own text from its doc row, not a sentence borrowed from
    another column and not empty. The verdict tables' columns get the
    same reading. The index warns of each table whose columns lack a doc
    of their own (`catalog.column_gaps`: none, a doc sentence's, text
    inside another column's parentheses, a colon's doc running on into
    other columns'), and the land gate wants no such line for the
    change's tables. The empty ones:

        python3 hack/sim-db/simdb.py lake 'SELECT tbl, name FROM metric_catalog WHERE tbl = $t AND description IS NULL' --param t=NAME

## A column on an existing table

Main's examples: 8f6f4977, e18b801c. Steps 3 to 10 and 14, the test
fixtures' columns, and `catalog.NORMALIZED` where the view changes it.
A column new to the lake takes the type most of its files give it.

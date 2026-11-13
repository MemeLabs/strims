# An event kind

Main's examples: cc950b02 (one vocabulary that both sims' events name);
f23ab376 (one name in both sims, and four ids retired).

1. **The entry.** A `[kinds.<id>]` table in
   `crates/strims-wan/vocab.toml`. The file's header gives every key and
   what it holds (class, sims, env_gate, description, what node, peer,
   value and detail hold, each sim's notes, a retired kind's
   replaced_by) and the keys that make a kind anchor a verdict phase.
   Only a scenario kind, something the scenario does to the run,
   anchors a phase; a response or a decision never does.
   `vocab.problems` doesn't check that yet, so check it by hand.
2. **The generated files.** `python3 hack/simlab/vocab.py write`, then
   `python3 hack/simlab/vocab.py check`. They are
   `crates/strims-wan/src/vocab.rs` and
   `src/devtools/components/console/vocab.ts`.
3. **Both sims write it through `strims_wan::vocab::Kind`:**
   strims-sim's `events_table` in `crates/strims-sim/src/tables.rs`, the
   ppspp sim's `log` and `TimedEvent` in
   `crates/strims-ppspp/tests/sim.rs`. Where a sim maps its own enum of
   kinds to the vocabulary, it does so in one exhaustive match, so a new
   variant can't go unnamed.
4. **Ids are immutable.** A rename, or a change of meaning, retires the
   old id with `replaced_by` (by sim where the sims differ) and keeps its
   old meaning, so older runs' events still read. Main's golden fixture
   holds main's ids, so a branch's own rename of a kind main writes
   can't land as it is: retire the id on main instead.
5. **Retiring an id means finding its readers.** Grep the old id across
   hack/, src/devtools and the skills, and update each reader: the
   catalog's events view (`catalog.view_sql`), the saved views' `$align`
   choices, `query.aligned`, the reports' kind lists
   (`forecast.ALIGN_KINDS`), the console's pickers, and the SQL comments
   and skills that name it.
6. **Tests:**
   - `cargo test -p strims-wan --test vocab`;
   - `test_vocab.py`, test_catalog's `Kinds`, `test_describe.py`;
   - `pnpm run test:js` (`vocab.test.ts` among them);
   - a sim test that the kind is written, as strims-sim's
     `a_runs_events_are_its_scenarios_and_losses` is for the scenario
     kinds.
7. **Neutrality:** only `events` (and `series_meta.json`, where it
   changes) may differ; every other table stays the parent's byte for
   byte (`checks.md`, `--expect` naming the events columns that move).
8. **The cli golden:** help, catalog (event_kinds) and lake-event-kinds
   move: re-capture and say why.
9. **A kind that anchors a phase** moves the verdict: `derived.md`.
10. **Colours:** each phase, episode and reason has a colour of its own
    in its section, which vocab.py checks. Pick one with the dataviz
    skill's validator.

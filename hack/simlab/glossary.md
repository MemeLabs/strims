<!--
Copyright 2026 Strims contributors
SPDX-License-Identifier: AGPL-3.0-only

The sims' terms and DuckDB's dialect, for whoever reads the catalog:
what `simdb.py describe glossary` prints (simlab.describe.glossary). The
console's Ask prompt takes in its sections where ask_prompt.md names
them, so a change here changes that prompt too.
-->

# The sims' results: the terms

## The sims and their results

- **Sims.** Two simulators of the strims swarm: `ppspp` (strims-ppspp's
  sim) and `strims-sim`. Both are deterministic: a binary, a cell's env
  and a seed give the same run every time.
- **Cell.** A scenario with its settings (`experiment_cells.env`): how
  many viewers, the seed's and viewers' uplinks, steps in them, churn,
  hostile viewers, a WAN profile. A tier is a set of cells
  (`raw.tier_cells`): robust, browsers, net, peers and the rest.
- **Arm.** What an experiment compares: a commit's binary (a `--refs`
  arm) or a knob's setting (a `--sets` arm), named by
  `experiment_jobs.arm_label`. `experiments.base` is the arm the others
  are scored against, where the run names one.
- **Experiment.** A suite farm run (`exp_id` like `sa0755`) or a local
  suite invocation (`suite/<worktree>/<invocation>`): its cells, arms and
  seeds (`experiments`, `experiment_cells`, `experiment_jobs`). A run is a
  job: one cell, arm and seed, keyed `k`, its tables joined
  `USING (k, src)`.
- **The oracle.** A setting (`STRIMS_SIM_KNOWN_UPLINK`, the meter
  benches' `oracle`) that tells hosts their true uplink capacity, steps
  included. It is the reference an uplink meter is measured against: how
  much a perfect meter could buy. No real node knows its uplink, so an
  oracle arm is never a candidate to ship, and a win that only the
  oracle shows is headroom, not a result.
- **The verdict.** analyze.py's judgment of each run (`verdicts`) and
  each second (`verdict_seconds`). A second is unhealthy when the push
  trees fail (`trees_bad`), too little is delivered (`deliv_bad`), or the
  honest viewers' q0.9 lag is high or rising (`lag_high`, `lag_rise`). A
  run **collapsed** when 8 seconds in a row are unhealthy past its
  startup (`verdicts.startup_s`: 25 s, less when a scenario event comes
  sooner): unhealthy seconds during startup never count toward a
  collapse, which is the **startup exclusion**. A **ramp** is a collapse whose honest q0.9 lag never
  passed 4250 ms (viewers some seconds behind who catch up); it still
  counts as a collapse.
- **Phases.** What each second is, from the run's scenario events by one
  rule (`describe phases` gives it, the phases and their anchors):
  `startup` (the run's first 25 s, and a viewer's first 20 s after each
  of its joins), `after_down` and `after_up` (an uplink's rate or cap
  stepping down or up, competing uploads), `after_departure` (a viewer
  departing or leaving, a crash), `after_disruption` (a latency step, a
  partition, the seed's new epoch), else `steady`. Each event opens a
  window for its kind's horizon (a span to its end, then the horizon);
  the latest event's window holds a second. Read a viewer-second's phase
  from `viewer_phases` (each viewer's [start, stop) spans: its own
  startups among them), not from `verdict_seconds.phase`, which is the
  run's; `phase_windows` explains either. A viewer doesn't count in the
  verdict for 20 s after each of its joins, whatever phase those seconds
  are.
- **Lag and playout delays.** A viewer's lag is from the seed's publish
  to the viewer holding the data. The latency budget leaves delivery
  1250 ms (the target: 2 s glass to glass) and 4250 ms (the most: 5 s).
  A player that plays D behind the live edge stalls when data arrives
  more than D late: `stalls` (each stall at each delay, played from the
  viewer's `reads`) and the rows' `stalls_per_min_<D>`,
  `stalled_share_<D>` and `startup_ms_<D>`.
- **Structural cells.** Cells where every arm of an experiment collapses
  on every seed (`structural(exp)`). No arm can be told from another
  there, so leave them out of totals, collapse counts and geomeans, and
  say how many collapses they hold (for example, 64 of an arm's 75).
- **Comparing arms.** Pair runs by seed: the same cell and seed in both
  arms ran the same scenario. A metric's effect in a cell is the
  geometric mean of the per-seed ratios B/A with a 95 % t-interval
  (`effects`); it moved when the interval excludes 1 and passes the
  metric's tie band. An arm's lag p99 over a tier is the geomean over
  cells of those ratios (`arm_score`). An arm's absolute lag p99 is the
  geomean over cells of each cell's geomean of `rows.lag_p99_ms`,
  structural cells left out.
- **Collapse counts.** Judge a difference in collapses by the counts per
  cell and their two-sided Fisher exact p (`collapse_test`, `fisher_p`),
  never by which seeds collapsed: a code change redraws which seeds
  collapse, so collapses move between seeds even when nothing improved.
  The same seed collapsing in both arms, or not, says little. Call a
  difference at p < 0.05, and say when the seeds are too few to tell
  (8 seeds rarely can).
- **Startup tails and steady spikes.** Always say which a tail is. A high
  lag p99 can be the startup of an adversarial cell (every viewer joins
  while the trees form, which no real audience does at once) or spikes in
  steady state that would rebuffer a player. `dynamics(exp)` gives each
  arm and cell's lag p99 in startup, steady and after events (every
  `after_*` phase pooled) and `spike_s_<D>`, the seconds per run past
  startup in which an honest viewer lags past D; `dynamics_phases(exp)`
  gives each phase's lag quantiles and share apart. Take startup and
  steady numbers from these, never from a rule of your own. Weigh steady
  state and joins into a running swarm over startup. `lag_since(exp)`
  gives the honest viewers' lag quantiles against the seconds since each
  phase window's anchor (an uplink step, a departure, a viewer's own
  join, the stream's start) and each span's end, from 10 s before to 120
  s after: how long a disturbance lasts.
- **Overhead.** What the swarm writes besides the stream: `bins`'
  `seed_<kind>_bytes` and `viewer_<kind>_bytes` by message kind (data,
  have, request, integrity, state, ack, announce, ping, header), per
  useful byte (`rows.useful_bytes`, the stream's bytes the viewers had
  to get) or as kbit/s over the run, and the wire (`*_up_bytes` with the
  acknowledgements beside them). The `overhead` view computes them.
- **Data that left S3.** A job's tables are local, on S3 (fetch copies
  them in) or expired after 7 days (`availability`); only a farm rerun
  brings expired data back.

## Writing the SQL

- DuckDB's dialect: `USING (k, src)`, `GROUP BY ALL`, `ORDER BY ALL`,
  `FILTER (WHERE ...)`, `quantile_cont`, `list(...)`, `COLUMNS('regex')`,
  `exp(avg(ln(x)))` for a geomean (only of x > 0).

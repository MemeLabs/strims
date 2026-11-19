---
name: sim-farm
description: "The sim farm (hack/sim-farm: strims-sim and testbed matrices on EC2 spot instances): quoting a run with farm.py estimate, launching, watching, stopping and cleaning up runs, slot memory, regions, S3, spot interruptions, the AWS permissions, what needs the user's approval, and sizing a run to its question. Use when launching, estimating, monitoring, stopping or editing the sim farm or its matrices, and when sizing a farm run. For reading results, use sim-lab."
---

# The sim farm

`hack/sim-farm/farm.py` runs matrices on EC2 spot instances: strims-sim
sweeps (one seed a job, `STRIMS_SIM_SEED`) and testbed matrices
(`kind = "testbed"`: the testbed skill; with a `TABLES_BIN` artifact a
testbed run's tables fold and flush to S3 as a sim job's do, and `simdb.py
pull --run` takes them in). Its module docstring documents
every key, flag and mechanism. The ppspp sim is a correctness harness:
no matrix runs it, and its timings are never evidence.

Every checkout's farm.py uses the main checkout's state (target/sim-farm)
and config, hack/sim-farm/config.toml (gitignored). The account's values
(ids, network, quotas, IAM names) live in that file and in the user's
private `sim-farm-aws` memory. Never write them into the repo.

## Commands

    python3 hack/sim-farm/farm.py estimate MATRIX.toml... [--regions R,...] [--json]
    python3 hack/sim-farm/farm.py run MATRIX.toml... [--job-timeout MIN] [--max-vcpus N] [--keep]
    python3 hack/sim-farm/farm.py run MATRIX.toml --local [--slots N]
    python3 hack/sim-farm/farm.py status [MATRIX.toml]
    python3 hack/sim-farm/farm.py stop RUN|MATRIX [--no-wait]
    python3 hack/sim-farm/farm.py down [MATRIX.toml | --run RUN]
    python3 hack/sim-farm/farm.py report MATRIX.toml [--group KEY,...]
    python3 hack/sim-farm/farm.py spend [--csv DIR]

`estimate` builds and launches nothing, and `--local` runs the jobs
here without AWS. `status` lists the live runs and the instances in
every region, plus a matrix's jobs with rows when given one. `report`
of a test matrix sums its [report] columns from its index; of a testbed
matrix it names the farm runs that hold its runs' tables, whose results
are the lake's (sim-lab). `spend` reports estimates by local day; AWS
bills by UTC day.

## Before a run

1. Size it to its question (below).
2. Quote it with `farm.py estimate`, with `--regions` and without.
   Never price a run by hand: the estimate times each job by the walls
   earlier runs of its test measured (target/sim-farm/walls.jsonl) and
   prices them at the pools' spot prices, which no guess matches.
3. Give the user the quote, the question the run answers, and why a
   narrower run can't answer it. Nothing runs without their explicit
   approval, unless it fits farm spend they already granted for the
   task. Spend past a grant needs a new approval.

## Sizing a run to its question

- Run the change's scoped tests with its switch on first. Then take
  direction from the free local suite (`suite.py run --only CELL,...`).
  The farm settles only what local runs can't.
- Pick tiers and cells by mechanism; skip what the change can't reach.
- Screen at most two arms. Confirm only the chosen arm on fresh seeds,
  on the tiers where the call is close.
- Don't rerun baselines for a row-only change. Batch such changes.
- No local run blocks for more than 15 minutes: put it under
  `timeout 900`. A broader screen goes to the farm, split so each run's
  estimated wall is about 15 minutes.
- One `farm.py run` of several matrices shares one queue, boot and
  canary: run a tier, or several, as one run.

## Running and watching

- Start `farm.py run` or `suite.py farm` in the background, and watch
  target/sim-farm/MATRIX/farm.log and `farm.py status MATRIX.toml`.
- **The build race.** Builds from any checkout take
  target/sim-farm/build.lock in turn ("waiting for another farm build"),
  and an arm built meanwhile is reused, so two runs of one commit can
  start together. Never remove target/sim-farm/src or the lock while a
  farm process runs.
- **Slots.** An instance runs one job per core, or fewer where memory
  binds. Its memory, less 4 GiB, is divided by the largest
  `slot_mem_gib` among the run's matrices, so run big cells apart from
  small ones. `suite.py farm` sizes each cell's slot from its `mem_gib`
  (`suite.cable_gib`: every cable cell fits 6 GiB). Where kept local rows
  measured the cell, it caps the slot at 1.25 times their peak.
- **The memory guard.** The worker starts no job while MemAvailable is
  under one slot. If "the worker isn't running; restarting it" repeats
  in farm.log, jobs outgrow their slots and the run loops while billing.
  Stop it and raise `slot_mem_gib`.
- A job past `--job-timeout` (60 min) is killed. From suite.py, pass it
  with `suite.py farm --farm-args ARGS`.

## Stopping a run

A run is live while its farm.py holds the run's lock
(target/sim-farm/live/RUN.pid); a second `farm.py run` of it refuses to
start. Runs from other sessions may be live: stop only your own.

1. `farm.py stop RUN` (the run's name, or a matrix only it includes)
   sends its farm.py one SIGINT. Never signal suite.py or a `timeout`
   wrapper: suite.py kills farm.py 0.25 s after a ^C, before its
   cleanup. Never kill what pgrep matches: it may be another session's
   run.
2. stop refuses a matrix that several live runs include, listing them,
   since suite.py names runs by their inputs and two sessions' runs can
   share a matrix: name the run then. It refuses a PID that isn't the
   run's farm.py, which a reused PID would be.
3. stop waits while the run winds down, printing its log: it flushes (2
   minutes at most), then terminates its instances, leaving interrupted
   ones for AWS to reclaim. A run stopped in its builds exits at once;
   one whose setup has started a host finishes the setup, then cleans up.
   `--no-wait` returns at once.
4. stop then lists the instances still tagged with the run's name. Run
   the `python3 hack/sim-farm/farm.py down --run RUN` it prints, never a
   bare `down` or `down MATRIX`: those terminate other runs' instances
   too.

With `--keep`, a stopped run leaves its instances for a rerun to adopt.

## Regions, S3 and interruptions

- **Regions.** A run uses `--regions`, else the config's `aws_regions`.
  Pools from every region rank as one by price per unit of work. A
  region's budget is its spot quota less the vCPUs in use outside the
  run, shutting-down instances included, reread every 2 minutes.
  `status` and `down` sweep every region in target/sim-farm/regions.
- **S3.** Boots fetch binaries from the config's bucket by presigned
  URL, falling back to SSH. Instances flush their tables to sim-data/
  and sim-samples/ (`hack/sim-farm/s3sign.py`), and a job's index line
  waits for its flush to land. A run pulls nothing; `simdb.py pull` or
  suite.py copies the data into the cache. Data expires after a week,
  and a job's line is reused for 6 days (`RETAIN_S`). A rerun of the
  same matrices flushes under the same run name, so `simdb.py pull
  --run RUN --commit SHA` is the pull of one run (sim-lab).
- **Spot interruptions.** At a notice, the instance gives back its jobs
  that won't end within 20 s, flushes within 90 s, and is left for AWS
  to reclaim (unbilled in its first hour). Its pool is skipped for 10
  minutes. Each run's costs.jsonl line lists its `interruptions`.

## Permissions and approval

- The account's IAM policy is narrow: what farm.py's docstring lists,
  and little more. On AccessDenied, ask the user for the permission,
  with the JSON policy statement it needs. Never work around it: the
  prescribed fix, the permission, beats a hack.
- A new permission needs the user's approval, as runs and spend do.

## Editing farm.py, worker.py or a matrix

- Every session's `suite.py farm` loads the main checkout's farm.py
  from disk, and boots ship its worker.py, s3sign.py and store_host.py.
  A worktree's own `farm.py run` runs its own code on main's state.
- Develop in a worktree and test with `farm.py run --local` of
  `scenarios-smoke.toml`. Land in one commit once `farm.py status` says
  `no live runs` and `pgrep -a python | grep 'farm.py run'` finds no run
  it misses: a farm.py from a checkout without the lock holds none. Keep
  every save of the main checkout's copy runnable.
- `pgrep -f` waits and guards match their own `bash -c` line. Use
  `pgrep -a python | grep` (a venv's farm.py runs as `python`), give a
  guard an `else echo` branch, and wait on a PID (`while kill -0 PID`).
  Kill hung sims by PID, never with `pkill -f`.

## Reading results

That is sim-lab's. `suite.py farm` prints its run's scorecards when the
run ends. `suite.py score RUN:ARM RUN:ARM` scores arms across runs.

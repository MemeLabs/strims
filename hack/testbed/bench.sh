#!/usr/bin/env bash
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
#
# The TCP bench (strims-sim's bench module) on the calibration testbed:
# run.sh's shaped containers, each running testbed_bench in place of a
# node, writing on the schedule given, and then the same bench under the
# sim's WAN model, so the two can be compared (bench.py).
#
#   hack/testbed/bench.sh <out dir> <schedule file>
#
# run.sh's settings apply (VIEWERS, SEED_UP_MBIT, QUEUE_MS, SEED_UP_STEPS,
# TOPOLOGY, ...; SECS is the schedule's length), from the environment or
# the schedule's `# env:` lines (hack/testbed/bench/ holds the cases). Writes run.sh's files to
# <out dir> (node-<i>.jsonl is each node's bench log), the schedule as
# <out dir>/schedule, and the sim's logs to <out dir>/sim (send buffers)
# and <out dir>/sim-tcp (and their window model). BENCH_BIN is the
# testbed_bench binary (default target/release/examples/testbed_bench),
# built by `cargo build --release -p strims-sim --example testbed_bench`.

set -euo pipefail

out=${1:?usage: bench.sh <out dir> <schedule file>}
schedule=${2:?usage: bench.sh <out dir> <schedule file>}
root=$(cd "$(dirname "$0")/../.." && pwd)
bin=${BENCH_BIN:-$root/target/release/examples/testbed_bench}
[ -x "$bin" ] || { echo "no testbed_bench at $bin" >&2; exit 1; }
# A schedule's `# env: KEY=value ...` lines set run.sh's settings.
while read -r kv; do
  [ -n "$kv" ] && export "${kv?}"
done < <(sed -n 's/^# env: //p' "$schedule" | tr ' ' '\n')
mkdir -p "$out"
cp "$schedule" "$out/schedule"
NODE_BIN=$bin NODE_ARGS="--schedule /out/schedule" SECS=${SECS:-30} \
  "$root/hack/testbed/run.sh" "$out"
"$bin" --sim "$out" --out "$out/sim"
"$bin" --sim "$out" --out "$out/sim-tcp" --window

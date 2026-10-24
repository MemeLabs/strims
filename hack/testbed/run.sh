#!/usr/bin/env bash
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
#
# The calibration testbed: real strims nodes (strims-sim's testbed_node
# example) in Docker containers on one bridge, each shaping its own uplink,
# so the upload meter can be measured on a real shaped link and compared
# with strims-sim's model of one (its calibration scenarios).
#
#   hack/testbed/run.sh <out dir>
#
# Every container is its own network namespace with a veth on the bridge.
# Each shapes its egress with netem (a fixed one-way delay) over a tbf
# (the access link's rate and queue):
#
#   tc qdisc replace dev eth0 root handle 1: netem delay <d>ms limit 100000
#   tc qdisc add dev eth0 parent 1:1 handle 10: tbf rate <up>mbit \
#       burst 3028 limit <up x queue x 125 bytes>
#
# netem stays at the root with a delay of at least 1 ms: it releases the
# socket's memory for the packets it holds, so TCP Small Queues doesn't
# keep the tbf queue empty. Offloads are off, so tbf shapes real packets.
# One viewer can get a slow downlink (an ifb device with a tbf).
#
# Settings (environment):
#   VIEWERS=<n>            viewers (default 3)
#   SEED_UP_MBIT=<m>       the seed's uplink (default 10)
#   VIEWER_UP_MBIT=<m>     each viewer's uplink (default 20)
#   QUEUE_MS=<ms>          every uplink's queue (default 100)
#   SEED_DELAY_MS=<ms>     the seed's one-way delay (default 20)
#   VIEWER_DELAY_MS=<ms>   each viewer's (default 10)
#   SECS=<s>               the stream's length (default 180)
#   JOIN_AT=<s>            when viewers dial in, after the start (default 0)
#   SLOW_DOWN_KBIT=<k>     viewer 1's downlink, if set
#   TOPOLOGY=<file>        links (`link a b` lines); default a star
#   NODE_BIN=<path>        the testbed_node binary (default
#                          target/release/examples/testbed_node)
#   CAPTURE=1              ppspp captures in <out dir>/capture
#
# Writes to <out dir>: topology, node-<i>.jsonl (a JSON line per second
# from each node), node-<i>.err, tc-<i>.log (the seed's tbf backlog
# at 10 Hz) and ss-<i>.log (its sockets once a second), and settings.
#
# Needs Docker (the user's docker group) and the sch_netem, sch_tbf and
# (for a slow downlink) ifb modules; if tc can't load them, run
# `sudo modprobe sch_netem sch_tbf ifb` once.

set -euo pipefail

out=${1:?usage: run.sh <out dir>}
root=$(cd "$(dirname "$0")/../.." && pwd)
viewers=${VIEWERS:-3}
seed_up=${SEED_UP_MBIT:-10}
viewer_up=${VIEWER_UP_MBIT:-20}
queue=${QUEUE_MS:-100}
seed_delay=${SEED_DELAY_MS:-20}
viewer_delay=${VIEWER_DELAY_MS:-10}
secs=${SECS:-180}
join_at=${JOIN_AT:-0}
slow=${SLOW_DOWN_KBIT:-}
bin=${NODE_BIN:-$root/target/release/examples/testbed_node}
image=strims-testbed
net=strims-testbed
subnet=10.77.0

mkdir -p "$out"
out=$(cd "$out" && pwd)
[ -x "$bin" ] || { echo "no testbed_node at $bin" >&2; exit 1; }
docker build -q -t "$image" "$root/hack/testbed" >/dev/null
docker network inspect "$net" >/dev/null 2>&1 \
  || docker network create --subnet "$subnet.0/24" "$net" >/dev/null

n=$((viewers + 1))
{
  for i in $(seq 0 $((n - 1))); do
    echo "node $i $subnet.$((10 + i)):4000"
  done
  if [ -n "${TOPOLOGY:-}" ]; then
    command grep '^link ' "$TOPOLOGY"
  else
    for i in $(seq 1 $viewers); do echo "link 0 $i"; done
  fi
} > "$out/topology"
env | command grep -E '^(VIEWERS|SEED_UP_MBIT|VIEWER_UP_MBIT|QUEUE_MS|SEED_DELAY_MS|VIEWER_DELAY_MS|SECS|JOIN_AT|SLOW_DOWN_KBIT|TOPOLOGY|CAPTURE)=' \
  > "$out/settings" || true

start=$(( $(date +%s%3N) + 8000 ))
names=()
cleanup() { docker rm -f "${names[@]}" >/dev/null 2>&1 || true; }
trap cleanup EXIT

for i in $(seq 0 $((n - 1))); do
  if [ "$i" -eq 0 ]; then up=$seed_up; delay=$seed_delay; else up=$viewer_up; delay=$viewer_delay; fi
  limit=$((up * queue * 125))
  shape="ethtool -K eth0 tso off gso off gro off >/dev/null 2>&1 || true
tc qdisc replace dev eth0 root handle 1: netem delay ${delay}ms limit 100000
tc qdisc add dev eth0 parent 1:1 handle 10: tbf rate ${up}mbit burst 3028 limit $limit"
  if [ "$i" -eq 1 ] && [ -n "$slow" ]; then
    shape="$shape
ip link add ifb0 type ifb && ip link set ifb0 up
tc qdisc add dev eth0 handle ffff: ingress
tc filter add dev eth0 parent ffff: matchall action mirred egress redirect dev ifb0
tc qdisc add dev ifb0 root tbf rate ${slow}kbit burst 3028 limit $((slow * queue / 8))"
  fi
  sample=""
  if [ "$i" -eq 0 ]; then
    sample="( while :; do echo \"\$(date +%s%3N) \$(tc -s qdisc show dev eth0 | tr '\n' ' ')\"; sleep 0.1; done > /out/tc-$i.log ) &
( while :; do echo \"@ \$(date +%s%3N)\"; ss -tin state established; sleep 1; done > /out/ss-$i.log ) &"
  fi
  capture=""
  if [ "${CAPTURE:-0}" = 1 ]; then capture="--capture /out/capture"; fi
  name=tb-$i
  names+=("$name")
  docker rm -f "$name" >/dev/null 2>&1 || true
  docker run -d --name "$name" --network "$net" --ip "$subnet.$((10 + i))" \
    --cap-add NET_ADMIN -v "$bin:/testbed_node:ro" -v "$out:/out" "$image" \
    bash -c "set -e
$shape
tc -s qdisc show dev eth0 > /out/qdisc-$i.txt
$sample
exec /testbed_node --index $i --topology /out/topology --start $start --secs $secs --join-at $join_at $capture > /out/node-$i.jsonl 2> /out/node-$i.err" >/dev/null
done

wait_ms=$(( start + (secs + 15) * 1000 - $(date +%s%3N) ))
echo "testbed: $n nodes, stream of $secs s; results in $out"
for name in "${names[@]}"; do
  docker wait "$name" >/dev/null &
done
sleep $(( wait_ms / 1000 + 1 ))
for i in $(seq 0 $((n - 1))); do
  docker logs "tb-$i" > "$out/docker-$i.log" 2>&1 || true
done

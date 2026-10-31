#!/usr/bin/env bash
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
#
# The calibration testbed: real strims nodes in Docker containers on one
# bridge, each shaping its own uplink, so the upload meter can be measured
# on a real shaped link and compared with strims-sim's model of one (its
# calibration scenarios). A node is native (strims-sim's testbed_node
# example) or a headless browser running strims-wasm's node
# (strims-testbed's testbed_browser example, in the node's container).
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
# Browser nodes (KINDS) are shaped the same way. The harness in a browser
# container serves the page and drives it on the container's loopback, so
# the shaped eth0 is the browser's only other interface and carries every
# link. Browsers can't listen, so a run with a browser node also runs a
# relay (tb-relay, 10.77.0.5, unshaped): a native node in the testbed
# network that runs no swarm. Every node dials it and joins the network,
# and browsers make their WebRTC links by peer exchange through it.
# Natives link to natives over TCP; a browser links to a native over a
# WebSocket (BROWSER_LINKS=ws) or WebRTC (webrtc), and to a browser over
# WebRTC. The topology file's header (strims-testbed's topology module)
# says who makes which link.
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
#   JOINS="<i>:<s> ..."    node i dials in <s> seconds after the start, in
#                          place of JOIN_AT
#   SLOW_DOWN_KBIT=<k>     viewer 1's downlink, if set
#   SEED_UP_STEPS="<s>:<m> ..."  the seed's uplink becomes <m> Mbit/s <s>
#                          seconds after the stream starts (tc qdisc change;
#                          the queue keeps QUEUE_MS at the new rate)
#   COMPETE="<s>:<d> ..."  a bulk TCP upload (iperf3, CUBIC) from the seed's
#                          namespace, through its shaped uplink, to a sink
#                          container (10 ms egress delay), from <s> seconds
#                          after the stream starts for <d> seconds
#   TOPOLOGY=<file>        links (`link a b` lines); default a star
#   NODE_BIN=<path>        the testbed_node binary (default
#                          target/release/examples/testbed_node), built by
#                          `cargo build --release -p strims-sim --example
#                          testbed_node`
#   RATE=<bytes/s>         the stream's rate, at every node (default
#                          strims-testbed's RATE, 256 KiB/s)
#   MEDIA=<flv file>       the seed streams the file's H.264 and AAC in the
#                          unit format (strims-testbed's media module), in
#                          place of the self-checking frames at RATE; it
#                          must be native. Browser viewers play it through
#                          the baseline player, fed from their node as an
#                          app page's player is; a player that restarts
#                          takes the stream from the next whole record
#   UNITS_CPS=<n>          with MEDIA, the swarm's chunks per signature,
#                          the file's signing as ingest chooses it
#                          (default 8)
#   RECORD="<i> ..."       with MEDIA, these native viewers record the Data
#                          messages they read in record-<i>.bin
#                          (testbed_node's --record)
#   NODE_ARGS=<args>       more flags for every native node
#   CAPTURE=1              ppspp captures in <out dir>/capture, one dump
#                          per node (native and browser), named by host id
#   KINDS="<k0> <k1> ..."  node i's kind: native (the default for nodes not
#                          listed), firefox or chromium
#   BROWSER_LINKS=ws|webrtc  how browsers link to native nodes (default ws)
#   BROWSER_BIN=<path>     the testbed_browser binary (default
#                          target/release/examples/testbed_browser), built by
#                          `cargo build --release -p strims-testbed --example
#                          testbed_browser`
#   WEB_BUNDLE=<dir>       the browsers' page (default dist/wasm-testbed),
#                          built by `npx webpack --mode production --env
#                          TARGET=wasm-testbed` (node_modules, the pinned
#                          wasm-bindgen-cli and the wasm32 target)
#
# Writes to <out dir>: topology, node-<i>.jsonl (a JSON line per second
# from each node, strims-testbed's node module; a browser's adds post_ms
# and host_t_ms, testbed_browser's), node-<i>.err, cpu-<i>.log (the
# container's CPU time and memory once a second: ms since the epoch, the
# cgroup's usage_usec, and its memory.current in bytes), tc-<i>.log (the
# seed's tbf backlog at 10 Hz) and ss-<i>.log (its sockets once a second),
# steps.log (when each of SEED_UP_STEPS applied, ms since the epoch, and
# its rate), compete-<s>.json (each upload's iperf3 report) and
# compete.log (when each started), and settings. With a browser node also
# node-relay.err, cpu-relay.log and hosts/ (each node's host id). With
# MEDIA, each browser viewer's player-<i>.jsonl (its player's line a
# second, testbed_browser's --player), and RECORD's record-<i>.bin.
#
# Images: strims-testbed (Dockerfile) for native nodes, the relay and the
# sink, and strims-testbed-browser (browser.Dockerfile) for browser nodes,
# built here. Container names (tb-<i>, tb-relay, tb-sink), the network
# (strims-testbed) and its subnet (10.77.0.0/24) are fixed: one testbed
# per Docker host at a time.
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
steps=${SEED_UP_STEPS:-}
compete=${COMPETE:-}
node_args=${NODE_ARGS:-}
rate_arg=${RATE:+--rate $RATE}
bin=${NODE_BIN:-$root/target/release/examples/testbed_node}
browser_bin=${BROWSER_BIN:-$root/target/release/examples/testbed_browser}
web=${WEB_BUNDLE:-$root/dist/wasm-testbed}
read -r -a kinds <<< "${KINDS:-}"
read -r -a joins <<< "${JOINS:-}"
read -r -a records <<< "${RECORD:-}"
media=${MEDIA:-}
units_arg=""
if [ -n "$media" ]; then
  [ -f "$media" ] || { echo "no media file at $media" >&2; exit 1; }
  media=$(cd "$(dirname "$media")" && pwd)/$(basename "$media")
  units_arg="--units ${UNITS_CPS:-8}"
fi
browser_links=${BROWSER_LINKS:-ws}
image=strims-testbed
browser_image=strims-testbed-browser
net=strims-testbed
subnet=10.77.0
relay=$subnet.5

n=$((viewers + 1))
kind_of() { echo "${kinds[$1]:-native}"; }
join_of() {
  local j
  for j in "${joins[@]}"; do
    if [ "${j%%:*}" = "$1" ]; then echo "${j#*:}"; return; fi
  done
  echo "$join_at"
}
browsers=0
nbrowsers=0
if [ -n "$media" ] && [ "$(kind_of 0)" != native ]; then
  echo "MEDIA needs a native seed" >&2; exit 1
fi
for i in $(seq 0 $((n - 1))); do
  case $(kind_of "$i") in
    native) ;;
    firefox|chromium) browsers=1; nbrowsers=$((nbrowsers + 1)) ;;
    *) echo "node $i: unknown kind $(kind_of "$i")" >&2; exit 1 ;;
  esac
done
case $browser_links in ws|webrtc) ;; *) echo "BROWSER_LINKS: ws or webrtc" >&2; exit 1 ;; esac

mkdir -p "$out"
out=$(cd "$out" && pwd)
[ -x "$bin" ] || { echo "no testbed_node at $bin" >&2; exit 1; }
docker build -q -t "$image" "$root/hack/testbed" >/dev/null
if [ "$browsers" = 1 ]; then
  [ -x "$browser_bin" ] || { echo "no testbed_browser at $browser_bin" >&2; exit 1; }
  [ -f "$web/e2e.html" ] || { echo "no web bundle at $web" >&2; exit 1; }
  web=$(cd "$web" && pwd)
  docker build -q -t "$browser_image" -f "$root/hack/testbed/browser.Dockerfile" \
    "$root/hack/testbed" >/dev/null
  # Written by the nodes, whatever user each runs as.
  mkdir -p "$out/hosts"
  chmod 1777 "$out/hosts"
fi
if [ "${CAPTURE:-0}" = 1 ]; then
  mkdir -p "$out/capture"
  chmod 1777 "$out/capture"
fi
docker network inspect "$net" >/dev/null 2>&1 \
  || docker network create --subnet "$subnet.0/24" "$net" >/dev/null

{
  for i in $(seq 0 $((n - 1))); do
    if [ "$browsers" = 1 ]; then
      echo "node $i $subnet.$((10 + i)):4000 $(kind_of "$i")"
    else
      echo "node $i $subnet.$((10 + i)):4000"
    fi
  done
  if [ "$browsers" = 1 ]; then
    echo "relay $relay:4000 $relay:4001"
    echo "browser-links $browser_links"
  fi
  if [ -n "${TOPOLOGY:-}" ]; then
    command grep '^link ' "$TOPOLOGY"
  else
    for i in $(seq 1 $viewers); do echo "link 0 $i"; done
  fi
} > "$out/topology"
env | command grep -E '^(VIEWERS|SEED_UP_MBIT|VIEWER_UP_MBIT|QUEUE_MS|SEED_DELAY_MS|VIEWER_DELAY_MS|SECS|JOIN_AT|JOINS|SLOW_DOWN_KBIT|SEED_UP_STEPS|COMPETE|RATE|NODE_ARGS|TOPOLOGY|CAPTURE|KINDS|BROWSER_LINKS|MEDIA|UNITS_CPS|RECORD)=' \
  > "$out/settings" || true

# The containers start one after another before the stream does, and
# browsers then load the page and compile its wasm.
lead=$(( 8000 + 1500 * (n > 4 ? n - 4 : 0) ))
if [ "$browsers" = 1 ]; then
  lead=$(( lead + 20000 + 1000 * nbrowsers ))
fi
start=$(( $(date +%s%3N) + lead ))
names=()
cleanup() { docker rm -f "${names[@]}" >/dev/null 2>&1 || true; }
trap cleanup EXIT

# The container's CPU time and memory, once a second, in the background of
# its script.
cpu_log() {
  echo "( while :; do echo \"\$(date +%s%3N) \$(sed -n 's/^usage_usec //p' /sys/fs/cgroup/cpu.stat) \$(cat /sys/fs/cgroup/memory.current)\"; sleep 1; done > /out/cpu-$1.log ) &"
}
hosts=""
if [ "$browsers" = 1 ]; then
  hosts="--hosts /out/hosts"
  names+=(tb-relay)
  docker rm -f tb-relay >/dev/null 2>&1 || true
  docker run -d --name tb-relay --network "$net" --ip "$relay" \
    -v "$bin:/testbed_node:ro" -v "$out:/out" "$image" \
    bash -c "$(cpu_log relay)
exec /testbed_node --relay-only --topology /out/topology --start $start --secs $secs 2> /out/node-relay.err" >/dev/null
fi

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
  join=$(join_of "$i")
  media_mount=()
  media_arg=""
  if [ -n "$media" ] && [ "$i" -eq 0 ]; then
    media_mount=(-v "$media:/media.flv:ro")
    media_arg="--media /media.flv"
  fi
  record_arg=""
  for r in "${records[@]}"; do
    if [ "$r" = "$i" ] && [ -n "$media" ]; then record_arg="--record /out/record-$i.bin"; fi
  done
  player_arg=""
  if [ -n "$media" ] && [ "$(kind_of "$i")" != native ]; then
    # Written by the browser's unprivileged user.
    : > "$out/player-$i.jsonl"
    chmod 666 "$out/player-$i.jsonl"
    player_arg="--player /out/player-$i.jsonl"
  fi
  name=tb-$i
  names+=("$name")
  docker rm -f "$name" >/dev/null 2>&1 || true
  if [ "$(kind_of "$i")" = native ]; then
    docker run -d --name "$name" --network "$net" --ip "$subnet.$((10 + i))" \
      --cap-add NET_ADMIN -v "$bin:/testbed_node:ro" -v "$out:/out" "${media_mount[@]}" "$image" \
      bash -c "set -e
$shape
tc -s qdisc show dev eth0 > /out/qdisc-$i.txt
$sample
$(cpu_log "$i")
exec /testbed_node --index $i --topology /out/topology --start $start --secs $secs --join-at $join $rate_arg $units_arg $media_arg $record_arg $hosts $capture $node_args > /out/node-$i.jsonl 2> /out/node-$i.err" >/dev/null
  else
    # The browser runs as the image's unprivileged user, after root has
    # shaped the link. Its profile, in /tmp, stays in memory, so its
    # writes don't stall the other containers' on the host's disk.
    docker run -d --name "$name" --network "$net" --ip "$subnet.$((10 + i))" \
      --cap-add NET_ADMIN --shm-size 1g --tmpfs /tmp:size=1g \
      -v "$browser_bin:/testbed_browser:ro" \
      -v "$web:/web:ro" -v "$out:/out" "$browser_image" \
      bash -c "set -e
$shape
tc -s qdisc show dev eth0 > /out/qdisc-$i.txt
$sample
$(cpu_log "$i")
exec setpriv --reuid=ubuntu --regid=ubuntu --init-groups --reset-env /testbed_browser --index $i --topology /out/topology --web /web --start $start --secs $secs --join-at $join $rate_arg $units_arg $hosts $capture $player_arg > /out/node-$i.jsonl 2> /out/node-$i.err" >/dev/null
  fi
done

if [ -n "$steps" ]; then
  (
    for step in $steps; do
      at=${step%%:*}
      rate=${step#*:}
      ms=$(awk "BEGIN { printf \"%d\", $start + $at * 1000 }")
      wait=$(( ms - $(date +%s%3N) ))
      if [ "$wait" -gt 0 ]; then sleep "$(awk "BEGIN { print $wait / 1000 }")"; fi
      limit=$(awk "BEGIN { printf \"%d\", $rate * $queue * 125 }")
      docker exec tb-0 tc qdisc change dev eth0 parent 1:1 handle 10: \
        tbf rate "${rate}mbit" burst 3028 limit "$limit"
      echo "$(date +%s%3N) $rate" >> "$out/steps.log"
    done
  ) &
fi

if [ -n "$compete" ]; then
  sink=$subnet.250
  names+=(tb-sink)
  docker rm -f tb-sink >/dev/null 2>&1 || true
  docker run -d --name tb-sink --network "$net" --ip "$sink" --cap-add NET_ADMIN "$image" \
    bash -c "set -e
tc qdisc replace dev eth0 root netem delay 10ms limit 100000
exec iperf3 -s" >/dev/null
  (
    for flow in $compete; do
      at=${flow%%:*}
      dur=${flow#*:}
      ms=$(awk "BEGIN { printf \"%d\", $start + $at * 1000 }")
      wait=$(( ms - $(date +%s%3N) ))
      if [ "$wait" -gt 0 ]; then sleep "$(awk "BEGIN { print $wait / 1000 }")"; fi
      echo "$(date +%s%3N) $at $dur" >> "$out/compete.log"
      docker exec tb-0 iperf3 -c "$sink" -t "$dur" -i 1 -C cubic -J \
        > "$out/compete-$at.json" 2>&1 || true
    done
  ) &
fi

wait_ms=$(( start + (secs + 15) * 1000 - $(date +%s%3N) ))
echo "testbed: $n nodes, stream of $secs s; results in $out"
for name in "${names[@]}"; do
  docker wait "$name" >/dev/null &
done
sleep $(( wait_ms / 1000 + 1 ))
for i in $(seq 0 $((n - 1))); do
  docker logs "tb-$i" > "$out/docker-$i.log" 2>&1 || true
done
if [ "$browsers" = 1 ]; then
  docker logs tb-relay > "$out/docker-relay.log" 2>&1 || true
fi

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
# Each shapes its egress as a host behind an access modem: eth0's clsact
# hands every packet it sends to an ifb whose netem holds it for the
# one-way delay, and the ifb hands it back to eth0, whose root tbf is the
# modem (the access link's rate, its bucket and queue):
#
#   tc qdisc replace dev eth0 root handle 10: tbf rate <up>mbit \
#       burst <bucket> limit <up x queue x 125 bytes> [peakrate <p>mbit mtu 3028]
#   tc qdisc add dev ifbd root netem delay <d>ms limit 100000
#   tc filter add dev eth0 egress matchall action mirred egress redirect dev ifbd
#
# The bucket, what leaves at once (or at the peak rate) once the link has
# idled, is 3028 bytes (two full frames) unless SEED_BURST (the seed's) or
# VIEWER_BURST (the viewers') says otherwise.
#
# The order is a modem's. The egress filter's redirect hands the ifb a
# clone of each packet (act_mirred clones at egress), and eth0's clsact
# hook frees the socket's own (sch_handle_egress consumes the stolen skb),
# so its memory returns to the socket as at a host's NIC once it has
# handed a packet to a modem: TCP Small Queues counts nothing the delay
# or the modem holds and doesn't keep the modem's queue empty, at any
# delay (at `netem delay 0ms` a CUBIC upload still filled a 2 Mbit/s root
# tbf to 120 KB). tbf, the root, is offered
# each packet as netem releases it and dequeues before the next, so a
# burst its tokens cover leaves as it comes, and its byte limit is met as
# the packets arrive. (With netem at the root and the tbf its child,
# netem_dequeue moves every packet due into the tbf before the tbf sends
# any, so a burst the tokens covered still met the limit at once and lost
# its tail.) A fixed delay ahead of a FIFO shaper delays its departures as
# one after it would, so a rate step applies the delay after its time.
# Offloads are off on eth0 and on the ifb, which so cuts TCP's GSO
# buffers (TCP always builds them) into the frames a modem would take:
# tbf shapes real packets. A peak's bucket holds two frames: tbf wakes
# for each frame its peak paces some 25 us late on this host, which a
# one-frame bucket would lose (100 Mbit/s read 86), and two frames back to
# back at most is what a line at the peak sends in 0.24 ms at 100 Mbit/s.
# One viewer can get a slow downlink (an ifb device with a tbf, on eth0's
# ingress).
#
# Browser nodes (KINDS) are shaped the same way. The harness in a browser
# container serves the page and drives it on the container's loopback, so
# the shaped eth0 is the browser's only other interface and carries every
# link. Browsers can't listen, so a run with a browser node also runs a
# relay (tb-relay, 10.77.0.5, unshaped; RELAY=0 leaves it out): a native
# node in the testbed network that runs no swarm. Every node dials it and
# joins the network, and browsers make their WebRTC links by peer exchange
# through it.
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
#   SEED_BURST=<bytes>     the seed's tbf bucket (default 3028; at least a
#                          full frame, 1514)
#   VIEWER_BURST=<bytes>   each viewer's tbf bucket (default 3028; at least
#                          a full frame)
#   SEED_PEAK_MBIT=<m>     the rate the seed's bucket sends at, its line's
#                          own (tbf's peakrate; more than its uplink's
#                          rate, its steps' too); unset, at once
#   VIEWER_PEAK_MBIT=<m>   each viewer's
#   SEED_LINK=<spec>       the seed's access link class (default tbf: the
#                          tbf above): `<class>[,<key>=<value>...]`, tbf,
#                          cable, dsl, fiber or trace (hack/testbed/links.sh,
#                          which documents each class, its keys and
#                          defaults); a class's rate is SEED_UP_MBIT, its
#                          queue QUEUE_MS, a tbf's or fiber's bucket
#                          SEED_BURST and a tbf's peak SEED_PEAK_MBIT. A
#                          trace's time 0 is the stream's start (its
#                          packets reach the shaper SEED_DELAY_MS later),
#                          and its files are mounted from the host
#   VIEWER_LINK=<spec>     each viewer's (default tbf)
#   LINKS="<i>:<spec> ..." node i's link, in place of SEED_LINK or
#                          VIEWER_LINK
#   SEED_DELAY_MS=<ms>     the seed's one-way delay (default 20)
#   VIEWER_DELAY_MS=<ms>   each viewer's (default 10)
#   SECS=<s>               the stream's length (default 180)
#   JOIN_AT=<s>            when viewers dial in, after the start (default 0)
#   JOINS="<i>:<s> ..."    node i dials in <s> seconds after the start, in
#                          place of JOIN_AT
#   SLOW_DOWN_KBIT=<k>     viewer 1's downlink, if set
#   SEED_UP_STEPS="<s>:<m> ..."  the seed's uplink becomes <m> Mbit/s <s>
#                          seconds after the stream starts (tc qdisc change,
#                          SEED_DELAY_MS later; the queue keeps QUEUE_MS at
#                          the new rate). tbf's change fills its bucket
#                          (tbf_change sets its tokens to the buffer), which
#                          a link whose rate changes doesn't do; no qdisc
#                          here steps a byte bucket's rate keeping its
#                          tokens (HTB keeps them as time at the rate, and
#                          passes a packet on any), so a step hands the seed
#                          a full bucket, SEED_BURST bytes at once
#   COMPETE="<s>:<d> ..."  a bulk TCP upload (iperf3, CUBIC) from the seed's
#                          namespace, through its shaped uplink, to a sink
#                          container (10 ms egress delay), from <s> seconds
#                          after the stream starts for <d> seconds
#   SEED_LOSS_PCT=<p>      the seed's site loses each packet it sends or
#                          receives with this probability, in percent, as
#                          the sim's site loss does: one in round(100/<p>)
#                          (tc's gact) as each packet reaches its receiver.
#                          netem's loss on the seed's egress would drop the
#                          whole GSO buffer a packet leaves in
#   SS_EVERY=<s>           how often the seed's sockets are sampled into
#                          ss-0.log (default 1)
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
#   PCAP=1                 each node's packets on its eth0, both ways, in
#                          <out dir>/pcap-<i>.pcap (tcpdump, after the
#                          egress queue: what it drops isn't there), and
#                          what it offers its egress queue in
#                          pcap-<i>-offered.pcap (a clsact mirror to an
#                          ifb, ahead of the delay and the queue): the first
#                          PCAP_SNAPLEN bytes of each (default 128: the
#                          headers; a data channel's payload past its DTLS
#                          record header is encrypted)
#   KINDS="<k0> <k1> ..."  node i's kind: native (the default for nodes not
#                          listed), firefox or chromium
#   BROWSER_LINKS=ws|webrtc  how browsers link to native nodes (default ws)
#   BROWSER_BIN=<path>     the testbed_browser binary (default
#                          target/release/examples/testbed_browser), built by
#                          `cargo build --release -p strims-testbed --example
#                          testbed_browser`
#   BROWSER_ARGS=<args>    more flags for every browser node's harness
#   CPU_PROFILE=<at>:<s>   each Chromium node's page and workers sampled by
#                          V8's profiler from <at> seconds after the start
#                          for <s> (testbed_browser's --cpu-profile), into
#                          <out dir>/profile/cpu-<i>-<target>.cpuprofile;
#                          wasm frames have names with a bundle built with
#                          CARGO_PROFILE_RELEASE_WASM_STRIP=false
#   RELAY=0                no relay, for a harness that makes its nodes'
#                          links itself (the bare bench's, bench.sh)
#   WEB_BUNDLE=<dir>       the browsers' page (default dist/wasm-testbed),
#                          built by `npx webpack --mode production --env
#                          TARGET=wasm-testbed` (node_modules, the pinned
#                          wasm-bindgen-cli and the wasm32 target); the
#                          harness checks it holds its page
#
# Writes to <out dir>: topology, node-<i>.jsonl (a JSON line per second
# from each node, strims-testbed's node module; a browser's adds post_ms
# and host_t_ms, testbed_browser's), node-<i>.err, cpu-<i>.log (the
# container's CPU time and memory once a second: ms since the epoch, the
# cgroup's usage_usec, and its memory.current in bytes), tc-<i>.log (the
# seed's eth0 qdiscs at 10 Hz: ms since the epoch, then `tc -s -j qdisc
# show`'s JSON, which hack/testbed/tcstats.py reads) and ss-<i>.log (its
# sockets once a second),
# steps.log (when each of SEED_UP_STEPS applied, ms since the epoch, and
# its rate), compete-<s>.json (each upload's iperf3 report) and
# compete.log (when each started), links (each node's link spec, rate,
# delay and queue: `node <i> <spec> <up_mbit> <delay_ms> <queue_ms>`), a
# trace link's link-<i>.log (tracelink's queue, packets, bytes and drops
# every 100 ms) and link-<i>.err, and settings. With a browser node also
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
# ifb modules (and sch_pie, sch_cake, sch_fq_codel, sch_prio and sch_fq for
# the link classes that use them); if tc can't load them, run
# `sudo modprobe sch_netem sch_tbf ifb` once. A trace link's container also
# gets CAP_BPF and CAP_PERFMON, and run.sh builds tracelink
# (hack/testbed/tracelink.c, into target/testbed) with gcc.

set -euo pipefail

out=${1:?usage: run.sh <out dir>}
root=$(cd "$(dirname "$0")/../.." && pwd)
viewers=${VIEWERS:-3}
seed_up=${SEED_UP_MBIT:-10}
viewer_up=${VIEWER_UP_MBIT:-20}
queue=${QUEUE_MS:-100}
seed_burst=${SEED_BURST:-3028}
viewer_burst=${VIEWER_BURST:-3028}
seed_peak=${SEED_PEAK_MBIT:-}
seed_link=${SEED_LINK:-tbf}
viewer_link=${VIEWER_LINK:-tbf}
read -r -a link_specs <<< "${LINKS:-}"
viewer_peak=${VIEWER_PEAK_MBIT:-}
seed_delay=${SEED_DELAY_MS:-20}
viewer_delay=${VIEWER_DELAY_MS:-10}
secs=${SECS:-180}
join_at=${JOIN_AT:-0}
slow=${SLOW_DOWN_KBIT:-}
steps=${SEED_UP_STEPS:-}
compete=${COMPETE:-}
ss_every=${SS_EVERY:-1}
seed_loss=${SEED_LOSS_PCT:-}
loss_every=""
if [ -n "$seed_loss" ]; then
  loss_every=$(awk -v p="$seed_loss" 'BEGIN { v = 100 / p; if (v < 1 || v > 10000) exit 1; printf "%.0f", v }') \
    || { echo "SEED_LOSS_PCT=$seed_loss: out of range" >&2; exit 1; }
fi
node_args=${NODE_ARGS:-}
browser_args=${BROWSER_ARGS:-}
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
relay_on=${RELAY:-1}
image=strims-testbed
browser_image=strims-testbed-browser
net=strims-testbed
subnet=10.77.0
relay=$subnet.5

n=$((viewers + 1))
kind_of() { echo "${kinds[$1]:-native}"; }
# shellcheck source=hack/testbed/links.sh
source "$root/hack/testbed/links.sh"
link_of() {
  local l
  for l in "${link_specs[@]}"; do
    if [ "${l%%:*}" = "$1" ]; then echo "${l#*:}"; return; fi
  done
  if [ "$1" -eq 0 ]; then echo "$seed_link"; else echo "$viewer_link"; fi
}
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
traces=0
for i in $(seq 0 $((n - 1))); do
  link_check "$(link_of "$i")" || exit 1
  if [ "$(link_class "$(link_of "$i")")" = trace ]; then
    traces=1
    while read -r f; do
      [ -f "$f" ] || { echo "node $i: no link file at $f" >&2; exit 1; }
    done < <(link_files "$(link_of "$i")")
  fi
done
if [ -n "$steps" ] && [ "$(link_class "$(link_of 0)")" = trace ]; then
  echo "SEED_UP_STEPS: a trace link takes no steps" >&2; exit 1
fi
tracelink=$root/target/testbed/tracelink
if [ "$traces" = 1 ] && { [ ! -x "$tracelink" ] || [ "$root/hack/testbed/tracelink.c" -nt "$tracelink" ]; }; then
  mkdir -p "$(dirname "$tracelink")"
  gcc -O2 -static -o "$tracelink" "$root/hack/testbed/tracelink.c"
fi

mkdir -p "$out"
out=$(cd "$out" && pwd)
[ -x "$bin" ] || { echo "no testbed_node at $bin" >&2; exit 1; }
docker build -q -t "$image" "$root/hack/testbed" >/dev/null
if [ "$browsers" = 1 ]; then
  [ -x "$browser_bin" ] || { echo "no testbed_browser at $browser_bin" >&2; exit 1; }
  [ -d "$web" ] || { echo "no web bundle at $web" >&2; exit 1; }
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
  if [ "$browsers" = 1 ] && [ "$relay_on" = 1 ]; then
    echo "relay $relay:4000 $relay:4001"
  fi
  if [ "$browsers" = 1 ]; then
    echo "browser-links $browser_links"
  fi
  if [ -n "${TOPOLOGY:-}" ]; then
    command grep '^link ' "$TOPOLOGY"
  else
    for i in $(seq 1 $viewers); do echo "link 0 $i"; done
  fi
} > "$out/topology"
env | command grep -E '^(VIEWERS|SEED_UP_MBIT|VIEWER_UP_MBIT|QUEUE_MS|SEED_BURST|VIEWER_BURST|SEED_PEAK_MBIT|VIEWER_PEAK_MBIT|SEED_LINK|VIEWER_LINK|LINKS|SEED_DELAY_MS|VIEWER_DELAY_MS|SECS|JOIN_AT|JOINS|SLOW_DOWN_KBIT|SEED_UP_STEPS|COMPETE|SS_EVERY|SEED_LOSS_PCT|RATE|NODE_ARGS|TOPOLOGY|CAPTURE|PCAP|PCAP_SNAPLEN|KINDS|BROWSER_LINKS|BROWSER_ARGS|RELAY|MEDIA|UNITS_CPS|RECORD)=' \
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
fi
if [ "$browsers" = 1 ] && [ "$relay_on" = 1 ]; then
  names+=(tb-relay)
  docker rm -f tb-relay >/dev/null 2>&1 || true
  docker run -d --name tb-relay --network "$net" --ip "$relay" \
    -v "$bin:/testbed_node:ro" -v "$out:/out" "$image" \
    bash -c "$(cpu_log relay)
exec /testbed_node --relay-only --topology /out/topology --start $start --secs $secs 2> /out/node-relay.err" >/dev/null
fi

: > "$out/links"
for i in $(seq 0 $((n - 1))); do
  if [ "$i" -eq 0 ]; then up=$seed_up; delay=$seed_delay; burst=$seed_burst; peak=$seed_peak
  else up=$viewer_up; delay=$viewer_delay; burst=$viewer_burst; peak=$viewer_peak; fi
  limit=$((up * queue * 125))
  link=$(link_of "$i")
  echo "node $i $link $up $delay $queue" >> "$out/links"
  # A trace link's files, and the capabilities its classifier needs.
  link_mounts=()
  if [ "$(link_class "$link")" = trace ]; then
    link_mounts=(--cap-add BPF --cap-add PERFMON -v "$tracelink:/tracelink:ro")
    while read -r f; do
      link_mounts+=(-v "$(cd "$(dirname "$f")" && pwd)/$(basename "$f"):/links/$(basename "$f"):ro")
    done < <(link_files "$link")
  fi
  # The egress filter's actions: PCAP's mirror of what the node offers,
  # then the delay's redirect.
  offered=""
  if [ "${PCAP:-0}" = 1 ]; then offered="action mirred egress mirror dev ifbo "; fi
  shape="ethtool -K eth0 tso off gso off gro off >/dev/null 2>&1 || true
$(link_root "$link" "$up" "$burst" "$limit" "$peak" $((start + delay)) "$i")
ip link add ifbd type ifb && ip link set ifbd up
ethtool -K ifbd tso off gso off >/dev/null 2>&1 || true
tc qdisc add dev ifbd root netem delay ${delay}ms$(link_slot "$link") limit 100000
tc qdisc add dev eth0 clsact
tc filter add dev eth0 egress matchall ${offered}action mirred egress redirect dev ifbd"
  if [ -n "$loss_every" ]; then
    # The seed loses what it receives; a viewer, what the seed sends it.
    if [ "$i" -eq 0 ]; then match=matchall; else match="protocol ip u32 match ip src $subnet.10/32"; fi
    shape="$shape
tc filter add dev eth0 ingress prio 1 $match action gact continue random netrand drop $loss_every"
  fi
  if [ "$i" -eq 1 ] && [ -n "$slow" ]; then
    shape="$shape
ip link add ifbs type ifb && ip link set ifbs up
tc filter add dev eth0 ingress prio 2 matchall action mirred egress redirect dev ifbs
tc qdisc add dev ifbs root tbf rate ${slow}kbit burst 3028 limit $((slow * queue / 8))"
  fi
  sample=""
  if [ "$i" -eq 0 ]; then
    sample="( while :; do echo \"\$(date +%s%3N) \$(tc -s -j qdisc show dev eth0 | tr '\n' ' ')\"; sleep 0.1; done > /out/tc-$i.log ) &
( while :; do echo \"@ \$(date +%s%3N)\"; ss -tin state established; sleep $ss_every; done > /out/ss-$i.log ) &"
  fi
  if [ "${PCAP:-0}" = 1 ]; then
    shape="ip link add ifbo type ifb && ip link set ifbo up
$shape"
    sample="$sample
( tcpdump -i eth0 -n -U -s ${PCAP_SNAPLEN:-128} -w /out/pcap-$i.pcap 2> /out/tcpdump-$i.err ) &
( tcpdump -i ifbo -n -U -s ${PCAP_SNAPLEN:-128} -w /out/pcap-$i-offered.pcap 2> /out/tcpdump-$i-offered.err ) &"
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
  profile_arg=""
  if [ -n "${CPU_PROFILE:-}" ] && [ "$(kind_of "$i")" = chromium ]; then
    # Written by the browser's unprivileged user.
    mkdir -p "$out/profile"
    chmod 777 "$out/profile"
    profile_arg="--cpu-profile /out/profile --cpu-profile-at ${CPU_PROFILE%%:*} --cpu-profile-secs ${CPU_PROFILE#*:}"
  fi
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
      --cap-add NET_ADMIN "${link_mounts[@]}" -v "$bin:/testbed_node:ro" -v "$out:/out" "${media_mount[@]}" "$image" \
      bash -c "set -e
$shape
{ tc -s qdisc show dev eth0; tc -s qdisc show dev ifbd; } > /out/qdisc-$i.txt
$sample
$(cpu_log "$i")
exec /testbed_node --index $i --topology /out/topology --start $start --secs $secs --join-at $join $rate_arg $units_arg $media_arg $record_arg $hosts $capture $node_args > /out/node-$i.jsonl 2> /out/node-$i.err" >/dev/null
  else
    # The browser runs as the image's unprivileged user, after root has
    # shaped the link. Its profile, in /tmp, stays in memory, so its
    # writes don't stall the other containers' on the host's disk.
    docker run -d --name "$name" --network "$net" --ip "$subnet.$((10 + i))" \
      --cap-add NET_ADMIN "${link_mounts[@]}" --shm-size 1g --tmpfs /tmp:size=1g \
      -v "$browser_bin:/testbed_browser:ro" \
      -v "$web:/web:ro" -v "$out:/out" "$browser_image" \
      bash -c "set -e
$shape
{ tc -s qdisc show dev eth0; tc -s qdisc show dev ifbd; } > /out/qdisc-$i.txt
$sample
$(cpu_log "$i")
exec setpriv --reuid=ubuntu --regid=ubuntu --init-groups --reset-env /testbed_browser --index $i --topology /out/topology --web /web --start $start --secs $secs --join-at $join $rate_arg $units_arg $hosts $capture $player_arg $profile_arg $browser_args > /out/node-$i.jsonl 2> /out/node-$i.err" >/dev/null
  fi
done

if [ -n "$steps" ]; then
  (
    for step in $steps; do
      at=${step%%:*}
      rate=${step#*:}
      # The delay is ahead of the shaper: what the seed sends from the
      # step's time on reaches it that much later.
      ms=$(awk "BEGIN { printf \"%d\", $start + $at * 1000 + $seed_delay }")
      wait=$(( ms - $(date +%s%3N) ))
      if [ "$wait" -gt 0 ]; then sleep "$(awk "BEGIN { print $wait / 1000 }")"; fi
      limit=$(awk "BEGIN { printf \"%d\", $rate * $queue * 125 }")
      docker exec tb-0 bash -c "$(link_change "$(link_of 0)" "$rate" "$seed_burst" "$limit" "$seed_peak" "$seed_up")"
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
if [ "$browsers" = 1 ] && [ "$relay_on" = 1 ]; then
  docker logs tb-relay > "$out/docker-relay.log" 2>&1 || true
fi

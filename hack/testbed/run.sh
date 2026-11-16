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
# hands every packet it sends to an ifb (ifbd) whose netem holds it for
# the one-way delay, and the ifb hands it back to eth0, whose root is the
# modem, the node's access link class (hack/testbed/links.sh's node_tree).
# The default, tbf, is a tbf of the link's rate, its bucket and queue:
#
#   tc qdisc replace dev eth0 root handle 10: tbf rate <up>mbit \
#       burst <bucket> limit <up x queue x 125 bytes> \
#       [peakrate <p>mbit mtu <two of the class's largest frames>]
#   tc qdisc add dev ifbd root netem delay <d>ms limit 100000
#   tc filter add dev eth0 egress matchall action mirred egress redirect dev ifbd
#
# The other classes put a size table for their framing on that root, a
# queue under it (cable's DOCSIS-PIE, a DSL modem's FIFO), a home router's
# AQM in its place, or tracelink's classifier; a class with slots (cable's
# grants, slot=) moves the modem, handle 10:, to a second ifb (ifbm), to
# which eth0's root hands what the delay returns, and holds what comes
# back from it in a netem of slots alone: delay, shaper, slots.
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
# tbf shapes real packets. A peak's bucket holds two of the class's
# largest frames (3028 bytes under tbf, 3036 under cable's and fiber's
# 18-byte framing): tbf wakes for each frame its peak paces some 25 us
# late on this host, which a one-frame bucket would lose (100 Mbit/s read
# 86), and two frames back to back at most is what a line at the peak
# sends in 0.24 ms at 100 Mbit/s.
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
#   SEED_BURST=<bytes>     the seed's tbf bucket (default 3028; at least its
#                          link's largest frame: 1514 under tbf, 1518 under
#                          fiber's framing, which a shaper charges a full
#                          packet)
#   VIEWER_BURST=<bytes>   each viewer's tbf bucket (default 3028; at least
#                          its link's largest frame)
#   SEED_PEAK_MBIT=<m>     the rate the seed's bucket sends at, its line's
#                          own (tbf's peakrate, which must be over its
#                          uplink's rate and every step's: run.sh refuses
#                          one that isn't); unset, at once
#   VIEWER_PEAK_MBIT=<m>   each viewer's
#   SEED_LINK=<spec>       the seed's access link class (default tbf: the
#                          tbf above): `<class>[,<key>=<value>...]`, tbf,
#                          cable, dsl, fiber or trace (hack/testbed/links.sh,
#                          which documents each class, its keys and
#                          defaults); a class's rate is SEED_UP_MBIT, its
#                          queue QUEUE_MS (a FIFO's bytes, cable's
#                          DOCSIS-PIE's full frames; a router's fq_codel and
#                          cake keep their own limits), a tbf's or fiber's
#                          bucket SEED_BURST and peak SEED_PEAK_MBIT. A
#                          trace's time 0 is the stream's start (its
#                          packets reach the shaper SEED_DELAY_MS later),
#                          and its files are mounted from the host: a trace
#                          link runs on a local testbed alone (a farm
#                          testbed matrix refuses one)
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
#                          (tc's gact) as each packet reaches its receiver,
#                          the node's and COMPETE's upload's alike: a filter
#                          on the seed's ingress, and on each viewer's and
#                          the sink's for what the seed sends them. netem's
#                          loss on the seed's egress would drop the whole
#                          GSO buffer a packet leaves in
#   SS_EVERY=<s>           how often the seed's sockets are sampled into
#                          ss-0.log (default 1)
#   TOPOLOGY=<file>        links (`link a b` lines); default a star
#   NODE_BIN=<path>        the testbed_node binary (default
#                          target/release/examples/testbed_node), built by
#                          `cargo build --release -p strims-sim --example
#                          testbed_node && hack/testbed/stamp.py write
#                          target/release/examples/testbed_node`, which
#                          writes its stamp beside it
#   LINK_BIN=<path>        the link specs' parser (strims-wan's testbed_link
#                          example); unset, run.sh builds it from its own
#                          tree with cargo, into CARGO_TARGET_DIR (default
#                          target/), so NODE_BIN may be another commit's;
#                          the build waits for any other cargo build on that
#                          dir (cargo says so), which LINK_BIN avoids
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
#                          built by `pnpm exec webpack --mode production
#                          --env TARGET=wasm-testbed &&
#                          hack/testbed/stamp.py write dist/wasm-testbed`
#                          (node_modules, the pinned wasm-bindgen-cli and
#                          the wasm32 target); the harness checks it holds
#                          its page. A run with browsers needs it and
#                          NODE_BIN stamped from one checkout's sources
#                          (stamp.py): a page from other sources may not
#                          speak the node's protocol (links come up, no
#                          stream flows), and run.sh refuses it, or an
#                          artifact without a stamp or rebuilt since,
#                          saying how to rebuild
#
# Writes to <out dir>: topology, node-<i>.jsonl (a JSON line per second
# from each node, strims-testbed's node module; a browser's adds post_ms
# and host_t_ms, testbed_browser's), node-<i>.err, cpu-<i>.log (the
# container's CPU time and memory once a second: ms since the epoch, the
# cgroup's usage_usec, and its memory.current in bytes), tc-<i>.log (each
# node's qdiscs on every device at 10 Hz: ms since the epoch, then `tc -s
# -j qdisc show`'s JSON, each qdisc with its dev, which
# hack/testbed/tcstats.py reads, finding the modem's 10: on eth0 or, with
# slots, on ifbm: a checkout from before it, whose bench.py, twin.py and
# hack/capture/analyze.py read tc's text, finds no queue in it and says
# nothing), the seed's ss-0.log (its sockets once a second), qdisc-<i>.txt (each
# node's qdiscs on every device as it shaped them, before the stream: `tc
# -s -d qdisc show`, its modem with its size table, the delay's ifbd, the
# slots' ifbm, the slow downlink's ifbs and PCAP's ifbo), steps.log (when
# each of SEED_UP_STEPS applied, ms since the epoch, and its rate),
# compete-<s>.json (each upload's iperf3 report) and compete.log (when each
# started), links (each node's link spec, rate, delay and queue, and the
# device whose root is its modem, as the parser gives it: `node <i> <spec>
# <up_mbit> <delay_ms> <queue_ms> <modem_dev>`), a
# trace link's link-<i>.log (tracelink's queue, packets, bytes and drops
# every 100 ms) and link-<i>.err, and settings: the settings given,
# QDISC, the shaping's qdisc tree (SHAPER below), so runs of one tree
# pool apart from another's, and STAMPS, the clock its own logs' stamps
# and its waits are read from (`clock`: bash's $EPOCHREALTIME, as now_ms
# and every sampler read it; a run before run.sh named it, stamped by
# uutils' `date +%s%3N`, which reads up to 900 ms late in a second's first
# 100 ms, names none, and strims-sim's testbed module keys it apart); while
# it runs, .ctl (its control files, which go with it). A node's lines
# carry the records strims-sim's tables take (`rec`), which strims-sim's
# testbed_tables example writes as a run's tables. With a browser node also
# node-relay.err, cpu-relay.log and hosts/ (each node's host id). With
# MEDIA, each browser viewer's player-<i>.jsonl (its player's line a
# second, testbed_browser's --player), and RECORD's record-<i>.bin.
#
# The stream starts once every node's link is up (each container marks it
# when it has shaped its link, a trace's link once it has loaded its
# trace). The run fails, removing its containers, if a node's link fails
# to build (a tc error, a missing module, a tracelink that fails), a node
# exits with an error, or a step fails to apply. However it ends, it ends
# its steps and uploads, and those check the run is theirs before they
# act on a container.
#
# Images: strims-testbed (Dockerfile) for native nodes, the relay and the
# sink, and strims-testbed-browser (browser.Dockerfile) for browser nodes,
# built here. Container names (tb-<i>, tb-relay, tb-sink), the network
# (strims-testbed-sealed, internal: no route out, removed when the run
# ends) and its subnet (10.77.0.0/24) are fixed: one testbed per Docker
# host at a time; run.sh stops if a tb-* container or a testbed network
# with containers on it is there.
#
# The containers send nothing off the testbed, but the host still talks
# to each run's new bridge: its avahi's mDNS and its IPv6 multicast
# (mDNS and listener reports from its link-local address), 16-19 mDNS and
# about 42 IPv6 packets into each container a run, most before the stream.
# They come in only, so no node's uplink carries them, and the metrics
# count node-to-node traffic alone.
#
# Needs Docker (the user's docker group) and the kernel modules the links
# are shaped with, which links.sh's link_modules lists (every run's netem,
# tbf, ifb, clsact and redirect, SEED_LOSS_PCT's filter, and the link
# classes' qdiscs, classifiers and marks). If tc can't load them, run once
# (a farm host's boot loads them and builds each class's tree as
# linkcheck.sh renders it)
#
#   sudo modprobe -a $(. hack/testbed/links.sh; link_modules)
#
# A trace link's container also gets CAP_BPF and CAP_PERFMON, and run.sh
# builds tracelink (hack/testbed/tracelink.c, into target/testbed) with
# gcc.

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
net=strims-testbed-sealed
old_net=strims-testbed
subnet=10.77.0
relay=$subnet.5
# An address on the subnet no container or host holds (the nodes are
# .10 up, the relay .5, the sink .250, the bridge .1).
unused=$subnet.254
# The packets the seed sends, which each viewer and the sink lose
# SEED_LOSS_PCT of (a tc match).
from_seed="protocol ip u32 match ip src $subnet.10/32"

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
[ -x "$bin" ] || { echo "no testbed_node at $bin" >&2; exit 1; }
# The seed's steps, and the highest rate its link runs at, which a peak is
# over.
for step in $steps; do
  [[ $step =~ ^[0-9]+(\.[0-9]+)?:[0-9]+(\.[0-9]+)?$ ]] \
    || { echo "SEED_UP_STEPS: $step isn't <s>:<mbit>" >&2; exit 1; }
done
seed_top=$(awk -v u="$seed_up" -v s="$steps" 'BEGIN {
  t = u; n = split(s, a, " ")
  for (i = 1; i <= n; i++) { split(a[i], p, ":"); if (p[2] + 0 > t) t = p[2] + 0 }
  print t }')
# Every node's link spec, parsed before any container starts by this
# tree's parser (strims-wan's testbed module, the twins' too), whatever
# commit NODE_BIN is, and the seed's at each of its steps.
link_bin=${LINK_BIN:-}
if [ -z "$link_bin" ]; then
  echo "run.sh: building testbed_link from this tree (LINK_BIN skips it)" >&2
  cargo build --release --locked --manifest-path "$root/Cargo.toml" \
    -p strims-wan --example testbed_link \
    || { echo "building testbed_link failed (or give LINK_BIN)" >&2; exit 1; }
  link_bin=${CARGO_TARGET_DIR:-$root/target}/release/examples/testbed_link
fi
[ -x "$link_bin" ] || { echo "no testbed_link at $link_bin" >&2; exit 1; }
traces=0
step_cmds=()
for i in $(seq 0 $((n - 1))); do
  if [ "$i" -eq 0 ]; then up=$seed_up; burst=$seed_burst; peak=$seed_peak; top=$seed_top
  else up=$viewer_up; burst=$viewer_burst; peak=$viewer_peak; top=""; fi
  link_eval "$(link_of "$i")" "$up" "$burst" $((up * queue * 125)) "$peak" "" "$top" \
    || { echo "node $i: link $(link_of "$i") refused" >&2; exit 1; }
  if [ "$LINK_CLASS" = trace ]; then
    traces=1
    for f in $LINK_FILES; do
      [ -f "${f%%:*}" ] || { echo "node $i: no link file at ${f%%:*}" >&2; exit 1; }
    done
    if [ "$i" -eq 0 ] && [ -n "$steps" ]; then
      echo "SEED_UP_STEPS: a trace link takes no steps" >&2; exit 1
    fi
  fi
  if [ "$i" -eq 0 ]; then
    for step in $steps; do
      rate=${step#*:}
      link_eval "$(link_of 0)" "$rate" "$seed_burst" \
        "$(awk "BEGIN { printf \"%d\", $rate * $queue * 125 }")" "$seed_peak" "$seed_up" "$seed_top" \
        || { echo "SEED_UP_STEPS: link $(link_of 0) refused at step $step" >&2; exit 1; }
      step_cmds+=("$(link_change)")
    done
  fi
done
tracelink=$root/target/testbed/tracelink
if [ "$traces" = 1 ] && { [ ! -x "$tracelink" ] || [ "$root/hack/testbed/tracelink.c" -nt "$tracelink" ]; }; then
  mkdir -p "$(dirname "$tracelink")"
  gcc -O2 -static -o "$tracelink" "$root/hack/testbed/tracelink.c"
fi

mkdir -p "$out"
out=$(cd "$out" && pwd)
docker build -q -t "$image" "$root/hack/testbed" >/dev/null
if [ "$browsers" = 1 ]; then
  [ -x "$browser_bin" ] || { echo "no testbed_browser at $browser_bin" >&2; exit 1; }
  [ -d "$web" ] || { echo "no web bundle at $web" >&2; exit 1; }
  web=$(cd "$web" && pwd)
  python3 "$root/hack/testbed/stamp.py" check "$bin" "$web" || exit 1
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
# The network has no route out (--internal), so the shaped uplinks carry
# the testbed's traffic alone: a browser's own fetches (component updates,
# safe browsing, first-run downloads) would cross them otherwise. Nothing
# in a run needs the internet: the binaries and the page are mounted, the
# images built beforehand. It has a name of its own, which no older run.sh
# reuses (they make strims-testbed, routable, whose browsers need its
# route), and goes when the run ends; the two share the subnet, so the
# old one goes first. One of the new name is kept only as this run.sh
# makes it.
#
# One testbed per Docker host: a container named tb-* is another run's,
# live or left by one killed before its cleanup, and this run's would
# take its name (each start removes a container of its name first), and
# a testbed network with containers on it is another run's. Either stops
# this run before it touches anything.
stale=$(docker ps -a --format '{{.Names}}' | grep '^tb-' | sort | tr '\n' ' ' || true)
if [ -n "$stale" ]; then
  echo "containers ${stale% } exist: another testbed run is live, or one killed before its cleanup" \
    "left them; if none is live, remove them: docker rm -f ${stale% }" >&2
  exit 1
fi
for nw in "$old_net" "$net"; do
  on=$(docker network inspect -f '{{len .Containers}}' "$nw" 2>/dev/null) || on=0
  if [ "$on" != 0 ]; then
    echo "network $nw has $on containers on it: another testbed run is live" >&2
    exit 1
  fi
done
names=()
# The run's id, in its control files (/ctl/run), which a step or an upload
# checks in tb-0 before it acts: one left from another run then acts on
# nothing.
run_id=$(od -An -N8 -tx8 /dev/urandom | tr -d ' ')
# The run's control files, in its out dir under its id (.ctl/<id>, which
# goes with the run; what a run killed before its cleanup left goes with
# the next run into the out dir, or with the out dir, and its own work
# left running finds its dir gone): each node's mark that its link is up
# (/ctl/ready-<i>), the stream's start (/ctl/start, which the containers
# wait for), each container's exit code, and what failed while the stream
# ran.
rm -rf "$out/.ctl"
ctl=$out/.ctl/$run_id
mkdir -p -m 755 "$ctl"
echo "$run_id" > "$ctl/run"
ours="[ \"\$(cat /ctl/run 2>/dev/null)\" = $run_id ] || exit 3"
# The run's background work (its steps, uploads and container waits),
# which ends with it, however it ends.
bg=()
kill_tree() {
  local kids c
  kids=$(pgrep -P "$1" 2>/dev/null || true)
  kill "$1" 2>/dev/null || true
  for c in $kids; do kill_tree "$c"; done
}
# The run's background work, its containers, its control files and its
# network, in that order.
cleanup() {
  local p
  for p in "${bg[@]}"; do kill_tree "$p"; done
  docker rm -f "${names[@]}" >/dev/null 2>&1 || true
  rm -rf "$ctl"
  rmdir "$out/.ctl" 2>/dev/null || true
  docker network rm "$net" >/dev/null 2>&1 || true
}
trap cleanup EXIT
for nw in "$old_net" "$net"; do
  have=$(docker network inspect -f '{{.Internal}} {{range .IPAM.Config}}{{.Subnet}}{{end}}' "$nw" 2>/dev/null) \
    || continue
  [ "$nw" = "$net" ] && [ "$have" = "true $subnet.0/24" ] && continue
  docker network rm "$nw" >/dev/null \
    || { echo "network $nw ($have) is in use: another testbed run?" >&2; exit 1; }
done
docker network inspect "$net" >/dev/null 2>&1 \
  || docker network create --internal --subnet "$subnet.0/24" "$net" >/dev/null
# Nor does a container speak IPv6, whose link-local chatter (router
# solicitations, neighbour discovery) would cross them too.
nov6=(--sysctl net.ipv6.conf.all.disable_ipv6=1 --sysctl net.ipv6.conf.default.disable_ipv6=1)

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
# The qdisc tree each node's shape (below) builds, by name: a change to
# the tree changes it, so runs of two trees don't pool. Its root is the
# node's access link class's (the `links` file names each node's).
SHAPER="clsact>ifb-netem>root-class"
env | command grep -E '^(VIEWERS|SEED_UP_MBIT|VIEWER_UP_MBIT|QUEUE_MS|SEED_BURST|VIEWER_BURST|SEED_PEAK_MBIT|VIEWER_PEAK_MBIT|SEED_LINK|VIEWER_LINK|LINKS|SEED_DELAY_MS|VIEWER_DELAY_MS|SECS|JOIN_AT|JOINS|SLOW_DOWN_KBIT|SEED_UP_STEPS|COMPETE|SS_EVERY|SEED_LOSS_PCT|RATE|NODE_ARGS|TOPOLOGY|CAPTURE|PCAP|PCAP_SNAPLEN|KINDS|BROWSER_LINKS|BROWSER_ARGS|RELAY|MEDIA|UNITS_CPS|RECORD)=' \
  > "$out/settings" || true
echo "QDISC=$SHAPER" >> "$out/settings"
echo "STAMPS=clock" >> "$out/settings"

# Milliseconds since the epoch, from bash's clock ($EPOCHREALTIME: seconds
# and six digits of microseconds, no fork); the containers' samplers stamp
# their lines the same way (`t=${EPOCHREALTIME/[.,]/}; t=${t%???}`). The
# images' and hosts' date (uutils coreutils 0.10) prints %3N from the
# nanoseconds without their leading zeros, so in a second's first 100 ms
# `date +%s%3N` read up to 900 ms late.
now_ms() {
  local t=${EPOCHREALTIME/[.,]/}
  echo "${t%???}"
}
# What a container waits for before its node starts: the stream's start,
# which run.sh writes once every node's link is up.
wait_start='until [ -s /ctl/start ]; do sleep 0.05; done
start=$(cat /ctl/start)'

# The container's CPU time and memory, once a second, in the background of
# its script.
cpu_log() {
  echo "( while :; do t=\${EPOCHREALTIME/[.,]/}; t=\${t%???}; echo \"\$t \$(sed -n 's/^usage_usec //p' /sys/fs/cgroup/cpu.stat) \$(cat /sys/fs/cgroup/memory.current)\"; sleep 1; done > /out/cpu-$1.log ) &"
}
hosts=""
if [ "$browsers" = 1 ]; then
  hosts="--hosts /out/hosts"
fi
if [ "$browsers" = 1 ] && [ "$relay_on" = 1 ]; then
  names+=(tb-relay)
  docker rm -f tb-relay >/dev/null 2>&1 || true
  docker run -d --name tb-relay --network "$net" --ip "$relay" "${nov6[@]}" \
    -v "$bin:/testbed_node:ro" -v "$out:/out" -v "$ctl:/ctl" "$image" \
    bash -c "$(cpu_log relay)
$wait_start
exec /testbed_node --relay-only --topology /out/topology --start \$start --secs $secs 2> /out/node-relay.err" >/dev/null
fi

: > "$out/links"
for i in $(seq 0 $((n - 1))); do
  if [ "$i" -eq 0 ]; then up=$seed_up; delay=$seed_delay; burst=$seed_burst; peak=$seed_peak
  else up=$viewer_up; delay=$viewer_delay; burst=$viewer_burst; peak=$viewer_peak; fi
  limit=$((up * queue * 125))
  link=$(link_of "$i")
  top=""
  if [ "$i" -eq 0 ]; then top=$seed_top; fi
  link_eval "$link" "$up" "$burst" "$limit" "$peak" "" "$top"
  echo "node $i $link $up $delay $queue $LINK_DEV" >> "$out/links"
  # A previous run's link log would pass for this one's.
  rm -f "$out/link-$i.log" "$out/link-$i.err"
  # A trace link's files, and the capabilities its classifier needs.
  link_mounts=()
  if [ "$LINK_CLASS" = trace ]; then
    link_mounts=(--cap-add BPF --cap-add PERFMON -v "$tracelink:/tracelink:ro")
    for f in $LINK_FILES; do
      h=${f%%:*}
      link_mounts+=(-v "$(cd "$(dirname "$h")" && pwd)/$(basename "$h"):${f#*:}:ro")
    done
  fi
  # The node's link (links.sh's node_tree, which linkcheck.sh checks on
  # a farm host): the seed loses what it receives, a viewer what the seed
  # sends it (as the sink does), and the first viewer's downlink is
  # SLOW_DOWN_KBIT's.
  if [ "$i" -eq 0 ]; then match=matchall; else match=$from_seed; fi
  node_slow=""
  if [ "$i" -eq 1 ]; then node_slow=$slow; fi
  shape=$(node_tree "$delay" "$i" "${PCAP:-0}" "$match" "$loss_every" "$node_slow" "$queue")
  sample="( while :; do t=\${EPOCHREALTIME/[.,]/}; t=\${t%???}; echo \"\$t \$(tc -s -j qdisc show | tr '\n' ' ')\"; sleep 0.1; done > /out/tc-$i.log ) &"
  if [ "$i" -eq 0 ]; then
    sample="$sample
( while :; do t=\${EPOCHREALTIME/[.,]/}; t=\${t%???}; echo \"@ \$t\"; ss -tin state established; sleep $ss_every; done > /out/ss-$i.log ) &"
  fi
  if [ "${PCAP:-0}" = 1 ]; then
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
    docker run -d --name "$name" --network "$net" --ip "$subnet.$((10 + i))" "${nov6[@]}" \
      --cap-add NET_ADMIN "${link_mounts[@]}" -v "$bin:/testbed_node:ro" -v "$out:/out" -v "$ctl:/ctl" "${media_mount[@]}" "$image" \
      bash -c "set -e
$shape
tc -s -d qdisc show > /out/qdisc-$i.txt
: > /ctl/ready-$i
$sample
$(cpu_log "$i")
$wait_start
$(link_attached "$i")
exec /testbed_node --index $i --topology /out/topology --start \$start --secs $secs --join-at $join $rate_arg $units_arg $media_arg $record_arg $hosts $capture $node_args > /out/node-$i.jsonl 2> /out/node-$i.err" >/dev/null
  else
    # The browser runs as the image's unprivileged user, after root has
    # shaped the link. Its profile, in /tmp, stays in memory, so its
    # writes don't stall the other containers' on the host's disk. The
    # internal network has no default route, and Firefox gathers its ICE
    # candidates from the address a route lookup gives (none, and its
    # links fail at once): a default via an address on the subnet nothing
    # holds ($unused) gives it eth0's, and reaches nothing. (One on-link,
    # `default dev eth0`, would reach the host's own addresses off the
    # subnet, which it answers ARP for.)
    docker run -d --name "$name" --network "$net" --ip "$subnet.$((10 + i))" "${nov6[@]}" \
      --cap-add NET_ADMIN "${link_mounts[@]}" --shm-size 1g --tmpfs /tmp:size=1g \
      -v "$browser_bin:/testbed_browser:ro" \
      -v "$web:/web:ro" -v "$out:/out" -v "$ctl:/ctl" "$browser_image" \
      bash -c "set -e
$shape
ip route add default via $unused dev eth0
tc -s -d qdisc show > /out/qdisc-$i.txt
: > /ctl/ready-$i
$sample
$(cpu_log "$i")
$wait_start
$(link_attached "$i")
exec setpriv --reuid=ubuntu --regid=ubuntu --init-groups --reset-env /testbed_browser --index $i --topology /out/topology --web /web --start \$start --secs $secs --join-at $join $rate_arg $units_arg $hosts $capture $player_arg $profile_arg $browser_args > /out/node-$i.jsonl 2> /out/node-$i.err" >/dev/null
  fi
done

# A container whose link failed to build (a tc error, a missing module,
# a tracelink that failed) ends before its node starts: the run fails.
for i in $(seq 0 $((n - 1))); do
  until [ -e "$ctl/ready-$i" ]; do
    if [ "$(docker inspect -f '{{.State.Running}}' "tb-$i" 2>/dev/null)" != true ]; then
      # It may have ended since, its node having started.
      [ -e "$ctl/ready-$i" ] && break
      echo "node $i: its link failed:" >&2
      cat "$out/link-$i.err" >&2 2>/dev/null || true
      docker logs "tb-$i" >&2 2>&1 || true
      exit 1
    fi
    sleep 0.2
  done
done
# The stream starts once every link is up (a trace's having loaded its
# trace, however long), its classifier going in at the start: the nodes
# start then, and browsers load the page and compile its wasm.
lead=$(( 3000 + 100 * (n > 4 ? n - 4 : 0) ))
if [ "$browsers" = 1 ]; then
  lead=$(( lead + 20000 + 1000 * nbrowsers ))
fi
start=$(( $(now_ms) + lead ))
echo "$start" > "$ctl/start.tmp"
mv "$ctl/start.tmp" "$ctl/start"

if [ -n "$steps" ]; then
  (
    k=0
    for step in $steps; do
      at=${step%%:*}
      rate=${step#*:}
      # The delay is ahead of the shaper: what the seed sends from the
      # step's time on reaches it that much later.
      ms=$(awk "BEGIN { printf \"%d\", $start + $at * 1000 + $seed_delay }")
      wait=$(( ms - $(now_ms) ))
      if [ "$wait" -gt 0 ]; then sleep "$(awk "BEGIN { print $wait / 1000 }")"; fi
      [ -e "$ctl/run" ] || exit 0
      err=$(docker exec tb-0 bash -c "$ours; ${step_cmds[k]}" 2>&1) && rc=0 || rc=$?
      [ "$rc" = 3 ] && exit 0
      if [ "$rc" != 0 ]; then
        echo "SEED_UP_STEPS: the step $step failed: $err" > "$ctl/failed"
        exit 1
      fi
      echo "$(now_ms) $rate" >> "$out/steps.log"
      k=$((k + 1))
    done
  ) &
  bg+=($!)
fi

if [ -n "$compete" ]; then
  sink=$subnet.250
  # The sink's 10 ms, and, as a viewer does, its loss of what the seed
  # sends it (the upload's data; the seed's own filter takes its acks),
  # each frame alone (no GRO).
  sink_shape="tc qdisc replace dev eth0 root netem delay 10ms limit 100000"
  if [ -n "$loss_every" ]; then
    sink_shape="$sink_shape
ethtool -K eth0 gro off >/dev/null 2>&1 || true
tc qdisc add dev eth0 clsact
$(loss_filter "$from_seed" "$loss_every")"
  fi
  names+=(tb-sink)
  docker rm -f tb-sink >/dev/null 2>&1 || true
  docker run -d --name tb-sink --network "$net" --ip "$sink" "${nov6[@]}" --cap-add NET_ADMIN "$image" \
    bash -c "set -e
$sink_shape
exec iperf3 -s" >/dev/null
  (
    for flow in $compete; do
      at=${flow%%:*}
      dur=${flow#*:}
      ms=$(awk "BEGIN { printf \"%d\", $start + $at * 1000 }")
      wait=$(( ms - $(now_ms) ))
      if [ "$wait" -gt 0 ]; then sleep "$(awk "BEGIN { print $wait / 1000 }")"; fi
      [ -e "$ctl/run" ] || exit 0
      echo "$(now_ms) $at $dur" >> "$out/compete.log"
      docker exec tb-0 bash -c "$ours; exec iperf3 -c $sink -t $dur -i 1 -C cubic -J" \
        > "$out/compete-$at.json" 2>&1 || true
    done
  ) &
  bg+=($!)
fi

end_ms=$(( start + (secs + 15) * 1000 ))
echo "testbed: $n nodes, stream of $secs s; results in $out"
for name in "${names[@]}"; do
  ( docker wait "$name" > "$ctl/exit-$name" 2>/dev/null || true ) &
  bg+=($!)
done
# Until the stream has ended and the nodes have had 15 s to write their
# last lines, a step fails, or a node exits with an error.
failing() {
  local f
  [ -s "$ctl/failed" ] && return 0
  for f in "$ctl"/exit-*; do
    if [ -s "$f" ] && [ "$(cat "$f")" != 0 ]; then return 0; fi
  done
  return 1
}
while [ "$(now_ms)" -lt "$end_ms" ] && ! failing; do
  sleep 1
done
for i in $(seq 0 $((n - 1))); do
  docker logs "tb-$i" > "$out/docker-$i.log" 2>&1 || true
done
if [ "$browsers" = 1 ] && [ "$relay_on" = 1 ]; then
  docker logs tb-relay > "$out/docker-relay.log" 2>&1 || true
fi
if [ -s "$ctl/failed" ]; then
  cat "$ctl/failed" >&2
  exit 1
fi
# A node that exited with an error (one that never started, or failed
# while the stream ran) fails the run.
failed=0
for name in "${names[@]}"; do
  code=$(cat "$ctl/exit-$name" 2>/dev/null || true)
  if [ -n "$code" ] && [ "$code" != 0 ]; then
    echo "$name exited with $code (its docker-<i>.log and node-<i>.err)" >&2
    failed=1
  fi
done
[ "$failed" = 0 ] || exit 1

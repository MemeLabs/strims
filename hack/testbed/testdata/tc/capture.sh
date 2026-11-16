#!/usr/bin/env bash
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
#
# tcstats_test.py's captures: each link class's tree as run.sh builds a
# node's (links.sh's node_tree on eth0, a dummy here, from what
# testbed_link parses the class's spec to), at 100 kbit/s under a UDP flood
# so a queue stands and drops, then the modem's device's tc text and JSON
# (eth0's, or ifbm's for a class with slots), every device's JSON (what
# run.sh's tc-0.log samples) and every device's detailed text
# (qdisc-<i>.txt's), read a moment apart (a packet leaves every ~100 ms),
# into OUT. From the repo's root, with tracelink and testbed_link built
# (run.sh builds both):
#
#   docker run --rm --network none --cap-add NET_ADMIN --cap-add BPF \
#     --cap-add PERFMON -v $PWD/hack/testbed/links.sh:/links.sh:ro \
#     -v $PWD/target/testbed/tracelink:/tracelink:ro \
#     -v $PWD/target/release/examples/testbed_link:/testbed_link:ro \
#     -v $PWD/hack/testbed/testdata/tc/capture.sh:/c.sh:ro \
#     -v $PWD/hack/testbed/testdata/tc:/out strims-testbed bash /c.sh
set -u
source /links.sh
link_bin=/testbed_link
payload=$(head -c 1200 /dev/zero | tr '\0' a)
flood() { local end=$(( SECONDS + 2 )); while [ $SECONDS -lt $end ]; do printf %s "$payload" > /dev/udp/10.99.0.2/9 2>/dev/null; done; }
mkdir -p /tmp/links /out /ctl
{ echo "# period_us 1000000"; for t in $(seq 100000 100000 1000000); do echo "$t 1514"; done; } > /tmp/links/trace
for spec in tbf cable cable,aqm=fifo dsl dsl,line=vdsl fiber cable,router=fq_codel dsl,router=cake trace; do
  for d in eth0 ifbd ifbm; do ip link del "$d" 2>/dev/null; done
  ip link add eth0 type dummy && ip link set eth0 up && ip addr add 10.99.0.1/24 dev eth0
  name=$(echo "$spec" | tr ',=' '__')
  if [ "$spec" = trace ]; then
    link_eval "trace,file=slow.us,format=us" 0.1 3028 30000
    # The stream's start is now: the trace's time 0.
    t=${EPOCHREALTIME/[.,]/}
    echo "${t%???}" > /ctl/start
  else
    link_eval "$spec" 0.1 3028 30000
  fi
  tracelink=""
  eval "$(node_tree 1 0 0 "" "" "" "" | sed -e 's|/links/|/tmp/links/|g' -e 's|/out/link-0|/tmp/link-0|g')"
  eval "$(link_attached 0 | sed -e 's|/out/link-0|/tmp/link-0|g')"
  flood
  dev=$(link_dev)
  for _ in 1 2 3 4 5; do
    a=$(tc -s qdisc show dev "$dev"); j=$(tc -s -j qdisc show dev "$dev"); all=$(tc -s -j qdisc show)
    b=$(tc -s qdisc show dev "$dev")
    [ "$a" = "$b" ] && break
  done
  printf '%s\n' "$a" > "/out/$name.txt"
  printf '%s\n' "$j" > "/out/$name.json"
  printf '%s\n' "$all" > "/out/$name.every.json"
  tc -s -d qdisc show > "/out/$name.all"
  pkill tracelink 2>/dev/null
  rm -f /ctl/start
  tc qdisc del dev eth0 root 2>/dev/null
done

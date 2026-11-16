#!/usr/bin/env bash
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
#
# tcstats_test.py's captures: each link class's tree as run.sh builds a
# node's (links.sh's link_root on eth0, a dummy here; the delay's ifb
# behind eth0's clsact redirect), at 100 kbit/s under a UDP flood so a
# queue stands and drops, then tc's text, its JSON and every device's
# detailed text, read a moment apart (a packet leaves every ~100 ms), into
# OUT. From the repo's root, with tracelink built (run.sh builds it):
#
#   docker run --rm --network none --cap-add NET_ADMIN --cap-add BPF \
#     --cap-add PERFMON -v $PWD/hack/testbed/links.sh:/links.sh:ro \
#     -v $PWD/target/testbed/tracelink:/tracelink:ro \
#     -v $PWD/hack/testbed/testdata/tc/capture.sh:/c.sh:ro \
#     -v $PWD/hack/testbed/testdata/tc:/out strims-testbed bash /c.sh
set -u
source /links.sh
payload=$(head -c 1200 /dev/zero | tr '\0' a)
flood() { local end=$(( SECONDS + 2 )); while [ $SECONDS -lt $end ]; do printf %s "$payload" > /dev/udp/10.99.0.2/9 2>/dev/null; done; }
{ echo "# period_us 1000000"; for t in $(seq 100000 100000 1000000); do echo "$t 1514"; done; } > /tmp/slow.us
for spec in tbf cable cable,aqm=fifo dsl dsl,line=vdsl fiber cable,router=fq_codel dsl,router=cake trace; do
  ip link del eth0 2>/dev/null; ip link del ifbd 2>/dev/null
  ip link add eth0 type dummy && ip link set eth0 up && ip addr add 10.99.0.1/24 dev eth0
  ip link add ifbd type ifb && ip link set ifbd up
  name=$(echo "$spec" | tr ',=' '__')
  mkdir -p /tmp/links /out
  if [ "$spec" = trace ]; then
    cp /tmp/slow.us /tmp/links/
    root=$(link_root "trace,file=slow.us,format=us" 0.1 3028 30000 "" "$(( ${EPOCHREALTIME/[.,]/} / 1000 ))" 0 \
      | sed -e 's|/links/|/tmp/links/|g' -e 's|/out/link-0|/tmp/link-0|g')
  else
    root=$(link_root "$spec" 0.1 3028 30000 "" 0 0)
  fi
  eval "$root"
  tc qdisc add dev ifbd root netem delay 1ms limit 100000
  tc qdisc add dev eth0 clsact
  tc filter add dev eth0 egress matchall action mirred egress redirect dev ifbd
  flood
  for _ in 1 2 3 4 5; do
    a=$(tc -s qdisc show dev eth0); j=$(tc -s -j qdisc show dev eth0); b=$(tc -s qdisc show dev eth0)
    [ "$a" = "$b" ] && break
  done
  printf '%s\n' "$a" > "/out/$name.txt"
  printf '%s\n' "$j" > "/out/$name.json"
  tc -s -d qdisc show > "/out/$name.all"
  pkill tracelink 2>/dev/null
  tc qdisc del dev eth0 root 2>/dev/null
done

#!/usr/bin/env bash
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
#
# A farm testbed host's boot check, as this checkout's run.sh shapes a
# node's link. farm.py runs it on the controller from each testbed arm's
# bundle (as a host runs run.sh: from the bundle's hack/testbed, each
# artifact's variable set to its path there), and the host
# (hack/sim-farm/user-data.sh) runs what it prints, so the host checks
# what each arm builds and parses no link spec itself. It prints
#
#   modules <module>...    what the host loads first; one that doesn't
#                          load fails the host, named
#   tree <name>            a tree: the lines after it, up to the next
#   <command>              tree, which the host runs with bash -e in a
#   ...                    network namespace of their own
#
# and fails (and the launch with it) on what it can't render. A tree
# stands for a node's link on a dummy eth0: run.sh's delay ifb behind
# eth0's clsact redirect (with the class's slots), link_root's modem (a
# trace's tracelink left out, its tc lines kept), link_change's step, and
# on the ingress SEED_LOSS_PCT's filter and the slow downlink's redirect.
#
#   hack/testbed/linkcheck.sh

set -euo pipefail
cd "$(dirname "$0")"
# shellcheck source=hack/testbed/links.sh
source ./links.sh

modules=$(link_modules) || { echo "link_modules failed" >&2; exit 1; }
[ -n "$modules" ] || { echo "link_modules lists no module" >&2; exit 1; }
# shellcheck disable=SC2086 # one line, a module a word
echo modules $modules dummy

for spec in tbf tbf,slot=2-8 cable cable,aqm=fifo dsl dsl,line=vdsl fiber \
  cable,router=fq_codel dsl,router=fq_codel cable,router=cake dsl,router=cake trace,file=up.mm; do
  link_check "$spec"
  class=$(link_class "$spec")
  peak=""
  [ "$class" = tbf ] && peak=100
  root=$(link_root "$spec" 10 3028 125000 "$peak" 0 0)
  [ -n "$root" ] || { echo "link $spec: link_root gives no tree" >&2; exit 1; }
  echo "tree $spec"
  echo "ip link add eth0 type dummy"
  echo "ip link set eth0 up"
  echo "ip link add ifbd type ifb"
  echo "ip link set ifbd up"
  echo "tc qdisc add dev ifbd root netem delay 20ms$(link_slot "$spec") limit 100000"
  echo "tc qdisc add dev eth0 clsact"
  echo "tc filter add dev eth0 egress matchall action mirred egress redirect dev ifbd"
  grep -E '^(tc|ip) ' <<< "$root"
  if [ "$class" != trace ]; then
    link_change "$spec" 5 3028 62500 "$peak" 10
  fi
  echo "tc filter add dev eth0 ingress prio 1 protocol ip u32 match ip src 10.77.0.10/32 action gact continue random netrand drop 100"
  echo "ip link add ifbs type ifb"
  echo "ip link set ifbs up"
  echo "tc filter add dev eth0 ingress prio 2 matchall action mirred egress redirect dev ifbs"
  echo "tc qdisc add dev ifbs root tbf rate 1000kbit burst 3028 limit 12500"
done

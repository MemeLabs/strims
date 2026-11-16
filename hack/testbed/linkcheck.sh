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
# stands for a node's link on a dummy eth0: links.sh's node_tree, as
# run.sh builds it, with the loss filter and the slow downlink (its tc and
# ip lines: a trace's tracelink and the offloads left out), then
# link_change's step.
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
  node_tree "$spec" 10 3028 125000 "$peak" 0 0 20 0 \
    "protocol ip u32 match ip src 10.77.0.10/32" 100 1000 100 | grep -E '^(tc|ip) '
  if [ "$class" != trace ]; then
    link_change "$spec" 5 3028 62500 "$peak" 10
  fi
done

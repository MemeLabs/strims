#!/usr/bin/env bash
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
#
# A farm testbed host's boot check of a node's link, shaped as this
# checkout's run.sh shapes it. farm.py runs this script on the controller
# from each testbed arm's bundle, as a host runs run.sh: from the bundle's
# hack/testbed, with each artifact's variable set to its path there. The
# host (hack/sim-farm/user-data.sh) runs what it prints, so the host checks
# what each arm builds and parses no link spec itself. It prints
#
#   modules <module>...    what the host loads first; one that doesn't
#                          load fails the host, named
#   tree <name>            a tree: the lines after it, up to the next
#   <command>              tree, which the host runs with bash -e in a
#   ...                    network namespace of their own
#
# and fails, and the launch with it, on what it can't render.
#
# A tree stands for a node's link on a dummy eth0: links.sh's node_tree, as
# run.sh builds it from the class's spec, with the loss filter and the slow
# downlink, then link_change's step. Only its tc and ip lines go in: a
# trace's tracelink and the offloads are left out. testbed_link parses each
# spec: the bundle's LINK_BIN, which the controller runs, or when that is
# unset one built from this tree as run.sh builds it. cable's tree is the
# slotted one: eth0's prio, its fw filter and skbedit mark, and the modem
# on ifbm.
#
#   hack/testbed/linkcheck.sh

set -euo pipefail
cd "$(dirname "$0")"
# shellcheck source=hack/testbed/links.sh
source ./links.sh
link_bin=${LINK_BIN:-}
if [ -z "$link_bin" ]; then
  cargo build -q --release --locked --manifest-path ../../Cargo.toml -p strims-wan --example testbed_link \
    || { echo "no LINK_BIN, and testbed_link doesn't build here" >&2; exit 1; }
  link_bin=${CARGO_TARGET_DIR:-../../target}/release/examples/testbed_link
fi

modules=$(link_modules) || { echo "link_modules failed" >&2; exit 1; }
[ -n "$modules" ] || { echo "link_modules lists no module" >&2; exit 1; }
# shellcheck disable=SC2086 # one line, a module a word
echo modules $modules dummy

for spec in tbf cable cable,aqm=fifo dsl dsl,line=vdsl fiber \
  cable,router=fq_codel dsl,router=fq_codel cable,router=cake dsl,router=cake trace,file=up.mm; do
  peak=""
  [ "${spec%%,*}" = tbf ] && peak=100
  link_eval "$spec" 10 3028 125000 "$peak" || { echo "link $spec: testbed_link refused it" >&2; exit 1; }
  root=$(link_root 20 0)
  [ -n "$root" ] || { echo "link $spec: link_root gives no tree" >&2; exit 1; }
  echo "tree $spec"
  echo "ip link add eth0 type dummy"
  echo "ip link set eth0 up"
  node_tree 20 0 0 "protocol ip u32 match ip src 10.77.0.10/32" 100 1000 100 | grep -E '^(tc|ip) '
  if [ "$LINK_CLASS" != trace ]; then
    link_eval "$spec" 5 3028 62500 "$peak" 10 || { echo "link $spec: testbed_link refused its step" >&2; exit 1; }
    link_change
  fi
done

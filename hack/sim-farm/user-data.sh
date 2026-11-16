#!/bin/bash
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
#
# The sim farm's instance boot (cloud-init user data; farm.py's user_data
# fills in the lifetime, whether the host runs testbeds, and
# hack/testbed/links.sh). The instance terminates when it shuts down, so
# the scheduled shutdown caps what a forgotten instance costs. Background
# apt work would take cores from the jobs.
#
# A testbed host gets Docker, with the ubuntu user in its group, and the
# kernel modules links.sh's link_modules lists, which the farm's image
# (Ubuntu 26.04, linux-aws) has in its base modules package: a module that
# doesn't load fails the host, named. Then each link class's tree is built
# with links.sh's own functions, in a network namespace of its own whose
# dummy eth0 stands for a node's: run.sh's delay ifb behind eth0's clsact
# redirect (with the class's slots), link_root's modem (a trace's
# tracelink left out, its tc lines kept), link_change's step, and on the
# ingress SEED_LOSS_PCT's filter and the slow downlink's redirect. farm.py
# waits for farm/bin, which comes last; a failed setup leaves its log in
# farm/host-failed instead.

shutdown -h +@MINUTES@
systemctl disable --now unattended-upgrades.service apt-daily.timer apt-daily-upgrade.timer || true
install -d -o ubuntu -g ubuntu /home/ubuntu/farm

links=/root/farm-links.sh

# One class's tree on the namespace's eth0, as run.sh builds a node's link.
check_tree() {
  local spec=$1 peak="" root
  link_check "$spec"
  [ "$(link_class "$spec")" = tbf ] && peak=100
  ip link add eth0 type dummy
  ip link set eth0 up
  ip link add ifbd type ifb
  ip link set ifbd up
  tc qdisc add dev ifbd root netem delay 20ms$(link_slot "$spec") limit 100000
  tc qdisc add dev eth0 clsact
  tc filter add dev eth0 egress matchall action mirred egress redirect dev ifbd
  root=$(link_root "$spec" 10 3028 125000 "$peak" 0 0)
  [ -n "$root" ]
  while read -r cmd; do
    case $cmd in tc\ * | ip\ *) eval "$cmd" ;; esac
  done <<< "$root"
  if [ "$(link_class "$spec")" != trace ]; then
    eval "$(link_change "$spec" 5 3028 62500 "$peak" 10)"
  fi
  tc filter add dev eth0 ingress prio 1 protocol ip u32 match ip src 10.77.0.10/32 \
    action gact continue random netrand drop 100
  ip link add ifbs type ifb
  ip link set ifbs up
  tc filter add dev eth0 ingress prio 2 matchall action mirred egress redirect dev ifbs
  tc qdisc add dev ifbs root tbf rate 1000kbit burst 3028 limit 12500
  tc qdisc show
}

testbed_host() {
  set -ex
  export DEBIAN_FRONTEND=noninteractive
  apt="apt-get -o DPkg::Lock::Timeout=300 -y -q"
  $apt update
  $apt install --no-install-recommends docker.io docker-buildx
  usermod -aG docker ubuntu
  base64 -d <<'LINKS_GZ' | gunzip > "$links"
@LINKS_GZ@
LINKS_GZ
  # shellcheck source=hack/testbed/links.sh
  source "$links"
  local m missing=""
  for m in $(link_modules) dummy; do
    modprobe "$m" || missing="$missing $m"
  done
  if [ -n "$missing" ]; then
    echo "kernel $(uname -r) can't load:$missing" >&2
    return 1
  fi
  local spec
  for spec in tbf tbf,slot=2-8 cable cable,aqm=fifo dsl dsl,line=vdsl fiber \
    cable,router=fq_codel dsl,router=fq_codel cable,router=cake dsl,router=cake trace,file=up.mm; do
    ip netns add farm-check
    ip netns exec farm-check bash -ex -c "source $links; $(declare -f check_tree); check_tree '$spec'"
    ip netns del farm-check
    echo "link ok: $spec"
  done
  docker info
}

if [ @DOCKER@ = 1 ]; then
  (testbed_host) > /var/log/sim-farm-setup.log 2>&1
  if [ $? -ne 0 ]; then
    install -o ubuntu -g ubuntu -m 644 /var/log/sim-farm-setup.log /home/ubuntu/farm/host-failed
    exit 1
  fi
fi
install -d -o ubuntu -g ubuntu /home/ubuntu/farm/bin

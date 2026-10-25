#!/bin/bash
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
#
# The sim farm's instance boot (cloud-init user data; farm.py fills in the
# lifetime, and whether the host runs testbeds). The instance terminates
# when it shuts down, so the scheduled shutdown caps what a forgotten
# instance costs. Background apt work would take cores from the jobs.
#
# A testbed host gets Docker, with the ubuntu user in its group, and the
# qdisc modules hack/testbed/run.sh shapes links with, from
# linux-modules-extra if the kernel splits them out. Shaping is tried on
# a dummy device as run.sh shapes a node's link. farm.py waits for
# farm/bin, which comes last; a failed setup leaves its log in
# farm/host-failed instead.

shutdown -h +@MINUTES@
systemctl disable --now unattended-upgrades.service apt-daily.timer apt-daily-upgrade.timer || true
install -d -o ubuntu -g ubuntu /home/ubuntu/farm

testbed_host() {
  set -ex
  export DEBIAN_FRONTEND=noninteractive
  apt="apt-get -o DPkg::Lock::Timeout=300 -y -q"
  $apt update
  $apt install --no-install-recommends docker.io docker-buildx
  usermod -aG docker ubuntu
  mods="sch_netem sch_tbf sch_ingress ifb cls_matchall act_mirred dummy"
  modprobe -a $mods || {
    $apt install --no-install-recommends "linux-modules-extra-$(uname -r)"
    modprobe -a $mods
  }
  ip link add farm-check type dummy
  ip link set farm-check up
  tc qdisc replace dev farm-check root handle 1: netem delay 1ms limit 100000
  tc qdisc add dev farm-check parent 1:1 handle 10: tbf rate 10mbit burst 3028 limit 125000
  ip link add farm-ifb type ifb
  ip link set farm-ifb up
  tc qdisc add dev farm-check handle ffff: ingress
  tc filter add dev farm-check parent ffff: matchall action mirred egress redirect dev farm-ifb
  tc qdisc add dev farm-ifb root tbf rate 1000kbit burst 3028 limit 12500
  ip link del farm-ifb
  ip link del farm-check
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

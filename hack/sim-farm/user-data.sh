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
# kernel modules hack/testbed/run.sh and links.sh shape links with (run.sh's
# header lists them), from linux-modules-extra if the kernel splits any
# out. The trees they build are tried on a dummy device: run.sh's delay
# ifb behind eth0's clsact redirect with cable's slots, its root tbf, the
# link classes' roots (cable's size table, peak and DOCSIS-PIE, ADSL's
# cells under a router's fq_codel, a router's cake, the trace's prio and
# fq), the slow downlink's ingress redirect and SEED_LOSS_PCT's filter.
# farm.py waits for farm/bin, which comes last; a failed setup leaves its
# log in farm/host-failed instead.

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
  mods="sch_netem sch_tbf ifb sch_ingress cls_matchall act_mirred sch_pie sch_fq_codel sch_cake sch_prio sch_fq
    cls_bpf act_gact cls_u32 dummy"
  modprobe -a $mods || {
    $apt install --no-install-recommends "linux-modules-extra-$(uname -r)"
    modprobe -a $mods
  }
  ip link add farm-check type dummy
  ip link set farm-check up
  ip link add farm-ifb type ifb
  ip link set farm-ifb up
  tc qdisc add dev farm-ifb root netem delay 20ms slot 2ms 8ms limit 100000
  tc qdisc add dev farm-check clsact
  tc filter add dev farm-check egress matchall action mirred egress redirect dev farm-ifb
  tc qdisc replace dev farm-check root handle 10: tbf rate 10000kbit burst 3028 limit 125000 peakrate 100mbit mtu 3028
  tc qdisc del dev farm-check root
  tc qdisc replace dev farm-check root handle 10: stab overhead 4 mpu 64 mtu 2048 tsize 2048 \
    tbf rate 10000kbit burst 156250 limit 125000 peakrate 100mbit mtu 3028
  tc qdisc add dev farm-check parent 10:1 handle 20: pie limit 10000 target 10ms tupdate 16ms
  tc qdisc del dev farm-check root
  tc qdisc replace dev farm-check root handle 10: stab linklayer atm overhead 26 mtu 2048 tsize 2048 \
    tbf rate 9500kbit burst 1749 limit 125000
  tc qdisc add dev farm-check parent 10:1 handle 20: fq_codel
  tc qdisc del dev farm-check root
  tc qdisc replace dev farm-check root handle 10: cake bandwidth 9500kbit docsis
  tc qdisc del dev farm-check root
  tc qdisc replace dev farm-check root handle 10: prio bands 2 priomap 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
  tc qdisc add dev farm-check parent 10:1 handle 20: fq nopacing limit 100000 flow_limit 100000 horizon 100s
  ip link add farm-ifbs type ifb
  ip link set farm-ifbs up
  tc filter add dev farm-check ingress prio 1 protocol ip u32 match ip src 10.77.0.10/32 \
    action gact continue random netrand drop 100
  tc filter add dev farm-check ingress prio 2 matchall action mirred egress redirect dev farm-ifbs
  tc qdisc add dev farm-ifbs root tbf rate 1000kbit burst 3028 limit 12500
  ip link del farm-ifbs
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

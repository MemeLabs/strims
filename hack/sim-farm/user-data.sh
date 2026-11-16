#!/bin/bash
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
#
# The sim farm's instance boot (cloud-init user data; farm.py's user_data
# fills in the lifetime, whether the host runs testbeds, and the link
# check). The instance terminates when it shuts down, so the scheduled
# shutdown caps what a forgotten instance costs. Background apt work would
# take cores from the jobs.
#
# A testbed host gets Docker, with the ubuntu user in its group, and runs
# the link check farm.py rendered from the run's testbed arms' bundles
# (each arm's hack/testbed/linkcheck.sh, whose header gives its form): it
# loads the check's modules, which the farm's image (Ubuntu 26.04,
# linux-aws) has in its base modules package (one that doesn't load fails
# the host, named), and builds each of its trees in a network namespace
# of its own. The host parses no link spec: it runs what the check says.
# An empty check (no arm's bundle has a linkcheck.sh) checks no link.
# farm.py waits for farm/bin, which comes last; a failed setup leaves its
# log in farm/host-failed instead.

shutdown -h +@MINUTES@
systemctl disable --now unattended-upgrades.service apt-daily.timer apt-daily-upgrade.timer || true
install -d -o ubuntu -g ubuntu /home/ubuntu/farm

check=/root/farm-check.txt
trees=/root/farm-trees

testbed_host() {
  set -ex
  export DEBIAN_FRONTEND=noninteractive
  apt="apt-get -o DPkg::Lock::Timeout=300 -y -q"
  $apt update
  $apt install --no-install-recommends docker.io docker-buildx
  usermod -aG docker ubuntu
  base64 -d <<'CHECK_GZ' | gunzip > "$check"
@CHECK_GZ@
CHECK_GZ
  if [ -s "$check" ]; then
    local modules m missing="" t
    modules=$(sed -n 's/^modules //p' "$check")
    if [ -z "$modules" ]; then
      echo "the link check lists no module" >&2
      return 1
    fi
    for m in $modules; do
      modprobe "$m" || missing="$missing $m"
    done
    if [ -n "$missing" ]; then
      echo "kernel $(uname -r) can't load:$missing" >&2
      return 1
    fi
    # Each tree as a script of its own, its name in its first line.
    rm -rf "$trees"
    mkdir "$trees"
    awk -v d="$trees" '/^tree / { f = sprintf("%s/%03d.sh", d, ++n); print "# " substr($0, 6) > f; next }
      n { print > f }' "$check"
    if ! ls "$trees"/*.sh >/dev/null 2>&1; then
      echo "the link check has no tree" >&2
      return 1
    fi
    for t in "$trees"/*.sh; do
      ip netns add farm-check
      ip netns exec farm-check bash -ex "$t"
      ip netns exec farm-check tc qdisc show
      ip netns del farm-check
      echo "link ok: $(head -1 "$t" | cut -c3-)"
    done
  else
    echo "no link check"
  fi
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

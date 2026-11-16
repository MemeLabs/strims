#!/usr/bin/env bash
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
#
# Boots user-data.sh as a farm testbed host does, in a container of the
# testbed's image (hack/testbed/run.sh builds strims-testbed: Ubuntu, ip
# and tc) with NET_ADMIN and this host's /lib/modules, so its module loads
# and its shaping checks run for real; kmod is installed first, and stubs
# log shutdown, systemctl, usermod, docker and apt-get. Three boots: one
# that passes and writes farm/bin; one whose first modprobe fails, as on a
# kernel that splits the modules out, which installs linux-modules-extra
# and passes on the second; and one whose tc refuses cake, which fails and
# leaves its log in farm/host-failed.
#
#   hack/sim-farm/user-data_test.sh
#
# The modules must be loaded on this host already (run.sh's header lists
# them): the container can't load any.

set -uo pipefail
here=$(cd "$(dirname "$0")" && pwd)
boot=$(cat <<'EOF'
set -u
apt-get update -qq >/dev/null 2>&1 && apt-get install -y -qq --no-install-recommends kmod >/dev/null 2>&1 || exit 9
mkdir -p /stubs
for c in shutdown systemctl usermod docker apt-get; do
  printf '#!/bin/sh\necho "%s $*" >> /stubs.log\n' "$c" > /stubs/$c
done
if [ "$CASE" = extra ]; then
  printf '#!/bin/sh\nif [ ! -e /stubs/.once ]; then : > /stubs/.once; exit 1; fi\necho "modprobe $*" >> /stubs.log\nexec /usr/sbin/modprobe "$@"\n' > /stubs/modprobe
fi
if [ "$CASE" = fail ]; then
  printf '#!/bin/sh\ncase "$*" in *cake*) echo "tc: cake refused" >&2; exit 2 ;; esac\nexec /usr/sbin/tc "$@"\n' > /stubs/tc
fi
chmod +x /stubs/*
sed -e 's/@MINUTES@/5/' -e 's/@DOCKER@/1/' /user-data.sh > /tmp/user-data.sh
PATH=/stubs:$PATH bash /tmp/user-data.sh
echo "exit $?"
cat /stubs.log
ls /home/ubuntu/farm
EOF
)
fails=0
check() { # case, what its output must hold...
  local c=$1 out want
  shift
  out=$(docker run --rm --cap-add NET_ADMIN -e CASE="$c" -v /lib/modules:/lib/modules:ro \
    -v "$here/user-data.sh:/user-data.sh:ro" strims-testbed bash -c "$boot" 2>/dev/null)
  for want in "$@"; do
    if ! grep -qxF -- "$want" <<< "$out"; then
      echo "FAIL $c: no line '$want' in:"
      sed 's/^/  /' <<< "$out"
      fails=$((fails + 1))
      return
    fi
  done
  echo "ok $c"
}
check pass "exit 0" "docker info" "bin"
check extra "exit 0" "apt-get -o DPkg::Lock::Timeout=300 -y -q install --no-install-recommends linux-modules-extra-$(uname -r)" "bin"
check fail "exit 1" "host-failed"
exit $((fails > 0))

#!/usr/bin/env bash
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
#
# Boots user-data.sh as a farm testbed host does, filled in by farm.py's
# user_data (links.sh embedded), in a container of the testbed's image
# (hack/testbed/run.sh builds strims-testbed: Ubuntu 26.04, ip and tc) with
# NET_ADMIN, SYS_ADMIN for the check's network namespaces and this host's
# /lib/modules, so its module loads and each class's tree run for real;
# kmod is installed first. Stubs log shutdown, systemctl, usermod and
# docker; apt-get's installs only docker.io and docker-buildx, refusing
# any other package as the archive would one it hasn't. Three boots: one
# that passes, building every class's tree and writing farm/bin; one whose
# kernel can't load sch_cake, which fails naming it; and one whose tc
# refuses cake, which fails at that command. Each leaves its setup log in
# farm/host-failed, which the test checks.
#
#   hack/sim-farm/user-data_test.sh
#
# The modules must be loaded on this host already (run.sh's header):
# the container loads none.

set -uo pipefail
here=$(cd "$(dirname "$0")" && pwd)
filled=$(mktemp)
trap 'rm -f "$filled"' EXIT
python3 -c "import sys; sys.path.insert(0, sys.argv[1]); import farm; sys.stdout.write(farm.user_data(5, True))" \
  "$here" > "$filled" || exit 1
boot=$(cat <<'EOF'
set -u
apt-get update -qq >/dev/null 2>&1 && apt-get install -y -qq --no-install-recommends kmod >/dev/null 2>&1 || exit 9
mkdir -p /stubs
for c in shutdown systemctl usermod docker; do
  printf '#!/bin/sh\necho "%s $*" >> /stubs.log\n' "$c" > /stubs/$c
done
cat > /stubs/apt-get <<'STUB'
#!/bin/sh
echo "apt-get $*" >> /stubs.log
for a in "$@"; do
  case $a in
    -*|install|update|Dpkg*|DPkg*|docker.io|docker-buildx) ;;
    *) echo "E: Unable to locate package $a" >&2; exit 100 ;;
  esac
done
STUB
case $CASE in
  missing)
    printf '#!/bin/sh\n[ "$1" = sch_cake ] && { echo "modprobe: FATAL: Module sch_cake not found" >&2; exit 1; }\nexec /usr/sbin/modprobe "$@"\n' > /stubs/modprobe ;;
  refuse)
    printf '#!/bin/sh\ncase "$*" in *" cake "*) echo "tc: cake refused" >&2; exit 2 ;; esac\nexec /usr/sbin/tc "$@"\n' > /stubs/tc ;;
esac
chmod +x /stubs/*
PATH=/stubs:$PATH bash /user-data.sh
echo "exit $?"
cat /stubs.log
ls /home/ubuntu/farm
grep -h "^link ok: \|can't load\|refused" /var/log/sim-farm-setup.log
EOF
)
specs="tbf tbf,slot=2-8 cable cable,aqm=fifo dsl dsl,line=vdsl fiber cable,router=fq_codel dsl,router=fq_codel cable,router=cake dsl,router=cake trace,file=up.mm"
fails=0
check() { # case, the lines its output must hold...
  local c=$1 out want
  shift
  out=$(docker run --rm --cap-add NET_ADMIN --cap-add SYS_ADMIN --security-opt apparmor=unconfined \
    -e CASE="$c" -v /lib/modules:/lib/modules:ro -v "$filled:/user-data.sh:ro" strims-testbed bash -c "$boot" 2>&1)
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
oks=()
for s in $specs; do oks+=("link ok: $s"); done
check pass "exit 0" "docker info" "bin" "${oks[@]}"
check missing "exit 1" "host-failed" "kernel $(uname -r) can't load: sch_cake"
check refuse "exit 1" "host-failed" "link ok: tbf" "link ok: dsl,router=fq_codel" "tc: cake refused"
# What the boot checks is what links.sh builds: no tree of its own.
if grep -nE "tc qdisc (add|replace) dev [^ ]+ root handle 10:" "$here/user-data.sh"; then
  echo "FAIL user-data.sh builds a modem of its own"
  fails=$((fails + 1))
fi
exit $((fails > 0))

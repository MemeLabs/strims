#!/usr/bin/env bash
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
#
# farm.py's link check and user-data.sh's boot of a testbed host.
#
# The controller's side: link_check over testbeds made from this
# checkout's hack/testbed, with this checkout's testbed_link as their
# LINK_BIN (LINK_BIN if given, else built as run.sh builds it): one
# renders this checkout's check; two of which one's cake trees differ
# merge their modules and keep each distinct tree once; one without
# linkcheck.sh is said to go unchecked; one whose link_modules fails, or
# lists none, stops the run.
#
# The host's side: user-data.sh filled in by user_data with this
# checkout's check, booted in a container of the testbed's image
# (hack/testbed/run.sh builds strims-testbed: Ubuntu 26.04, ip and tc)
# with NET_ADMIN, SYS_ADMIN for the check's network namespaces and this
# host's /lib/modules, so its module loads and each tree run for real;
# kmod is installed first. Stubs log shutdown, systemctl, usermod and
# docker; apt-get's installs only docker.io and docker-buildx, refusing
# any other package as the archive would one it hasn't. Boots: one that
# passes, building every tree and writing farm/bin; one whose kernel can't
# load sch_cake, which fails naming it; one whose tc refuses cake, which
# fails at that command; one whose check lists no module, which fails;
# and an empty check's, which checks no link and passes. Each failure
# leaves its setup log in farm/host-failed, which the test checks.
#
#   hack/sim-farm/user-data_test.sh
#
# The modules must be loaded on this host already (run.sh's header):
# the container loads none.

set -uo pipefail
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fails=0
fail() { echo "FAIL $*"; fails=$((fails + 1)); }
# The testbeds' parser, as a bundle's LINK_BIN.
if [ -z "${LINK_BIN:-}" ]; then
  cargo build -q --release --locked --manifest-path "$repo/Cargo.toml" -p strims-wan --example testbed_link \
    || { echo "FAIL building testbed_link"; exit 1; }
  LINK_BIN=${CARGO_TARGET_DIR:-$repo/target}/release/examples/testbed_link
fi
export LINK_BIN

# A testbed of this checkout's hack/testbed, whose links.sh gets a tail.
testbed() { # dir, links.sh's tail
  mkdir -p "$1/hack"
  cp -r "$repo/hack/testbed" "$1/hack/"
  printf '%s\n' "$2" >> "$1/hack/testbed/links.sh"
}
# farm.link_check over label=dir pairs: its check, then its log, then its exit.
check_of() {
  python3 - "$here" "$@" <<'EOF'
import sys
from pathlib import Path
sys.path.insert(0, sys.argv[1])
import farm
farm.log = lambda msg: print(f"log: {msg}", file=sys.stderr)
sys.stdout.write(farm.link_check({a.split("=")[0]: Path(a.split("=", 1)[1]) for a in sys.argv[2:]}))
EOF
}
testbed "$work/a" ""
testbed "$work/b" 'eval "orig_$(declare -f link_eval)"
link_eval() { orig_link_eval "$@" || return 1; LINK_SHAPER=${LINK_SHAPER/docsis/overhead 20}; LINK_SHAPER=${LINK_SHAPER/pppoe-llcsnap/overhead 20}; }'
testbed "$work/none" ""
rm "$work/none/hack/testbed/linkcheck.sh"
testbed "$work/broken" 'link_modules() { return 3; }'
testbed "$work/empty" 'link_modules() { :; }'

one=$(check_of "a=$work/a") || fail "link_check of this checkout failed"
[ "$one" = "$(bash "$repo/hack/testbed/linkcheck.sh")" ] || fail "one testbed's check isn't its linkcheck.sh's"
two=$(check_of "a=$work/a" "b=$work/b" "a again=$work/a")
[ "$(grep -c '^modules ' <<< "$two")" = 1 ] || fail "two testbeds' check has $(grep -c '^modules ' <<< "$two") module lines"
[ "$(grep -c '^tree ' <<< "$one")" = 11 ] || fail "this checkout's check has $(grep -c '^tree ' <<< "$one") trees"
[ "$(grep -c '^tree ' <<< "$two")" = 13 ] || fail "two testbeds' check has $(grep -c '^tree ' <<< "$two") trees (11 and b's 2 cake trees)"
grep -q "cake bandwidth .*overhead 20" <<< "$two" || fail "b's cake tree is missing"
out=$(check_of "none=$work/none" "a=$work/a" 2>&1 >/dev/null)
grep -qx "log: none: its bundle has no hack/testbed/linkcheck.sh, so a host checks none of its links" <<< "$out" \
  || fail "no word of an unchecked testbed: $out"
[ -z "$(check_of "none=$work/none" 2>/dev/null)" ] || fail "an unchecked testbed's check isn't empty"
for t in "broken:link_modules failed" "empty:link_modules lists no module"; do
  if out=$(check_of "${t%%:*}=$work/${t%%:*}" 2>&1); then fail "${t%%:*}'s check rendered"; fi
  grep -qxF "${t%%:*}: hack/testbed/linkcheck.sh failed: ${t#*:}" <<< "$out" || fail "${t%%:*}: $out"
done
echo "ok link_check"

filled() { # the check's file, the user data's
  python3 -c "import sys; sys.path.insert(0, sys.argv[1]); import farm; sys.stdout.write(farm.user_data(5, open(sys.argv[2]).read()))" \
    "$here" "$1" > "$2" || exit 1
}
printf '%s\n' "$one" > "$work/one.txt"
printf 'tree x\nip link add eth0 type dummy\n' > "$work/nomodules.txt"
: > "$work/nocheck.txt"
for c in one nomodules nocheck; do filled "$work/$c.txt" "$work/$c.sh"; done
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
grep -h "^link ok: \|can't load\|refused\|^the link check\|^no link check" /var/log/sim-farm-setup.log
EOF
)
check() { # case, user data, the lines its output must hold...
  local c=$1 data=$2 out want
  shift 2
  out=$(docker run --rm --cap-add NET_ADMIN --cap-add SYS_ADMIN --security-opt apparmor=unconfined \
    -e CASE="$c" -v /lib/modules:/lib/modules:ro -v "$data:/user-data.sh:ro" strims-testbed bash -c "$boot" 2>&1)
  for want in "$@"; do
    if ! grep -qxF -- "$want" <<< "$out"; then
      fail "$c: no line '$want' in:"
      sed 's/^/  /' <<< "$out"
      return
    fi
  done
  echo "ok $c"
}
oks=()
while read -r s; do oks+=("link ok: $s"); done < <(sed -n 's/^tree //p' <<< "$one")
check pass "$work/one.sh" "exit 0" "docker info" "bin" "${oks[@]}"
check missing "$work/one.sh" "exit 1" "host-failed" "kernel $(uname -r) can't load: sch_cake"
check refuse "$work/one.sh" "exit 1" "host-failed" "link ok: tbf" "link ok: dsl,router=fq_codel" "tc: cake refused"
check nomodules "$work/nomodules.sh" "exit 1" "host-failed" "the link check lists no module"
check nocheck "$work/nocheck.sh" "exit 0" "docker info" "bin" "no link check"
# The host parses no link spec and builds no tree of its own: it runs the check.
if grep -nE "link_[a-z]+|tc (qdisc|filter) (add|replace|change)" "$here/user-data.sh"; then
  fail "user-data.sh shapes a link of its own"
fi
exit $((fails > 0))

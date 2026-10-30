#!/usr/bin/env bash
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
#
# Runs the Rust checks of crates/CONVENTIONS.md.
#
#   hack/check.sh          fmt, both clippy runs, the wasm32 check, the
#                          workspace's tests and doctests, and the wasm32
#                          tests in Firefox
#   hack/check.sh --full   also the browser suites (e2e_links, e2e_app,
#                          ui_smoke) and the release simulations
#
# Cargo locks a profile's build directory while it builds (a wasm32 build
# also locks its host profile's), and releases it before tests run. The
# builds are split into lanes by the directories they lock. A lane builds
# one thing at a time, and the lanes run side by side:
#
#   debug  the wasm32 test binaries, the dev wasm build of the e2e bundle
#          (--full), the workspace's test binaries, then the doctests
#   lint   fmt, clippy, the wasm32 check and clippy, in target/lint, so
#          they never hold target/debug
#   sims   the release simulations' binaries, then their run (--full)
#   web    the release-wasm build of ui_smoke's bundle (--full)
#
# A test run starts once its binaries exist and never calls cargo: nextest
# runs the binaries of a binary list, and the browser stages run theirs
# directly. The webpack builds inside the browser suites do call cargo,
# for wasm builds the debug and web lanes have already made.
#
# The browser stages run one at a time at normal priority: snap Firefox
# instances must not start concurrently, and every browser step has a time
# limit. Everything else runs under nice, so it can't starve them; the
# debug lane, which the browser suites wait for, less so than the rest.
#
# Each stage logs to target/check/<stage>.log. The script prints when each
# stage started and how long it took, the tail of every failed stage's
# log, and exits non-zero if any stage failed. A browser test that skips
# for a missing tool fails its stage.
#
# GECKODRIVER defaults to the geckodriver on PATH. Without node on PATH,
# nvm's node for .nvmrc is used.

set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.." || exit 1
root=$PWD

full=0
case "${1-}" in
"") ;;
--full) full=1 ;;
*)
  echo "usage: hack/check.sh [--full]" >&2
  exit 2
  ;;
esac

out=$root/target/check
rm -rf "$out"
mkdir -p "$out"
t0=$EPOCHREALTIME

if ! command -v node >/dev/null && [ -s "${NVM_DIR:-$HOME/.nvm}/nvm.sh" ]; then
  set +u
  # shellcheck disable=SC1091
  . "${NVM_DIR:-$HOME/.nvm}/nvm.sh" --no-use
  nvm use --silent >/dev/null
  set -u
fi
GECKODRIVER=${GECKODRIVER:-$(command -v geckodriver)}
export GECKODRIVER

missing=()
# need TOOL [HOW]: notes TOOL, and how to get it, when it isn't on PATH.
need() { command -v "$1" >/dev/null || missing+=("$1${2:+ ($2)}"); }
need firefox
need cargo-nextest "cargo install cargo-nextest --locked"
need wasm-bindgen-test-runner "it comes with wasm-bindgen-cli"
[ -x "$GECKODRIVER" ] || missing+=("geckodriver (set GECKODRIVER)")
if [ $full = 1 ]; then
  need node
  need npx
  [ -d node_modules ] || missing+=("node_modules (npm ci)")
  pin=$(sed -n 's/^wasm-bindgen = "=\(.*\)"$/\1/p' crates/strims-wasm/Cargo.toml)
  [ "$(wasm-bindgen --version 2>/dev/null)" = "wasm-bindgen $pin" ] ||
    missing+=("wasm-bindgen $pin (cargo install wasm-bindgen-cli --version $pin --locked)")
fi
if [ ${#missing[@]} -gt 0 ]; then
  printf 'missing: %s\n' "${missing[@]}" >&2
  exit 1
fi

seconds() { awk -v a="$1" -v b="$2" 'BEGIN { printf "%.1f", b - a }'; }

# stage NAME DEPS CMD...: once every stage in DEPS (space-separated) has
# passed, runs CMD in a subshell with its output in NAME.log, and records
# its exit status, start and wall time in NAME.done. A stage whose
# dependency failed is recorded as skipped.
stage() {
  local name=$1 deps=$2 d rc start
  shift 2
  for d in $deps; do
    until [ -e "$out/$d.done" ]; do sleep 0.2; done
    read -r rc _ <"$out/$d.done"
    if [ "$rc" != 0 ]; then
      echo "skip $(seconds "$t0" "$EPOCHREALTIME") -" >"$out/$name.done"
      return
    fi
  done
  start=$EPOCHREALTIME
  ("$@") >"$out/$name.log" 2>&1
  rc=$?
  echo "$rc $(seconds "$t0" "$start") $(seconds "$start" "$EPOCHREALTIME")" >"$out/$name.done"
}

# The executables of a cargo --message-format=json build.
executables() { grep -o '"executable":"[^"]*"' "$1" | cut -d'"' -f4; }

# The path of test binary ID in a nextest binary list.
binary() {
  node -e '
    const list = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8"));
    const bin = list["rust-binaries"][process.argv[2]];
    if (!bin) process.exit(1);
    console.log(bin["binary-path"]);
  ' "$1" "$2"
}

wasm_excludes=(--exclude strims-node --exclude strims-sim --exclude strims-kv-redb
  --exclude strims-testworld --exclude strims-svc)

fmt() { cargo fmt --all --check; }
clippy() {
  CARGO_TARGET_DIR=target/lint cargo clippy --locked --workspace --all-targets \
    --features strims-node/turmoil -- -D warnings
}
wasm_check() {
  CARGO_TARGET_DIR=target/lint cargo check --locked --workspace "${wasm_excludes[@]}" \
    --target wasm32-unknown-unknown
}
wasm_clippy() {
  CARGO_TARGET_DIR=target/lint cargo clippy --locked -p strims-node-core -p strims-wasm -p strims-player \
    --target wasm32-unknown-unknown --all-targets --features strims-wasm/e2e -- -D warnings
}

wasm_build() {
  cargo test --locked -p strims-wasm --target wasm32-unknown-unknown --no-run \
    --message-format=json-render-diagnostics >"$out/wasm-tests.json"
}
# The builds strims-wasm's webpack loader asks for (vendor_modules/strims-wasm-loader).
e2e_wasm() {
  cargo build --locked -p strims-wasm --lib --target wasm32-unknown-unknown --profile dev \
    --features e2e
}
web_wasm() {
  cargo build --locked -p strims-wasm --lib --target wasm32-unknown-unknown \
    --profile release-wasm
}
workspace_build() {
  cargo nextest list --locked --workspace --list-type binaries-only --message-format json \
    >"$out/workspace-binaries.json"
}
doctest() { cargo test --locked --workspace --doc; }
sims_build() {
  cargo nextest list --locked --release -p strims-sim -p strims-ppspp \
    --list-type binaries-only --message-format json >"$out/sims-binaries.json"
}

tests() {
  cargo nextest run --no-fail-fast --binaries-metadata "$out/workspace-binaries.json" \
    --cargo-metadata "$out/metadata.json"
}
sims() {
  cargo nextest run --no-fail-fast --run-ignored all --success-output final \
    --binaries-metadata "$out/sims-binaries.json" --cargo-metadata "$out/metadata.json"
}

# Runs each wasm32 test binary as cargo's runner would.
wasm_tests() {
  local exe rc=0
  cd crates/strims-wasm || return 1
  for exe in $(executables "$out/wasm-tests.json"); do
    echo "=== $exe"
    wasm-bindgen-test-runner "$exe" || rc=1
  done
  return $rc
}

# browser_suite NAME: strims-wasm's ignored browser test binary NAME.
browser_suite() {
  local bin
  bin=$(binary "$out/workspace-binaries.json" "strims-wasm::$1") || return 1
  cd crates/strims-wasm || return 1
  "$bin" --ignored --show-output || return 1
  if grep -q '^skipping:' "$out/$1.log"; then
    echo "check: a test skipped" >&2
    return 1
  fi
}

# Refreshes Cargo.lock if it must change; every later cargo call is
# --locked, so the lanes never write it.
if ! cargo metadata --format-version 1 >"$out/metadata.json" 2>"$out/metadata.log"; then
  cat "$out/metadata.log" >&2
  exit 1
fi

# Every lane is a job of its own process group, killed if the script ends
# early.
set -m
cleanup() {
  local j
  for j in $(jobs -p); do kill -TERM -- "-$j" 2>/dev/null; done
}
trap cleanup EXIT
trap 'exit 130' INT TERM

# lane NICE FUNCTION: runs FUNCTION NICE steps below the script's priority.
lane() {
  renice -n "$(($(nice) + $1))" -p "$BASHPID" >/dev/null
  "$2"
}

lint_lane() {
  stage fmt "" fmt
  stage clippy "" clippy
  stage wasm-check "" wasm_check
  stage wasm-clippy "" wasm_clippy
}
tests_lane() { stage tests "workspace-build" tests; }

if [ $full = 1 ]; then
  debug_lane() {
    stage wasm-build "" wasm_build
    stage e2e-wasm "" e2e_wasm
    stage workspace-build "" workspace_build
    stage doctest "" doctest
  }
  browser_lane() {
    stage wasm-tests "wasm-build" wasm_tests
    stage e2e_links "workspace-build e2e-wasm" browser_suite e2e_links
    stage e2e_app "workspace-build e2e-wasm" browser_suite e2e_app
    stage ui_smoke "workspace-build web-wasm" browser_suite ui_smoke
  }
  sims_lane() {
    stage sims-build "" sims_build
    stage sims "sims-build" sims
  }
  web_lane() { stage web-wasm "" web_wasm; }
  lane 10 sims_lane &
  lane 10 web_lane &
else
  debug_lane() {
    stage wasm-build "" wasm_build
    stage workspace-build "" workspace_build
    stage doctest "" doctest
  }
  browser_lane() { stage wasm-tests "wasm-build" wasm_tests; }
fi
lane 5 debug_lane &
lane 10 lint_lane &
lane 10 tests_lane &
lane 0 browser_lane &
wait

status=0
printf '%-16s %7s %7s  %s\n' stage start wall result
for f in "$out"/*.done; do
  name=$(basename "$f" .done)
  read -r rc start wall <"$f"
  echo "$start $name $wall $rc"
done | sort -n | while read -r start name wall rc; do
  case $rc in
  0) result=ok ;;
  skip) result="skipped: a stage it needs failed" ;;
  *) result="FAILED ($rc)" ;;
  esac
  printf '%-16s %7s %7s  %s\n' "$name" "$start" "$wall" "$result"
done
echo "total: $(seconds "$t0" "$EPOCHREALTIME") s"

for f in "$out"/*.done; do
  name=$(basename "$f" .done)
  read -r rc _ <"$f"
  [ "$rc" = 0 ] && continue
  status=1
  [ "$rc" = skip ] && continue
  echo
  echo "--- $name failed; the end of $out/$name.log:"
  tail -n 40 "$out/$name.log"
done
for f in $(grep -l "Blocking waiting for file lock on build" "$out"/*.log 2>/dev/null); do
  echo "$(basename "$f" .log) waited for a cargo lock"
done
exit $status

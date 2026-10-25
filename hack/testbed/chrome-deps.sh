#!/bin/sh
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
#
# The packages that satisfy Chrome for Testing's deb.deps ($1) here: per
# line, the first alternative (without its version) that apt can install,
# as named or as its t64 build. libc6 and the desktop tools are left out.
sed -E 's/\([^)]*\)//g' "$1" | grep -vxE ' *(libc6|wget|xdg-utils) *' | while read -r line; do
  echo "$line" | tr '|' '\n' | while read -r p; do
    for name in "$p" "${p}t64"; do
      if apt-cache policy "$name" | grep -q 'Candidate: [0-9]'; then echo "$name"; exit 1; fi
    done
  done && echo "no package for: $line" >&2
done

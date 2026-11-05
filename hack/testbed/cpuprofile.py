#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""Summarises the V8 CPU profiles a testbed run's Chromium nodes wrote
(run.sh's CPU_PROFILE): how busy each profiled thread was, and where its
time went.

    hack/testbed/cpuprofile.py <files.cpuprofile...>
    hack/testbed/cpuprofile.py --crate strims_ppspp <files...>
    hack/testbed/cpuprofile.py --match 'rtc::Host>::' <files...>

Without a flag, each sample's time goes to the deepest strims crate on its
stack (else crypto, the wasm-bindgen glue or the browser call it was in),
as a share of a core averaged over the profiles. `--crate` splits one
crate's share by its deepest function and the leaf below it. `--match`
gives the inclusive share of the functions whose names match the regex.
Wasm frames have names only in a bundle built with
CARGO_PROFILE_RELEASE_WASM_STRIP=false.
"""

import argparse
import collections
import json
import re

STRIMS = re.compile(r"\bstrims_[a-z0-9_]+\[")
CRYPTO = re.compile(r"\b(blake3|sha2|ed25519_dalek|curve25519_dalek)\[")
SPECIAL = ("(idle)", "(program)", "(garbage collector)")


def load(path):
    p = json.load(open(path))
    nodes = {n["id"]: n for n in p["nodes"]}
    parent = {}
    for n in p["nodes"]:
        for c in n.get("children", []):
            parent[c] = n["id"]
    samples = list(zip(p["samples"][1:], p["timeDeltas"][1:]))
    return nodes, parent, samples


def name_of(nodes, x):
    return nodes[x]["callFrame"].get("functionName") or ""


def short(name):
    return re.sub(r"\[[0-9a-f]+\]", "", name)


def stack(nodes, parent, leaf):
    x = leaf
    while x is not None:
        yield x
        x = parent.get(x)


def bucket(nodes, parent, leaf):
    """The deepest strims crate on the stack, else crypto, glue or the
    browser call."""
    fallback = None
    for x in stack(nodes, parent, leaf):
        name = name_of(nodes, x)
        if name in SPECIAL:
            return name
        m = STRIMS.search(name)
        if m:
            return m.group(0)[:-1]
        if CRYPTO.search(name):
            return "crypto"
        if fallback is None and name.startswith("__wbg_"):
            fallback = "js glue: " + re.sub(r"_[0-9a-f]{8,}$", "", name[6:])
        if fallback is None and name and not nodes[x]["callFrame"].get("url"):
            fallback = "browser: " + name
    return fallback or "other"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--crate")
    ap.add_argument("--match")
    ap.add_argument("--top", type=int, default=25)
    ap.add_argument("files", nargs="+")
    a = ap.parse_args()
    agg = collections.Counter()
    busy = []
    rx = re.compile(a.match) if a.match else None
    for f in a.files:
        nodes, parent, samples = load(f)
        wall = sum(dt for _, dt in samples) or 1
        idle = 0
        for leaf, dt in samples:
            if name_of(nodes, leaf) == "(idle)":
                idle += dt
                continue
            if rx:
                if any(rx.search(name_of(nodes, x)) for x in stack(nodes, parent, leaf)):
                    agg["matched"] += dt / wall
            elif a.crate:
                for x in stack(nodes, parent, leaf):
                    m = STRIMS.search(name_of(nodes, x))
                    if m:
                        if m.group(0)[:-1] == a.crate:
                            below = "self" if x == leaf else short(name_of(nodes, leaf))[:50]
                            agg[(short(name_of(nodes, x))[:100], below)] += dt / wall
                        break
            else:
                agg[bucket(nodes, parent, leaf)] += dt / wall
        busy.append(1 - idle / wall)
    n = len(a.files)
    print(f"{n} profiles; thread busy {100 * sum(busy) / n:.1f} % of a core "
          f"(least {100 * min(busy):.1f}, most {100 * max(busy):.1f})")
    for k, v in sorted(agg.items(), key=lambda x: -x[1])[: a.top]:
        label = f"{k[0]}  <- {k[1]}" if isinstance(k, tuple) else k
        print(f"  {100 * v / n:6.2f} %  {label}")


if __name__ == "__main__":
    main()

# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The goldens' files: named outputs, each `##### NAME` then its text, in
one gzipped file (no timestamp, so the same outputs make the same bytes),
and the diff `check` prints of those that differ."""

import difflib
import gzip
import sys
from pathlib import Path

TESTDATA = Path(__file__).with_name("testdata") / "golden"


def render(got):
    return "".join(f"##### {name}\n{text}" for name, text in got.items())


def parse(text):
    out, name = {}, None
    for line in text.splitlines(keepends=True):
        if line.startswith("##### "):
            name = line[6:].strip()
            out[name] = ""
        elif name is not None:
            out[name] += line
    return out


def read(path):
    """{name: text} of the golden at `path`."""
    return parse(gzip.decompress(path.read_bytes()).decode())


def write(path, got):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(gzip.compress(render(got).encode(), compresslevel=9, mtime=0))


def compare(label, want, got, out=sys.stdout):
    """Whether `got` is `want`, output by output; prints a unified diff of
    each that isn't, and a line of the count."""
    bad = [n for n in sorted(set(want) | set(got)) if want.get(n) != got.get(n)]
    for n in bad:
        out.writelines(difflib.unified_diff((want.get(n) or "").splitlines(keepends=True),
                                            (got.get(n) or "").splitlines(keepends=True),
                                            f"golden/{n}", f"now/{n}"))
    print(f"{label}: {len(got)} outputs, " + (f"{len(bad)} differ: {', '.join(bad)}" if bad else "all the same"),
          file=out)
    return not bad

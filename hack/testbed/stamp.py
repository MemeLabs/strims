#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The testbed's build stamps: which sources a testbed_node binary and a
browser bundle (dist/wasm-testbed) were built from, so run.sh can refuse a
bundle that may not speak the node's protocol (built from other sources,
its links come up and no stream flows).

    stamp.py sources              the checkout's sources hash
    stamp.py write ARTIFACT ...   writes ARTIFACT.stamp beside each
    stamp.py check NODE WEB       exit 1, saying why and how to rebuild,
                                  unless both carry one sources hash

The sources hash covers what reaches the wire between the node and the
page: every crate of this checkout in testbed_node's dependency closure
(strims-sim's, its dev-dependencies with it) or the page's (strims-wasm
with `e2e`, for wasm32), as `cargo metadata` resolves them, both transport
ends (strims-node, strims-wasm) with them, but the simulator's own crates
(SIMULATOR: strims-sim, strims-wan, turmoil), which testbed_node carries
and no message crosses; each crate's directory whole but its tests/ and
benches/ (its manifest, sources and build script, and what the build
script reads there: strims-proto's proto/); testbed_node's own source
(strims-sim's examples/testbed_node.rs); the page's TypeScript, the e2e
entry (src/web/e2e.ts) and what it imports of this checkout, src/web/
wasm.ts among them, which opens the page's data channel as
strims-webrtc's DATA_CHANNEL_INIT does on the node; and schema/, which
strims-proto's build script reads, the root Cargo.toml and Cargo.lock
(which pins every other crate). Each file goes in in path order as its
path from the checkout, a NUL, its length (8 bytes, little-endian) and
its bytes, under SHA-256.

ARTIFACT.stamp is one JSON object: `sources`, that hash when it was
written, and `sha256`, the artifact's own (a file's bytes; a directory's
files as above). `check` takes an artifact whose stamp is missing, or whose
`sha256` isn't the artifact's (rebuilt since), as unstamped. It needs no
cargo, only the stamps and the artifacts, so it runs where the artifacts
were shipped (a farm host's bundle).
"""
import hashlib
import json
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
SKIP = {"tests", "benches", "target", "node_modules"}
SIMULATOR = {"strims-sim", "strims-wan", "turmoil"}
BUILD = {
    "node": "cargo build --release -p strims-sim --example testbed_node"
            " && hack/testbed/stamp.py write target/release/examples/testbed_node",
    "web": "pnpm exec webpack --mode production --env TARGET=wasm-testbed"
           " && hack/testbed/stamp.py write dist/wasm-testbed",
}


def files_under(path):
    """Every file under `path` (or `path` itself), skipping SKIP's
    directories and hidden ones."""
    if os.path.isfile(path):
        yield path
        return
    for d, dirs, names in os.walk(path):
        dirs[:] = sorted(x for x in dirs if x not in SKIP and not x.startswith("."))
        for n in sorted(names):
            yield os.path.join(d, n)


def digest(paths, root):
    """SHA-256 over `paths` (files) by their path from `root`, in order."""
    h = hashlib.sha256()
    for name, p in sorted((os.path.relpath(p, root).replace(os.sep, "/"), p) for p in paths):
        with open(p, "rb") as f:
            data = f.read()
        h.update(name.encode())
        h.update(b"\0")
        h.update(len(data).to_bytes(8, "little"))
        h.update(data)
    return h.hexdigest()


def closure(root, platform, package, features=(), dev=False):
    """The local crates (name: manifest dir) `package` builds with, on
    `platform`: its dependencies of every kind but dev ones (with them for
    `dev`: an example's), and theirs but dev ones, as `cargo metadata`
    resolves them."""
    cmd = ["cargo", "metadata", "--format-version", "1", "--locked", "--offline",
           "--filter-platform", platform]
    if features:
        cmd += ["--features", ",".join(features)]
    meta = json.loads(subprocess.run(cmd, cwd=root, capture_output=True, text=True, check=True).stdout)
    pkgs = {p["id"]: p for p in meta["packages"]}
    nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
    start = next(i for i, p in pkgs.items() if p["name"] == package and p["source"] is None)
    seen, todo = set(), [(start, dev)]
    while todo:
        pid, with_dev = todo.pop()
        if pid in seen:
            continue
        seen.add(pid)
        for d in nodes[pid]["deps"]:
            kinds = {k["kind"] for k in d["dep_kinds"]}
            if kinds - {"dev"} or (with_dev and "dev" in kinds):
                todo.append((d["pkg"], False))
    return {pkgs[p]["name"]: os.path.dirname(pkgs[p]["manifest_path"])
            for p in seen if pkgs[p]["source"] is None}


IMPORT = re.compile(r"""(?:\bfrom\s*|\bimport\s*\(?\s*)["'](\.{1,2}/[^"']+)["']""")


def imported(entry):
    """`entry` and the files of this checkout it imports, transitively:
    relative specifiers, as a .ts, .tsx or .js file or a directory's
    index."""
    seen, todo = set(), [os.path.abspath(entry)]
    while todo:
        f = todo.pop()
        if f in seen:
            continue
        seen.add(f)
        with open(f, encoding="utf-8") as fh:
            text = fh.read()
        for spec in IMPORT.findall(text):
            base = os.path.normpath(os.path.join(os.path.dirname(f), spec))
            for c in (base, *(base + e for e in (".ts", ".tsx", ".js")),
                      *(os.path.join(base, "index" + e) for e in (".ts", ".tsx", ".js"))):
                if os.path.isfile(c):
                    todo.append(c)
                    break
    return sorted(seen)


def sources(root=ROOT):
    """The checkout's sources hash (the module's doc)."""
    host = subprocess.run(["rustc", "-vV"], capture_output=True, text=True, check=True).stdout
    triple = next(line.split()[1] for line in host.splitlines() if line.startswith("host:"))
    node = closure(root, triple, "strims-sim", dev=True)
    page = closure(root, "wasm32-unknown-unknown", "strims-wasm", features=("strims-wasm/e2e",))
    crates = {**node, **page}
    paths = [f for name, d in sorted(crates.items()) if name not in SIMULATOR for f in files_under(d)]
    paths += imported(os.path.join(root, "src", "web", "e2e.ts"))
    paths.append(os.path.join(node["strims-sim"], "examples", "testbed_node.rs"))
    paths += list(files_under(os.path.join(root, "schema")))
    paths += [os.path.join(root, "Cargo.toml"), os.path.join(root, "Cargo.lock")]
    return digest(paths, root)


def artifact_sha256(path):
    if os.path.isdir(path):
        return digest(list(files_under(path)), path)
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


def stamp_path(path):
    return os.path.normpath(path) + ".stamp"


def write(paths, root=ROOT):
    src = sources(root)
    for p in paths:
        with open(stamp_path(p), "w") as f:
            json.dump({"sources": src, "sha256": artifact_sha256(p)}, f)
            f.write("\n")
        print(f"{stamp_path(p)}: sources {src[:12]}")


def read(path):
    """The sources hash `path`'s stamp holds, or why it holds none."""
    sp = stamp_path(path)
    try:
        with open(sp) as f:
            st = json.load(f)
    except FileNotFoundError:
        return None, f"has no stamp ({sp})"
    except (OSError, ValueError) as e:
        return None, f"has an unreadable stamp ({sp}: {e})"
    if st.get("sha256") != artifact_sha256(path):
        return None, f"isn't the build its stamp is of ({sp}): it was rebuilt or changed since"
    return st.get("sources"), None


def check(node, web):
    """None when `node` and `web` carry one sources hash, else why not."""
    ns, nwhy = read(node)
    ws, wwhy = read(web)
    if nwhy:
        return f"the testbed_node at {node} {nwhy}; rebuild it: {BUILD['node']}"
    if wwhy:
        return f"the web bundle at {web} {wwhy}; rebuild it: {BUILD['web']}"
    if ns != ws:
        return (f"the web bundle at {web} was built from other sources than the testbed_node at {node}"
                f" (stamps {ws[:12]} and {ns[:12]}), so it may not speak the node's protocol; rebuild"
                f" both from one checkout: {BUILD['node']}; {BUILD['web']}")
    return None


def main():
    args = sys.argv[1:]
    if args[:1] == ["sources"]:
        print(sources())
    elif args[:1] == ["write"] and len(args) > 1:
        write(args[1:])
    elif args[:1] == ["check"] and len(args) == 3:
        why = check(args[1], args[2])
        if why:
            print(why, file=sys.stderr)
            return 1
    else:
        print(__doc__.split("\n\n")[1], file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())

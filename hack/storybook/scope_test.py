#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""check.py --compare's scope over a storybook in miniature
(testdata/scope), apart from the app's own: each test copies the fixture,
changes one thing, builds the copy with webpack and the manifest plugin
(webpack/lib/StorybookManifestPlugin.js), and holds the stories its
manifest and the unchanged build's leave to capture to the ones the change
reaches. No browser runs.

    python3 hack/storybook/scope_test.py

needs the checkout's node_modules (pnpm install --frozen-lockfile), and
the plugin compiled (tsc -p webpack)."""

import json
import os
import shutil
import struct
import subprocess
import sys
import tempfile
import unittest
import zlib
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
FIXTURE = HERE / "testdata" / "scope"
sys.path.insert(0, str(HERE))
import check  # noqa: E402

STORIES = {"Both", "Counter", "Lazy", "Leaf", "Picture", "Shared"}


def png(rgb):
    """A 1x1 PNG of the colour `rgb`."""
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 1, 1, 8, 2, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(b"\x00" + bytes(rgb))) + chunk(b"IEND", b""))


def edit(rel, old, new):
    """A change: `old` in the copy's `rel` (once) becomes `new`."""
    def change(src):
        p = src / rel
        text = p.read_text()
        assert text.count(old) == 1, (rel, old)
        p.write_text(text.replace(old, new))
    return change


def write(rel, data):
    """A change: the copy's `rel` holds `data` (bytes or text)."""
    def change(src):
        p = src / rel
        p.write_bytes(data) if isinstance(data, bytes) else p.write_text(data)
    return change


class Scope(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = Path(tempfile.mkdtemp(prefix="scope-test-"))
        cls.built = 0
        cls.base = cls.build("base")

    @classmethod
    def tearDownClass(cls):
        shutil.rmtree(cls.tmp, ignore_errors=True)

    @classmethod
    def build(cls, name, change=None, commit="a" * 40, **env):
        """The fixture, copied to <tmp>/<name> (each copy a sibling, so its
        path to the checkout's node_modules is the same) and `change`d:
        (its manifest, its dist directory). Each build has a build time
        of its own."""
        src = cls.tmp / name
        shutil.copytree(FIXTURE, src)
        (src / "src" / "components" / "pic.png").write_bytes(png((255, 0, 0)))
        if change:
            change(src)
        cls.built += 1
        args = {"COMMIT": commit, "TIME": str(1767225600000 + cls.built), **env}
        res = subprocess.run(
            [str(ROOT / "node_modules" / ".bin" / "webpack"), "--config", str(src / "webpack.config.js"),
             *(f"--env={k}={v}" for k, v in args.items())],
            cwd=src, capture_output=True, text=True,
            env={**os.environ, "NODE_PATH": str(ROOT / "node_modules"), "NODE_MODULES": str(ROOT / "node_modules"),
                 "MANIFEST_PLUGIN": str(ROOT / "webpack" / "lib" / "StorybookManifestPlugin.js")})
        if res.returncode:
            raise AssertionError(f"{name}: webpack failed:\n{res.stdout[-4000:]}{res.stderr[-2000:]}")
        manifest = json.loads((src / "dist" / check.MANIFEST).read_text())
        if "error" in manifest:
            raise AssertionError(f"{name}: the manifest says: {manifest['error']}")
        return manifest, src / "dist"

    def scope(self, head, unreachable=()):
        """(the stories both builds have that the scope captures, the ones
        only the head has), each story by its module, against the base."""
        def side(name, manifest):
            b = check.Build(name, f"http://127.0.0.1:1/{name}", name, "frames")
            b.manifest = manifest
            b.index = {"stale": [{"kind": "unreachable", "route": f"/storybook/{m}/Default"} for m in unreachable]}
            b.stories = {f"/storybook/{m}/Default": {"module": m, "name": "Default"} for m in manifest["stories"]}
            return b
        builds = (side("head", head[0]), side("base", self.base[0]))
        sc = check.scope(builds)
        self.assertIsNone(sc["why"])
        both = set(builds[0].stories) & set(builds[1].stories)
        module = lambda route: route.split("/")[2]  # noqa: E731
        return ({module(r) for r in both - sc["identical"]},
                {module(r) for r in set(builds[0].stories) - set(builds[1].stories)})

    def test_fixture(self):
        self.assertEqual(set(self.base[0]["stories"]), STORIES)

    def test_elsewhere(self):
        """The same tree at a longer path, of another commit, built later."""
        head = self.build("the-same-tree-at-a-longer-path", commit="b" * 40)
        self.assertEqual(self.scope(head), (set(), set()))

    def test_chunks_cut_otherwise(self):
        """splitChunks cutting the chunks otherwise (its minSize, as module
        sizes would) changes which files load, not what they hold."""
        head = self.build("min-size", MIN_SIZE="0")
        files = lambda b: {f.name for f in b[1].glob("*.js")}  # noqa: E731
        self.assertNotEqual(files(head), files(self.base))
        self.assertEqual(self.scope(head), (set(), set()))

    def test_leaf(self):
        head = self.build("leaf", edit("src/components/Leaf.tsx", "<b>leaf</b>", "<b>a leaf</b>"))
        self.assertEqual(self.scope(head), ({"Leaf", "Both"}, set()))

    def test_style_sheet(self):
        """A component's style sheet, which a lazily imported one imports too."""
        head = self.build("style", edit("src/components/Shared.scss", "teal", "navy"))
        self.assertEqual(self.scope(head), ({"Shared", "Both", "Lazy"}, set()))

    def test_asset(self):
        head = self.build("asset", write("src/components/pic.png", png((0, 0, 255))))
        self.assertEqual(self.scope(head), ({"Picture"}, set()))

    def test_lazy_import(self):
        head = self.build("deep", edit("src/components/Deep.tsx", "deep", "deeper"))
        self.assertEqual(self.scope(head), ({"Lazy"}, set()))

    def test_worker(self):
        head = self.build("worker", edit("src/components/counter.worker.ts", "postMessage(1)", "postMessage(2)"))
        self.assertEqual(self.scope(head), ({"Counter"}, set()))

    def test_story_added(self):
        """A stories module added changes no other story's inputs (the
        catalog's contexts list it)."""
        head = self.build("added", write("src/stories/New.stories.tsx",
                                         'import React from "react";\n\nimport Leaf from "../components/Leaf";\n\n'
                                         "const Story = () => <Leaf />;\n\nexport default Story;\n"))
        self.assertEqual(self.scope(head), (set(), {"New"}))

    def test_app(self):
        """The app, which no frame loads."""
        head = self.build("app", edit("src/app/App.tsx", "the app", "the app, changed"))
        self.assertEqual(self.scope(head), (set(), set()))

    def test_shared(self):
        """What every frame loads: the frame, the entry's style sheet, a
        file no chunk owns."""
        for name, change in (("frame", edit("src/host/Frame.tsx", "createRoot(document.body)",
                                            "createRoot(document.documentElement)")),
                             ("global-style", edit("src/main.scss", "margin: 0", "margin: 1px")),
                             ("data", edit("data/strings.json", '"Hello"', '"Hi"'))):
            with self.subTest(name):
                self.assertEqual(self.scope(self.build(name, change)), (STORIES, set()))

    def test_unreachable(self):
        """A story an index finds its route leads elsewhere is captured."""
        self.assertEqual(self.scope(self.base, unreachable=["Leaf"]), ({"Leaf"}, set()))

    def test_files(self):
        """The files each story may load, as the request audit reads them:
        its image, its worker's chunk, and what every frame loads."""
        files = self.base[0]["files"]
        self.assertTrue(any(f.endswith(".png") for f in files["stories"]["Picture"]))
        self.assertFalse(any(f.endswith(".png") for f in files["stories"]["Leaf"] + files["shared"]))
        worker = set(files["stories"]["Counter"]) - set(files["stories"]["Leaf"]) - set(files["shared"])
        self.assertTrue(worker)
        self.assertTrue({"devtools.html", "locales/strings.json"} <= set(files["shared"]))
        self.assertFalse(any(f.endswith(".map") for fs in (files["shared"], *files["stories"].values()) for f in fs))


if __name__ == "__main__":
    unittest.main()

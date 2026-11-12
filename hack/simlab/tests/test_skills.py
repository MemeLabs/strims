#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The project's skills name only what exists, stdlib alone. Every skill's
SKILL.md and references/*.md (.claude/skills/*/), in their inline code
and code blocks:

  Front       the frontmatter's name is the skill's directory, and it has
              a description
  Paths       a repo path exists (a skill's own references/ and evals/
              under its directory); a placeholder in one (NAME, <name>,
              {name}, *) makes it a glob that must match; a source file
              named alone (`tables.rs`) is one beside the skill's file or
              a tracked file's name; paths under target/ are left out
              (they come and go)
  Modules     `module.NAME` (a module of hack/ by its file's name,
              `simlab.` optional) is defined at the top of a file of that
              name, `module.Class.NAME` in the class (ast, no imports)
  Rust        `Type::item` names an item defined in a file under crates/
              that defines or implements Type, `module::item` one in a
              crate with that module, and `crate::...` a workspace crate
  Names       an identifier alone (`held_ms`, `FORECAST_CAP`, `SIM_SPEC`) is a
              word of the sources (crates/, hack/, src/); with a
              placeholder, some word matches it
  Commands    `X.py SUB` and its `--flag`s are string literals of the
              scripts named X.py that run as __main__, or of the files
              beside them; `npm run S` is a package.json script; `cargo`'s
              `-p CRATE` a workspace crate and `--test T` a crate's
              tests/T.rs
  Commits     every commit cited (7 to 40 hex digits, a digit among them)
              is HEAD or an ancestor of it
  Pending     PENDING's entries (what a skill names before it lands, with
              the branch building it) are skipped by the checks above and
              must still be missing: once one lands this fails, and its
              entry goes
  Checks      each check, over text made for it, finds what's missing and
              passes what's there

  python3 hack/simlab/tests/test_skills.py
"""

import ast
import functools
import json
import re
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SKILLS = ROOT / ".claude" / "skills"
# What a skill names before it is on main: a path, or `SCRIPT SUB`, with
# the branch that adds it.
PENDING: dict[str, str] = {}
SOURCE_EXTS = (".rs", ".py", ".sql", ".ts", ".tsx", ".toml", ".sh", ".js", ".scss")
# A name alone with one of these is a file, never module.NAME.
FILE_EXTS = {"py", "rs", "md", "sql", "toml", "json", "ts", "tsx", "scss", "csv", "zst", "gz", "txt", "sh", "html",
             "jsonl", "parquet", "yaml", "js", "lock", "vl"}
SQL_TYPES = {"INT", "INTEGER", "BIGINT", "HUGEINT", "UBIGINT", "DOUBLE", "FLOAT", "VARCHAR", "TEXT", "BOOLEAN",
             "DATE", "TIMESTAMPTZ", "TIMESTAMP"}
# A placeholder: <name>, {name}, *, an ellipsis; in a path, NAME too.
PLACEHOLDER = re.compile(r"<[^<>/]*>|\{[^{}/]*\}|\*|…|\.\.\.")
UPPER = re.compile(r"\b[A-Z][A-Z0-9_]+\b")
IDENT = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
INLINE = re.compile(r"``\s?(.+?)\s?``|`([^`\n]+)`")
COMMIT = re.compile(r"(?<![0-9a-zA-Z_#/.-])([0-9a-f]{7,40})(?![0-9a-zA-Z_])")
SCRIPT = re.compile(r"((?:[\w.-]+/)*)([a-z_][a-z0-9_-]*\.py)\b([^|;&]*)")
PY_REF = re.compile(r"(?<![\w./-])(?:simlab\.)?([a-z_][a-z0-9_]*)\.([A-Za-z_*<{][\w*<>{}]*)(?:\.([A-Za-z_]\w*))?")
# A Rust path; `::<` is a turbofish, which ends one.
RUST_PATH = re.compile(r"(?<![\w:])([A-Za-z_]\w*(?:::[A-Za-z_*{][\w*<>{}]*)+)")


def git(*args):
    return subprocess.run(["git", "-C", str(ROOT), *args], capture_output=True, text=True, check=True).stdout


@functools.cache
def tracked():
    return [p for p in git("ls-files").splitlines() if (ROOT / p).is_file()]


@functools.cache
def text_of(rel):
    try:
        return (ROOT / rel).read_text(errors="replace")
    except OSError:
        return ""


@functools.cache
def words():
    """Every identifier of the sources."""
    out = set()
    for p in tracked():
        if p.split("/", 1)[0] in ("crates", "hack", "src") and p.endswith(SOURCE_EXTS):
            out.update(IDENT.findall(text_of(p)))
    return out


@functools.cache
def rust_files():
    return {p: text_of(p) for p in tracked() if p.startswith("crates/") and p.endswith(".rs")}


@functools.cache
def crates():
    """The workspace's crates by name, as `-p` and paths (`strims_wan`) give them."""
    out = {}
    for p in tracked():
        if re.fullmatch(r"crates/[^/]+/Cargo\.toml", p):
            m = re.search(r'^\[package\][^\[]*?^name\s*=\s*"([^"]+)"', text_of(p), re.M | re.S)
            if m:
                out[m.group(1)] = p.rsplit("/", 1)[0]
    return out


def glob_of(s, upper=False):
    """`s` with its placeholders made `*` (upper-case words too with
    `upper`), or None without any."""
    g = PLACEHOLDER.sub("*", s)
    g = UPPER.sub("*", g) if upper else g
    return g if g != s else None


def matcher(glob):
    """A regex of identifiers glob `glob` matches."""
    return re.escape(glob).replace(r"\*", r"\w*")


# --- reading the skills ---------------------------------------------------------------------------

def skill_files():
    return sorted([*SKILLS.glob("*/SKILL.md"), *SKILLS.glob("*/references/*.md")])


def items(path):
    """A skill file's code: (line, text, kind), kind "inline" for a span,
    "block" for a code block's line (its `# ...` comment left out)."""
    out, fenced = [], False
    for i, line in enumerate(path.read_text().splitlines(), 1):
        if line.lstrip().startswith("```"):
            fenced = not fenced
            continue
        if fenced or (line.startswith("    ") and "`" not in line and not line.lstrip().startswith(("-", "|"))):
            out.append((i, re.split(r"\s#\s", line, maxsplit=1)[0].strip(), "block"))
            continue
        for m in INLINE.finditer(line):
            out.append((i, (m.group(1) or m.group(2)).strip(), "inline"))
    return out


def every_item():
    for path in skill_files():
        for line, text, kind in items(path):
            yield path, f"{path.relative_to(ROOT)}:{line}", text, kind


# --- the checks -----------------------------------------------------------------------------------

def tokens(text):
    return [t.strip(".,;:()[]'\"") for t in re.split(r"[\s,()'\"|]+", text) if t.strip(".,;:()[]'\"")]


def is_file(token):
    """Whether `token` names a file (`vocab.test.ts`), never module.NAME."""
    return "/" in token or token.rsplit(".", 1)[-1] in FILE_EXTS


def path_problem(token, skill_dir, here=None):
    """Why repo path `token` doesn't hold, or None (also for no path): a
    name alone is a file beside the skill's file `here` names, or a
    tracked file's."""
    token = re.sub(r":\d+(-\d+)?$", "", token.split("#", 1)[0])
    first = token.split("/", 1)[0]
    if "/" in token and first in ("references", "evals"):
        base = skill_dir
    elif "/" in token and first not in ("target", "") and (ROOT / first).exists():
        base = ROOT
    elif "/" not in token and token.endswith(SOURCE_EXTS + (".md",)) and IDENT.match(token):
        if (glob_of(token, upper=True) is None and not (here and (here / token).exists())
                and not any(p.rsplit("/", 1)[-1] == token for p in tracked())):
            return f"no tracked file is named {token}"
        return None
    else:
        return None
    if token in PENDING or (base / token).exists():
        return None
    g = glob_of(token, upper=True)
    if g and any(base.glob(g)):
        return None
    return f"{token} doesn't exist" + (f" (nor does any {g})" if g else "")


@functools.cache
def py_modules():
    """hack/'s modules by name: their files."""
    out = {}
    for p in tracked():
        if p.startswith("hack/") and p.endswith(".py"):
            out.setdefault(p.rsplit("/", 1)[-1][:-3], []).append(p)
    return out


@functools.cache
def py_names(rel):
    """A module's top-level names, {name: the class's own names or None}."""
    try:
        tree = ast.parse(text_of(rel))
    except SyntaxError:
        return {}
    out = {}

    def targets(t):
        if isinstance(t, ast.Name):
            yield t.id
        elif isinstance(t, (ast.Tuple, ast.List)):
            for e in t.elts:
                yield from targets(e)

    def walk(body, into):
        for node in body:
            if isinstance(node, ast.ClassDef):
                inner = {}
                walk(node.body, inner)
                into[node.name] = inner
            elif isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                into[node.name] = None
            elif isinstance(node, ast.Assign):
                for t in node.targets:
                    into.update(dict.fromkeys(targets(t)))
            elif isinstance(node, (ast.AnnAssign, ast.AugAssign)):
                into.update(dict.fromkeys(targets(node.target)))
            elif isinstance(node, (ast.Import, ast.ImportFrom)):
                into.update(dict.fromkeys((a.asname or a.name).split(".")[0] for a in node.names))
            elif isinstance(node, (ast.If, ast.Try, ast.With, ast.For)):
                for part in ("body", "orelse", "finalbody"):
                    walk(getattr(node, part, []), into)
                for h in getattr(node, "handlers", []):
                    walk(h.body, into)

    walk(tree.body, out)
    return out


def py_problem(module, name, attr):
    if module == "simlab":
        return None if name in py_modules() else f"simlab has no module {name}"
    files = py_modules().get(module)
    if not files or name in FILE_EXTS:
        return None
    for rel in files:
        names = py_names(rel)
        hit = [n for n in names if re.fullmatch(matcher(glob_of(name) or name), n)]
        if hit and (attr is None or any(attr in (names[n] or {}) for n in hit)):
            return None
    return f"{module}.{name}{'.' + attr if attr else ''} isn't defined in {', '.join(files)}"


def rust_defines(pattern, text):
    """Whether `text` defines an item named by glob `pattern`: a fn, type,
    const, static, module or macro, an enum variant or a field."""
    n = f"(?:{matcher(pattern)})"
    return bool(re.search(rf"\b(?:fn|struct|enum|trait|type|const|static|mod|macro_rules!)\s+{n}\b", text)
                or re.search(rf"^\s*(?:pub(?:\([^)]*\))?\s+)?{n}\s*(?:[,({{:=]|$)", text, re.M))


def rust_problem(path):
    segs = path.split("::")
    if segs[-1] in SQL_TYPES:
        return None
    files = rust_files()
    if segs[0] in ("crate", "self", "super", "std", "core") or len(segs) < 2:
        return None
    scope = files
    for i, seg in enumerate(segs[:-1]):
        item = segs[i + 1]
        if seg.replace("_", "-") in crates():
            root = crates()[seg.replace("_", "-")] + "/"
            scope = {p: t for p, t in files.items() if p.startswith(root)}
            continue
        if seg[:1].isupper():
            owner = re.compile(rf"\b(?:struct|enum|trait|type)\s+{seg}\b|\bimpl\b[^{{;]*?\b{seg}\b(?:<[^{{]*>)?\s*(?:where[^{{]*)?\{{")
            if not any(owner.search(t) and rust_defines(glob_of(item) or item, t) for t in scope.values()):
                return f"{path}: no file under crates/ defines {item} for {seg}"
            return None
        mods = {p: t for p, t in scope.items()
                if p.endswith((f"/{seg}.rs", f"/{seg}/mod.rs")) or re.search(rf"\bmod\s+{seg}\b", t)}
        if not mods:
            return f"{path}: no module {seg} under crates/"
        scope = {p: t for p, t in scope.items() if p.endswith((f"/{seg}.rs", f"/{seg}/mod.rs"))} or scope
    last = glob_of(segs[-1]) or segs[-1]
    if not any(rust_defines(last, t) for t in scope.values()):
        return f"{path}: {segs[-1]} isn't defined under crates/"
    return None


@functools.cache
def cli_texts(script, prefix):
    """The text of the scripts named `script` (under `prefix` when given)
    that run as __main__, and of the .py files beside each but tests."""
    mains = [p for p in tracked() if p.rsplit("/", 1)[-1] == script and p.startswith(prefix)
             and "__main__" in text_of(p)]
    dirs = {p.rsplit("/", 1)[0] for p in mains}
    beside = [p for p in tracked() if p.endswith(".py") and p.rsplit("/", 1)[0] in dirs
              and not re.match(r"(test_.*|.*_test)\.py$", p.rsplit("/", 1)[-1])]
    return "\n".join(text_of(p) for p in {*mains, *beside})


def literal(word, text):
    return f'"{word}"' in text or f"'{word}'" in text


def command_problems(text):
    out = []
    for m in SCRIPT.finditer(text):
        prefix, script, rest = m.group(1).lstrip("./"), m.group(2), m.group(3)
        if (prefix + script) in PENDING:
            continue
        src = cli_texts(script, prefix)
        if not src:
            if prefix or script not in {p.rsplit("/", 1)[-1] for p in tracked()}:
                continue
            out.append(f"{script} runs as no script")
            continue
        args = rest.split()
        sub = args[0] if args and re.fullmatch(r"[a-z][a-z0-9-]*", args[0]) else None
        if sub and f"{script} {sub}" in PENDING:
            continue
        if sub and not literal(sub, src):
            out.append(f"{script} has no command {sub}")
        for a in args:
            flag = a.split("=", 1)[0].strip(".,;:)]'\"")
            if re.fullmatch(r"--[a-z][a-z0-9-]*", flag) and flag != "--help" and not literal(flag, src):
                out.append(f"{script} has no flag {flag}")
    scripts = json.loads(text_of("package.json") or "{}").get("scripts", {})
    for m in re.finditer(r"\bnpm run ([\w:.-]+)", text):
        if m.group(1) not in scripts:
            out.append(f"package.json has no script {m.group(1)}")
    if re.search(r"\bcargo\b", text):
        for m in re.finditer(r"(?:-p|--package)\s+([\w-]+)", text):
            if m.group(1) not in crates():
                out.append(f"no workspace crate {m.group(1)}")
        for m in re.finditer(r"--test\s+([\w-]+)", text):
            if not any(re.fullmatch(rf"crates/[^/]+/tests/{m.group(1)}\.rs", p) for p in tracked()):
                out.append(f"no crate has tests/{m.group(1)}.rs")
    return out


@functools.cache
def history():
    return git("rev-list", "HEAD").split()


# --- the tests ------------------------------------------------------------------------------------

class Skills(unittest.TestCase):
    def failures(self, check):
        out = []
        for path, where, text, kind in every_item():
            for why in check(path, text, kind):
                out.append(f"{where}: {why}")
        return out

    def test_front(self):
        bad = []
        for path in sorted(SKILLS.glob("*/SKILL.md")):
            m = re.match(r"\A---\n(.*?)\n---\n", path.read_text(), re.S)
            front = dict(re.findall(r"^(\w+):\s*(.*)$", m.group(1), re.M)) if m else {}
            if front.get("name") != path.parent.name:
                bad.append(f"{path.relative_to(ROOT)}: name {front.get('name')!r}, not {path.parent.name!r}")
            if not front.get("description", "").strip():
                bad.append(f"{path.relative_to(ROOT)}: no description")
        self.assertEqual(bad, [])

    def test_paths(self):
        def check(path, text, kind):
            skill_dir = path.parent if path.name == "SKILL.md" else path.parent.parent
            return [p for t in tokens(text) if (p := path_problem(t, skill_dir, path.parent))]
        self.assertEqual(self.failures(check), [])

    def test_modules(self):
        def check(path, text, kind):
            return [p for t in tokens(text) if not is_file(t)
                    for m in PY_REF.finditer(t) if (p := py_problem(*m.groups()))]
        self.assertEqual(self.failures(check), [])

    def test_rust(self):
        def check(path, text, kind):
            return [p for m in RUST_PATH.finditer(text) if (p := rust_problem(m.group(1)))]
        self.assertEqual(self.failures(check), [])

    def test_names(self):
        def check(path, text, kind):
            if kind != "inline" or not re.fullmatch(r"[A-Za-z_<{*][\w<>{}*]*", text) or text in FILE_EXTS:
                return []
            g = glob_of(text)
            if g is None:
                return [] if text in words() else [f"{text} is no word of the sources"]
            if g.strip("*_") == "":
                return []
            m = re.compile(matcher(g))
            return [] if any(m.fullmatch(w) for w in words()) else [f"no word of the sources is {g}"]
        self.assertEqual(self.failures(check), [])

    def test_commands(self):
        self.assertEqual(self.failures(lambda path, text, kind: command_problems(text)), [])

    def test_commits(self):
        bad = []
        for path in skill_files():
            for i, line in enumerate(path.read_text().splitlines(), 1):
                for m in COMMIT.finditer(line):
                    c = m.group(1)
                    if re.search(r"\d", c) and not any(h.startswith(c) for h in history()):
                        bad.append(f"{path.relative_to(ROOT)}:{i}: {c} is no commit of HEAD's history")
        self.assertEqual(bad, [])

    def test_pending(self):
        for key, branch in PENDING.items():
            with self.subTest(key=key):
                if " " in key:
                    script, sub = key.split()
                    landed = literal(sub, cli_texts(script, ""))
                else:
                    landed = (ROOT / key).exists()
                self.assertFalse(landed, f"{key} has landed ({branch}): remove it from test_skills' PENDING")


class Checks(unittest.TestCase):
    """Each check finds what's missing, and passes what's there."""

    def test_paths(self):
        skill = SKILLS / "sim-lab"
        for good in ("hack/simlab/catalog.py", "hack/console/views/NAME.sql", "hack/simlab/tests/test_<name>.py",
                     "crates/strims-sim/src/tables.rs:13", "tables.rs", "target/anything", "SKILL.md"):
            self.assertIsNone(path_problem(good, skill), good)
        for bad in ("hack/simlab/nope.py", "hack/nope/NAME.sql", "nope.rs", "references/nope.md"):
            self.assertIsNotNone(path_problem(bad, skill), bad)

    def test_modules(self):
        for good in ("store_host.TABLES", "simlab.engine", "query.<t>_view", "forecast.Selection",
                     "golden.SETS", "simdb.py"):
            m = PY_REF.match(good)
            self.assertIsNone(m and py_problem(*m.groups()), good)
        for bad in ("store_host.NOPE", "simlab.nope", "query.nope_<t>"):
            self.assertIsNotNone(py_problem(*PY_REF.match(bad).groups()), bad)

    def test_rust(self):
        for good in ("Runner::take_held", "Sim::keep_*", "strims_wan::vocab::Kind", "Table::new", "v.collapsed::INT"):
            self.assertIsNone(rust_problem(RUST_PATH.search(good).group(1)), good)
        for bad in ("Runner::nope", "strims_wan::vocab::Nope", "strims_wan::nope::Kind"):
            self.assertIsNotNone(rust_problem(bad), bad)

    def test_commands(self):
        for good in ("python3 hack/sim-db/simdb.py describe NAME --json", "vocab.py write", "golden.py check --skip x",
                     "python3 hack/simlab/tests/test_views.py --write-panels", "npm run test:js",
                     "cargo test -p strims-wan --test vocab", "suite.py neutral --base REF --expect t.c"):
            self.assertEqual(command_problems(good), [], good)
        for bad in ("simdb.py nope", "simdb.py lake --nope", "npm run nope", "cargo test -p strims-nope",
                    "cargo test --test nope"):
            self.assertNotEqual(command_problems(bad), [], bad)


if __name__ == "__main__":
    unittest.main()

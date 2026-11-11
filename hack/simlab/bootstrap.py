# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""Runs a script under hack/'s uv venv when the Python running it lacks
the duckdb module: the system python3 has no pip, and hack/pyproject.toml
pins the module (and hack/uv.lock its build).

A script calls it from its `__main__` block, never at import, so a module
that loads the script by path (simlab.simdb(), golden.py's
reference suite.py) never runs it:

  if __name__ == "__main__":
      sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
      from simlab import bootstrap
      bootstrap.engine(__file__)
      main()

`engine` returns when this Python has duckdb. Else it execs `uv run --project
hack --locked --quiet python SCRIPT ARGS` (uv makes or syncs hack/.venv
first, from its cache), with SIMLAB_UV set so the script under uv, still
lacking the module, exits rather than exec again. uv is the one on PATH,
else ~/.local/bin/uv (its installer's place); without it the error says to
install it. The script runs from any directory, as `python3 SCRIPT` or
`uv run SCRIPT`, and its arguments, stdin and cwd carry over.
"""

import importlib.util
import os
import shutil
import sys
from pathlib import Path

HACK = Path(__file__).resolve().parents[1]
# Set in the script's environment once it has been exec'd under uv.
ENV = "SIMLAB_UV"
INSTALL = "curl -LsSf https://astral.sh/uv/install.sh | sh"


def uv():
    """uv's path: the one on PATH, else ~/.local/bin/uv; None without
    either."""
    found = shutil.which("uv")
    if found:
        return found
    local = Path.home() / ".local" / "bin" / "uv"
    return str(local) if os.access(local, os.X_OK) else None


def has_engine():
    """Whether this Python has the duckdb module."""
    return importlib.util.find_spec("duckdb") is not None


def command(script, argv):
    """The command that runs `script` with `argv` under hack/'s venv."""
    return [uv() or "uv", "run", "--project", str(HACK), "--locked", "--quiet", "python",
            str(Path(script).resolve()), *argv]


def engine(script, argv=None):
    """Returns when this Python has duckdb; else re-execs `script`
    (with `argv`, by default this process's arguments) under hack/'s uv
    venv, or exits saying why it can't."""
    if has_engine():
        return
    name = Path(script).name
    if os.environ.get(ENV):
        sys.exit(f"{name}: no duckdb module under {sys.executable}, though uv ran it: "
                 f"uv sync --project {HACK} --locked")
    if uv() is None:
        sys.exit(f"{name} needs uv, which runs it with the duckdb module (hack/pyproject.toml): "
                 f"install it ({INSTALL}), or run it as uv run --project {HACK} python {script}")
    args = command(script, sys.argv[1:] if argv is None else list(argv))
    sys.stdout.flush()
    sys.stderr.flush()
    os.environ[ENV] = "1"
    os.execv(args[0], args)

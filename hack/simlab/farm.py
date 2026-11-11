# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""farm.py's quotes and approved runs, for the console: a farm run of the
suite's matrices (suite.write_matrices') starts only once a person
approves its quote.

  quote(matrices, regenerations=None) -> Quote
                 `farm.py estimate --json MATRICES` (the regenerations'
                 farm.py, by default the main checkout's) as its own
                 process, under a fresh id: Run.estimate()'s dict (jobs,
                 have_rows, timed, work_slot_h, boot_s, instances, slots,
                 slots_by_region, model_wall_s, model_cost, past_runs,
                 wall_s, cost, s3_cost), the text it printed for people
                 (its stderr) and the error it exited with. An estimate
                 reads spot prices, zones and quotas, and launches
                 nothing. It is kept as a regeneration, `quoted` (or
                 `failed`): {id, state, created_at, commands, matrices,
                 quote (the text), estimate, warnings, error?}, which the
                 console's server saves again with its request (exp_id,
                 cells, arms, seeds) and the matrices' warnings.
  approve(id, regenerations=None) -> Approval
                 `farm.py run MATRICES` for a quoted regeneration, started
                 detached (a session of its own, which outlives this
                 process), its output in <id>.log: once, as the log is
                 made only once, across processes too. QuoteError, with
                 .status 404 for an unknown id, 409 for one not quoted
                 (approved already, or failed) and 500 for a run that
                 didn't start (failed).
  Regenerations(root=REGENERATE, farm_py=None, python="python3")
                 the regenerations, each <root>/<id>.json in the console's
                 Node server's shape (and the estimate beside its text),
                 once approved with its run's pid, log, approved_at, state
                 `running` and then `finished` with its exit code (null
                 for a signal).
                 get(id) reads it from disk with the log's last 60 lines
                 (log_tail), or None; save(record) writes one.

A matrix is its path, or a suite.Matrix (its .path). A run this process
started is waited on here and finished when it exits; one another
process started is finished (code null) once its pid has gone.

stdlib alone: farm.py runs as its own process, never in this one.
"""

import dataclasses
import datetime
import json
import os
import re
import secrets
import subprocess
import threading
from pathlib import Path

from . import ROOT, simdb
from .bootstrap import ENV as UV_ENV

REGENERATE = ROOT / "target" / "console" / "regenerate"
# An id as quote makes one (secrets.token_urlsafe(9), as the Node server's
# randomBytes(9) base64url), or any the routes pass: a file name.
ID = re.compile(r"[A-Za-z0-9_-]{1,64}")
LOG_TAIL = 60
# farm.py estimate reads each region's prices and quotas: a quote that
# hasn't answered by then has failed.
ESTIMATE_TIMEOUT_S = 600


class QuoteError(Exception):
    def __init__(self, message, status=409):
        super().__init__(message)
        self.status = status


@dataclasses.dataclass
class Quote:
    id: str
    matrices: list
    estimate: dict = None
    text: str = ""
    error: str = None


@dataclasses.dataclass
class Approval:
    id: str
    pid: int
    log: str


def now():
    """The time as the Node server's records had it (toISOString)."""
    return datetime.datetime.now(datetime.UTC).isoformat(timespec="milliseconds").replace("+00:00", "Z")


def alive(pid):
    """Whether process `pid` exists (os.kill(0) and below would signal a
    group)."""
    if not isinstance(pid, int) or pid <= 0:
        return False
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        pass
    return True


class Regenerations:
    def __init__(self, root=REGENERATE, farm_py=None, python="python3"):
        self.root = Path(root)
        self._farm_py = Path(farm_py) if farm_py else None
        self.python = python
        self._lock = threading.RLock()
        # id -> the run this process started, until it exits
        self._runs = {}

    @property
    def farm_py(self):
        """The farm.py quotes and runs use: the main checkout's, which keeps
        the farm's state, unless one was given."""
        if self._farm_py is None:
            self._farm_py = simdb().main_checkout() / "hack" / "sim-farm" / "farm.py"
        return self._farm_py

    def path(self, id, ext=".json"):
        if not isinstance(id, str) or not ID.fullmatch(id):
            raise QuoteError(f"not a regeneration id: {id!r}", 404)
        return self.root / f"{id}{ext}"

    def read(self, id):
        try:
            return json.loads(self.path(id).read_text())
        except FileNotFoundError:
            return None

    def get(self, id):
        """The regeneration `id` with its log's last lines, or None."""
        if not isinstance(id, str) or not ID.fullmatch(id):
            return None
        with self._lock:
            r = self.read(id)
            if r is None:
                return None
            if r.get("state") == "running" and id not in self._runs and not alive(r.get("pid")):
                r.update(state="finished", code=None)
                self.save(r)
        if not r.get("log"):
            return r
        try:
            tail = Path(r["log"]).read_text(errors="replace").split("\n")[-LOG_TAIL:]
        except OSError:
            tail = []
        return {**r, "log_tail": tail}

    def save(self, record):
        """Writes `record` (a regeneration, as get returns it but log_tail)
        as <root>/<id>.json."""
        path = self.path(record["id"])
        record = {k: v for k, v in record.items() if k != "log_tail"}
        path.parent.mkdir(parents=True, exist_ok=True)
        tmp = path.with_name(f".{path.name}.{os.getpid()}.{threading.get_ident()}")
        tmp.write_text(json.dumps(record, indent=2, ensure_ascii=False) + "\n")
        os.replace(tmp, path)

    def approve(self, id):
        """Starts farm.py run on quoted regeneration `id`'s matrices,
        detached: QuoteError 404 or 409 when it isn't one to start, 500
        when it doesn't start."""
        with self._lock:
            r = self.read(id)
            if r is None:
                raise QuoteError("no such regeneration", 404)
            if r.get("state") != "quoted":
                raise QuoteError(f"regeneration {id} is {r.get('state')}, not quoted", 409)
            log = self.path(id, ".log")
            try:
                out = log.open("x")
            except FileExistsError:
                raise QuoteError(f"regeneration {id} is approved already", 409) from None
            cmd = [self.python, str(self.farm_py), "run", *r["matrices"]]
            # A farm run is a script of its own: one uv ran this process
            # under would refuse to run again under uv itself.
            env = {k: v for k, v in os.environ.items() if k != UV_ENV}
            try:
                with out:
                    p = subprocess.Popen(cmd, stdin=subprocess.DEVNULL, stdout=out, stderr=subprocess.STDOUT,
                                         start_new_session=True, env=env)
            except OSError as e:
                log.write_text(f"{cmd}: {e}\n")
                r.update(state="failed", error=f"farm.py run: {e}", approved_at=now(), log=str(log))
                self.save(r)
                raise QuoteError(f"farm.py run didn't start: {e}", 500) from None
            r.update(state="running", approved_at=now(), pid=p.pid, log=str(log))
            self.save(r)
            self._runs[id] = p
        threading.Thread(target=self._wait, args=(id, p), name=f"farm-run-{id}", daemon=True).start()
        return Approval(id, p.pid, str(log))

    def _wait(self, id, p):
        code = p.wait()
        with self._lock:
            r = self.read(id)
            if r is not None:
                r.update(state="finished", code=code if code >= 0 else None)
                self.save(r)
            self._runs.pop(id, None)


def paths_of(matrices):
    return [os.path.abspath(str(getattr(m, "path", m))) for m in matrices]


def quote(matrices, regenerations=None):
    """farm.py estimate's quote for `matrices`, kept as a quoted (or failed)
    regeneration under a fresh id; nothing runs until approve(id)."""
    regs = regenerations or Regenerations()
    q = Quote(secrets.token_urlsafe(9), paths_of(matrices))
    if not q.matrices:
        q.error = "no matrices to quote"
    else:
        cmd = [regs.python, str(regs.farm_py), "estimate", "--json", *q.matrices]
        try:
            p = subprocess.run(cmd, stdin=subprocess.DEVNULL, capture_output=True, text=True,
                               timeout=ESTIMATE_TIMEOUT_S)
        except subprocess.TimeoutExpired:
            q.error = f"farm.py estimate: no answer in {ESTIMATE_TIMEOUT_S} s"
        except OSError as e:
            q.error = f"farm.py estimate: {e}"
        else:
            q.text = p.stderr.rstrip("\n")
            if p.returncode:
                last = next((l for l in reversed(p.stderr.splitlines()) if l.strip()), "")
                q.error = f"farm.py estimate: exit {p.returncode}" + (f": {last}" if last else "")
            else:
                try:
                    q.estimate = json.loads(p.stdout)
                except ValueError:
                    pass
                if not isinstance(q.estimate, dict):
                    q.estimate = None
                    q.error = f"farm.py estimate --json printed no JSON object: {p.stdout[:200]!r}"
    regs.save({
        "id": q.id,
        "state": "failed" if q.error else "quoted",
        "created_at": now(),
        "commands": [],
        "matrices": q.matrices,
        "quote": q.text,
        "estimate": q.estimate,
        "warnings": [],
        **({"error": q.error} if q.error else {}),
    })
    return q


def approve(id, regenerations=None):
    """Starts quoted regeneration `id`'s farm run (Regenerations.approve)."""
    return (regenerations or Regenerations()).approve(id)

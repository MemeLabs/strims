# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""DuckDB for simlab, in the duckdb module: the pinned release and
extensions, the catalog's connections, and queries over them.

Pins. DUCKDB_VERSION is simdb.py's (the farm hosts' CLI, read from its
text, so the one copy stays there); hack/pyproject.toml pins the module to
the same release. EXTENSIONS are the ducklake and sqlite builds for it, as
DuckDB's repository installs them (~/.duckdb/extensions/, which the CLI
and the module share). `ensure_extensions` installs them where missing;
every connection loads them, then checks the release and the builds
(PinError).

Connections. Each is a new in-memory instance with extension autoloading
and autoinstalling off:

  reader(lake)       the catalog (lake_root's catalog.sqlite) attached
                     READ_ONLY as `lake`, and used
  reader(lake, guard, tmp)
                     the same, then shut in before any query runs: no
                     external access but the lake's data files and the
                     spill directory `tmp` (allowed_directories), a memory
                     limit, threads and a spill cap, all locked
                     (lock_configuration); `cursors` more connections of
                     the instance, each using `lake` (a cursor starts in
                     the in-memory catalog, and the gate refuses USE),
                     come with it
  writer(lake)       the catalog for the indexer: new data under data/,
                     none of it inlined in the SQLite file
  store(db)          a fold's DuckDB file

Queries. `run` executes one statement on a connection: `gate` first for
an untrusted one, its `$name` parameters bound (`bind`), its rows
streamed up to `max_rows` (one more tells `truncated`), and interrupted
at `timeout_s`. The gate asks DuckDB's parser alone (json_serialize_sql
serializes SELECTs only), before anything binds it: one SELECT (DESCRIBE,
SHOW, SUMMARIZE and FROM-first among them) or an EXPLAIN whose rest is
one, else 403. DuckDB may write the allowed directories, and binding an
EXPORT DATABASE already makes its directory, so a COPY, an ATTACH or a
CREATE never gets that far. A parameter binds as the console's Node
server bound it: an integer a double holds exactly as BIGINT, another
number as DOUBLE, a list as a list of its numbers and strings; only the
names the statement has bind (the module refuses extra ones). The timeout
is a timer whose interrupt holds the query's lock: closing the query
stops and joins the timer before its connection is used again, so an
interrupt meant for one query never reaches the next. `script` runs every
statement of a script, as the CLI does, and returns the results of those
the CLI prints. `Pool` keeps `size` guarded readers for a server: a query
waits for a free one; `refresh` retires them for new ones (the catalog
changed under them) once the queries on them finish.

Results. A Result holds a statement's columns (their names and types as
the console's Node server named them: STRUCT fields quoted), its rows as
the module gives them, `truncated` and `elapsed_ms`.
`values()` gives them as the CLI's JSON gave them (simdb.duck): integers
and decimals as numbers (a decimal with a scale as a float), times,
dates, intervals, blobs and UUIDs as DuckDB's strings, MAP keys as
strings. `json()` gives them as the console's API does: BIGINT, HUGEINT
and DECIMAL as JSON numbers where a double holds them exactly, else as
strings, NaN and the infinities as "NaN", "Infinity" and "-Infinity",
INTERVALs as {months, days, micros}, MAPs as objects. Two deviations from
the Node server: a TIMESTAMPTZ takes the session time zone's offset at its
own instant (Node's took the server's offset at its start for every
value), and an INTERVAL's months come back folded into its days (30 a
month, as the module returns a timedelta; no catalog column is an
INTERVAL). A query run with `text` returns DuckDB's own strings for its
values (each column cast to VARCHAR in SQL, names kept), and `table()`
lays them out as the CLI's -table mode does. One run with `temporal_text`
casts only its top-level dates, times and timestamps (their text is what
values() and json() give): DuckDB formats them faster than Python, and
exactly where the module's values can't hold them (nanoseconds, the
infinities, years past 9999). A JSON column's type is VARCHAR, as the
Node server named it.
"""

import contextlib
import dataclasses
import datetime
import decimal
import json
import os
import re
import shutil
import threading
import time
import unicodedata
from pathlib import Path

try:
    import duckdb
except ImportError:
    # The pins import under any Python (catalog.py's EXTENSIONS); a
    # connection needs the module (bootstrap.engine).
    duckdb = None

from . import SIMDB_PY

DUCKDB_VERSION = re.search(r'^DUCKDB_VERSION = "([^"]+)"', SIMDB_PY.read_text(), re.M).group(1)
# The extensions' builds for DUCKDB_VERSION, as DuckDB's repository
# installs them (~/.duckdb/extensions/).
EXTENSIONS = {"ducklake": "ac7595b0", "sqlite_scanner": "5274128"}
# The instance settings every connection starts with.
CONFIG = {"autoinstall_known_extensions": "false", "autoload_known_extensions": "false",
          "allow_community_extensions": "false"}
# The largest integer a double holds exactly, and its negation.
SAFE = 2 ** 53 - 1


class PinError(RuntimeError):
    """DuckDB or an extension isn't the pinned build."""


class QueryError(Exception):
    """A statement refused or failed, with the HTTP status that says why:
    400 (it doesn't parse, bind or run), 403 (it isn't a SELECT), 408 (it
    ran past its timeout)."""

    def __init__(self, message, status=400):
        super().__init__(message)
        self.status = status


def need_module():
    if duckdb is None:
        raise RuntimeError("no duckdb module: run under hack/'s uv venv (simlab.bootstrap.engine)")


def q(s):
    """A SQL string literal."""
    return "'" + str(s).replace("'", "''") + "'"


def ident(s):
    """A quoted SQL identifier."""
    return '"' + str(s).replace('"', '""') + '"'


# --- pins and connections -------------------------------------------------------


def check_pins(conn):
    """Raises PinError unless `conn` runs DUCKDB_VERSION with EXTENSIONS'
    builds loaded."""
    version = conn.execute("SELECT library_version FROM pragma_version()").fetchone()[0]
    if version != DUCKDB_VERSION:
        raise PinError(f"DuckDB {version}, not the pinned {DUCKDB_VERSION}")
    loaded = dict(conn.execute("SELECT extension_name, extension_version FROM duckdb_extensions() "
                               "WHERE loaded").fetchall())
    bad = {n: loaded.get(n) for n, v in EXTENSIONS.items() if loaded.get(n) != v}
    if bad:
        raise PinError(f"DuckDB extensions {bad}, not the pinned {EXTENSIONS}: reinstall them (FORCE INSTALL), "
                       "or pin the new builds in simlab.engine.EXTENSIONS")


def ensure_extensions():
    """Installs the pinned extensions where missing (DuckDB's repository)
    and checks DuckDB's release and their builds: PinError."""
    need_module()
    conn = duckdb.connect(":memory:", config=CONFIG)
    try:
        conn.execute("INSTALL ducklake; INSTALL sqlite; LOAD ducklake; LOAD sqlite;")
        check_pins(conn)
    finally:
        conn.close()


def connect():
    """A new in-memory instance with the pinned extensions loaded."""
    need_module()
    conn = duckdb.connect(":memory:", config=CONFIG)
    try:
        conn.execute("LOAD ducklake; LOAD sqlite;")
        check_pins(conn)
    except BaseException:
        conn.close()
        raise
    return conn


@dataclasses.dataclass(frozen=True)
class Guard:
    """A guarded reader's limits."""
    memory_limit: str = "8GB"
    threads: int = 8
    max_temp_size: str = "8GiB"


def catalog_file(lake):
    return Path(lake) / "catalog.sqlite"


def attach_sql(lake, write=False):
    """The ATTACH of the catalog at `lake` as `lake`: read-only, or for the
    indexer writing new data under data/ (none of it inlined in the SQLite
    file)."""
    how = (f"(DATA_PATH {q(str(Path(lake) / 'data') + '/')}, DATA_INLINING_ROW_LIMIT 0)" if write
           else "(READ_ONLY)")
    return f"ATTACH {q('ducklake:sqlite:' + str(catalog_file(lake)))} AS lake {how}"


def settings_sql(threads=None, memory=None, tmp=None):
    out = []
    if threads:
        out.append(f"SET threads={max(1, int(threads))}")
    if memory:
        out.append(f"SET memory_limit={q(memory)}")
    if tmp:
        out.append(f"SET temp_directory={q(str(tmp))}")
    return out


def reader(lake, guard=None, tmp=None, cursors=0, threads=None, memory=None):
    """The catalog at `lake` attached READ_ONLY, in use. With `guard` (a
    Guard), shut in before it returns: external access only to the lake's
    data/ and the spill directory `tmp` (made), its limits, the
    configuration locked. With `cursors`, (conn, [cursor, ...]): that many
    more connections of the instance, each using the catalog. `threads`
    and `memory` set an unguarded one's limits."""
    conn = connect()
    try:
        conn.execute(attach_sql(lake) + "; USE lake;")
        more = [conn.cursor() for _ in range(cursors)]
        for c in more:
            c.execute("USE lake")
        if guard is None:
            for s in settings_sql(threads, memory):
                conn.execute(s)
        else:
            if tmp is None:
                raise ValueError("a guarded reader needs its spill directory, tmp")
            Path(tmp).mkdir(parents=True, exist_ok=True)
            data = str(Path(lake) / "data") + "/"
            conn.execute(";\n".join([
                f"SET memory_limit={q(guard.memory_limit)}",
                f"SET threads={max(1, int(guard.threads))}",
                f"SET temp_directory={q(str(tmp))}",
                f"SET max_temp_directory_size={q(guard.max_temp_size)}",
                f"SET allowed_directories=[{q(data)}, {q(str(tmp).rstrip('/') + '/')}]",
                "SET enable_external_access=false",
                "SET lock_configuration=true",
            ]))
    except BaseException:
        conn.close()
        raise
    return (conn, more) if cursors else conn


def writer(lake, threads=None, memory=None, tmp=None):
    """The catalog at `lake` attached for writing, in use: the indexer's
    (one at a time, under catalog.py's lock)."""
    conn = connect()
    try:
        conn.execute(attach_sql(lake, write=True) + "; USE lake;")
        for s in settings_sql(threads, memory, tmp):
            conn.execute(s)
    except BaseException:
        conn.close()
        raise
    return conn


def store(db, threads=2, memory="1GB", tmp=None):
    """A fold's DuckDB file `db` (simdb.fold's store), made if missing."""
    need_module()
    conn = duckdb.connect(str(db), config=CONFIG)
    try:
        for s in settings_sql(threads, memory, tmp):
            conn.execute(s)
    except BaseException:
        conn.close()
        raise
    return conn


# --- the gate, parameters and the timeout ------------------------------------------

LEADING = re.compile(r"^(?:\s+|--[^\n]*(?:\n|$)|/\*.*?\*/)*", re.S)
EXPLAIN = re.compile(r"^EXPLAIN\s+(?:ANALYZE\s+|\(\s*(?:ANALYZE|FORMAT\s+\w+)(?:\s*,\s*(?:ANALYZE|FORMAT\s+\w+))*"
                     r"\s*\)\s*)?", re.I)


def selects(conn, sql):
    """How many SELECT statements `sql` is, or None if it isn't SELECTs
    alone: DuckDB's parser only, nothing bound or planned."""
    text = conn.execute("SELECT json_serialize_sql($1::VARCHAR)", [sql]).fetchone()[0]
    parsed = json.loads(text)
    if not parsed.get("error"):
        return len(parsed.get("statements") or [])
    message = parsed.get("error_message") or ""
    if message.startswith("Only SELECT statements"):
        return None
    raise QueryError(message or "the statement doesn't parse", 400)


def gate(conn, sql):
    """`sql`'s one statement, if it is a SELECT or an EXPLAIN of one
    (QueryError 403 if not, 400 if it is more than one or doesn't
    parse)."""
    body = LEADING.sub("", sql, count=1)
    n = selects(conn, sql)
    if n is None:
        # EXPLAIN ANALYZE runs what it explains: that must be a SELECT too.
        m = EXPLAIN.match(body)
        if m:
            n = selects(conn, body[m.end():])
        if n is None:
            word = re.match(r"\w*", body).group().upper() or "that"
            raise QueryError(f"only SELECT and EXPLAIN SELECT run here, not {word}", 403)
    if n != 1:
        raise QueryError(f"a query is one statement, not {n}", 400)
    statement = one(conn, sql)
    if statement.type not in (duckdb.StatementType.SELECT, duckdb.StatementType.EXPLAIN):
        raise QueryError(f"only SELECT and EXPLAIN SELECT run here, not {statement.type.name}", 403)
    return statement


def one(conn, sql):
    """`sql`'s one statement (QueryError 400 if it is more or none)."""
    try:
        got = conn.extract_statements(sql)
    except duckdb.Error as e:
        raise QueryError(str(e), 400) from None
    if len(got) != 1:
        raise QueryError(f"a query is one statement, not {len(got)}", 400)
    return got[0]


def js_string(x):
    """String(x), as JavaScript makes a list item that isn't a number."""
    if isinstance(x, bool):
        return "true" if x else "false"
    return "null" if x is None else str(x)


def number(v):
    """A JSON number as the Node server bound it: a whole one a double
    holds exactly as BIGINT, any other as DOUBLE."""
    if isinstance(v, int) and abs(v) <= SAFE or isinstance(v, float) and v.is_integer() and abs(v) <= SAFE:
        return duckdb.LongValue(int(v))
    return duckdb.DoubleValue(float(v))


def bind(statement, params):
    """The values of `statement`'s `$name` parameters from `params`
    ({name: value}), for execute(): QueryError 400 for one missing or of
    a type that doesn't bind. Names it lacks are left out."""
    out = {}
    for name in sorted(statement.named_parameters):
        if name not in params:
            raise QueryError(f"no value for ${name}", 400)
        v = params[name]
        if v is None or isinstance(v, (bool, str)):
            out[name] = v
        elif isinstance(v, (int, float)):
            out[name] = number(v)
        elif isinstance(v, (list, tuple)):
            out[name] = [x if isinstance(x, (int, float)) and not isinstance(x, bool) else js_string(x) for x in v]
        else:
            raise QueryError(f"${name} is no string, number, boolean or list", 400)
    return out or None


class Deadline:
    """Interrupts `conn` at `timeout_s` unless closed first. The interrupt
    and close() take one lock, and close() stops and joins the timer, so
    once it returns no interrupt can reach the connection's next query."""

    def __init__(self, conn, timeout_s):
        self.conn = conn
        self.fired = False
        self._lock = threading.Lock()
        self._live = True
        self._timer = threading.Timer(timeout_s, self._fire)
        self._timer.daemon = True
        self._timer.start()

    def _fire(self):
        with self._lock:
            if self._live:
                self.fired = True
                self.conn.interrupt()

    def close(self):
        with self._lock:
            self._live = False
        self._timer.cancel()
        if self._timer is not threading.current_thread():
            self._timer.join()


# --- results ------------------------------------------------------------------------


@dataclasses.dataclass
class Column:
    name: str
    type: str


@dataclasses.dataclass
class Result:
    """A statement's columns, its rows as the module gives them (DuckDB's
    strings for a query run with `text`), whether it had more than were
    fetched, and its time."""
    columns: list
    rows: list
    truncated: bool = False
    elapsed_ms: int = 0
    # The columns' DuckDBPyTypes (VARCHAR's for a `text` query).
    types: list = dataclasses.field(default_factory=list, repr=False)

    @property
    def names(self):
        return [c.name for c in self.columns]

    def values(self):
        """The rows as lists of the CLI's JSON values (the module docs)."""
        return converted(self.rows, [converter(t, plain=True) for t in self.types])

    def records(self):
        """The rows as dicts of the CLI's JSON values, as simdb.duck gave
        them (a name a row repeats keeps its last value)."""
        names = self.names
        return [dict(zip(names, row)) for row in self.values()]

    def json(self):
        """The console API's answer: {columns, rows, truncated,
        elapsed_ms}, its values JSON's (the module docs)."""
        return {"columns": [dataclasses.asdict(c) for c in self.columns],
                "rows": converted(self.rows, [converter(t, plain=False) for t in self.types]),
                "truncated": self.truncated, "elapsed_ms": self.elapsed_ms}

    def table(self):
        """The rows as the CLI's -table mode lays them out (render_table),
        their values DuckDB's strings for a `text` query."""
        return render_table(self.names, [[v if v is None or isinstance(v, str) else varchar(v) for v in row]
                                         for row in self.rows])


def converted(rows, conv):
    """`rows` as new lists, each column's converter applied (none called
    for a column whose converter is `same`)."""
    todo = [(i, f) for i, f in enumerate(conv) if f is not same]
    if not todo:
        return [list(r) for r in rows]
    out = []
    for r in rows:
        r = list(r)
        for i, f in todo:
            r[i] = f(r[i])
        out.append(r)
    return out


def type_name(t):
    """A DuckDBPyType's name as the console's Node server gave it:
    DuckDB's, with STRUCT and UNION fields quoted, JSON as VARCHAR."""
    kind = t.id
    if kind == "varchar":
        return "VARCHAR"
    if kind in ("struct", "union"):
        fields = ", ".join(f"{ident(n)} {type_name(c)}" for n, c in t.children)
        return f"{kind.upper()}({fields})" if t.children else kind.upper()
    if kind == "map":
        kids = dict(t.children)
        return f"MAP({type_name(kids['key'])}, {type_name(kids['value'])})"
    if kind == "list":
        return f"{type_name(dict(t.children)['child'])}[]"
    if kind == "array":
        kids = dict(t.children)
        return f"{type_name(kids['child'])}[{kids['size']}]"
    return str(t)


BIG_INTS = {"bigint", "ubigint", "hugeint", "uhugeint", "varint", "bignum"}
INTS = BIG_INTS | {"tinyint", "smallint", "integer", "utinyint", "usmallint", "uinteger"}
TIMESTAMPS = {"timestamp", "timestamp_s", "timestamp_ms", "timestamp_ns", "timestamp with time zone"}
# The types `temporal_text` casts to VARCHAR, as DESCRIBE names them.
TEMPORAL = {"DATE", "TIME", "TIME_NS", "TIME WITH TIME ZONE", "TIMESTAMP", "TIMESTAMP_S", "TIMESTAMP_MS",
            "TIMESTAMP_NS", "TIMESTAMP WITH TIME ZONE"}


def fraction(us):
    """A time's fractional seconds as DuckDB prints them: none, or its
    microseconds less their trailing zeros."""
    return "" if not us else "." + f"{us:06d}".rstrip("0")


def offset_text(delta):
    """A UTC offset as DuckDB prints it: +HH, or +HH:MM."""
    s = int(delta.total_seconds())
    sign, s = ("-", -s) if s < 0 else ("+", s)
    h, m = divmod(s // 60, 60)
    return f"{sign}{h:02d}" + (f":{m:02d}" if m else "")


def date_text(d):
    return f"{d.year:04d}-{d.month:02d}-{d.day:02d}"


def time_text(t):
    s = f"{t.hour:02d}:{t.minute:02d}:{t.second:02d}{fraction(t.microsecond)}"
    off = t.utcoffset()
    return s if off is None else s + offset_text(off)


def timestamp_text(v):
    s = f"{date_text(v)} {time_text(v.time())}"
    off = v.utcoffset()
    return s if off is None else s + offset_text(off)


def interval_text(td):
    """A timedelta as DuckDB prints an INTERVAL of its days and time (its
    months, folded into days by the module, gone)."""
    us = td // datetime.timedelta(microseconds=1)
    days = int(us / 86_400_000_000)
    us -= days * 86_400_000_000
    parts = [f"{days} day{'s' if abs(days) != 1 else ''}"] if days else []
    if us or not parts:
        sign, us = ("-", -us) if us < 0 else ("", us)
        secs, frac = divmod(us, 1_000_000)
        h, rem = divmod(secs, 3600)
        parts.append(f"{sign}{h:02d}:{rem // 60:02d}:{rem % 60:02d}{fraction(frac)}")
    return " ".join(parts)


def blob_text(b):
    """A BLOB as DuckDB prints it: control characters, quotes and bytes
    past ASCII as \\xHH."""
    return "".join(chr(x) if 0x20 <= x < 0x7f and x not in (0x22, 0x27) else f"\\x{x:02X}" for x in b)


def varchar(v):
    """A value the module gave, as text, where DuckDB's own strings
    aren't to hand (a statement `text` can't wrap: PRAGMA, CALL)."""
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, datetime.datetime):
        return timestamp_text(v)
    if isinstance(v, datetime.date):
        return date_text(v)
    if isinstance(v, datetime.time):
        return time_text(v)
    if isinstance(v, datetime.timedelta):
        return interval_text(v)
    if isinstance(v, (bytes, bytearray)):
        return blob_text(v)
    return str(v)


def float_json(v):
    if v != v:
        return "NaN"
    if v in (float("inf"), float("-inf")):
        return "Infinity" if v > 0 else "-Infinity"
    return v


def same(v):
    return v


def converter(t, plain):
    """A function from a value of type `t` (a DuckDBPyType) as the module
    gives it to its CLI JSON value (`plain`) or its API JSON value: `same`
    for one it leaves as it is."""
    kind = t.id if t is not None else None
    ident_ = same
    if kind in INTS:
        if plain or kind not in BIG_INTS:
            return ident_
        return lambda v: v if v is None or -SAFE <= v <= SAFE else str(v)
    if kind in ("double", "float"):
        return ident_ if plain else lambda v: None if v is None else float_json(v)
    if kind == "decimal":
        scale = int(dict(t.children)["scale"])
        if plain:
            return lambda v: None if v is None else float(v) if scale else int(v)
        limit = decimal.Decimal(SAFE).scaleb(-scale)
        return lambda v: None if v is None else float(v) if -limit <= v <= limit else str(v)
    if kind in TIMESTAMPS:
        return lambda v: None if v is None or isinstance(v, str) else timestamp_text(v)
    if kind == "date":
        return lambda v: None if v is None or isinstance(v, str) else date_text(v)
    if kind in ("time", "time with time zone", "time_ns"):
        return lambda v: None if v is None or isinstance(v, str) else time_text(v)
    if kind == "interval":
        if plain:
            return lambda v: None if v is None else interval_text(v)

        def interval_json(v):
            if v is None:
                return None
            us = v // datetime.timedelta(microseconds=1)
            days = int(us / 86_400_000_000)
            return {"months": 0, "days": days, "micros": str(us - days * 86_400_000_000)}
        return interval_json
    if kind == "blob":
        return lambda v: None if v is None else blob_text(v)
    if kind == "uuid":
        return lambda v: None if v is None else str(v)
    if kind in ("list", "array"):
        f = converter(dict(t.children)["child"], plain)
        return lambda v: None if v is None else [f(x) for x in v]
    if kind == "struct":
        fs = [(n, converter(c, plain)) for n, c in t.children]
        return lambda v: None if v is None else {n: f(v.get(n)) for n, f in fs}
    if kind == "map":
        kids = dict(t.children)
        fk, fv = converter(kids["key"], plain), converter(kids["value"], plain)

        def key(k):
            k = fk(k)
            return js_string(k) if not isinstance(k, str) else k
        return lambda v: None if v is None else {key(k): fv(x) for k, x in v.items()}
    return ident_


# --- running statements -----------------------------------------------------------


def described(conn, statement):
    """The names of a SELECT's columns, bound but not run."""
    rows = conn.execute(f"DESCRIBE {statement.query}").fetchall()
    return [r[0] for r in rows]


def temporal_as_text(conn, statement, params):
    """SQL for `statement` (a SELECT) with its top-level dates, times and
    timestamps cast to VARCHAR, named as they are, and their types as
    DESCRIBE names them ({index: type}); (None, {}) when it has none or
    DESCRIBE can't take it."""
    try:
        described = conn.execute(f"DESCRIBE {statement.query}", params).fetchall()
    except duckdb.Error:
        return None, {}
    cast = {i: r[1] for i, r in enumerate(described) if r[1] in TEMPORAL}
    if not cast:
        return None, {}
    cols = ", ".join(f"#{i + 1}{'::VARCHAR' if i in cast else ''} AS {ident(r[0])}" for i, r in enumerate(described))
    return f"SELECT {cols} FROM (\n{TAIL.sub('', statement.query)}\n)", cast


def as_text(conn, statement):
    """SQL for `statement` (a SELECT) with each column cast to VARCHAR,
    named as it is, or None when DESCRIBE can't take it."""
    try:
        names = described(conn, statement)
    except duckdb.Error:
        return None
    cols = ", ".join(f"#{i}::VARCHAR AS {ident(n)}" for i, n in enumerate(names, 1))
    return f"SELECT {cols} FROM (\n{TAIL.sub('', statement.query)}\n)"


# A statement's end as extract_statements leaves it: its semicolon, and
# the whitespace and comments after it.
TAIL = re.compile(r";(?:\s+|--[^\n]*(?:\n|$)|/\*.*?\*/)*$", re.S)


def run(conn, sql, params=None, max_rows=None, timeout_s=None, guard=False, text=False, temporal_text=False):
    """Runs `sql`, one statement, on `conn`: through `gate` when `guard`,
    its parameters bound from `params`, its rows streamed to `max_rows`
    (all of them when None), interrupted at `timeout_s`; with `text` its
    values DuckDB's strings, with `temporal_text` those of its top-level
    dates, times and timestamps. QueryError says what failed, and how
    (status)."""
    t0 = time.perf_counter()
    deadline = Deadline(conn, timeout_s) if timeout_s else None
    try:
        statement = gate(conn, sql) if guard else one(conn, sql)
        values = bind(statement, params or {})
        query, cast = statement, {}
        if text and statement.type == duckdb.StatementType.SELECT:
            query = as_text(conn, statement) or statement
        elif temporal_text and statement.type == duckdb.StatementType.SELECT:
            wrapped, cast = temporal_as_text(conn, statement, values)
            query = wrapped or statement
        cur = conn.execute(query, values)
        desc = cur.description or []
        rows = cur.fetchmany(max_rows + 1) if max_rows is not None else cur.fetchall()
    except duckdb.Error as e:
        if deadline:
            deadline.close()
            if deadline.fired:
                raise QueryError(f"interrupted at the timeout ({timeout_s:g} s)", 408) from None
        raise QueryError(str(e), 400) from None
    finally:
        if deadline:
            deadline.close()
    truncated = max_rows is not None and len(rows) > max_rows
    if truncated:
        rows = rows[:max_rows]
    types = [d[1] for d in desc]
    columns = [Column(d[0], cast[i] if i in cast else type_name(d[1])) for i, d in enumerate(desc)]
    if text:
        rows = [[None if v is None else v if isinstance(v, str) else varchar(v) for v in r] for r in rows]
    else:
        rows = [list(r) for r in rows]
    return Result(columns, rows, truncated, round((time.perf_counter() - t0) * 1000), types)


def shown(statement):
    """Whether the CLI prints a table for `statement`: a query's rows, not
    a count or a success."""
    return list(statement.expected_result_type) == [duckdb.ExpectedResultType.QUERY_RESULT]


def script(conn, sql, params=None, text=False):
    """Runs every statement of `sql` in order, each with its own
    parameters from `params`, and returns the Results of those the CLI
    prints (`shown`), in order."""
    try:
        statements = conn.extract_statements(sql)
    except duckdb.Error as e:
        raise QueryError(str(e), 400) from None
    out = []
    for s in statements:
        if not shown(s):
            try:
                conn.execute(s, bind(s, params or {}))
            except duckdb.Error as e:
                raise QueryError(str(e), 400) from None
            continue
        out.append(run(conn, s.query, params, text=text))
    return out


# --- the CLI's tables -------------------------------------------------------------


def width(s):
    """`s`'s width on a terminal: wide East Asian characters 2, combining
    marks 0, a tab to the next multiple of 8, its widest line."""
    best = col = 0
    for ch in s:
        if ch == "\n":
            best, col = max(best, col), 0
        elif ch == "\t":
            col = (col // 8 + 1) * 8
        elif unicodedata.combining(ch):
            continue
        else:
            col += 2 if unicodedata.east_asian_width(ch) in ("W", "F") else 1
    return max(best, col)


def render_table(names, rows):
    """`rows` (lists of strings, None for NULL) under `names`, as the
    DuckDB CLI's -table mode prints them: a border, the names centred, a
    border, each row's values left-aligned, a border."""
    cells = [["NULL" if v is None else v for v in r] for r in rows]
    widths = [max([width(n)] + [width(r[i]) for r in cells]) for i, n in enumerate(names)]
    border = "+" + "+".join("-" * (w + 2) for w in widths) + "+"

    def centred(s, w):
        left = (w - width(s)) // 2
        return " " * left + s + " " * (w - width(s) - left)

    lines = [border, "|" + "|".join(f" {centred(n, w)} " for n, w in zip(names, widths)) + "|", border]
    for r in cells:
        lines.append("|" + "|".join(f" {v}{' ' * (w - width(v))} " for v, w in zip(r, widths)) + "|")
    lines.append(border)
    return "\n".join(lines) + "\n"


# --- the pool -------------------------------------------------------------------------


class _Generation:
    """One instance's readers: idle ones, how many are busy, and whether a
    refresh retired it."""

    def __init__(self, base, conns, tmp):
        self.base = base
        self.idle = list(conns)
        self.busy = 0
        self.retired = False
        self.tmp = tmp

    def close(self):
        for c in self.idle:
            with contextlib.suppress(Exception):
                c.close()
        self.idle = []
        with contextlib.suppress(Exception):
            self.base.close()
        shutil.rmtree(self.tmp, ignore_errors=True)


class Pool:
    """`size` guarded readers of the catalog at `lake` (one instance's
    cursors), spilling under `tmp`: a query takes a free one or waits.
    `refresh` retires them for a new instance's (the catalog's files or
    views changed): queries running finish on theirs, which then close."""

    def __init__(self, lake, tmp, size=2, guard=Guard()):
        self.lake = Path(lake)
        self.tmp = Path(tmp)
        self.size = max(1, int(size))
        self.guard = guard
        self._cond = threading.Condition()
        self._gen = None
        self._opening = False
        self._epoch = 0
        self._serial = 0
        self._closed = False

    def _open(self):
        self._serial += 1
        tmp = self.tmp / f"{os.getpid()}-{self._serial}"
        base, conns = reader(self.lake, self.guard, tmp, cursors=self.size)
        return _Generation(base, conns, tmp)

    def _acquire(self):
        while True:
            with self._cond:
                while True:
                    if self._closed:
                        raise QueryError("the catalog's readers are closed", 503)
                    gen = self._gen
                    if gen is not None and gen.idle:
                        gen.busy += 1
                        return gen, gen.idle.pop()
                    if gen is None and not self._opening:
                        self._opening = True
                        epoch = self._epoch
                        break
                    self._cond.wait()
            try:
                gen = self._open()
            except BaseException:
                with self._cond:
                    self._opening = False
                    self._cond.notify_all()
                raise
            with self._cond:
                self._opening = False
                if self._epoch == epoch and not self._closed:
                    self._gen = gen
                    gen = None
                self._cond.notify_all()
            if gen is not None:
                # A refresh came while it opened: it may predate the change.
                gen.close()

    def _release(self, gen, conn):
        with self._cond:
            gen.busy -= 1
            if gen.retired:
                with contextlib.suppress(Exception):
                    conn.close()
                if gen.busy == 0:
                    gen.close()
            else:
                gen.idle.append(conn)
            self._cond.notify_all()

    @contextlib.contextmanager
    def connection(self):
        """A reader, held while the block runs."""
        gen, conn = self._acquire()
        try:
            yield conn
        finally:
            self._release(gen, conn)

    def query(self, sql, params=None, max_rows=10_000, timeout_s=30.0, text=False, temporal_text=False):
        """Runs `sql` through the gate on a free reader (run): its first
        `max_rows` rows, interrupted at `timeout_s`."""
        t0 = time.perf_counter()
        with self.connection() as conn:
            r = run(conn, sql, params, max_rows=max_rows, timeout_s=timeout_s, guard=True, text=text,
                    temporal_text=temporal_text)
        r.elapsed_ms = round((time.perf_counter() - t0) * 1000)
        return r

    @property
    def generation(self):
        """How many instances the pool has opened."""
        return self._serial

    def refresh(self):
        """New readers for the queries after this; those running finish on
        theirs."""
        with self._cond:
            self._epoch += 1
            gen, self._gen = self._gen, None
            if gen is not None:
                gen.retired = True
                if gen.busy == 0:
                    gen.close()
            self._cond.notify_all()

    def close(self):
        """Closes every reader once its query finishes; queries after this
        fail (503)."""
        with self._cond:
            self._closed = True
        self.refresh()

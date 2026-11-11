// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Guarded, read-only connections to the sims' results catalog (hack/sim-db/catalog.py's DuckLake).
 *
 * One DuckDB instance holds the catalog attached READ_ONLY and a few connections that use it. Before
 * any query runs, the instance loads the pinned ducklake and sqlite builds (simlab/engine.py's EXTENSIONS,
 * for simdb.py's DUCKDB_VERSION) and then shuts itself in: no external access but the lake's data
 * files and its own spill directory (`allowed_directories`), no extension autoloading, a memory limit
 * and a thread count, all locked (`lock_configuration`). A query is one statement, a SELECT or an
 * EXPLAIN of one, which DuckDB's parser alone tells (json_serialize_sql serializes SELECTs only)
 * before anything binds it: DuckDB may write the allowed directories, and binding some statements
 * (EXPORT DATABASE) writes already, so a COPY, an ATTACH or a CREATE never gets that far. Every
 * query streams at most its row cap and is interrupted at its timeout.
 *
 * Values go out as JSON: BIGINT, HUGEINT and DECIMAL as numbers where a double holds them exactly,
 * else as strings (the column's type says which), MAPs as objects, lists as arrays.
 */

import { mkdirSync, readFileSync, rmSync } from "node:fs";
import path from "node:path";

import {
  DuckDBInstance,
  DuckDBTypeId,
  JsonDuckDBValueConverter,
  StatementType,
} from "@duckdb/node-api";
import type {
  DuckDBConnection,
  DuckDBDecimalValue,
  DuckDBMapValue,
  DuckDBPreparedStatement,
  DuckDBValueConverter,
  Json,
} from "@duckdb/node-api";

/** A query's bound value: `$name` in the SQL takes `params.name`. */
export type Param = string | number | boolean | null | readonly (string | number)[];

export interface Column {
  name: string;
  type: string;
}

export interface QueryResult {
  columns: Column[];
  rows: Json[][];
  /** The query had more rows than `max_rows`; `rows` holds the first `max_rows`. */
  truncated: boolean;
  elapsed_ms: number;
}

/** A query the catalog refused or failed, with the HTTP status that says why. */
export class QueryError extends Error {
  readonly status: number;

  constructor(message: string, status: number) {
    super(message);
    this.status = status;
  }
}

export interface LakeOptions {
  /** The catalog's directory: catalog.sqlite and data/ (catalog.py's lake_root). */
  lake: string;
  /** Where DuckDB spills; made, and removed on close. */
  tmp: string;
  /** Queries that run at once. */
  connections?: number;
  memoryLimit?: string;
  threads?: number;
  maxTempSize?: string;
  /** hack/sim-db, whose catalog.py and simdb.py pin the extensions and DuckDB. */
  simDb: string;
}

/** The DuckDB release and extension builds the catalog pins (simdb.py, hack/simlab/engine.py). */
export const pins = (simDb: string): { duckdb: string; extensions: Record<string, string> } => {
  const simdb = readFileSync(path.join(simDb, "simdb.py"), "utf8");
  const engine = readFileSync(path.join(simDb, "..", "simlab", "engine.py"), "utf8");
  const duckdb = /^DUCKDB_VERSION = "([^"]+)"/m.exec(simdb)?.[1];
  const ext = /^EXTENSIONS = \{([^}]*)\}/m.exec(engine)?.[1];
  if (!duckdb || !ext) throw new Error("no DUCKDB_VERSION in simdb.py or EXTENSIONS in simlab/engine.py");
  const extensions: Record<string, string> = {};
  for (const m of ext.matchAll(/"([a-z_]+)":\s*"([^"]+)"/g)) extensions[m[1]] = m[2];
  return { duckdb, extensions };
};

const SAFE = BigInt(Number.MAX_SAFE_INTEGER);

const bigintJson = (v: bigint): Json => (v <= SAFE && v >= -SAFE ? Number(v) : v.toString());

const BIG_INTS = new Set<DuckDBTypeId>([
  DuckDBTypeId.BIGINT,
  DuckDBTypeId.UBIGINT,
  DuckDBTypeId.HUGEINT,
  DuckDBTypeId.UHUGEINT,
]);

/** DuckDB's JSON conversion, but with exact numbers as JSON numbers and MAPs as objects. */
export const toJson: DuckDBValueConverter<Json> = (value, type, converter) => {
  if (value === null) return null;
  if (BIG_INTS.has(type.typeId) && typeof value === "bigint") return bigintJson(value);
  if (type.typeId === DuckDBTypeId.DECIMAL) {
    const d = value as DuckDBDecimalValue;
    return d.value <= SAFE && d.value >= -SAFE ? d.toDouble() : d.toString();
  }
  if (type.typeId === DuckDBTypeId.MAP) {
    const out: Record<string, Json> = {};
    for (const { key, value: v } of (value as DuckDBMapValue).entries) {
      out[String(key)] = v === null ? null : converter(v, type.valueType, converter);
    }
    return out;
  }
  return JsonDuckDBValueConverter(value, type, converter);
};

const LEADING = /^(?:\s+|--[^\n]*(?:\n|$)|\/\*[\s\S]*?\*\/)*/;
const EXPLAIN =
  /^EXPLAIN\s+(?:ANALYZE\s+|\(\s*(?:ANALYZE|FORMAT\s+\w+)(?:\s*,\s*(?:ANALYZE|FORMAT\s+\w+))*\s*\)\s*)?/i;

interface Pool {
  instance: DuckDBInstance;
  idle: DuckDBConnection[];
  busy: number;
  retired: boolean;
  /** Its spill directory. */
  tmp: string;
}

/** The catalog, read through a pool of guarded connections. */
export class Lake {
  readonly options: Required<LakeOptions>;
  private pool: Pool | null = null;
  private opening: Promise<Pool> | null = null;
  private waiters: (() => void)[] = [];
  private generation = 0;

  constructor(options: LakeOptions) {
    this.options = {
      connections: 2,
      memoryLimit: "8GB",
      threads: 8,
      maxTempSize: "8GiB",
      ...options,
    };
  }

  private async open(): Promise<Pool> {
    const o = this.options;
    const n = this.generation++;
    const tmp = path.join(o.tmp, `${process.pid}-${n}`);
    mkdirSync(tmp, { recursive: true });
    const { duckdb, extensions } = pins(o.simDb);
    const instance = await DuckDBInstance.create(":memory:", {
      autoinstall_known_extensions: "false",
      autoload_known_extensions: "false",
      allow_community_extensions: "false",
    });
    const conns: DuckDBConnection[] = [];
    try {
      for (let i = 0; i < o.connections; i++) conns.push(await instance.connect());
      const c = conns[0];
      const quote = (s: string) => `'${s.replaceAll("'", "''")}'`;
      await c.run("LOAD ducklake; LOAD sqlite;");
      const got = await c.runAndReadAll(
        "SELECT library_version AS v, (SELECT map(list(extension_name), list(extension_version)) " +
          "FROM duckdb_extensions() WHERE loaded) AS e FROM pragma_version()"
      );
      const [[version, loaded]] = got.convertRows(toJson) as [[string, Record<string, string>]];
      if (version !== duckdb) throw new Error(`DuckDB ${version}, not the pinned ${duckdb}`);
      for (const [name, build] of Object.entries(extensions)) {
        if (loaded[name] !== build) {
          throw new Error(`${name} build ${loaded[name]}, not the pinned ${build}`);
        }
      }
      const catalog = path.join(o.lake, "catalog.sqlite");
      await c.run(`ATTACH ${quote(`ducklake:sqlite:${catalog}`)} AS lake (READ_ONLY)`);
      for (const conn of conns) await conn.run("USE lake");
      const data = path.join(o.lake, "data") + "/";
      await c.run(
        [
          `SET memory_limit=${quote(o.memoryLimit)}`,
          `SET threads=${Math.max(1, Math.floor(o.threads))}`,
          `SET temp_directory=${quote(tmp)}`,
          `SET max_temp_directory_size=${quote(o.maxTempSize)}`,
          `SET allowed_directories=[${quote(data)}, ${quote(tmp + "/")}]`,
          "SET enable_external_access=false",
          "SET lock_configuration=true",
        ].join(";\n")
      );
    } catch (e) {
      for (const conn of conns) conn.closeSync();
      instance.closeSync();
      rmSync(tmp, { recursive: true, force: true });
      throw e;
    }
    return { instance, idle: conns, busy: 0, retired: false, tmp };
  }

  private async current(): Promise<Pool> {
    if (this.pool) return this.pool;
    if (!this.opening) {
      this.opening = this.open().then(
        (p) => {
          this.pool = p;
          this.opening = null;
          return p;
        },
        (e: unknown) => {
          this.opening = null;
          throw e;
        }
      );
    }
    return this.opening;
  }

  private async acquire(): Promise<[Pool, DuckDBConnection]> {
    for (;;) {
      const pool = await this.current();
      const conn = pool.idle.pop();
      if (conn) {
        pool.busy++;
        return [pool, conn];
      }
      await new Promise<void>((resolve) => this.waiters.push(resolve));
    }
  }

  private release(pool: Pool, conn: DuckDBConnection): void {
    pool.busy--;
    if (pool.retired) {
      conn.closeSync();
      if (pool.busy === 0) this.close(pool);
    } else {
      pool.idle.push(conn);
    }
    this.waiters.shift()?.();
  }

  private close(pool: Pool): void {
    for (const conn of pool.idle) conn.closeSync();
    pool.idle = [];
    pool.instance.closeSync();
    rmSync(pool.tmp, { recursive: true, force: true });
  }

  /**
   * Opens new connections for the queries after this (the catalog's views or files changed under
   * the old ones); queries running finish on theirs.
   */
  refresh(): void {
    const pool = this.pool;
    this.pool = null;
    if (!pool) return;
    pool.retired = true;
    if (pool.busy === 0) this.close(pool);
    for (const w of this.waiters.splice(0)) w();
  }

  /** Closes every connection. */
  shutdown(): void {
    this.refresh();
  }

  /**
   * Runs one statement over the catalog: a SELECT (DESCRIBE, SHOW, SUMMARIZE and FROM-first among
   * them) or an EXPLAIN of one, with `params` bound to the `$name`s it names.
   *
   * @param sql - the statement
   * @param params - the values of its `$name` parameters
   * @param maxRows - the most rows it returns
   * @param timeoutS - the seconds after which it's interrupted
   * @returns its columns, its first `maxRows` rows and whether there were more
   */
  async query(
    sql: string,
    params: Record<string, Param> = {},
    maxRows = 10_000,
    timeoutS = 30
  ): Promise<QueryResult> {
    const t0 = performance.now();
    const [pool, conn] = await this.acquire();
    let timedOut = false;
    const timer = setTimeout(() => {
      timedOut = true;
      conn.interrupt();
    }, timeoutS * 1000);
    let prepared: DuckDBPreparedStatement | null = null;
    try {
      prepared = await this.prepare(conn, sql);
      bind(prepared, params);
      const reader = await prepared.streamAndReadUntil(maxRows + 1);
      const types = reader.columnTypes();
      const columns = reader.columnNames().map((name, i) => ({ name, type: types[i].toString() }));
      const rows = reader.convertRows(toJson);
      const truncated = rows.length > maxRows;
      if (truncated) rows.length = maxRows;
      return { columns, rows, truncated, elapsed_ms: Math.round(performance.now() - t0) };
    } catch (e) {
      if (e instanceof QueryError) throw e;
      const message = e instanceof Error ? e.message : String(e);
      if (timedOut) throw new QueryError(`interrupted at the timeout (${timeoutS} s)`, 408);
      throw new QueryError(message, 400);
    } finally {
      clearTimeout(timer);
      prepared?.destroySync();
      this.release(pool, conn);
    }
  }

  /**
   * How many SELECT statements `sql` is, or null if it isn't SELECTs alone: DuckDB's parser only,
   * nothing bound or planned (preparing an EXPORT DATABASE already makes its directory).
   */
  private async selects(conn: DuckDBConnection, sql: string): Promise<number | null> {
    const r = await conn.runAndReadAll("SELECT json_serialize_sql($1::VARCHAR)", [sql]);
    const [[text]] = r.getRowsJS() as [[string]];
    const parsed = JSON.parse(text) as {
      error?: boolean;
      error_message?: string;
      statements?: unknown[];
    };
    if (!parsed.error) return parsed.statements?.length ?? 0;
    if (parsed.error_message?.startsWith("Only SELECT statements")) return null;
    throw new QueryError(parsed.error_message ?? "the statement doesn't parse", 400);
  }

  private async prepare(conn: DuckDBConnection, sql: string): Promise<DuckDBPreparedStatement> {
    const body = sql.replace(LEADING, "");
    let n = await this.selects(conn, sql);
    if (n === null) {
      // EXPLAIN ANALYZE runs what it explains: that must be a SELECT too.
      const m = EXPLAIN.exec(body);
      if (m) n = await this.selects(conn, body.slice(m[0].length));
      if (n === null) {
        const word = /^\w+/.exec(body)?.[0]?.toUpperCase() ?? "that";
        throw new QueryError(`only SELECT and EXPLAIN SELECT run here, not ${word}`, 403);
      }
    }
    if (n !== 1) throw new QueryError(`a query is one statement, not ${n}`, 400);
    const extracted = await conn.extractStatements(sql);
    if (extracted.count !== 1) {
      throw new QueryError(`a query is one statement, not ${extracted.count}`, 400);
    }
    const prepared = await extracted.prepare(0);
    const type = prepared.statementType;
    if (type === StatementType.SELECT || type === StatementType.EXPLAIN) return prepared;
    prepared.destroySync();
    throw new QueryError(
      `only SELECT and EXPLAIN SELECT run here, not ${StatementType[type]}`,
      403
    );
  }
}

const bind = (prepared: DuckDBPreparedStatement, params: Record<string, Param>): void => {
  for (let i = 1; i <= prepared.parameterCount; i++) {
    const name = prepared.parameterName(i);
    if (!Object.hasOwn(params, name)) throw new QueryError(`no value for $${name}`, 400);
    const v = params[name];
    if (v === null) prepared.bindNull(i);
    else if (typeof v === "string") prepared.bindVarchar(i, v);
    else if (typeof v === "boolean") prepared.bindBoolean(i, v);
    else if (typeof v === "number") {
      if (Number.isSafeInteger(v)) prepared.bindBigInt(i, BigInt(v));
      else prepared.bindDouble(i, v);
    } else if (Array.isArray(v)) {
      prepared.bindList(
        i,
        v.map((x) => (typeof x === "number" ? x : String(x)))
      );
    } else throw new QueryError(`$${name} is no string, number, boolean or list`, 400);
  }
};

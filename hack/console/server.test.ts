// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The console server's tests, over a small catalog of its own and stand-in commands:
 *
 *   node --test hack/console/
 */

import assert from "node:assert/strict";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { mkdirSync } from "node:fs";
import type { AddressInfo } from "node:net";
import os from "node:os";
import path from "node:path";

import { DuckDBInstance } from "@duckdb/node-api";
import { after, before, describe, test } from "node:test";

import { Lake } from "./db.ts";
import { Console, ROOT } from "./server.ts";
import { matrixEnv, suiteArgs } from "./tasks.ts";
import type { Commands, Experiment } from "./tasks.ts";

const dir = mkdtempSync(path.join(os.tmpdir(), "console-test-"));
const lakeDir = path.join(dir, "lake");
const calls = path.join(dir, "calls.log");

/** A catalog with the tables the server reads, written as catalog.py writes one. */
const makeLake = async (): Promise<void> => {
  mkdirSync(path.join(lakeDir, "data"), { recursive: true });
  const db = await DuckDBInstance.create(":memory:");
  const c = await db.connect();
  const catalog = path.join(lakeDir, "catalog.sqlite");
  await c.run(`LOAD ducklake; LOAD sqlite;
    ATTACH 'ducklake:sqlite:${catalog}' AS w (DATA_PATH '${lakeDir}/data/', DATA_INLINING_ROW_LIMIT 0);
    USE w;
    CREATE SCHEMA raw;
    CREATE TABLE experiments AS SELECT 'e1' AS exp_id, 'farm' AS kind,
      TIMESTAMPTZ '2026-11-10 10:00:00+00' AS started_at, ['robust'] AS tiers, NULL::MAP(VARCHAR, VARCHAR) AS knob,
      NULL::VARCHAR[] AS sets, MAP {'base': 'aaaa', 'new': 'bbbb'} AS arms, ['base', 'new'] AS arm_labels,
      'base' AS base, 'does it?' AS question, 2::BIGINT AS cells, 8::BIGINT AS jobs;
    CREATE TABLE experiment_cells AS SELECT * FROM (VALUES
      ('e1', 'calm', MAP {'SIM_VIEWERS': '8'}), ('e1', 'storm', MAP {'SIM_VIEWERS': '16'})) t(exp_id, cell, env);
    CREATE TABLE raw.tier_cells AS SELECT * FROM (VALUES
      ('robust', 'calm', 'ppspp', MAP {'SIM_VIEWERS': '8'}), ('robust', 'storm', 'ppspp', MAP {'SIM_VIEWERS': '16'}))
      t(tier, cell, sim, env);
    CREATE TABLE table_catalog AS SELECT 'nums' AS name, 'table' AS kind, 'one a number' AS grain,
      'i' AS keys, '' AS joins, 'numbers' AS description;
    CREATE TABLE metric_catalog AS SELECT 'i' AS name, 'nums' AS tbl, 'BIGINT' AS type,
      NOW() AS first_seen, NOW() AS last_seen;
    CREATE TABLE event_kinds AS SELECT 'join' AS kind, 'ppspp' AS sim, 'a viewer joins' AS description;
    CREATE TABLE catalog_info AS SELECT 'catalog_version' AS key, '1' AS value;
    CREATE TABLE nums AS SELECT range AS i FROM range(100000);`);
  c.closeSync();
  db.closeSync();
};

/** Stand-ins for simdb.py, suite.py and farm.py: each logs its arguments to calls.log. */
const makeCommands = (): Commands => {
  const bin = path.join(dir, "bin");
  mkdirSync(bin, { recursive: true });
  const matrices = path.join(dir, "matrices");
  mkdirSync(matrices, { recursive: true });
  const log = `import { appendFileSync, writeFileSync } from "node:fs";
const args = process.argv.slice(2);
appendFileSync(${JSON.stringify(
    calls
  )}, JSON.stringify([process.argv[1].split("/").pop(), ...args]) + "\\n");
`;
  writeFileSync(
    path.join(bin, "simdb.mjs"),
    log +
      `if (args[0] === "fetch") { console.log("pulling 2 flushes"); console.log(args[2] + ": fetched; S3 charged $0.0123 (2 ledger lines)"); }`
  );
  writeFileSync(
    path.join(bin, "suite.mjs"),
    log +
      `const cells = args[args.indexOf("--only") + 1].split(",");
cells.forEach((c, i) => writeFileSync(${JSON.stringify(
        matrices
      )} + "/s0abcd-" + String(i).padStart(2, "0") + ".toml",
  "# sim-suite farm s0abcd: " + c + ", why\\n[env]\\nSIM_VIEWERS = " + JSON.stringify(c === "calm" ? "8" : "32") + "\\n[axes]\\n"));
console.log("rows: none");`
  );
  writeFileSync(
    path.join(bin, "farm.mjs"),
    log +
      `if (args[0] === "estimate") console.log("16 jobs, ~$0.05");
if (args[0] === "run") console.log("running");`
  );
  return {
    python: process.execPath,
    simdb: path.join(bin, "simdb.mjs"),
    suite: path.join(bin, "suite.mjs"),
    farm: path.join(bin, "farm.mjs"),
    matrices,
    work: path.join(dir, "work"),
  };
};

const called = (): string[][] =>
  existsSync(calls)
    ? readFileSync(calls, "utf8")
        .trim()
        .split("\n")
        .map((l) => JSON.parse(l) as string[])
    : [];

let base = "";
let console_: Console;
let close: () => void;

before(async () => {
  await makeLake();
  const lake = new Lake({
    lake: lakeDir,
    tmp: path.join(dir, "tmp"),
    simDb: path.join(ROOT, "hack", "sim-db"),
    connections: 2,
    memoryLimit: "1GB",
    threads: 2,
  });
  console_ = new Console({ lake, commands: makeCommands(), views: path.join(dir, "views") });
  const server = await console_.listen(0);
  base = `http://127.0.0.1:${(server.address() as AddressInfo).port}`;
  close = () => {
    server.close();
    console_.close();
  };
});

after(() => {
  close();
  rmSync(dir, { recursive: true, force: true });
});

interface Answer {
  status: number;
  body: Record<string, unknown>;
}

const post = async (route: string, body: unknown, type = "application/json"): Promise<Answer> => {
  const res = await fetch(base + route, {
    method: "POST",
    headers: { "Content-Type": type },
    body: JSON.stringify(body),
  });
  return { status: res.status, body: (await res.json()) as Record<string, unknown> };
};

const sql = (q: string, more: Record<string, unknown> = {}): Promise<Answer> =>
  post("/console/sql", { sql: q, ...more });

describe("sql", () => {
  test("a query's columns, rows and types", async () => {
    const r = await sql(
      "SELECT 2::BIGINT AS a, 9007199254740993::BIGINT AS b, 1.25::DECIMAL(10, 2) AS c, " +
        "sum(i) AS d, MAP {'x': 1} AS e, [1, 2] AS f FROM nums"
    );
    assert.equal(r.status, 200);
    assert.deepEqual(r.body.columns, [
      { name: "a", type: "BIGINT" },
      { name: "b", type: "BIGINT" },
      { name: "c", type: "DECIMAL(10,2)" },
      { name: "d", type: "HUGEINT" },
      { name: "e", type: "MAP(VARCHAR, INTEGER)" },
      { name: "f", type: "INTEGER[]" },
    ]);
    assert.deepEqual(r.body.rows, [[2, "9007199254740993", 1.25, 4999950000, { x: 1 }, [1, 2]]]);
    assert.equal(r.body.truncated, false);
  });

  test("parameters bind by name; a missing one fails", async () => {
    const r = await sql("SELECT $exp || '!' AS s, $seed + 1 AS n, $xs AS xs", {
      params: { exp: "e1", seed: 2001, xs: ["a", "b"], unused: 1 },
    });
    assert.deepEqual(r.body.rows, [["e1!", 2002, ["a", "b"]]]);
    const m = await sql("SELECT $exp", { params: {} });
    assert.equal(m.status, 400);
    assert.match(String(m.body.error), /no value for \$exp/);
  });

  test("EXPLAIN, DESCRIBE and the like run", async () => {
    for (const q of [
      "EXPLAIN SELECT count(*) FROM nums",
      "EXPLAIN ANALYZE SELECT count(*) FROM nums",
      "-- a comment\nEXPLAIN (ANALYZE, FORMAT json) SELECT 1",
      "DESCRIBE nums",
      "SUMMARIZE nums",
      "SHOW TABLES",
      "FROM nums LIMIT 1",
    ]) {
      const r = await sql(q);
      assert.equal(r.status, 200, `${q}: ${JSON.stringify(r.body)}`);
    }
    const r = await sql("EXPLAIN SELECT 1; COPY nums TO '/tmp/x.csv'");
    assert.equal(r.status, 403);
  });

  test("rows stop at max_rows", async () => {
    const r = await sql("SELECT i FROM nums ORDER BY i", { max_rows: 10 });
    assert.equal((r.body.rows as unknown[]).length, 10);
    assert.equal(r.body.truncated, true);
  });

  test("a long query is interrupted at its timeout, and its connection works after", async () => {
    const t0 = Date.now();
    const r = await sql("SELECT sum(hash(i)) FROM range(100000000000) t(i)", { timeout_s: 1 });
    assert.equal(r.status, 408);
    assert.match(String(r.body.error), /interrupted at the timeout/);
    assert.ok(Date.now() - t0 < 5000, `took ${Date.now() - t0} ms`);
    for (let i = 0; i < 3; i++) assert.equal((await sql("SELECT 1")).status, 200);
  });

  test("writes, COPY, ATTACH, INSTALL, LOAD, SET and more than one statement are refused", async () => {
    const data = path.join(lakeDir, "data");
    const refused = [
      "INSERT INTO nums VALUES (1)",
      "UPDATE nums SET i = 0",
      "DELETE FROM nums",
      "CREATE TABLE t AS SELECT 1",
      "CREATE TEMP TABLE t AS SELECT 1",
      "DROP TABLE nums",
      `COPY (SELECT 1) TO '${data}/x.csv'`,
      `COPY nums TO '${data}/y.parquet'`,
      `EXPORT DATABASE '${data}/z'`,
      "ATTACH ':memory:' AS m",
      `ATTACH '${data}/w.duckdb' AS m`,
      "DETACH lake",
      "INSTALL httpfs",
      "LOAD httpfs",
      "SET threads = 64",
      "SET enable_external_access = true",
      "RESET lock_configuration",
      "USE memory",
      "PRAGMA enable_profiling",
      "CALL ducklake_snapshots('lake')",
      `EXPLAIN ANALYZE COPY (SELECT 1) TO '${data}/v.csv'`,
    ];
    for (const q of refused) {
      const r = await sql(q);
      assert.equal(r.status, 403, `${q}: ${JSON.stringify(r.body)}`);
    }
    const two = await sql("SELECT 1; SELECT 2");
    assert.equal(two.status, 400);
    assert.match(String(two.body.error), /one statement/);
    const inner = await sql(`SELECT * FROM query('COPY (SELECT 1) TO ''${data}/u.csv''')`);
    assert.equal(inner.status, 400);
    for (const f of ["x.csv", "y.parquet", "z", "w.duckdb", "v.csv", "u.csv"]) {
      assert.ok(!existsSync(path.join(data, f)), f);
    }
    assert.equal((await sql("SELECT count(*) FROM nums")).body.rows?.toString(), "100000");
  });

  test("the connection reads only the lake's files, its settings locked", async () => {
    for (const q of [
      "SELECT * FROM read_csv('/etc/passwd')",
      "SELECT * FROM read_text('/etc/hostname')",
      "SELECT * FROM glob('/etc/*')",
      `SELECT * FROM read_csv('${path.join(dir, "calls.log")}')`,
    ]) {
      const r = await sql(q);
      assert.equal(r.status, 400, q);
      assert.match(String(r.body.error), /Permission Error|disabled/, q);
    }
    const s = await sql(
      "SELECT current_setting('enable_external_access') AS ext, current_setting('lock_configuration') AS lock, " +
        "current_setting('autoload_known_extensions') AS autoload, current_setting('threads') AS threads, " +
        "current_setting('memory_limit') AS mem, " +
        "(SELECT readonly FROM duckdb_databases() WHERE database_name = 'lake') AS ro"
    );
    assert.deepEqual(s.body.rows, [[false, true, false, 2, "953.6 MiB", true]]);
    const files = await sql(
      `SELECT count(*) FROM read_parquet('${path.join(lakeDir, "data")}/**/*.parquet')`
    );
    assert.equal(files.status, 200, JSON.stringify(files.body));
  });

  test("a POST that isn't JSON is refused", async () => {
    const r = await post("/console/sql", { sql: "SELECT 1" }, "text/plain");
    assert.equal(r.status, 415);
  });
});

describe("schema and views", () => {
  test("schema has the catalog's tables, catalogs and experiments", async () => {
    const r = (await (await fetch(base + "/console/schema")).json()) as Record<string, unknown[]>;
    const names = (r.tables as { name: string }[]).map((t) => t.name);
    assert.ok(
      names.includes("experiments") && names.includes("nums") && names.includes("tier_cells")
    );
    assert.equal(r.experiments.length, 1);
    assert.equal(r.table_catalog.length, 1);
    assert.equal(r.event_kinds.length, 1);
    assert.deepEqual(r.views, []);
  });

  test("a saved view comes back", async () => {
    const res = await fetch(base + "/console/views/my-view", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        title: "Mine",
        sql: "SELECT $exp AS e",
        spec: { mark: "bar" },
        params: [{ name: "exp", kind: "exp" }],
      }),
    });
    assert.equal(res.status, 200);
    const views = (await (await fetch(base + "/console/views")).json()) as {
      name: string;
      spec: unknown;
    }[];
    assert.deepEqual(
      views.map((v) => [v.name, v.spec]),
      [["my-view", { mark: "bar" }]]
    );
    const bad = await fetch(base + "/console/views/..%2Fescape", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ sql: "SELECT 1" }),
    });
    assert.equal(bad.status, 400);
  });
});

describe("commands", () => {
  test("fetch streams simdb.py fetch's output and its S3 cost", async () => {
    const res = await fetch(base + "/console/fetch", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ exp_id: "e1", cells: ["calm"], tables: ["reads"] }),
    });
    const lines = (await res.text())
      .trim()
      .split("\n")
      .map((l) => JSON.parse(l) as Record<string, unknown>);
    assert.deepEqual(lines.at(-1), {
      done: true,
      code: 0,
      s3_usd: 0.0123,
      elapsed_ms: lines.at(-1)?.elapsed_ms,
    });
    assert.ok(lines.some((l) => l.line === "pulling 2 flushes"));
    assert.deepEqual(called().at(-1), [
      "simdb.mjs",
      "fetch",
      "--exp",
      "e1",
      "--cells",
      "calm",
      "--tables",
      "reads",
    ]);
  });

  test("fetch refuses a name that isn't one", async () => {
    const r = await post("/console/fetch", { exp_id: "e1", cells: ["--all"] });
    assert.equal(r.status, 400);
  });

  test("index runs simdb.py index and reports it", async () => {
    const r = await post("/console/index", {});
    assert.equal(r.body.code, 0);
    assert.deepEqual(called().at(-1), ["simdb.mjs", "index", "--quiet"]);
    const s = (await (await fetch(base + "/console/index")).json()) as Record<string, unknown>;
    assert.equal((s.last as Record<string, unknown>).code, 0);
  });

  test("regenerate quotes, and only approve starts farm.py run", async () => {
    const r = await post("/console/regenerate", {
      exp_id: "e1",
      cells: ["calm", "storm"],
      arms: ["new", "base"],
      seeds: [2001, 2002, 2003],
    });
    assert.equal(r.status, 200, JSON.stringify(r.body));
    assert.equal(r.body.state, "quoted");
    assert.equal(r.body.quote, "16 jobs, ~$0.05");
    assert.deepEqual(r.body.warnings, ["storm: env differs from e1's at SIM_VIEWERS (16 -> 32)"]);
    const id = String(r.body.id);
    const suite = called().find((c) => c[0] === "suite.mjs");
    assert.deepEqual(suite, [
      "suite.mjs",
      "farm",
      "--tier",
      "robust",
      "--only",
      "calm,storm",
      "--seeds",
      "3",
      "--first-seed",
      "2001",
      "--refs",
      "base=aaaa,new=bbbb",
      "--no-run",
    ]);
    const farm = () => called().filter((c) => c[0] === "farm.mjs");
    assert.deepEqual(
      farm().map((c) => c[1]),
      ["estimate"]
    );
    // Nothing runs without the approve call: not a read of it, nor another quote.
    await fetch(`${base}/console/regenerate/${id}`);
    await post("/console/regenerate", {
      exp_id: "e1",
      cells: ["calm"],
      arms: ["new"],
      seeds: [2001],
    });
    await new Promise((res) => setTimeout(res, 300));
    assert.ok(!farm().some((c) => c[1] === "run"));
    assert.equal((await post("/console/regenerate/nope/approve", {})).status, 404);
    const a = await post(`/console/regenerate/${id}/approve`, {});
    assert.equal(a.status, 200);
    assert.equal(a.body.state, "running");
    for (let i = 0; i < 50 && !farm().some((c) => c[1] === "run"); i++) {
      await new Promise((res) => setTimeout(res, 100));
    }
    const runs = farm().filter((c) => c[1] === "run");
    assert.equal(runs.length, 1);
    assert.deepEqual(
      runs[0].slice(2).map((p) => path.basename(p)),
      ["s0abcd-00.toml", "s0abcd-01.toml"]
    );
    assert.equal((await post(`/console/regenerate/${id}/approve`, {})).status, 409);
  });

  test("regenerate refuses what suite.py farm can't run", async () => {
    const exp: Experiment = {
      exp_id: "e1",
      kind: "farm",
      knob: { ppspp: "SIM_EXP", "strims-sim": "STRIMS_SIM_EXP" },
      sets: ["off", "a", "b"],
      arms: { x: "cccc" },
      base: "off",
      tiers: { calm: "robust" },
    };
    const c = makeCommands();
    const one = (r: Partial<Parameters<typeof suiteArgs>[2]>) =>
      suiteArgs(c, exp, { exp_id: "e1", cells: ["calm"], arms: ["a"], seeds: [1], ...r });
    assert.deepEqual(one({ arms: ["b", "a"] }), [
      [
        c.suite,
        "farm",
        "--tier",
        "robust",
        "--only",
        "calm",
        "--seeds",
        "1",
        "--first-seed",
        "1",
        "--knob",
        "SIM_EXP,STRIMS_SIM_EXP",
        "--ref",
        "cccc",
        "--sets",
        "a;b",
        "--no-off",
        "--against",
        "a",
        "--no-run",
      ],
    ]);
    assert.equal((one({ arms: ["off", "b"] }) as string[][])[0].includes("--no-off"), false);
    assert.match(String(one({ seeds: [1, 3] })), /consecutive seeds/);
    assert.match(String(one({ cells: ["gone"] })), /no tier/);
    assert.match(String(one({ arms: ["zz"] })), /not a set/);
    assert.match(
      String(
        suiteArgs(
          c,
          { ...exp, kind: "local" },
          { exp_id: "e1", cells: ["calm"], arms: ["a"], seeds: [1] }
        )
      ),
      /local/
    );
  });

  test("a matrix's env", () => {
    assert.deepEqual(
      matrixEnv(
        '# x\n[arms]\na = "b"\n[env]\nSIM_A = "1"\nSIM_B = "x \\"y\\""\n[axes]\nSIM_SEED = ["1"]\n'
      ),
      { SIM_A: "1", SIM_B: 'x "y"' }
    );
  });
});

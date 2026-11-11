// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The sim-results console's API: SQL over the sims' results catalog (hack/sim-db/catalog.py), its
 * schema and saved views, and the commands that bring data into it.
 *
 *   node hack/console/server.ts [--port 8086] [--connections 2] [--memory 8GB] [--threads 8]
 *                               [--no-index]
 *
 * It listens on 127.0.0.1; the dev server proxies /console to it (webpack.config.js), and the
 * devtools app's Console page (src/devtools/pages/Console.tsx) is its UI. The catalog is
 * <cache root>/lake (simdb.py's cache root: `SIM_DB_ROOT`, else the main checkout's target/sim-db),
 * attached read-only through hack/console/db.ts's guards; simdb.py index runs every 2 minutes
 * (`--no-index` for none), and after each index that succeeds the queries after it see the new
 * catalog.
 *
 *   POST /console/sql          {sql, params?, max_rows?, timeout_s?}: one SELECT (or EXPLAIN of
 *                              one), run as given, `$name`s bound from params; returns {columns:
 *                              [{name, type}], rows, truncated, elapsed_ms}
 *   GET  /console/schema       the catalog's tables and columns, table_catalog, metric_catalog,
 *                              event_kinds, experiments, catalog_info and the saved views
 *   GET  /console/views        the saved views
 *   PUT  /console/views/NAME   {title, description, params, sql, spec}: saves a view
 *   POST /console/fetch        {exp_id, cells?, arms?, tables?}: simdb.py fetch (what S3 still
 *                              holds of the experiment, into the cache, indexed); streams its
 *                              output as NDJSON lines {line}, then {done, code, s3_usd}
 *   POST /console/regenerate   {exp_id, cells, arms, seeds}: writes the suite's farm matrices for
 *                              them (suite.py farm --no-run) and returns farm.py estimate's quote
 *                              under an id; starts nothing
 *   POST /console/regenerate/ID/approve
 *                              starts farm.py run on that quote's matrices: the only way the
 *                              console starts a farm run
 *   GET  /console/regenerate/ID
 *                              a regeneration's state and its run's log
 *   POST /console/index        starts simdb.py index (or joins the one running) and returns
 *                              when it's done
 *   GET  /console/index        the index running and the last one, and catalog_info
 *
 * A saved view is hack/console/views/NAME.sql (one statement, with `$exp`, `$cell`, `$arm`, `$seed`,
 * `$delay`, `$min_stall_ms` and the like for the UI's pickers), NAME.vl.json (its Vega-Lite spec,
 * whose data are the statement's rows) and NAME.json ({title, description, params: [{name, kind,
 * default, label}]}). Every POST and PUT takes `Content-Type: application/json`, which a page of
 * another origin can't send without a preflight the console never answers.
 */

import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import type { IncomingMessage, Server, ServerResponse } from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

import { Lake, QueryError } from "./db.ts";
import type { Param, QueryResult } from "./db.ts";
import { Indexer, NAME, Regenerations, fetchArgs, fetchCost, run } from "./tasks.ts";
import type { Commands, Experiment, FetchArgs, RegenerateArgs } from "./tasks.ts";

const HERE = path.dirname(fileURLToPath(import.meta.url));
/** The checkout this server is in. */
export const ROOT = path.resolve(HERE, "..", "..");

const INDEX_EVERY_MS = 2 * 60 * 1000;
const MAX_BODY = 4 << 20;
const MAX_ROWS = 200_000;
const MAX_TIMEOUT_S = 600;

export interface ConsoleOptions {
  lake: Lake;
  commands: Commands;
  /** hack/console/views */
  views: string;
  /** simdb.py index's period, ms; 0 for none. */
  indexEveryMs?: number;
}

export interface SavedView {
  name: string;
  title: string;
  description: string;
  params: { name: string; kind: string; default?: Param; label?: string }[];
  sql: string;
  spec: unknown;
  /** Notes for whoever edits the view next. */
  todo?: string;
}

/** The saved views in `dir`: NAME.json, NAME.sql and NAME.vl.json. */
export const readViews = (dir: string): SavedView[] => {
  if (!existsSync(dir)) return [];
  return readdirSync(dir)
    .filter((n) => n.endsWith(".sql"))
    .sort()
    .map((n) => {
      const name = n.slice(0, -4);
      const read = (ext: string) => {
        const p = path.join(dir, name + ext);
        return existsSync(p) ? readFileSync(p, "utf8") : null;
      };
      const meta = JSON.parse(read(".json") ?? "{}") as Partial<SavedView>;
      const spec = read(".vl.json");
      return {
        name,
        title: meta.title ?? name,
        description: meta.description ?? "",
        params: meta.params ?? [],
        todo: meta.todo,
        sql: read(".sql") ?? "",
        spec: spec ? (JSON.parse(spec) as unknown) : null,
      };
    });
};

const VIEW_NAME = /^[a-z][a-z0-9_-]{0,63}$/;

class HttpError extends Error {
  readonly status: number;

  constructor(status: number, message: string) {
    super(message);
    this.status = status;
  }
}

const send = (res: ServerResponse, status: number, body: unknown): void => {
  const text = JSON.stringify(body);
  res.writeHead(status, {
    "Content-Type": "application/json; charset=utf-8",
    "Cache-Control": "no-store",
    "X-Content-Type-Options": "nosniff",
  });
  res.end(text);
};

const readBody = (req: IncomingMessage): Promise<unknown> =>
  new Promise((resolve, reject) => {
    const type = req.headers["content-type"] ?? "";
    if (!/^application\/json\b/.test(type)) {
      reject(new HttpError(415, "the body must be application/json"));
      req.resume();
      return;
    }
    const chunks: Buffer[] = [];
    let size = 0;
    req.on("data", (c: Buffer) => {
      size += c.length;
      if (size > MAX_BODY) {
        reject(new HttpError(413, "body too large"));
        req.destroy();
        return;
      }
      chunks.push(c);
    });
    req.on("end", () => {
      try {
        resolve(JSON.parse(Buffer.concat(chunks).toString("utf8") || "{}"));
      } catch {
        reject(new HttpError(400, "the body isn't JSON"));
      }
    });
    req.on("error", reject);
  });

const isRecord = (v: unknown): v is Record<string, unknown> =>
  typeof v === "object" && v !== null && !Array.isArray(v);

const strings = (v: unknown, what: string): string[] => {
  if (v === undefined || v === null) return [];
  if (!Array.isArray(v) || !v.every((x) => typeof x === "string")) {
    throw new HttpError(400, `${what}: a list of strings`);
  }
  return v as string[];
};

const isParam = (v: unknown): v is Param =>
  v === null ||
  ["string", "number", "boolean"].includes(typeof v) ||
  (Array.isArray(v) && v.every((x) => typeof x === "string" || typeof x === "number"));

/** The console's HTTP API over `o.lake` (the module docs). */
export class Console {
  readonly lake: Lake;
  readonly commands: Commands;
  readonly views: string;
  readonly indexer: Indexer;
  readonly regenerations: Regenerations;
  private fetching: Promise<unknown> = Promise.resolve();
  private timer: NodeJS.Timeout | null = null;
  private schema: Promise<unknown> | null = null;

  constructor(o: ConsoleOptions) {
    this.lake = o.lake;
    this.commands = o.commands;
    this.views = o.views;
    this.indexer = new Indexer(o.commands, () => this.refresh());
    this.regenerations = new Regenerations(o.commands);
    if (o.indexEveryMs) {
      this.timer = setInterval(() => void this.indexer.start(), o.indexEveryMs);
      this.timer.unref();
    }
  }

  /** New connections and schema for the queries after this. */
  refresh(): void {
    this.lake.refresh();
    this.schema = null;
  }

  close(): void {
    if (this.timer) clearInterval(this.timer);
    this.lake.shutdown();
  }

  listen(port: number, host = "127.0.0.1"): Promise<Server> {
    const server = createServer((req, res) => void this.handle(req, res));
    return new Promise((resolve, reject) => {
      server.once("error", reject);
      server.listen(port, host, () => resolve(server));
    });
  }

  async handle(req: IncomingMessage, res: ServerResponse): Promise<void> {
    const url = new URL(req.url ?? "/", "http://console");
    const route = `${req.method} ${url.pathname.replace(/\/+$/, "")}`;
    try {
      let m: RegExpExecArray | null;
      if (route === "POST /console/sql") return send(res, 200, await this.sql(await readBody(req)));
      if (route === "GET /console/schema") return send(res, 200, await this.getSchema());
      if (route === "GET /console/views") return send(res, 200, readViews(this.views));
      if ((m = /^PUT \/console\/views\/([^/]+)$/.exec(route))) {
        return send(res, 200, this.saveView(decodeURIComponent(m[1]), await readBody(req)));
      }
      if (route === "POST /console/fetch") return await this.fetch(await readBody(req), res);
      if (route === "POST /console/regenerate") {
        return send(res, 200, await this.regenerate(await readBody(req)));
      }
      if ((m = /^POST \/console\/regenerate\/([\w-]+)\/approve$/.exec(route))) {
        await readBody(req);
        const r = this.regenerations.approve(m[1]);
        if (typeof r === "string") throw new HttpError(r.startsWith("no such") ? 404 : 409, r);
        return send(res, 200, r);
      }
      if ((m = /^GET \/console\/regenerate\/([\w-]+)$/.exec(route))) {
        const r = this.regenerations.get(m[1]);
        if (!r) throw new HttpError(404, "no such regeneration");
        return send(res, 200, r);
      }
      if (route === "POST /console/index") {
        await readBody(req);
        return send(res, 200, await this.indexer.start());
      }
      if (route === "GET /console/index") return send(res, 200, await this.indexState());
      throw new HttpError(404, `no ${route}`);
    } catch (e) {
      const status = e instanceof HttpError || e instanceof QueryError ? e.status : 500;
      const message = e instanceof Error ? e.message : String(e);
      if (!res.headersSent) send(res, status, { error: message });
      else res.end();
    }
  }

  private async sql(body: unknown): Promise<QueryResult> {
    if (!isRecord(body) || typeof body.sql !== "string") throw new HttpError(400, "{sql: string}");
    const params = body.params ?? {};
    if (!isRecord(params) || !Object.values(params).every(isParam)) {
      throw new HttpError(400, "params: an object of strings, numbers, booleans or lists");
    }
    const maxRows = Math.min(MAX_ROWS, Math.max(0, Math.floor(Number(body.max_rows ?? 10_000))));
    const timeout = Math.min(MAX_TIMEOUT_S, Math.max(0.1, Number(body.timeout_s ?? 30)));
    if (!Number.isFinite(maxRows) || !Number.isFinite(timeout)) {
      throw new HttpError(400, "max_rows and timeout_s: numbers");
    }
    return this.lake.query(body.sql, params as Record<string, Param>, maxRows, timeout);
  }

  private rows = async (
    sql: string,
    params: Record<string, Param> = {}
  ): Promise<Record<string, unknown>[]> => {
    const r = await this.lake.query(sql, params, MAX_ROWS, 60);
    return r.rows.map((row) => Object.fromEntries(r.columns.map((c, i) => [c.name, row[i]])));
  };

  private getSchema(): Promise<unknown> {
    if (!this.schema) {
      this.schema = (async () => {
        const columns = await this.rows(
          "SELECT table_schema AS schema, table_name AS name, " +
            "list({name: column_name, type: data_type} ORDER BY ordinal_position) AS columns " +
            "FROM information_schema.columns WHERE table_catalog = 'lake' GROUP BY ALL ORDER BY ALL"
        );
        return {
          tables: columns,
          table_catalog: await this.rows("SELECT * FROM table_catalog ORDER BY name"),
          metric_catalog: await this.rows(
            "SELECT * EXCLUDE (first_seen, last_seen) FROM metric_catalog ORDER BY tbl, name"
          ),
          event_kinds: await this.rows("SELECT * FROM event_kinds ORDER BY sim, kind"),
          experiments: await this.rows(
            "SELECT exp_id, kind, started_at, tiers, arm_labels, base, question, cells, jobs " +
              "FROM experiments ORDER BY started_at DESC NULLS LAST, exp_id"
          ),
          catalog_info: await this.rows("SELECT * FROM catalog_info"),
          views: readViews(this.views),
        };
      })();
      this.schema.catch(() => (this.schema = null));
    }
    return this.schema;
  }

  private saveView(name: string, body: unknown): SavedView {
    if (!VIEW_NAME.test(name)) throw new HttpError(400, `a view's name is ${VIEW_NAME.source}`);
    if (!isRecord(body) || typeof body.sql !== "string" || !body.sql.trim()) {
      throw new HttpError(400, "{sql, spec?, title?, description?, params?}");
    }
    const meta = {
      title: typeof body.title === "string" ? body.title : name,
      description: typeof body.description === "string" ? body.description : "",
      params: Array.isArray(body.params) ? (body.params as unknown[]) : [],
    };
    mkdirSync(this.views, { recursive: true });
    const base = path.join(this.views, name);
    writeFileSync(base + ".sql", body.sql.endsWith("\n") ? body.sql : body.sql + "\n");
    writeFileSync(base + ".json", JSON.stringify(meta, null, 2) + "\n");
    if (body.spec !== undefined && body.spec !== null) {
      writeFileSync(base + ".vl.json", JSON.stringify(body.spec, null, 2) + "\n");
    }
    this.schema = null;
    const saved = readViews(this.views).find((v) => v.name === name);
    if (!saved) throw new HttpError(500, "the view didn't save");
    return saved;
  }

  private async fetch(body: unknown, res: ServerResponse): Promise<void> {
    if (!isRecord(body) || typeof body.exp_id !== "string") throw new HttpError(400, "{exp_id}");
    const a: FetchArgs = {
      exp_id: body.exp_id,
      cells: strings(body.cells, "cells"),
      arms: strings(body.arms, "arms"),
      tables: strings(body.tables, "tables"),
    };
    const args = fetchArgs(this.commands, a);
    if (typeof args === "string") throw new HttpError(400, args);
    res.writeHead(200, {
      "Content-Type": "application/x-ndjson; charset=utf-8",
      "Cache-Control": "no-store",
      "X-Content-Type-Options": "nosniff",
    });
    const line = (o: unknown) => res.write(JSON.stringify(o) + "\n");
    // One fetch at a time: each indexes what it fetched under the index's lock.
    const prior = this.fetching;
    let done: () => void = () => undefined;
    this.fetching = new Promise<void>((r) => (done = r));
    try {
      await prior;
      line({ line: `$ simdb.py ${args.slice(1).join(" ")}` });
      const f = await run(this.commands, args, (l) => line({ line: l }));
      if (f.code === 0) this.refresh();
      line({ done: true, code: f.code, s3_usd: fetchCost(f.output), elapsed_ms: f.elapsed_ms });
    } finally {
      done();
      res.end();
    }
  }

  private async regenerate(body: unknown): Promise<unknown> {
    if (!isRecord(body) || typeof body.exp_id !== "string" || !NAME.test(body.exp_id)) {
      throw new HttpError(400, "{exp_id, cells, arms, seeds}");
    }
    const seeds = body.seeds;
    if (!Array.isArray(seeds) || !seeds.every((s) => Number.isSafeInteger(s))) {
      throw new HttpError(400, "seeds: a list of integers");
    }
    const args: RegenerateArgs = {
      exp_id: body.exp_id,
      cells: strings(body.cells, "cells"),
      arms: strings(body.arms, "arms"),
      seeds: seeds as number[],
    };
    const [x] = await this.rows(
      "SELECT exp_id, kind, knob, sets, arms, base FROM experiments WHERE exp_id = $exp",
      { exp: args.exp_id }
    );
    if (!x) throw new HttpError(404, `no experiment ${args.exp_id}`);
    // Each cell's tier: the experiment's tier whose cell of that name has the env it ran with,
    // else its first tier with the cell.
    const cells = await this.rows(
      "SELECT c.cell, c.env, (SELECT t.tier FROM raw.tier_cells t " +
        "WHERE t.cell = c.cell AND list_contains(x.tiers, t.tier) " +
        "ORDER BY (t.env = c.env) DESC, t.tier LIMIT 1) AS tier " +
        "FROM experiment_cells c JOIN experiments x USING (exp_id) WHERE c.exp_id = $exp",
      { exp: args.exp_id }
    );
    const exp: Experiment = {
      exp_id: String(x.exp_id),
      kind: String(x.kind),
      knob: (x.knob as Record<string, string> | null) ?? null,
      sets: (x.sets as string[] | null) ?? null,
      arms: (x.arms as Record<string, string> | null) ?? {},
      base: (x.base as string | null) ?? null,
      tiers: Object.fromEntries(cells.filter((c) => c.tier).map((c) => [c.cell, c.tier])) as Record<
        string,
        string
      >,
    };
    const env = Object.fromEntries(cells.map((c) => [c.cell, c.env ?? {}])) as Record<
      string,
      Record<string, string>
    >;
    return this.regenerations.quote(exp, args, env);
  }

  private async indexState(): Promise<unknown> {
    let info: Record<string, unknown>[] = [];
    try {
      info = await this.rows("SELECT * FROM catalog_info");
    } catch {
      info = [];
    }
    return { running: this.indexer.current, last: this.indexer.last, catalog_info: info };
  }
}

/** The main checkout, whose target/ holds the sims' cache and whose hack/sim-farm the farm's state. */
const mainCheckout = (): string =>
  path.dirname(
    execFileSync("git", ["-C", ROOT, "rev-parse", "--path-format=absolute", "--git-common-dir"], {
      encoding: "utf8",
    }).trim()
  );

/** Where things are for a console in this checkout. */
export const defaultCommands = (): Commands => ({
  python: "python3",
  simdb: path.join(ROOT, "hack", "sim-db", "simdb.py"),
  suite: path.join(ROOT, "hack", "sim-suite", "suite.py"),
  farm: path.join(mainCheckout(), "hack", "sim-farm", "farm.py"),
  matrices: path.join(ROOT, "target", "sim-suite", "farm"),
  work: path.join(ROOT, "target", "console"),
});

/** simdb.py's cache root's lake (catalog.py's lake_root). */
export const defaultLake = (): string =>
  path.join(process.env.SIM_DB_ROOT ?? path.join(mainCheckout(), "target", "sim-db"), "lake");

const main = async (): Promise<void> => {
  const { values } = parseArgs({
    options: {
      port: { type: "string", default: "8086" },
      connections: { type: "string", default: "2" },
      memory: { type: "string", default: "8GB" },
      threads: { type: "string", default: "8" },
      "no-index": { type: "boolean", default: false },
    },
  });
  const lakeDir = defaultLake();
  const commands = defaultCommands();
  const lake = new Lake({
    lake: lakeDir,
    tmp: path.join(path.dirname(lakeDir), "tmp", "console"),
    simDb: path.join(ROOT, "hack", "sim-db"),
    connections: Number(values.connections),
    memoryLimit: values.memory,
    threads: Number(values.threads),
  });
  const c = new Console({
    lake,
    commands,
    views: path.join(HERE, "views"),
    indexEveryMs: values["no-index"] ? 0 : INDEX_EVERY_MS,
  });
  const server = await c.listen(Number(values.port));
  const addr = server.address();
  const port = typeof addr === "object" && addr ? addr.port : values.port;
  process.stderr.write(`console: http://127.0.0.1:${port}/console (catalog ${lakeDir})\n`);
  const stop = () => {
    server.close();
    c.close();
    process.exit(0);
  };
  process.on("SIGINT", stop);
  process.on("SIGTERM", stop);
};

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main();
}

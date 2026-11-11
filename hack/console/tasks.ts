// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The console's commands: hack/sim-db/simdb.py's index and fetch, and a farm regeneration's
 * hack/sim-suite/suite.py farm --no-run, hack/sim-farm/farm.py estimate and, once approved,
 * farm.py run.
 *
 * A regeneration is quoted first and runs only when its approve call names it: `quote` writes the
 * matrices and returns farm.py estimate's quote under a fresh random id, and `approve` (that id, once)
 * starts farm.py run on those matrices, detached, its output in target/console/regenerate/<id>.log.
 * Nothing else starts a farm run.
 */

import { spawn } from "node:child_process";
import { randomBytes } from "node:crypto";
import {
  closeSync,
  mkdirSync,
  openSync,
  readFileSync,
  readdirSync,
  statSync,
  writeFileSync,
} from "node:fs";
import path from "node:path";

export interface Commands {
  python: string;
  /** hack/sim-db/simdb.py */
  simdb: string;
  /** hack/sim-suite/suite.py */
  suite: string;
  /** The main checkout's hack/sim-farm/farm.py, which keeps the farm's state. */
  farm: string;
  /** Where suite.py farm writes its matrices (its target/sim-suite/farm). */
  matrices: string;
  /** The console's own state: target/console. */
  work: string;
}

export interface Finished {
  code: number | null;
  output: string[];
  elapsed_ms: number;
}

/**
 * Runs `args` (no shell), handing each line of its stdout and stderr to `onLine`.
 *
 * @param commands - where python is
 * @param args - the script and its arguments
 * @param onLine - takes each line as it comes
 * @returns its exit code and every line
 */
export const run = (
  commands: Commands,
  args: string[],
  onLine: (line: string) => void = () => undefined
): Promise<Finished> =>
  new Promise((resolve) => {
    const t0 = performance.now();
    const output: string[] = [];
    const child = spawn(commands.python, args, { stdio: ["ignore", "pipe", "pipe"] });
    const take = (stream: NodeJS.ReadableStream) => {
      let rest = "";
      stream.setEncoding("utf8");
      stream.on("data", (chunk: string) => {
        const lines = (rest + chunk).split("\n");
        rest = lines.pop() ?? "";
        for (const l of lines) {
          output.push(l);
          onLine(l);
        }
      });
      stream.on("end", () => {
        if (rest) {
          output.push(rest);
          onLine(rest);
        }
      });
    };
    take(child.stdout);
    take(child.stderr);
    child.on("error", (e) => {
      output.push(String(e));
      resolve({ code: -1, output, elapsed_ms: Math.round(performance.now() - t0) });
    });
    child.on("close", (code) => {
      resolve({ code, output, elapsed_ms: Math.round(performance.now() - t0) });
    });
  });

export interface IndexRun {
  started_at: string;
  finished_at: string | null;
  code: number | null;
  elapsed_ms: number | null;
  /** The output's last lines. */
  tail: string[];
}

/** simdb.py index, one at a time: a call while one runs joins it. */
export class Indexer {
  private readonly commands: Commands;
  private readonly onIndexed: () => void;
  private running: Promise<IndexRun> | null = null;
  current: IndexRun | null = null;
  last: IndexRun | null = null;

  constructor(commands: Commands, onIndexed: () => void) {
    this.commands = commands;
    this.onIndexed = onIndexed;
  }

  get busy(): boolean {
    return this.running !== null;
  }

  start(): Promise<IndexRun> {
    if (this.running) return this.running;
    const r: IndexRun = {
      started_at: new Date().toISOString(),
      finished_at: null,
      code: null,
      elapsed_ms: null,
      tail: [],
    };
    this.current = r;
    this.running = run(this.commands, [this.commands.simdb, "index", "--quiet"], (l) => {
      r.tail.push(l);
      if (r.tail.length > 40) r.tail.shift();
    }).then((f) => {
      r.finished_at = new Date().toISOString();
      r.code = f.code;
      r.elapsed_ms = f.elapsed_ms;
      this.last = r;
      this.current = null;
      this.running = null;
      if (f.code === 0) this.onIndexed();
      return r;
    });
    return this.running;
  }
}

/** What S3 charged a fetch, as simdb.py fetch prints it. */
export const fetchCost = (output: string[]): number | null => {
  for (const l of output) {
    const m = /S3 charged \$([0-9.]+)/.exec(l);
    if (m) return Number(m[1]);
  }
  return null;
};

export interface FetchArgs {
  exp_id: string;
  cells?: string[];
  arms?: string[];
  tables?: string[];
}

/** A name a command takes: an experiment, cell, arm or table. */
export const NAME = /^[A-Za-z0-9][A-Za-z0-9_.@:+=/-]*$/;

/**
 * simdb.py fetch's arguments for `a`.
 *
 * @param commands - where simdb.py is
 * @param a - the experiment and what of it to fetch
 * @returns the arguments, or a reason they can't be made
 */
export const fetchArgs = (commands: Commands, a: FetchArgs): string[] | string => {
  const names = [a.exp_id, ...(a.cells ?? []), ...(a.arms ?? []), ...(a.tables ?? [])];
  const bad = names.find((n) => typeof n !== "string" || !NAME.test(n));
  if (bad !== undefined) return `not a name: ${JSON.stringify(bad)}`;
  const args = [commands.simdb, "fetch", "--exp", a.exp_id];
  if (a.cells?.length) args.push("--cells", a.cells.join(","));
  if (a.arms?.length) args.push("--arms", a.arms.join(","));
  if (a.tables?.length) args.push("--tables", a.tables.join(","));
  return args;
};

/** An experiment as the catalog's `experiments` holds it, and the tiers of the cells asked for. */
export interface Experiment {
  exp_id: string;
  kind: string;
  knob: Record<string, string> | null;
  sets: string[] | null;
  /** Arm (a --refs name, or `x` for a --sets run's one commit) to commit. */
  arms: Record<string, string>;
  base: string | null;
  /** Cell to the tier suite.py runs it in. */
  tiers: Record<string, string>;
}

export interface RegenerateArgs {
  exp_id: string;
  cells: string[];
  arms: string[];
  seeds: number[];
}

/**
 * The suite.py farm --no-run invocations that write `r`'s matrices: one per tier.
 *
 * @param commands - where suite.py is
 * @param exp - the experiment regenerated
 * @param r - its cells, arms and seeds to run again
 * @returns the invocations, or why there are none
 */
export const suiteArgs = (
  commands: Commands,
  exp: Experiment,
  r: RegenerateArgs
): string[][] | string => {
  if (exp.kind !== "farm") return `${exp.exp_id} is a local invocation; only farm runs regenerate`;
  if (!r.cells.length || !r.arms.length || !r.seeds.length)
    return "cells, arms and seeds, one each at least";
  const bad = [...r.cells, ...r.arms].find((n) => typeof n !== "string" || !NAME.test(n));
  if (bad !== undefined) return `not a name: ${JSON.stringify(bad)}`;
  const seeds = [...new Set(r.seeds)].sort((a, b) => a - b);
  if (!seeds.every((s, i) => Number.isSafeInteger(s) && s === seeds[0] + i)) {
    return "suite.py farm runs consecutive seeds: give a range";
  }
  const missing = r.cells.filter((c) => !exp.tiers[c]);
  if (missing.length) return `no tier of ${exp.exp_id} has ${missing.join(", ")}`;
  let arms: string[];
  if (exp.knob) {
    const sets = exp.sets ?? [];
    const other = r.arms.filter((a) => !sets.includes(a));
    if (other.length) return `${other.join(", ")}: not a set of ${exp.exp_id} (${sets.join(", ")})`;
    const knob = `${exp.knob["ppspp"] ?? ""},${exp.knob["strims-sim"] ?? ""}`;
    const chosen = sets.filter((s) => r.arms.includes(s));
    const base = exp.base && chosen.includes(exp.base) ? exp.base : chosen[0];
    arms = [
      "--knob",
      knob,
      "--ref",
      exp.arms["x"],
      "--sets",
      chosen.filter((s) => s !== "off").join(";"),
    ];
    if (!chosen.includes("off")) arms.push("--no-off");
    arms.push("--against", base);
  } else {
    const other = r.arms.filter((a) => !exp.arms[a]);
    if (other.length) return `${other.join(", ")}: not an arm of ${exp.exp_id}`;
    // The first ref is the base.
    const order = Object.keys(exp.arms).filter((a) => r.arms.includes(a));
    if (exp.base && order.includes(exp.base))
      order.sort((a, b) => +(b === exp.base) - +(a === exp.base));
    arms = ["--refs", order.map((a) => `${a}=${exp.arms[a]}`).join(",")];
  }
  const byTier = new Map<string, string[]>();
  for (const c of r.cells) byTier.set(exp.tiers[c], [...(byTier.get(exp.tiers[c]) ?? []), c]);
  return [...byTier].map(([tier, cells]) => [
    commands.suite,
    "farm",
    "--tier",
    tier,
    "--only",
    cells.join(","),
    "--seeds",
    String(seeds.length),
    "--first-seed",
    String(seeds[0]),
    ...arms,
    "--no-run",
  ]);
};

/** The matrices suite.py farm wrote since `since` (ms) for `cells`: <run>-NN.toml, newest run first. */
export const writtenMatrices = (dir: string, since: number, cells: string[]): string[] => {
  let names: string[];
  try {
    names = readdirSync(dir).filter((n) => /^s[0-9a-f]+-\d+\.toml$/.test(n));
  } catch {
    return [];
  }
  const runs = new Map<string, string[]>();
  for (const n of names) {
    const p = path.join(dir, n);
    if (statSync(p).mtimeMs < since - 1000) continue;
    const run = n.replace(/-\d+\.toml$/, "");
    runs.set(run, [...(runs.get(run) ?? []), p]);
  }
  for (const paths of runs.values()) {
    const named = paths.map(
      (p) => /^# sim-suite farm \S+: (\S+),/.exec(readFileSync(p, "utf8"))?.[1]
    );
    if (paths.length === cells.length && cells.every((c) => named.includes(c))) return paths.sort();
  }
  return [];
};

/** A matrix's [env], as suite.py writes it. */
export const matrixEnv = (toml: string): Record<string, string> => {
  const env: Record<string, string> = {};
  const section = /^\[env\]\n([\s\S]*?)(?:^\[|$(?![\s\S]))/m.exec(toml)?.[1] ?? "";
  for (const m of section.matchAll(/^(\w+) = ("(?:[^"\\]|\\.)*")$/gm)) {
    env[m[1]] = JSON.parse(m[2]) as string;
  }
  return env;
};

export type RegenerationState = "quoted" | "failed" | "running" | "finished";

export interface Regeneration extends RegenerateArgs {
  id: string;
  state: RegenerationState;
  created_at: string;
  /** suite.py's invocations, as run. */
  commands: string[][];
  matrices: string[];
  /** farm.py estimate's output. */
  quote: string;
  /** Cells whose matrix env isn't the one the experiment ran. */
  warnings: string[];
  error?: string;
  approved_at?: string;
  pid?: number;
  log?: string;
  /** The log's last lines (`get`). */
  log_tail?: string[];
  code?: number | null;
}

/** Quoted regenerations and their runs, in target/console/regenerate/<id>.json. */
export class Regenerations {
  private readonly commands: Commands;
  private readonly dir: string;
  private readonly all = new Map<string, Regeneration>();

  constructor(commands: Commands) {
    this.commands = commands;
    this.dir = path.join(commands.work, "regenerate");
    mkdirSync(this.dir, { recursive: true });
    for (const n of readdirSync(this.dir).filter((f) => f.endsWith(".json"))) {
      try {
        const r = JSON.parse(readFileSync(path.join(this.dir, n), "utf8")) as Regeneration;
        this.all.set(r.id, r);
      } catch {
        continue;
      }
    }
  }

  get(id: string): Regeneration | undefined {
    const r = this.all.get(id);
    if (!r?.log) return r;
    let tail: string[] = [];
    try {
      tail = readFileSync(r.log, "utf8").split("\n").slice(-60);
    } catch {
      tail = [];
    }
    return { ...r, log_tail: tail };
  }

  private save(r: Regeneration): void {
    writeFileSync(path.join(this.dir, `${r.id}.json`), JSON.stringify(r, null, 2) + "\n");
  }

  /**
   * Writes the matrices for `args` (suite.py farm --no-run) and quotes them (farm.py estimate).
   *
   * @param exp - the experiment regenerated
   * @param args - its cells, arms and seeds
   * @param env - each cell's env as the experiment ran it
   * @returns the quoted regeneration, which runs only on `approve(id)`
   */
  async quote(
    exp: Experiment,
    args: RegenerateArgs,
    env: Record<string, Record<string, string>>
  ): Promise<Regeneration> {
    const id = randomBytes(9).toString("base64url");
    const r: Regeneration = {
      ...args,
      id,
      state: "quoted",
      created_at: new Date().toISOString(),
      commands: [],
      matrices: [],
      quote: "",
      warnings: [],
    };
    const invocations = suiteArgs(this.commands, exp, args);
    if (typeof invocations === "string") {
      r.state = "failed";
      r.error = invocations;
      return r;
    }
    for (const inv of invocations) {
      const since = Date.now();
      r.commands.push(inv);
      const f = await run(this.commands, inv);
      const cells = inv[inv.indexOf("--only") + 1].split(",");
      const written = writtenMatrices(this.commands.matrices, since, cells);
      if (f.code !== 0 || written.length !== cells.length) {
        r.state = "failed";
        r.error = `suite.py farm --no-run: exit ${f.code}, ${written.length} of ${
          cells.length
        } matrices\n${f.output.slice(-20).join("\n")}`;
        this.all.set(id, r);
        this.save(r);
        return r;
      }
      r.matrices.push(...written);
    }
    for (const m of r.matrices) {
      const text = readFileSync(m, "utf8");
      const cell = /^# sim-suite farm \S+: (\S+),/.exec(text)?.[1] ?? "";
      const got = matrixEnv(text);
      const want = env[cell] ?? {};
      const keys = [...new Set([...Object.keys(got), ...Object.keys(want)])].filter(
        (k) => got[k] !== want[k]
      );
      if (keys.length) {
        r.warnings.push(
          `${cell}: env differs from ${exp.exp_id}'s at ${keys
            .map((k) => `${k} (${want[k] ?? "unset"} -> ${got[k] ?? "unset"})`)
            .join(", ")}`
        );
      }
    }
    const est = await run(this.commands, [this.commands.farm, "estimate", ...r.matrices]);
    r.quote = est.output.join("\n");
    if (est.code !== 0) {
      r.state = "failed";
      r.error = `farm.py estimate: exit ${est.code}`;
    }
    this.all.set(id, r);
    this.save(r);
    return r;
  }

  /**
   * Starts farm.py run on a quoted regeneration's matrices, detached (it outlives the console).
   *
   * @param id - the id its quote returned
   * @returns the regeneration, or why it didn't start
   */
  approve(id: string): Regeneration | string {
    const r = this.all.get(id);
    if (!r) return "no such regeneration";
    if (r.state !== "quoted") return `regeneration ${id} is ${r.state}, not quoted`;
    const log = path.join(this.dir, `${id}.log`);
    const fd = openSync(log, "a");
    const child = spawn(this.commands.python, [this.commands.farm, "run", ...r.matrices], {
      detached: true,
      stdio: ["ignore", fd, fd],
    });
    closeSync(fd);
    r.state = "running";
    r.approved_at = new Date().toISOString();
    r.pid = child.pid;
    r.log = log;
    this.save(r);
    child.on("exit", (code) => {
      r.state = "finished";
      r.code = code;
      this.save(r);
    });
    child.unref();
    return r;
  }
}

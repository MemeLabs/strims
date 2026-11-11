// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

/** The sim-results console's API (hack/console/server.py), which the dev server proxies at /console. */

export type Json = string | number | boolean | null | Json[] | { [key: string]: Json };

/** A query's bound value: `$name` in the SQL takes `params.name`. */
export type Param = string | number | boolean | null | (string | number)[];

export interface Column {
  name: string;
  type: string;
}

export interface QueryResult {
  columns: Column[];
  rows: Json[][];
  truncated: boolean;
  elapsed_ms: number;
}

export interface ViewParam {
  name: string;
  /** Which picker sets it: exp, cell, arm, arm2, seed, number, text, choice. */
  kind: string;
  default?: Param;
  label?: string;
  /** A choice's options. */
  options?: string[];
}

export interface SavedView {
  name: string;
  title: string;
  description: string;
  params: ViewParam[];
  sql: string;
  spec: Record<string, unknown> | null;
  todo?: string;
}

export interface TableColumns {
  schema: string;
  name: string;
  columns: Column[];
}

export interface Schema {
  tables: TableColumns[];
  table_catalog: Record<string, Json>[];
  metric_catalog: Record<string, Json>[];
  event_kinds: Record<string, Json>[];
  experiments: Record<string, Json>[];
  catalog_info: Record<string, Json>[];
  views: SavedView[];
}

export interface Regeneration {
  id: string;
  exp_id: string;
  cells: string[];
  arms: string[];
  seeds: number[];
  state: "quoted" | "failed" | "running" | "finished";
  commands: string[][];
  matrices: string[];
  quote: string;
  /** farm.py estimate's: jobs, wall_s, cost, s3_cost and the rest. */
  estimate?: Record<string, Json> | null;
  warnings: string[];
  error?: string;
  log_tail?: string[];
}

export interface IndexRun {
  started_at: string;
  finished_at: string | null;
  code: number | null;
  elapsed_ms: number | null;
  tail: string[];
}

/** Why the console answered with an error. */
export class ApiError extends Error {
  constructor(
    message: string,
    /** The HTTP status. */
    readonly status: number
  ) {
    super(message);
  }
}

const call = async <T>(path: string, init?: RequestInit): Promise<T> => {
  const res = await fetch(path, { cache: "no-store", ...init });
  const body = (await res.json()) as unknown;
  if (!res.ok) {
    const error = (body as { error?: string }).error;
    throw new ApiError(error ?? `${path} answered ${res.status}`, res.status);
  }
  return body as T;
};

const post = <T>(path: string, body: unknown, method = "POST", signal?: AbortSignal): Promise<T> =>
  call<T>(path, {
    method,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
    signal,
  });

/** Runs one SELECT over the catalog. */
export const runSql = (
  sql: string,
  params: Record<string, Param> = {},
  opts: { maxRows?: number; timeoutS?: number; signal?: AbortSignal } = {}
): Promise<QueryResult> =>
  post<QueryResult>(
    "/console/sql",
    { sql, params, max_rows: opts.maxRows ?? 50_000, timeout_s: opts.timeoutS ?? 60 },
    "POST",
    opts.signal
  );

export const getSchema = (): Promise<Schema> => call<Schema>("/console/schema");

export const getViews = (): Promise<SavedView[]> => call<SavedView[]>("/console/views");

export const saveView = (
  name: string,
  view: Pick<SavedView, "title" | "description" | "params" | "sql" | "spec">
): Promise<SavedView> => post<SavedView>(`/console/views/${encodeURIComponent(name)}`, view, "PUT");

export interface FetchLine {
  line?: string;
  done?: boolean;
  code?: number | null;
  s3_usd?: number | null;
}

/**
 * Fetches what S3 holds of an experiment into the cache, and indexes it, handing each line of its
 * progress to `onLine`.
 *
 * @param args - the experiment, and which of its cells, arms and tables
 * @param onLine - takes each line of the command's output
 * @returns the last line: the exit code and what S3 charged
 */
export const fetchData = async (
  args: { exp_id: string; cells?: string[]; arms?: string[]; tables?: string[] },
  onLine: (l: FetchLine) => void
): Promise<FetchLine> => {
  const res = await fetch("/console/fetch", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(args),
  });
  if (!res.ok || !res.body) {
    const body = (await res.json()) as { error?: string };
    throw new ApiError(body.error ?? `fetch answered ${res.status}`, res.status);
  }
  const reader = res.body.pipeThrough(new TextDecoderStream()).getReader();
  let rest = "";
  let last: FetchLine = {};
  for (;;) {
    const { value, done } = await reader.read();
    if (done) break;
    const lines = (rest + value).split("\n");
    rest = lines.pop() ?? "";
    for (const l of lines.filter(Boolean)) {
      last = JSON.parse(l) as FetchLine;
      onLine(last);
    }
  }
  return last;
};

export const regenerate = (args: {
  exp_id: string;
  cells: string[];
  arms: string[];
  seeds: number[];
}): Promise<Regeneration> => post<Regeneration>("/console/regenerate", args);

export const approveRegeneration = (id: string): Promise<Regeneration> =>
  post<Regeneration>(`/console/regenerate/${encodeURIComponent(id)}/approve`, {});

export const getRegeneration = (id: string): Promise<Regeneration> =>
  call<Regeneration>(`/console/regenerate/${encodeURIComponent(id)}`);

export const startIndex = (): Promise<IndexRun> => post<IndexRun>("/console/index", {});

export const getIndex = (): Promise<{
  running: IndexRun | null;
  last: IndexRun | null;
  catalog_info: Record<string, Json>[];
}> => call("/console/index");

/** A statement's first rows, as an ask's `result` event gives them. */
export interface AskResult extends QueryResult {
  row_count?: number;
}

export interface AskIteration {
  type: string;
  model: string;
  input_tokens: number;
  output_tokens: number;
  cache_read_input_tokens: number;
  cache_creation_input_tokens: number;
  billed: boolean;
  usd: number;
}

/** A line of POST /console/ask's stream (simlab.ask's events). */
export type AskEvent =
  | {
      type: "start";
      conversation_id: string;
      question: number;
      model: string;
      effort: string;
      fallbacks: string;
      task_budget: number;
    }
  | { type: "thinking" | "text"; turn: number; block: number; text: string }
  | { type: "tool_start"; turn: number; block: number; id: string; name: string }
  | { type: "tool"; id: string; name: string; input: Record<string, Json> }
  | { type: "sql"; id: string; sql: string; params: Record<string, Param>; purpose: string }
  | { type: "result"; id: string; result: AskResult }
  | { type: "chart"; id: string; title: string; spec: Record<string, unknown>; sql: string; result: AskResult }
  | { type: "progress"; id: string; line: string }
  | { type: "proposal"; id: string; why: string; regeneration: Regeneration }
  | {
      type: "tool_result";
      id: string;
      name: string;
      is_error: boolean;
      preview: string;
      chars: number;
      elapsed_ms: number;
    }
  | { type: "fallback"; turn: number; from: string; to: string; category: string | null }
  | {
      type: "usage";
      turn: number;
      model: string;
      iterations: AskIteration[];
      usd: number;
      question_usd: number;
      conversation_usd: number;
      budget: { total: number; used: number };
      tokens: { input: number; output: number; cache_read: number; cache_write: number };
    }
  | { type: "refusal"; category: string | null; explanation: string | null }
  | { type: "error"; message: string }
  | { type: "ping" }
  | {
      type: "done";
      stop_reason: string;
      usd?: number;
      conversation_usd?: number;
      turns?: number;
      budget_used?: number;
      elapsed_ms?: number;
    };

/**
 * Asks a question of the catalog: Claude answers it by running queries, each step of which comes
 * to `onEvent` as it happens. Aborting `signal` cancels the question.
 *
 * @param body - the question, the conversation it follows up (none for a new one) and its task
 *   budget in tokens
 * @param onEvent - takes each event of the stream
 * @param signal - cancels the question
 */
export const askStream = async (
  body: { question: string; conversation_id?: string; task_budget?: number },
  onEvent: (e: AskEvent) => void,
  signal?: AbortSignal
): Promise<void> => {
  const res = await fetch("/console/ask", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
    signal,
  });
  if (!res.ok || !res.body) {
    const b = (await res.json()) as { error?: string };
    throw new ApiError(b.error ?? `ask answered ${res.status}`, res.status);
  }
  const reader = res.body.pipeThrough(new TextDecoderStream()).getReader();
  let rest = "";
  for (;;) {
    const { value, done } = await reader.read();
    if (done) break;
    const lines = (rest + value).split("\n");
    rest = lines.pop() ?? "";
    for (const l of lines.filter(Boolean)) onEvent(JSON.parse(l) as AskEvent);
  }
};

export interface Spend {
  days: { day: string; usd: number; questions: number; requests: number }[];
  questions: {
    day: string;
    conversation: string;
    question: number;
    text: string;
    usd: number;
    requests: number;
  }[];
  total_usd: number;
  ledger: string;
}

export const getSpend = (): Promise<Spend> => call<Spend>("/console/spend");

/** A result's rows as objects, keyed by column name (what Vega-Lite's data takes). */
export const rowObjects = (r: QueryResult): Record<string, Json>[] =>
  r.rows.map((row) => Object.fromEntries(r.columns.map((c, i) => [c.name, row[i]])));

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
  /**
   * The id of the catalog's snapshot the server had when it ran the query (POST /console/sql's;
   * null before it could read one).
   */
  snapshot?: number | null;
}

/** A snapshot of the catalog: every index that changes it makes one. */
export interface Snapshot {
  id: number;
  /** When it was made (ISO 8601). */
  at: string;
}

/**
 * Whether the catalog holds results newer than rows of snapshot `shown`: both known, and the latest
 * snapshot `latest` later. Rows of a later snapshot than `latest` (an index after the page last read
 * it, such as the page's own fetch) have nothing newer to show. A catalog made again numbers its
 * snapshots from the start: rows of the old one offer nothing until they load again.
 *
 * @param latest - the catalog's latest snapshot, as the page last read it
 * @param shown - the snapshot the rows came from
 * @returns whether to offer to load them again
 */
export const newResults = (
  latest: Snapshot | null | undefined,
  shown: number | null | undefined
): latest is Snapshot =>
  latest !== null &&
  latest !== undefined &&
  typeof shown === "number" &&
  typeof latest.id === "number" &&
  latest.id > shown;

export interface ViewParam {
  name: string;
  /** Which picker sets it: exp, cell, arm, arm2, seed, node, number, text, choice. */
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
  let last: FetchLine = {};
  await readLines<FetchLine>(res.body, (l) => {
    last = l;
    onLine(l);
  });
  return last;
};

/**
 * Hands each line of an NDJSON body to `onLine`, parsed, until the body ends.
 *
 * @param body - the response's body
 * @param onLine - takes each line
 * @returns once the body has ended
 */
const readLines = async <T>(body: NonNullable<Response["body"]>, onLine: (line: T) => void) => {
  const reader = body.pipeThrough(new TextDecoderStream()).getReader();
  let rest = "";
  for (;;) {
    const { value, done } = await reader.read();
    if (done) return;
    const lines = (rest + value).split("\n");
    rest = lines.pop() ?? "";
    for (const l of lines.filter(Boolean)) onLine(JSON.parse(l) as T);
  }
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

/** GET /console/index's answer. */
export interface IndexState {
  running: IndexRun | null;
  last: IndexRun | null;
  catalog_info: Record<string, Json>[];
  /** The catalog's latest snapshot (null before the server could read one). */
  snapshot: Snapshot | null;
}

export const getIndex = (): Promise<IndexState> => call<IndexState>("/console/index");

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
  | {
      type: "chart";
      id: string;
      title: string;
      spec: Record<string, unknown>;
      sql: string;
      result: AskResult;
    }
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
  await readLines<AskEvent>(res.body, onEvent);
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

/** How often a tab reports its address while nothing changes: the server forgets one unseen for 2 minutes. */
const TAB_EVERY_MS = 30_000;
/** Where a tab keeps its id, which a reload keeps. */
const TAB_KEY = "console-tab";
/** Where an open page marks its tab's id taken, from its first report until it goes (pagehide). */
const TAB_OPEN_KEY = "console-tab-open";
/** A tab's id, as the server takes one. */
const TAB_ID = /^[A-Za-z0-9_-]{1,64}$/;

let tab: string | null = null;

/** The part of sessionStorage a tab's id is kept in. */
export type TabStore = Pick<Storage, "getItem" | "setItem" | "removeItem">;

/**
 * The id a page takes from its tab's storage, which it then marks taken: the id kept there, unless
 * it is marked taken, else `fresh()`, which it keeps. A page unmarks its id as it goes
 * (releaseTabId), so a reload keeps the id; a duplicated tab starts with a copy of an open page's
 * storage, its id marked, and so takes one of its own.
 *
 * @param store - the tab's storage
 * @param fresh - a new random id
 * @returns the id
 */
export const tabIdIn = (store: TabStore, fresh: () => string): string => {
  const kept = store.getItem(TAB_KEY);
  const id = kept && TAB_ID.test(kept) && store.getItem(TAB_OPEN_KEY) !== kept ? kept : fresh();
  if (id !== kept) store.setItem(TAB_KEY, id);
  store.setItem(TAB_OPEN_KEY, id);
  return id;
};

/**
 * Unmarks the id a page took (tabIdIn) as the page goes, for the next page in its tab to keep.
 *
 * @param store - the tab's storage
 */
export const releaseTabId = (store: TabStore): void => {
  store.removeItem(TAB_OPEN_KEY);
};

const randomId = (): string => {
  const bytes = new Uint8Array(12);
  crypto.getRandomValues(bytes);
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
};

/**
 * This tab's id: random, kept in sessionStorage so that a reload keeps it and a duplicated tab
 * doesn't (tabIdIn; this page's alone where the storage is out of reach).
 *
 * @returns the id
 */
export const tabId = (): string => {
  if (tab !== null) return tab;
  try {
    tab = tabIdIn(sessionStorage, randomId);
    const id = tab;
    window.addEventListener("pagehide", () => releaseTabId(sessionStorage));
    // A page restored from the back-forward cache is open again.
    window.addEventListener("pageshow", (e) => {
      if (e.persisted) sessionStorage.setItem(TAB_OPEN_KEY, id);
    });
  } catch {
    // No storage: the id is this page's.
    tab ??= randomId();
  }
  return tab;
};

/** An address the console server sends this tab to (hack/console/ui.py go), and its seq. */
export interface Goto {
  seq: number;
  url: string;
}

/** A line of the tab's goto stream (GET /console/tabs/ID/goto). */
type GotoLine = ({ type: "goto" } & Goto) | { type: "ping" };

/** How long a tab waits to open its goto stream again: once it ends, once it fails, and after a server without one. */
const GOTO_AGAIN_MS = 1_000;
const GOTO_RETRY_MS = 5_000;
const GOTO_NONE_MS = 30_000;

/**
 * The seq of the last address the server sent this tab that it took (0: none), which its reports
 * carry: a seq is the goto's time in ms, so one from a server started since passes it.
 */
let took = 0;
/** What takes this tab to a console route, while startTabReports runs. */
let goTo: ((route: string) => void) | null = null;

/**
 * The console route an address names: its fragment, when that is /console or a route under it
 * (server.py console_route's test).
 *
 * @param url - the address
 * @returns its route, else null
 */
export const routeOf = (url: string): string | null => {
  const i = url.indexOf("#");
  if (i < 0) return null;
  const route = url.slice(i + 1);
  const path = route.split("?")[0];
  return path === "/console" || path.startsWith("/console/") ? route : null;
};

/**
 * What a tab that last took `last` does with an address the server sent it: nothing for one it
 * took already (its seq no later), else it takes its seq and goes to its route (none for an address
 * outside the console, which it takes without going anywhere).
 *
 * @param last - the seq of the last address the tab took
 * @param goto - the address sent
 * @returns the seq it now took and the route it goes to, or null for nothing
 */
export const gotoOf = (last: number, goto: Goto): { took: number; route: string | null } | null =>
  goto.seq > last ? { took: goto.seq, route: routeOf(goto.url) } : null;

/**
 * Takes this tab to an address the server sent it (gotoOf), then reports, which tells the server
 * it took it.
 *
 * @param goto - the address, if any
 */
const take = (goto: Goto | null | undefined): void => {
  if (!goto || !goTo) return;
  const next = gotoOf(took, goto);
  if (!next) return;
  took = next.took;
  if (next.route !== null) goTo(next.route);
  void reportTab();
};

/**
 * Reports this tab's address, whether it is focused and visible, and the last address it was sent
 * and took, to the console server (PUT /console/tabs/ID), which keeps each tab's last report for
 * hack/console/ui.py: what the user means by "my selection". An address the answer says waits for
 * the tab, it goes to: how a tab in the background takes one. A failed report is dropped.
 *
 * @returns once it's sent and answered, or failed
 */
export const reportTab = (): Promise<void> =>
  fetch(`/console/tabs/${tabId()}`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      url: window.location.href,
      focused: document.hasFocus(),
      visible: document.visibilityState === "visible",
      took,
    }),
    keepalive: true,
  })
    .then(async (res) => {
      if (res.ok) take(((await res.json()) as { goto?: Goto | null }).goto);
    })
    .catch(() => undefined);

const pause = (ms: number, signal: AbortSignal): Promise<void> =>
  new Promise((resolve) => {
    const t = setTimeout(resolve, ms);
    signal.addEventListener(
      "abort",
      () => {
        clearTimeout(t);
        resolve();
      },
      { once: true }
    );
  });

/**
 * Holds this tab's goto stream (GET /console/tabs/ID/goto) until `signal` aborts, taking each
 * address it pushes: the server pushes one as ui.py go posts it. It opens the stream again once it
 * ends.
 *
 * @param signal - what ends it
 */
const followGotos = async (signal: AbortSignal): Promise<void> => {
  while (!signal.aborted) {
    let wait = GOTO_AGAIN_MS;
    try {
      const res = await fetch(`/console/tabs/${tabId()}/goto?took=${took}`, { signal });
      if (res.ok && res.body) {
        await readLines<GotoLine>(res.body, (l) => {
          if (l.type === "goto") take(l);
        });
      } else {
        void res.body?.cancel();
        wait = res.status === 404 ? GOTO_NONE_MS : GOTO_RETRY_MS;
      }
    } catch {
      wait = GOTO_RETRY_MS;
    }
    await pause(wait, signal);
  }
};

/**
 * Reports this tab on each focus, blur and visibility change, and every 30 s (the page reports its
 * address's changes itself: reportTab), and while it is visible holds its goto stream: each address
 * hack/console/ui.py go sends it to, it goes to with `go`. A tab in the background holds no stream
 * (a browser's few connections to a host are the page's queries'); it takes an address from its
 * next report's answer, or when it's shown.
 *
 * @param go - takes the tab to a console route
 * @returns what stops the reports and the stream
 */
export const startTabReports = (go: (route: string) => void): (() => void) => {
  goTo = go;
  let following: AbortController | null = null;
  const follow = () => {
    const visible = document.visibilityState === "visible";
    if (visible && !following) {
      following = new AbortController();
      void followGotos(following.signal);
    } else if (!visible && following) {
      following.abort();
      following = null;
    }
  };
  const report = () => void reportTab();
  const shown = () => {
    follow();
    report();
  };
  window.addEventListener("focus", report);
  window.addEventListener("blur", report);
  document.addEventListener("visibilitychange", shown);
  const timer = setInterval(report, TAB_EVERY_MS);
  follow();
  return () => {
    window.removeEventListener("focus", report);
    window.removeEventListener("blur", report);
    document.removeEventListener("visibilitychange", shown);
    clearInterval(timer);
    following?.abort();
    goTo = null;
  };
};

/** A result's rows as objects, keyed by column name (what Vega-Lite's data takes). */
export const rowObjects = (r: QueryResult): Record<string, Json>[] =>
  r.rows.map((row) => Object.fromEntries(r.columns.map((c, i) => [c.name, row[i]])));

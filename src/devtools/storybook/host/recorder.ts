// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Records the errors a story makes while it is mounted: uncaught errors,
// unhandled rejections, console.error calls (React's dev warnings among
// them) and render errors its error boundary catches.

export type ErrorKind = "error" | "unhandledrejection" | "console.error" | "render";

export interface RecordedError {
  kind: ErrorKind;
  /** Message, then stack (and a render error's component stack). */
  text: string;
  /** The allow list entry that matched, if one did: "host: why" or "story: /re/". */
  allowedBy?: string;
}

interface AllowRule {
  pattern: RegExp;
  reason: string;
}

// Third-party noise any story may make, each entry with why it's noise. The
// stories so far make none: react-beautiful-dnd's dev warnings go to
// console.warn (its console.errors are setup problems, worth failing on), and
// the devtools page's failed wss://HOST/api socket is a network log, not a
// console.error.
export const HOST_ALLOW: AllowRule[] = [];

export type StoryState = "loading" | "ok" | "error";

export interface SessionSnapshot {
  route: string;
  state: StoryState;
  errors: RecordedError[];
}

const MAX_TEXT = 8000;

const NONE: RegExp[] = [];

const show = (v: unknown): string => {
  if (typeof v === "string") {
    return v;
  }
  if (v instanceof Error) {
    return errorText(v);
  }
  if (v && typeof v === "object" && typeof (v as { message?: unknown }).message === "string") {
    // The RPC host's errors are messages, not Errors.
    return `${v.constructor?.name ?? "Object"}: ${(v as { message: string }).message}`;
  }
  try {
    return JSON.stringify(v, (_, x: unknown) => (typeof x === "bigint" ? `${x}n` : x)) ?? String(v);
  } catch {
    return String(v);
  }
};

export const errorText = (e: unknown): string => {
  if (!(e instanceof Error)) {
    return e === undefined ? "(no reason)" : show(e);
  }
  const head = `${e.name}: ${e.message}`;
  const stack = e.stack ?? "";
  return stack.startsWith(head) ? stack : stack ? `${head}\n${stack}` : head;
};

// console.error's arguments as the console prints them: %s and kin from the
// first, the rest after it.
const formatConsole = (args: unknown[]): string => {
  if (typeof args[0] !== "string") {
    return args.map(show).join(" ");
  }
  const rest = args.slice(1);
  const head = args[0].replace(/%[sdifoOc%]/g, (m) => {
    if (m === "%%") {
      return "%";
    }
    if (!rest.length) {
      return m;
    }
    const v = rest.shift();
    return m === "%c" ? "" : show(v);
  });
  return [head, ...rest.map(show)].join(" ");
};

export class Session {
  readonly route: string;
  private errors: RecordedError[] = [];
  private storyAllow: RegExp[] = NONE;
  private mounted = false;
  private crashed = false;
  private listeners = new Set<() => void>();
  private snapshot: SessionSnapshot;
  private notifying = false;

  constructor(route: string) {
    this.route = route;
    this.snapshot = { route, state: "loading", errors: [] };
  }

  private allowedBy(text: string): string | undefined {
    const host = HOST_ALLOW.find(({ pattern }) => pattern.test(text));
    if (host) {
      return `host: ${host.reason}`;
    }
    const story = this.storyAllow.find((pattern) => pattern.test(text));
    return story ? `story: ${String(story)}` : undefined;
  }

  // Listeners hear of changes in a task of their own: errors are recorded
  // during renders, where setting another component's state is itself an
  // error.
  private changed(): void {
    const errors = this.errors.slice();
    const state: StoryState =
      this.crashed || errors.some((e) => !e.allowedBy) ? "error" : this.mounted ? "ok" : "loading";
    this.snapshot = { route: this.route, state, errors };
    if (!this.notifying) {
      this.notifying = true;
      setTimeout(() => {
        this.notifying = false;
        this.listeners.forEach((l) => l());
      });
    }
  }

  record(kind: ErrorKind, text: string): void {
    text = text.length > MAX_TEXT ? text.substring(0, MAX_TEXT) + "…" : text;
    const error: RecordedError = { kind, text };
    const allowedBy = kind === "render" ? undefined : this.allowedBy(text);
    if (allowedBy) {
      error.allowedBy = allowedBy;
    }
    this.errors.push(error);
    this.changed();
  }

  // A render error the story's boundary caught: never allowed.
  crash(text: string): void {
    this.crashed = true;
    this.record("render", text);
  }

  // The story module's allowErrors, known once it loads.
  setStoryAllow(patterns: RegExp[] = NONE): void {
    if (patterns === this.storyAllow) {
      return;
    }
    this.storyAllow = patterns;
    this.errors = this.errors.map(({ kind, text }) => {
      const allowedBy = kind === "render" ? undefined : this.allowedBy(text);
      return allowedBy ? { kind, text, allowedBy } : { kind, text };
    });
    this.changed();
  }

  setMounted(): void {
    this.mounted = true;
    this.changed();
  }

  subscribe = (l: () => void): (() => void) => {
    this.listeners.add(l);
    return () => this.listeners.delete(l);
  };

  getSnapshot = (): SessionSnapshot => this.snapshot;
}

let active: Session | null = null;
let installed = false;
let reentered = false;

const install = (): void => {
  if (installed) {
    return;
  }
  installed = true;

  const original = console.error.bind(console) as (...args: unknown[]) => void;
  console.error = (...args: unknown[]) => {
    if (active && !reentered) {
      reentered = true;
      try {
        active.record("console.error", formatConsole(args));
      } finally {
        reentered = false;
      }
    }
    original(...args);
  };

  window.addEventListener("error", (e: ErrorEvent) =>
    active?.record(
      "error",
      e.error !== undefined && e.error !== null
        ? errorText(e.error)
        : `${e.message} (${e.filename}:${e.lineno}:${e.colno})`
    )
  );
  window.addEventListener("unhandledrejection", (e: PromiseRejectionEvent) =>
    active?.record("unhandledrejection", errorText(e.reason))
  );
};

// Makes `session` the one errors go to, until the returned function runs.
export const activate = (session: Session): (() => void) => {
  install();
  active = session;
  return () => {
    if (active === session) {
      active = null;
    }
  };
};

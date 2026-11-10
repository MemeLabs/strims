// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useCallback, useEffect, useRef, useState } from "react";

import { useSession } from "../contexts/Session";
import { startAnonymousSession } from "./profiles";

export interface AnonymousSession {
  /** The session's client is ready. */
  ready: boolean;
  error: Error | null;
  /** Tries again after an error. */
  retry: () => void;
}

/** Starts the page's anonymous session ([`startAnonymousSession`]) once the stored profiles load. */
export const useAnonymousSession = (): AnonymousSession => {
  const [session, ops] = useSession();
  const [error, setError] = useState<Error | null>(null);
  const [attempt, setAttempt] = useState(0);
  const started = useRef(-1);

  useEffect(() => {
    if (session.loading || session.profile || started.current === attempt) return;
    started.current = attempt;
    setError(null);
    startAnonymousSession(ops).catch((e: unknown) =>
      setError(e instanceof Error ? e : new Error(String(e)))
    );
  }, [session.loading, session.profile, attempt]);

  return {
    ready: !!session.client && !!session.profile,
    error,
    retry: useCallback(() => setAttempt((n) => n + 1), []),
  };
};

// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect, useRef, useState } from "react";

import { LinkedProfile } from "../apis/strims/auth/v1/auth";
import { useSession } from "../contexts/Session";
import {
  rememberProfileId,
  rememberedProfileId,
  signInCandidates,
  throwawayPassword,
  viewerName,
} from "./profiles";

/** Sign-ups tried under fresh names before giving up. */
const SIGN_UP_ATTEMPTS = 3;

export interface AnonymousSession {
  /** The session's client is ready. */
  ready: boolean;
  error: Error | null;
}

/**
 * Signs the page's node in without asking: with a local profile it has the
 * key of (the one remembered first), else with a new anonymous one, which
 * the main UI then lists like any other.
 */
export const useAnonymousSession = (): AnonymousSession => {
  const [session, { signIn, createProfile }] = useSession();
  const [error, setError] = useState<Error | null>(null);
  const started = useRef(false);

  useEffect(() => {
    if (session.loading || session.profile || started.current) return;
    started.current = true;

    void (async () => {
      for (const p of signInCandidates(session.linkedProfiles, rememberedProfileId())) {
        if (p.credentials.case !== LinkedProfile.CredentialsCase.KEY) continue;
        if (await signIn("", { credentials: { key: p.credentials.key } })) {
          rememberProfileId(p.id);
          return;
        }
      }
      for (let i = 0; i < SIGN_UP_ATTEMPTS; i++) {
        const p = await createProfile("", {
          name: viewerName(),
          password: throwawayPassword(),
          persistLogin: true,
        });
        if (p) {
          rememberProfileId(p.id);
          return;
        }
      }
      setError(new Error("couldn't create a profile"));
    })();
  }, [session.loading, session.profile]);

  return {
    ready: !!session.client && !!session.profile,
    error,
  };
};

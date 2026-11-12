// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { ReactNode, useEffect, useMemo, useRef, useState } from "react";

import { LinkedProfile } from "../../../apis/strims/auth/v1/auth";
import { Ops, SessionContext, State } from "../../../contexts/Session";
import { fixtures } from "../../storybook";

/** How long a sign-in or sign-up takes, loading meanwhile. */
const OP_DELAY_MS = 1500;

export interface StorySessionProps {
  /** Fields over a signed-out session with no linked profiles. */
  state?: Partial<State>;
  /** What a sign-in or sign-up ends in: this error, else signed in as the fixture profile. */
  failWith?: Error;
  children: ReactNode;
}

// A SessionContext of the story's own, without the real provider's client or
// IndexedDB: its ops load for a moment, then sign in or fail.
export const StorySession: React.FC<StorySessionProps> = ({ state, failWith, children }) => {
  const [session, setSession] = useState<State>(() => ({
    linkedProfiles: [],
    profile: null,
    loading: false,
    ...state,
  }));
  const timers = useRef(new Set<ReturnType<typeof setTimeout>>());
  useEffect(() => () => timers.current.forEach(clearTimeout), []);

  const ops = useMemo<Ops>(() => {
    const run = <T,>(signedIn: T, failed: T) =>
      new Promise<T>((resolve) => {
        setSession((prev) => ({ ...prev, loading: true, error: null }));
        const timer = setTimeout(() => {
          timers.current.delete(timer);
          setSession((prev) =>
            failWith
              ? { ...prev, loading: false, error: failWith }
              : { ...prev, loading: false, profile: fixtures.profile() }
          );
          resolve(failWith ? failed : signedIn);
        }, OP_DELAY_MS);
        timers.current.add(timer);
      });
    return {
      signIn: () => run(true, false),
      createProfile: (serverAddress, { name }) =>
        run(new LinkedProfile({ id: BigInt(1), name, serverAddress }), null),
      loadLinkedProfiles: () => Promise.resolve(state?.linkedProfiles ?? []),
    };
  }, []);

  const value = useMemo<[State, Ops]>(() => [session, ops], [session]);
  return <SessionContext.Provider value={value}>{children}</SessionContext.Provider>;
};

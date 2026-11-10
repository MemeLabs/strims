// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import { LinkedProfile } from "../apis/strims/auth/v1/auth";
import { validNamePattern } from "../lib/validation";
import { SessionOps, signInCandidates, startAnonymousSession, viewerName } from "./profiles";

const keyed = (id: number, serverAddress = "") =>
  new LinkedProfile({
    id: BigInt(id),
    name: `p${id}`,
    serverAddress,
    credentials: { key: { profileId: BigInt(id), profileKey: new Uint8Array([id]) } },
  });

const ids = (ps: LinkedProfile[]) => ps.map((p) => Number(p.id));

describe("watch profiles", () => {
  it("names viewers validly whatever the random bytes", () => {
    for (const fill of [0, 35, 36, 255]) {
      const name = viewerName(new Uint8Array(10).fill(fill));
      assert.match(name, validNamePattern);
      assert.match(name, /^viewer_[a-z0-9]{10}$/);
    }
    assert.match(viewerName(), validNamePattern);
  });

  it("tries the remembered profile first, then the newest, local keys only", () => {
    const profiles = [
      keyed(1),
      keyed(4),
      keyed(2),
      keyed(5, "wss://node.example/manage"),
      new LinkedProfile({ id: BigInt(6), name: "p6", credentials: { password: {} } }),
    ];
    assert.deepEqual(ids(signInCandidates(profiles, null)), [4, 2, 1]);
    assert.deepEqual(ids(signInCandidates(profiles, "1")), [1, 4, 2]);
    assert.deepEqual(ids(signInCandidates(profiles, "5")), [4, 2, 1]);
    assert.deepEqual(ids(signInCandidates([], "1")), []);
  });
});

// One browser's stored profiles, which every tab's session reads, and the
// sign-ups and sign-ins its tabs made.
const browser = () => {
  const stored: LinkedProfile[] = [];
  const created: string[] = [];
  const signedIn: number[] = [];
  const ops: SessionOps = {
    loadLinkedProfiles: () => Promise.resolve([...stored]),
    signIn: (_, req) => {
      const key = req.credentials && "key" in req.credentials ? req.credentials.key : null;
      signedIn.push(Number(key?.profileId));
      return Promise.resolve(true);
    },
    createProfile: async (_, req) => {
      // A sign-up takes a while (a key derivation in the node).
      await new Promise((resolve) => setTimeout(resolve, 5));
      const p = keyed(stored.length + 1);
      stored.push(p);
      created.push(req.name);
      return p;
    },
  };
  return { ops, created, signedIn };
};

// A lock tabs take in turn, as the browser's Web Locks do.
const turns = () => {
  let tail: Promise<unknown> = Promise.resolve();
  return <T>(f: () => Promise<T>): Promise<T> => {
    const run = tail.then(f, f);
    tail = run.catch(() => undefined);
    return run;
  };
};

describe("watch session start", () => {
  it("signs tabs opened together into the one profile the first makes", async () => {
    const { ops, created, signedIn } = browser();
    const lock = turns();
    await Promise.all([startAnonymousSession(ops, lock), startAnonymousSession(ops, lock)]);
    assert.equal(created.length, 1);
    assert.match(created[0], /^viewer_[a-z0-9]{10}$/);
    assert.deepEqual(signedIn, [1]);
  });

  it("rejects when the node or the store throws, so the page can offer a retry", async () => {
    const { ops } = browser();
    await assert.rejects(
      startAnonymousSession(
        { ...ops, createProfile: () => Promise.reject(new Error("QuotaExceededError")) },
        turns()
      ),
      /QuotaExceededError/
    );
    await assert.rejects(
      startAnonymousSession(
        { ...ops, loadLinkedProfiles: () => Promise.reject(new Error("store closed")) },
        turns()
      ),
      /store closed/
    );
  });
});

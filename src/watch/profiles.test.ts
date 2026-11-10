// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import { LinkedProfile } from "../apis/strims/auth/v1/auth";
import { validNamePattern } from "../lib/validation";
import { signInCandidates, viewerName } from "./profiles";

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

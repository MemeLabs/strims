// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import { ErrorCode } from "../apis/strims/network/v1/errors/errors";
import {
  ICreateNetworkFromInvitationRequest,
  ListNetworksResponse,
  Network,
} from "../apis/strims/network/v1/network";
import { Certificate, CertificateBody } from "../apis/strims/type/certificate";
import {
  MAX_RENAMES,
  NetworkApi,
  RenewalError,
  aliasFor,
  ensureMember,
  joinNetwork,
} from "./membership";

const cert = (key: number[], parent?: Certificate) =>
  new Certificate({
    body: CertificateBody.encode(new CertificateBody({ key: new Uint8Array(key) })).finish(),
    parent,
  });

// A member's network record: its certificate under the network's root.
const network = (rootKey: number[]) => new Network({ certificate: cert([0xaa], cert(rootKey)) });

const fakeApi = (held: Network[]) => {
  const joined: ICreateNetworkFromInvitationRequest[] = [];
  const api: NetworkApi = {
    list: () => Promise.resolve(new ListNetworksResponse({ networks: held })),
    createNetworkFromInvitation: (req) => {
      joined.push(req);
      return Promise.resolve({});
    },
  };
  return { api, joined };
};

describe("watch membership", () => {
  it("joins a network it doesn't hold with the vended invitation", async () => {
    const { api, joined } = fakeApi([network([1])]);
    const invitation = new Uint8Array([7, 7]);
    let asked = 0;
    const did = await ensureMember(api, new Uint8Array([2]), () => {
      asked++;
      return Promise.resolve(invitation);
    });
    assert.equal(did, true);
    assert.equal(asked, 1);
    assert.equal(joined.length, 1);
    assert.equal(joined[0].alias, "");
    assert.deepEqual(joined[0].invitation, { invitationBytes: invitation });
  });

  it("asks for nothing when the node holds the network", async () => {
    const { api, joined } = fakeApi([network([1]), network([2])]);
    const did = await ensureMember(api, new Uint8Array([2]), () =>
      Promise.reject(new Error("asked"))
    );
    assert.equal(did, false);
    assert.equal(joined.length, 0);
  });

  it("fails as the invitation fails, joining nothing", async () => {
    const { api, joined } = fakeApi([]);
    await assert.rejects(
      ensureMember(api, new Uint8Array([2]), () => Promise.reject(new Error("404"))),
      /404/
    );
    assert.equal(joined.length, 0);
  });
});

// A joined record: provisional (viewer, invitation, owner, root) until the
// root signs it, its last renewal's error with it.
const joinedRecord = (certified: boolean, error = ErrorCode.UNDEFINED) =>
  new Network({
    certificate: certified
      ? cert([0xaa], cert([2]))
      : cert([0xaa], cert([0xbb], cert([0xcc], cert([2])))),
    certificateRenewalError: error,
  });

// The network frontend as a join sees it: no record until the first join,
// then each list answers the next of `records`, the last one repeating.
const joiningApi = (records: Network[]) => {
  const joined: ICreateNetworkFromInvitationRequest[] = [];
  let reads = 0;
  const api: NetworkApi = {
    list: () => {
      const networks = joined.length ? [records[Math.min(reads++, records.length - 1)]] : [];
      return Promise.resolve(new ListNetworksResponse({ networks }));
    },
    createNetworkFromInvitation: (req) => {
      joined.push(req);
      return Promise.resolve({});
    },
  };
  return { api, joined };
};

const join = (api: NetworkApi) => {
  let member = false;
  const done = joinNetwork(api, new Uint8Array([2]), {
    invite: () => Promise.resolve(new Uint8Array([7])),
    name: "alice",
    onMember: () => {
      member = true;
    },
    sleep: () => Promise.resolve(),
    signal: new AbortController().signal,
  });
  return { done, member: () => member };
};

describe("watch join", () => {
  it("joins under a fresh alias when the network's CA finds the name taken", async () => {
    const { api, joined } = joiningApi([
      joinedRecord(false),
      joinedRecord(false, ErrorCode.ALIAS_IN_USE),
      joinedRecord(false),
      joinedRecord(true),
    ]);
    const { done, member } = join(api);
    await done;
    assert.equal(member(), true);
    assert.deepEqual(
      joined.map((j) => j.alias.replace(/_[a-z0-9]{4}$/, "_****")),
      ["", "alice_****"]
    );
  });

  it("fails on any other refusal, and once fresh aliases run out", async () => {
    const refused = joiningApi([joinedRecord(false, ErrorCode.INVITATION_QUOTA_EXCEEDED)]);
    await assert.rejects(join(refused.api).done, RenewalError);
    assert.equal(refused.joined.length, 1);

    const taken = joiningApi([joinedRecord(false, ErrorCode.ALIAS_IN_USE)]);
    await assert.rejects(join(taken.api).done, RenewalError);
    assert.equal(taken.joined.length, 1 + MAX_RENAMES);
  });

  it("asks nothing of a network the node is already certified in", async () => {
    const { api, joined } = fakeApi([network([2])]);
    const { done, member } = join(api);
    await done;
    assert.equal(member(), true);
    assert.equal(joined.length, 0);
  });

  it("makes aliases validNamePattern takes from any name", () => {
    const random = new Uint8Array([0, 1, 2, 35]);
    assert.equal(aliasFor("alice", random), "alice_abc9");
    assert.equal(aliasFor("a_very_long_profile_name", random), "a_very_long_pro_abc9");
    assert.equal(aliasFor("dé jà", random), "d__j__abc9");
    assert.equal(aliasFor("", random), "viewer_abc9");
    for (const name of ["alice", "a_very_long_profile_name", "dé jà", ""]) {
      assert.match(aliasFor(name, random), /^[A-Za-z0-9_]{3,20}$/);
    }
  });
});

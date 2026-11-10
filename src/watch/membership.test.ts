// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import {
  ICreateNetworkFromInvitationRequest,
  ListNetworksResponse,
  Network,
} from "../apis/strims/network/v1/network";
import { Certificate, CertificateBody } from "../apis/strims/type/certificate";
import { NetworkApi, ensureMember } from "./membership";

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

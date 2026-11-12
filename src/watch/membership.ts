// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { isEqual } from "lodash";

import { ErrorCode } from "../apis/strims/network/v1/errors/errors";
import {
  ICreateNetworkFromInvitationRequest,
  IListNetworksRequest,
  ListNetworksResponse,
  Network,
} from "../apis/strims/network/v1/network";
import { certificateChain } from "../lib/certificate";
import { networkKey } from "../lib/network";

/** How often a joined network's record is read until its CA certifies it. */
export const STANDING_POLL_MS = 2000;
/** Fresh aliases tried after the network's CA finds the alias taken. */
export const MAX_RENAMES = 2;

const ALIAS_ALPHABET = "abcdefghijklmnopqrstuvwxyz0123456789";
const ALIAS_SUFFIX_LENGTH = 4;
// validNamePattern's longest name, less the suffix and its separator.
const ALIAS_BASE_LENGTH = 20 - ALIAS_SUFFIX_LENGTH - 1;

/** What joining asks of the node's network frontend. */
export interface NetworkApi {
  list(req?: IListNetworksRequest): Promise<ListNetworksResponse>;
  createNetworkFromInvitation(req?: ICreateNetworkFromInvitationRequest): Promise<unknown>;
}

/**
 * Makes the node a member of the network `key` names: nothing when it is
 * one, else it joins with an invitation from `invite`. Resolves to whether
 * it joined.
 */
export const ensureMember = async (
  network: NetworkApi,
  key: Uint8Array,
  invite: () => Promise<Uint8Array>
): Promise<boolean> => {
  const { networks } = await network.list();
  if (networks.some((n) => isEqual(networkKey(n), key))) {
    return false;
  }
  const invitationBytes = await invite();
  await network.createNetworkFromInvitation({ alias: "", invitation: { invitationBytes } });
  return true;
};

/**
 * Where the node's membership of the network `key` names stands, by its
 * record: certified once the network's root signs its certificate,
 * provisional until then, or refused when its last renewal at the
 * network's CA failed.
 */
export type Standing =
  | { state: "absent" }
  | { state: "certified" }
  | { state: "provisional" }
  | { state: "refused"; code: ErrorCode };

export const standing = (networks: Network[], key: Uint8Array): Standing => {
  const n = networks.find((n) => n.certificate && isEqual(networkKey(n), key));
  if (!n) return { state: "absent" };
  if (certificateChain(n.certificate).length <= 2) return { state: "certified" };
  if (n.certificateRenewalError !== ErrorCode.UNDEFINED) {
    return { state: "refused", code: n.certificateRenewalError };
  }
  return { state: "provisional" };
};

/** An alias for `name` that another key is less likely to hold, valid under `validNamePattern`. */
export const aliasFor = (
  name: string,
  random = crypto.getRandomValues(new Uint8Array(ALIAS_SUFFIX_LENGTH))
): string => {
  const base = name.replace(/[^A-Za-z0-9_]/g, "_").slice(0, ALIAS_BASE_LENGTH) || "viewer";
  const suffix = Array.from(random, (b) => ALIAS_ALPHABET[b % ALIAS_ALPHABET.length]).join("");
  return `${base}_${suffix}`;
};

/** The network's CA refused the node's membership. */
export class RenewalError extends Error {
  constructor(readonly code: ErrorCode) {
    super(`the network refused the membership (${ErrorCode[code] ?? code})`);
  }
}

export interface JoinOptions {
  /** A fresh invitation, for each join. */
  invite: () => Promise<Uint8Array>;
  /** The profile's name, which a fresh alias builds on. */
  name: string;
  /** Called once the node holds the network, before its CA certifies it. */
  onMember: () => void;
  /** Waits `ms`, resolving early once the join is abandoned. */
  sleep: (ms: number) => Promise<void>;
  signal: AbortSignal;
}

/**
 * Makes the node a member of the network `key` names ([`ensureMember`]),
 * then follows its record until the network's CA certifies it. A join
 * stores the record alone: the CA renews it later, and refuses an alias
 * another key holds, so a refusal for that joins again under a fresh
 * alias, at most `MAX_RENAMES` times. Rejects with a `RenewalError` on any
 * other refusal, and when the record goes.
 */
export const joinNetwork = async (
  network: NetworkApi,
  key: Uint8Array,
  { invite, name, onMember, sleep, signal }: JoinOptions
): Promise<void> => {
  await ensureMember(network, key, invite);
  if (signal.aborted) return;
  onMember();
  for (let renames = 0; !signal.aborted;) {
    const s = standing((await network.list()).networks, key);
    switch (s.state) {
      case "certified":
        return;
      case "absent":
        throw new Error("the network's record is gone");
      case "refused":
        if (s.code !== ErrorCode.ALIAS_IN_USE || renames >= MAX_RENAMES) {
          throw new RenewalError(s.code);
        }
        renames++;
        await network.createNetworkFromInvitation({
          alias: aliasFor(name),
          invitation: { invitationBytes: await invite() },
        });
        break;
      case "provisional":
        await sleep(STANDING_POLL_MS);
        break;
    }
  }
};

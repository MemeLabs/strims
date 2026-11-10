// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";

import { Invitation } from "../apis/strims/network/v1/network";

/** The origin of the native node's bootstrap API: the build's, else this page's. */
export const apiOrigin = (): string => WATCH_API_URL || window.location.origin;

/** `path`, a path the bootstrap API names, as a URL. */
export const apiUrl = (path: string): string => new URL(path, apiOrigin()).href;

export const eventsUrl = (): string => apiUrl("/api/v1/directory/events");

export const invitationUrl = (networkKey: Uint8Array): string =>
  apiUrl(`/api/v1/invitation/${Base64.fromUint8Array(networkKey, true)}`);

/** Why the API gave no invitation. */
export class InvitationError extends Error {
  constructor(
    message: string,
    /** Whether asking again later may succeed. */
    readonly retryable: boolean
  ) {
    super(message);
  }
}

/**
 * The encoded `Invitation` the native node vends for `networkKey`, which
 * `createNetworkFromInvitation` takes as is.
 */
export const fetchInvitation = async (
  networkKey: Uint8Array,
  signal?: AbortSignal
): Promise<Uint8Array> => {
  const res = await fetch(invitationUrl(networkKey), { signal, cache: "no-store" });
  switch (res.status) {
    case 200:
      break;
    case 404:
      throw new InvitationError("this network takes no visitors", false);
    default:
      throw new InvitationError(`invitation answered ${res.status}`, true);
  }
  const bytes = new Uint8Array(await res.arrayBuffer());
  // A body that doesn't decode would only fail later, in the node.
  Invitation.decode(bytes);
  return bytes;
};

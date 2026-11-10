// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { isEqual } from "lodash";

import {
  ICreateNetworkFromInvitationRequest,
  IListNetworksRequest,
  ListNetworksResponse,
} from "../apis/strims/network/v1/network";
import { networkKey } from "../lib/network";

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

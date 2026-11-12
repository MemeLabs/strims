// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { PassThrough } from "stream";

import { Readable } from "@memelabs/protobuf/lib/rpc/stream";

import * as networkv1 from "../../../apis/strims/network/v1/network";
import { Network, NetworkEvent, UIConfig } from "../../../apis/strims/network/v1/network";
import { NetworkFrontendService } from "../../../apis/strims/network/v1/network_rpc";
import {
  aliasReservations as fixtureAliasReservations,
  invitation,
  network as fixtureNetwork,
  peers as fixturePeers,
} from "../../storybook/fixtures/networks";

export interface NetworkServiceOptions {
  /** How many fixture networks (1..count) it starts with; default 30. */
  count?: number;
  /** The networks it starts with, in place of the fixtures. */
  networks?: Network[];
  /** Peer counts by network id (default 2^(id-1)). */
  peerCounts?: Map<bigint, number>;
}

const errNotFound = (what: string) => new Error(`${what} not found`);

// One in-memory node's networks: unary calls change them and every watch
// stream hears of it as the node's events would say.
export default class NetworkService implements NetworkFrontendService {
  networks: Network[];
  peerCounts: Map<bigint, number>;
  peers = new Map<bigint, networkv1.Peer[]>();
  reservations = new Map<bigint, networkv1.AliasReservation[]>();
  displayOrder: bigint[];
  watchers = new Set<PassThrough>();
  nextId: number;

  constructor({ count = 30, networks, peerCounts }: NetworkServiceOptions = {}) {
    this.networks = networks ?? Array.from({ length: count }, (_, i) => fixtureNetwork(i + 1));
    this.peerCounts =
      peerCounts ??
      new Map(this.networks.map((n) => [n.id, Math.pow(2, Number(n.id) - 1)] as const));
    this.displayOrder = this.networks.map((n) => n.id);
    this.nextId = Math.max(0, ...this.networks.map((n) => Number(n.id))) + 1;
  }

  destroy(): void {
    for (const ch of this.watchers) {
      ch.destroy();
    }
    this.watchers.clear();
  }

  private find(id: bigint): Network {
    const network = this.networks.find((n) => n.id === id);
    if (!network) {
      throw errNotFound(`network ${id}`);
    }
    return network;
  }

  private emit(body: networkv1.NetworkEvent.IBody): void {
    for (const ch of this.watchers) {
      ch.push(new networkv1.WatchNetworksResponse({ event: new NetworkEvent({ body }) }));
    }
  }

  private replace(network: Network): Network {
    this.networks = this.networks.map((n) => (n.id === network.id ? network : n));
    this.emit({ networkUpdate: network });
    return network;
  }

  private add(network: Network): Network {
    this.networks = [...this.networks, network];
    this.displayOrder = [...this.displayOrder, network.id];
    this.peerCounts.set(network.id, 1);
    this.emit({ networkStart: { network, peerCount: 1 } });
    return network;
  }

  private peersOf(networkId: bigint): networkv1.Peer[] {
    if (!this.peers.has(networkId)) {
      this.peers.set(networkId, fixturePeers(networkId));
    }
    return this.peers.get(networkId);
  }

  private updatePeer(
    id: bigint,
    update: (p: networkv1.Peer) => networkv1.IPeer
  ): networkv1.Peer {
    for (const [networkId, peers] of this.peers) {
      const i = peers.findIndex((p) => p.id === id);
      if (i !== -1) {
        const peer = new networkv1.Peer(update(peers[i]));
        this.peers.set(networkId, peers.map((p, j) => (j === i ? peer : p)));
        return peer;
      }
    }
    throw errNotFound(`peer ${id}`);
  }

  createServer(req: networkv1.CreateServerRequest): networkv1.CreateServerResponse {
    const id = this.nextId++;
    const network = fixtureNetwork(id, { name: req.name, alias: req.alias, owner: true });
    if (req.icon) {
      network.serverConfig.icon = req.icon;
    }
    return new networkv1.CreateServerResponse({ network: this.add(network) });
  }

  updateServerConfig(req: networkv1.UpdateServerConfigRequest): networkv1.UpdateServerConfigResponse {
    const network = new Network({ ...this.find(req.networkId), serverConfig: req.serverConfig });
    return new networkv1.UpdateServerConfigResponse({ network: this.replace(network) });
  }

  delete(req: networkv1.DeleteNetworkRequest): networkv1.DeleteNetworkResponse {
    this.find(req.id);
    this.networks = this.networks.filter((n) => n.id !== req.id);
    this.displayOrder = this.displayOrder.filter((id) => id !== req.id);
    this.emit({ networkStop: { networkId: req.id } });
    return new networkv1.DeleteNetworkResponse();
  }

  get(req: networkv1.GetNetworkRequest): networkv1.GetNetworkResponse {
    return new networkv1.GetNetworkResponse({ network: this.find(req.id) });
  }

  list(): networkv1.ListNetworksResponse {
    return new networkv1.ListNetworksResponse({ networks: this.networks });
  }

  createInvitation(req: networkv1.CreateInvitationRequest): networkv1.CreateInvitationResponse {
    return new networkv1.CreateInvitationResponse({
      invitation: invitation(this.find(req.networkId)),
    });
  }

  createNetworkFromInvitation(
    req: networkv1.CreateNetworkFromInvitationRequest
  ): networkv1.CreateNetworkFromInvitationResponse {
    const id = this.nextId++;
    const network = fixtureNetwork(id, { alias: req.alias, owner: false });
    return new networkv1.CreateNetworkFromInvitationResponse({ network: this.add(network) });
  }

  watch(): Readable<networkv1.WatchNetworksResponse> {
    const ch = new PassThrough({ objectMode: true });
    this.watchers.add(ch);
    ch.on("close", () => this.watchers.delete(ch));

    for (const network of this.networks) {
      ch.push(
        new networkv1.WatchNetworksResponse({
          event: new NetworkEvent({
            body: {
              networkStart: { network, peerCount: this.peerCounts.get(network.id) ?? 0 },
            },
          }),
        })
      );
    }
    return ch;
  }

  updateDisplayOrder(req: networkv1.UpdateDisplayOrderRequest): networkv1.UpdateDisplayOrderResponse {
    this.displayOrder = req.networkIds;
    this.emit({ uiConfigUpdate: new UIConfig({ networkDisplayOrder: req.networkIds }) });
    return new networkv1.UpdateDisplayOrderResponse();
  }

  updateAlias(req: networkv1.UpdateAliasRequest): networkv1.UpdateAliasResponse {
    const prev = this.find(req.id);
    const network = new Network({ ...prev, alias: req.alias });
    return new networkv1.UpdateAliasResponse({ network: this.replace(network) });
  }

  getUIConfig(): networkv1.GetUIConfigResponse {
    return new networkv1.GetUIConfigResponse({
      config: new UIConfig({ networkDisplayOrder: this.displayOrder }),
    });
  }

  listPeers(req: networkv1.ListPeersRequest): networkv1.ListPeersResponse {
    this.find(req.networkId);
    return new networkv1.ListPeersResponse({ peers: this.peersOf(req.networkId) });
  }

  grantPeerInvitation(req: networkv1.GrantPeerInvitationRequest): networkv1.GrantPeerInvitationResponse {
    const peer = this.updatePeer(req.id, (p) => ({ ...p, inviteQuota: p.inviteQuota + req.count }));
    return new networkv1.GrantPeerInvitationResponse({ peer });
  }

  togglePeerBan(req: networkv1.TogglePeerBanRequest): networkv1.TogglePeerBanResponse {
    const peer = this.updatePeer(req.id, (p) => ({ ...p, isBanned: req.value }));
    return new networkv1.TogglePeerBanResponse({ peer });
  }

  resetPeerRenameCooldown(
    req: networkv1.ResetPeerRenameCooldownRequest
  ): networkv1.ResetPeerRenameCooldownResponse {
    const peer = this.updatePeer(req.id, (p) => ({ ...p, aliasChangedAt: BigInt(0) }));
    return new networkv1.ResetPeerRenameCooldownResponse({ peer });
  }

  deletePeer(req: networkv1.DeletePeerRequest): networkv1.DeletePeerResponse {
    for (const [networkId, peers] of this.peers) {
      if (peers.some((p) => p.id === req.id)) {
        this.peers.set(
          networkId,
          peers.filter((p) => p.id !== req.id)
        );
        return new networkv1.DeletePeerResponse();
      }
    }
    throw errNotFound(`peer ${req.id}`);
  }

  listAliasReservations(
    req: networkv1.ListAliasReservationsRequest
  ): networkv1.ListAliasReservationsResponse {
    this.find(req.networkId);
    if (!this.reservations.has(req.networkId)) {
      this.reservations.set(req.networkId, fixtureAliasReservations(req.networkId));
    }
    return new networkv1.ListAliasReservationsResponse({
      aliasReservations: this.reservations.get(req.networkId),
    });
  }

  resetAliasReservationCooldown(
    req: networkv1.ResetAliasReservationCooldownRequest
  ): networkv1.ResetAliasReservationCooldownResponse {
    for (const [networkId, reservations] of this.reservations) {
      if (reservations.some((r) => r.id === req.id)) {
        this.reservations.set(
          networkId,
          reservations.map((r) =>
            r.id === req.id ? new networkv1.AliasReservation({ ...r, reservedUntil: BigInt(0) }) : r
          )
        );
        return new networkv1.ResetAliasReservationCooldownResponse();
      }
    }
    throw errNotFound(`alias reservation ${req.id}`);
  }
}

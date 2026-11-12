// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import * as bootstrapv1 from "../../../apis/strims/network/v1/bootstrap/bootstrap";
import {
  BootstrapClient,
  BootstrapPeer,
  Config,
} from "../../../apis/strims/network/v1/bootstrap/bootstrap";
import {
  BootstrapFrontendService,
  UnimplementedBootstrapFrontendService,
} from "../../../apis/strims/network/v1/bootstrap/bootstrap_rpc";

const urls = [
  "wss://strims.gg/_bootstrap",
  "wss://bootstrap.example.org:8082/_bootstrap",
  "ws://192.168.1.20:8082/_bootstrap",
  "wss://eu-west.relay.example.net/_bootstrap",
];

// A websocket bootstrap client to `url`.
export const websocketClient = (id: number, url: string, insecureSkipVerifyTls = false) =>
  new BootstrapClient({
    id: BigInt(id),
    clientOptions: { websocketOptions: { url, insecureSkipVerifyTls } },
  });

// `count` websocket clients, ids from 1; the third skips TLS verification.
export const bootstrapClients = (count = 3): BootstrapClient[] =>
  Array.from({ length: count }, (_, i) =>
    websocketClient(
      i + 1,
      i < urls.length ? urls[i] : `wss://bootstrap-${i + 1}.example.org/_bootstrap`,
      i === 2
    )
  );

const labels = ["strims.gg:443", "bootstrap.example.org:8082", "192.168.1.20:8082"];

// `count` bootstrap peers, ids 101...
export const bootstrapPeers = (count = 3): BootstrapPeer[] =>
  Array.from(
    { length: count },
    (_, i) =>
      new BootstrapPeer({
        peerId: BigInt(101 + i),
        label: i < labels.length ? labels[i] : `peer-${i + 1}.example.org:8082`,
      })
  );

export interface BootstrapServiceOptions {
  /** Default bootstrapClients(3). */
  clients?: BootstrapClient[];
  /** Default bootstrapPeers(3). */
  peers?: BootstrapPeer[];
  /** Default publishing enabled. */
  config?: Config;
}

const errNotFound = (what: string) => new Error(`${what} not found`);

// The node's bootstrap config, clients and peers in memory.
export default class BootstrapService
  extends UnimplementedBootstrapFrontendService
  implements BootstrapFrontendService
{
  config: Config;
  clients: BootstrapClient[];
  peers: BootstrapPeer[];
  published: bootstrapv1.PublishNetworkToBootstrapPeerRequest[] = [];
  nextId: number;

  constructor({ clients, peers, config }: BootstrapServiceOptions = {}) {
    super();
    this.clients = clients ?? bootstrapClients();
    this.peers = peers ?? bootstrapPeers();
    this.config = config ?? new Config({ enablePublishing: true });
    this.nextId = Math.max(0, ...this.clients.map((c) => Number(c.id))) + 1;
  }

  private find(id: bigint): BootstrapClient {
    const client = this.clients.find((c) => c.id === id);
    if (!client) {
      throw errNotFound(`bootstrap client ${id}`);
    }
    return client;
  }

  private websocketOptions(
    options:
      | bootstrapv1.CreateBootstrapClientRequest.TClientOptions
      | bootstrapv1.UpdateBootstrapClientRequest.TClientOptions
  ): bootstrapv1.IBootstrapClient["clientOptions"] {
    if (!("websocketOptions" in options)) {
      throw new Error("bootstrap client options missing");
    }
    return { websocketOptions: options.websocketOptions };
  }

  getConfig(): bootstrapv1.GetConfigResponse {
    return new bootstrapv1.GetConfigResponse({ config: this.config });
  }

  setConfig(req: bootstrapv1.SetConfigRequest): bootstrapv1.SetConfigResponse {
    this.config = new Config(req.config);
    return new bootstrapv1.SetConfigResponse({ config: this.config });
  }

  createClient(
    req: bootstrapv1.CreateBootstrapClientRequest
  ): bootstrapv1.CreateBootstrapClientResponse {
    const client = new BootstrapClient({
      id: BigInt(this.nextId++),
      clientOptions: this.websocketOptions(req.clientOptions),
    });
    this.clients = [...this.clients, client];
    return new bootstrapv1.CreateBootstrapClientResponse({ bootstrapClient: client });
  }

  updateClient(
    req: bootstrapv1.UpdateBootstrapClientRequest
  ): bootstrapv1.UpdateBootstrapClientResponse {
    this.find(req.id);
    const client = new BootstrapClient({
      id: req.id,
      clientOptions: this.websocketOptions(req.clientOptions),
    });
    this.clients = this.clients.map((c) => (c.id === req.id ? client : c));
    return new bootstrapv1.UpdateBootstrapClientResponse({ bootstrapClient: client });
  }

  deleteClient(
    req: bootstrapv1.DeleteBootstrapClientRequest
  ): bootstrapv1.DeleteBootstrapClientResponse {
    this.find(req.id);
    this.clients = this.clients.filter((c) => c.id !== req.id);
    return new bootstrapv1.DeleteBootstrapClientResponse();
  }

  getClient(req: bootstrapv1.GetBootstrapClientRequest): bootstrapv1.GetBootstrapClientResponse {
    return new bootstrapv1.GetBootstrapClientResponse({ bootstrapClient: this.find(req.id) });
  }

  listClients(): bootstrapv1.ListBootstrapClientsResponse {
    return new bootstrapv1.ListBootstrapClientsResponse({ bootstrapClients: this.clients });
  }

  listPeers(): bootstrapv1.ListBootstrapPeersResponse {
    return new bootstrapv1.ListBootstrapPeersResponse({ peers: this.peers });
  }

  publishNetworkToPeer(
    req: bootstrapv1.PublishNetworkToBootstrapPeerRequest
  ): bootstrapv1.PublishNetworkToBootstrapPeerResponse {
    if (!this.peers.some((p) => p.peerId === req.peerId)) {
      throw errNotFound(`bootstrap peer ${req.peerId}`);
    }
    this.published.push(req);
    return new bootstrapv1.PublishNetworkToBootstrapPeerResponse();
  }
}

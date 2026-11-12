// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { PairingToken } from "../../../apis/strims/auth/v1/auth";
import { VersionVector } from "../../../apis/strims/dao/v1/dao";
import { BootstrapClient } from "../../../apis/strims/network/v1/bootstrap/bootstrap";
import { Network } from "../../../apis/strims/network/v1/network";
import { Device, Profile } from "../../../apis/strims/profile/v1/profile";
import * as replicationv1 from "../../../apis/strims/replication/v1/replication";
import { Checkpoint } from "../../../apis/strims/replication/v1/replication";
import {
  ReplicationFrontendService,
  UnimplementedReplicationFrontendService,
} from "../../../apis/strims/replication/v1/replication_rpc";
import { fixtureTime } from "../../storybook/fixtures/ids";
import { network } from "../../storybook/fixtures/networks";
import { profile as fixtureProfile } from "../../storybook/fixtures/profile";
import { bootstrapClients } from "../bootstrap/service";
import { devices as fixtureDevices } from "../profile/service";

// A checkpoint for each device: its replica's version, `i + 1` events in.
export const checkpoints = (devices: Device[]): Checkpoint[] =>
  devices.map(
    (d, i) =>
      new Checkpoint({
        id: d.id,
        version: new VersionVector({
          value: new Map([[d.id, BigInt(i + 1)]]),
          updatedAt: fixtureTime(300 - i * 37),
        }),
      })
  );

export interface ReplicationServiceOptions {
  profile?: Profile;
  /** What a pairing token carries; default network 1 (its server config left out, as the node does). */
  networks?: Network[];
  /** Default bootstrapClients(1). */
  bootstraps?: BootstrapClient[];
  /** Default the profile mock's devices(4). */
  devices?: Device[];
}

// The profile's replication state: device checkpoints and pairing tokens.
export default class ReplicationService
  extends UnimplementedReplicationFrontendService
  implements ReplicationFrontendService
{
  profile: Profile;
  networks: Network[];
  bootstraps: BootstrapClient[];
  devices: Device[];
  nextDeviceId = BigInt(1000);

  constructor({ profile, networks, bootstraps, devices }: ReplicationServiceOptions = {}) {
    super();
    this.profile = profile ?? fixtureProfile();
    this.networks = networks ?? [new Network({ ...network(1), serverConfig: undefined })];
    this.bootstraps = bootstraps ?? bootstrapClients(1);
    this.devices = devices ?? fixtureDevices();
  }

  // As the node: the token names the networks (all, or the one asked for),
  // the bootstraps, the devices and a new device id for the paired one.
  createPairingToken(
    req: replicationv1.CreatePairingTokenRequest
  ): replicationv1.CreatePairingTokenResponse {
    const networks = this.networks.filter((n) => !req.networkId || n.id === req.networkId);
    if (!networks.length) {
      throw new Error("invalid: pairing without a network");
    }
    const token = new PairingToken({
      auth: { id: this.profile.id, name: this.profile.name },
      profile: { ...this.profile, deviceId: this.nextDeviceId++ },
      networks,
      bootstraps: this.bootstraps.filter((b) => !req.bootstrapId || b.id === req.bootstrapId),
      devices: this.devices,
    });
    return new replicationv1.CreatePairingTokenResponse({ token });
  }

  listCheckpoints(): replicationv1.ListCheckpointsResponse {
    return new replicationv1.ListCheckpointsResponse({ checkpoints: checkpoints(this.devices) });
  }
}

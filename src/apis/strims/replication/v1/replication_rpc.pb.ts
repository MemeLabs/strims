// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { default as strims_rpc_Host, UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import type strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import type { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc.pb";

import {
  type ICreatePairingTokenRequest as strims_replication_v1_ICreatePairingTokenRequest,
  CreatePairingTokenRequest as strims_replication_v1_CreatePairingTokenRequest,
  CreatePairingTokenResponse as strims_replication_v1_CreatePairingTokenResponse,
  type IListCheckpointsRequest as strims_replication_v1_IListCheckpointsRequest,
  ListCheckpointsRequest as strims_replication_v1_ListCheckpointsRequest,
  ListCheckpointsResponse as strims_replication_v1_ListCheckpointsResponse,
} from "./replication.pb";

export interface ReplicationFrontendService {
  createPairingToken(req: strims_replication_v1_CreatePairingTokenRequest, call: strims_rpc_Call): Promise<strims_replication_v1_CreatePairingTokenResponse> | strims_replication_v1_CreatePairingTokenResponse;
  listCheckpoints(req: strims_replication_v1_ListCheckpointsRequest, call: strims_rpc_Call): Promise<strims_replication_v1_ListCheckpointsResponse> | strims_replication_v1_ListCheckpointsResponse;
}

export class UnimplementedReplicationFrontendService implements ReplicationFrontendService {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  createPairingToken(req: strims_replication_v1_CreatePairingTokenRequest, call: strims_rpc_Call): Promise<strims_replication_v1_CreatePairingTokenResponse> | strims_replication_v1_CreatePairingTokenResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  listCheckpoints(req: strims_replication_v1_ListCheckpointsRequest, call: strims_rpc_Call): Promise<strims_replication_v1_ListCheckpointsResponse> | strims_replication_v1_ListCheckpointsResponse { throw new Error("not implemented"); }
}

export const registerReplicationFrontendService = (host: strims_rpc_Service, service: ReplicationFrontendService): void => {
  host.registerMethod<strims_replication_v1_CreatePairingTokenRequest, strims_replication_v1_CreatePairingTokenResponse>("strims.replication.v1.ReplicationFrontend.CreatePairingToken", (req, call) => service.createPairingToken(req, call), strims_replication_v1_CreatePairingTokenRequest);
  host.registerMethod<strims_replication_v1_ListCheckpointsRequest, strims_replication_v1_ListCheckpointsResponse>("strims.replication.v1.ReplicationFrontend.ListCheckpoints", (req, call) => service.listCheckpoints(req, call), strims_replication_v1_ListCheckpointsRequest);
}

export class ReplicationFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public createPairingToken(req?: strims_replication_v1_ICreatePairingTokenRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_replication_v1_CreatePairingTokenResponse> {
    return this.host.expectOne(this.host.call("strims.replication.v1.ReplicationFrontend.CreatePairingToken", new strims_replication_v1_CreatePairingTokenRequest(req)), strims_replication_v1_CreatePairingTokenResponse, opts);
  }

  public listCheckpoints(req?: strims_replication_v1_IListCheckpointsRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_replication_v1_ListCheckpointsResponse> {
    return this.host.expectOne(this.host.call("strims.replication.v1.ReplicationFrontend.ListCheckpoints", new strims_replication_v1_ListCheckpointsRequest(req)), strims_replication_v1_ListCheckpointsResponse, opts);
  }
}


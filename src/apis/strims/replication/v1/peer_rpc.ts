import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  strims_replication_v1_IPeerOpenRequest,
  strims_replication_v1_PeerOpenRequest,
  strims_replication_v1_PeerOpenResponse,
  strims_replication_v1_IPeerSendEventsRequest,
  strims_replication_v1_PeerSendEventsRequest,
  strims_replication_v1_PeerSendEventsResponse,
  strims_replication_v1_IPeerAllocateProfileIDsRequest,
  strims_replication_v1_PeerAllocateProfileIDsRequest,
  strims_replication_v1_PeerAllocateProfileIDsResponse,
} from "./peer";

export interface ReplicationPeerService {
  open(req: strims_replication_v1_PeerOpenRequest, call: strims_rpc_Call): Promise<strims_replication_v1_PeerOpenResponse> | strims_replication_v1_PeerOpenResponse;
  sendEvents(req: strims_replication_v1_PeerSendEventsRequest, call: strims_rpc_Call): Promise<strims_replication_v1_PeerSendEventsResponse> | strims_replication_v1_PeerSendEventsResponse;
  allocateProfileIDs(req: strims_replication_v1_PeerAllocateProfileIDsRequest, call: strims_rpc_Call): Promise<strims_replication_v1_PeerAllocateProfileIDsResponse> | strims_replication_v1_PeerAllocateProfileIDsResponse;
}

export class UnimplementedReplicationPeerService implements ReplicationPeerService {
  open(req: strims_replication_v1_PeerOpenRequest, call: strims_rpc_Call): Promise<strims_replication_v1_PeerOpenResponse> | strims_replication_v1_PeerOpenResponse { throw new Error("not implemented"); }
  sendEvents(req: strims_replication_v1_PeerSendEventsRequest, call: strims_rpc_Call): Promise<strims_replication_v1_PeerSendEventsResponse> | strims_replication_v1_PeerSendEventsResponse { throw new Error("not implemented"); }
  allocateProfileIDs(req: strims_replication_v1_PeerAllocateProfileIDsRequest, call: strims_rpc_Call): Promise<strims_replication_v1_PeerAllocateProfileIDsResponse> | strims_replication_v1_PeerAllocateProfileIDsResponse { throw new Error("not implemented"); }
}

export const registerReplicationPeerService = (host: strims_rpc_Service, service: ReplicationPeerService): void => {
  host.registerMethod<strims_replication_v1_PeerOpenRequest, strims_replication_v1_PeerOpenResponse>("strims.replication.v1.ReplicationPeer.Open", service.open.bind(service), strims_replication_v1_PeerOpenRequest);
  host.registerMethod<strims_replication_v1_PeerSendEventsRequest, strims_replication_v1_PeerSendEventsResponse>("strims.replication.v1.ReplicationPeer.SendEvents", service.sendEvents.bind(service), strims_replication_v1_PeerSendEventsRequest);
  host.registerMethod<strims_replication_v1_PeerAllocateProfileIDsRequest, strims_replication_v1_PeerAllocateProfileIDsResponse>("strims.replication.v1.ReplicationPeer.AllocateProfileIDs", service.allocateProfileIDs.bind(service), strims_replication_v1_PeerAllocateProfileIDsRequest);
}

export class ReplicationPeerClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public open(req?: strims_replication_v1_IPeerOpenRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_replication_v1_PeerOpenResponse> {
    return this.host.expectOne(this.host.call("strims.replication.v1.ReplicationPeer.Open", new strims_replication_v1_PeerOpenRequest(req)), strims_replication_v1_PeerOpenResponse, opts);
  }

  public sendEvents(req?: strims_replication_v1_IPeerSendEventsRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_replication_v1_PeerSendEventsResponse> {
    return this.host.expectOne(this.host.call("strims.replication.v1.ReplicationPeer.SendEvents", new strims_replication_v1_PeerSendEventsRequest(req)), strims_replication_v1_PeerSendEventsResponse, opts);
  }

  public allocateProfileIDs(req?: strims_replication_v1_IPeerAllocateProfileIDsRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_replication_v1_PeerAllocateProfileIDsResponse> {
    return this.host.expectOne(this.host.call("strims.replication.v1.ReplicationPeer.AllocateProfileIDs", new strims_replication_v1_PeerAllocateProfileIDsRequest(req)), strims_replication_v1_PeerAllocateProfileIDsResponse, opts);
  }
}


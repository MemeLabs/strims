import { EventEmitter } from "events";
import * as api_pb from "./api_pb";
import { RPCHost } from "./rpc_host";

import "./types";

export default class Client extends RPCHost {
  public joinSwarm(v: api_pb.JoinSwarmRequest): Promise<api_pb.JoinSwarmResponse> {
    return this.expectOne(this.call("joinSwarm", v));
  }

  public leaveSwarm(v: api_pb.LeaveSwarmRequest): Promise<api_pb.LeaveSwarmResponse> {
    return this.expectOne(this.call("leaveSwarm", v));
  }

  public getIngressStreams(v: api_pb.GetIngressStreamsRequest): EventEmitter {
    return this.expectMany(this.call("getIngressStreams", v));
  }

  public startHLSIngress(v: api_pb.StartHLSIngressRequest): Promise<api_pb.StartHLSIngressResponse> {
    return this.expectOne(this.call("startHLSIngress", v));
  }

  public stopHLSIngress(v: api_pb.StartHLSIngressRequest): Promise<api_pb.StartHLSIngressResponse> {
    return this.expectOne(this.call("stopHLSIngress", v));
  }

  public startHLSEgress(v: api_pb.StopHLSEgressRequest): Promise<api_pb.StopHLSEgressResponse> {
    return this.expectOne(this.call("startHLSEgress", v));
  }
}

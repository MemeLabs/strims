import { EventEmitter } from "events";
import * as api_pb from "./api_pb";
import { registerType, RPCHost } from "./rpc_host";

registerType("GetIngressStreamsRequest", api_pb.GetIngressStreamsRequest);
registerType("GetIngressStreamsResponse", api_pb.GetIngressStreamsResponse);
registerType("JoinSwarmRequest", api_pb.JoinSwarmRequest);
registerType("JoinSwarmResponse", api_pb.JoinSwarmResponse);
registerType("LeaveSwarmRequest", api_pb.LeaveSwarmRequest);
registerType("LeaveSwarmResponse", api_pb.LeaveSwarmResponse);

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
}

import * as pb from "../pb";
import { RPCHost } from "../rpc/host";
import { Readable as GenericReadable } from "../rpc/stream";

export default class Video {
  constructor(private readonly host: RPCHost) {}

  public openClient(
    arg: pb.IOpenVideoClientRequest = new pb.OpenVideoClientRequest()
  ): GenericReadable<pb.VideoClientEvent> {
    return this.host.expectMany(
      this.host.call("Video/OpenClient", new pb.OpenVideoClientRequest(arg))
    );
  }
  public openServer(
    arg: pb.IOpenVideoServerRequest = new pb.OpenVideoServerRequest()
  ): Promise<pb.VideoServerOpenResponse> {
    return this.host.expectOne(
      this.host.call("Video/OpenServer", new pb.OpenVideoServerRequest(arg))
    );
  }
  public writeToServer(
    arg: pb.IWriteToVideoServerRequest = new pb.WriteToVideoServerRequest()
  ): Promise<pb.WriteToVideoServerResponse> {
    return this.host.expectOne(
      this.host.call("Video/WriteToServer", new pb.WriteToVideoServerRequest(arg))
    );
  }
  public publishSwarm(
    arg: pb.IPublishSwarmRequest = new pb.PublishSwarmRequest()
  ): Promise<pb.PublishSwarmResponse> {
    return this.host.expectOne(
      this.host.call("Video/PublishSwarm", new pb.PublishSwarmRequest(arg))
    );
  }
  public startRTMPIngress(
    arg: pb.IStartRTMPIngressRequest = new pb.StartRTMPIngressRequest()
  ): Promise<pb.StartRTMPIngressResponse> {
    return this.host.expectOne(
      this.host.call("Video/StartRTMPIngress", new pb.StartRTMPIngressRequest(arg))
    );
  }
  public startHLSEgress(
    arg: pb.IStartHLSEgressRequest = new pb.StartHLSEgressRequest()
  ): Promise<pb.StartHLSEgressResponse> {
    return this.host.expectOne(
      this.host.call("Video/StartHLSEgress", new pb.StartHLSEgressRequest(arg))
    );
  }
  public stopHLSEgress(
    arg: pb.IStopHLSEgressRequest = new pb.StopHLSEgressRequest()
  ): Promise<pb.StopHLSEgressResponse> {
    return this.host.expectOne(
      this.host.call("Video/StopHLSEgress", new pb.StopHLSEgressRequest(arg))
    );
  }
}

import * as pb from "../pb";
import { RPCHost } from "../rpc/host";
import { Readable as GenericReadable } from "../rpc/stream";

export default class VideoIngress {
  constructor(private readonly host: RPCHost) {}

  public isSupported(
    arg: pb.IVideoIngressIsSupportedRequest = new pb.VideoIngressIsSupportedRequest()
  ): Promise<pb.VideoIngressIsSupportedResponse> {
    return this.host.expectOne(
      this.host.call("VideoIngress/IsSupported", new pb.VideoIngressIsSupportedRequest(arg))
    );
  }
  public getConfig(
    arg: pb.IVideoIngressGetConfigRequest = new pb.VideoIngressGetConfigRequest()
  ): Promise<pb.VideoIngressGetConfigResponse> {
    return this.host.expectOne(
      this.host.call("VideoIngress/GetConfig", new pb.VideoIngressGetConfigRequest(arg))
    );
  }
  public setConfig(
    arg: pb.IVideoIngressSetConfigRequest = new pb.VideoIngressSetConfigRequest()
  ): Promise<pb.VideoIngressSetConfigResponse> {
    return this.host.expectOne(
      this.host.call("VideoIngress/SetConfig", new pb.VideoIngressSetConfigRequest(arg))
    );
  }
  public listStreams(
    arg: pb.IVideoIngressListStreamsRequest = new pb.VideoIngressListStreamsRequest()
  ): Promise<pb.VideoIngressListStreamsResponse> {
    return this.host.expectOne(
      this.host.call("VideoIngress/ListStreams", new pb.VideoIngressListStreamsRequest(arg))
    );
  }
  public getChannelURL(
    arg: pb.IVideoIngressGetChannelURLRequest = new pb.VideoIngressGetChannelURLRequest()
  ): Promise<pb.VideoIngressGetChannelURLResponse> {
    return this.host.expectOne(
      this.host.call("VideoIngress/GetChannelURL", new pb.VideoIngressGetChannelURLRequest(arg))
    );
  }
}

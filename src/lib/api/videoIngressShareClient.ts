import * as pb from "../pb";
import { RPCHost } from "../rpc/host";
import { Readable as GenericReadable } from "../rpc/stream";

export default class VideoIngressShare {
  constructor(private readonly host: RPCHost) {}

  public createChannel(
    arg: pb.IVideoIngressShareCreateChannelRequest = new pb.VideoIngressShareCreateChannelRequest()
  ): Promise<pb.VideoIngressShareCreateChannelResponse> {
    return this.host.expectOne(
      this.host.call(
        "VideoIngressShare/CreateChannel",
        new pb.VideoIngressShareCreateChannelRequest(arg)
      )
    );
  }
  public updateChannel(
    arg: pb.IVideoIngressShareUpdateChannelRequest = new pb.VideoIngressShareUpdateChannelRequest()
  ): Promise<pb.VideoIngressShareUpdateChannelResponse> {
    return this.host.expectOne(
      this.host.call(
        "VideoIngressShare/UpdateChannel",
        new pb.VideoIngressShareUpdateChannelRequest(arg)
      )
    );
  }
  public deleteChannel(
    arg: pb.IVideoIngressShareDeleteChannelRequest = new pb.VideoIngressShareDeleteChannelRequest()
  ): Promise<pb.VideoIngressShareDeleteChannelResponse> {
    return this.host.expectOne(
      this.host.call(
        "VideoIngressShare/DeleteChannel",
        new pb.VideoIngressShareDeleteChannelRequest(arg)
      )
    );
  }
}

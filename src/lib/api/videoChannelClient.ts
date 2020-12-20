import * as pb from "../pb";
import { RPCHost } from "../rpc/host";
import { Readable as GenericReadable } from "../rpc/stream";

export default class VideoChannel {
  constructor(private readonly host: RPCHost) {}

  public list(
    arg: pb.IVideoChannelListRequest = new pb.VideoChannelListRequest()
  ): Promise<pb.VideoChannelListResponse> {
    return this.host.expectOne(
      this.host.call("VideoChannel/List", new pb.VideoChannelListRequest(arg))
    );
  }
  public create(
    arg: pb.IVideoChannelCreateRequest = new pb.VideoChannelCreateRequest()
  ): Promise<pb.VideoChannelCreateResponse> {
    return this.host.expectOne(
      this.host.call("VideoChannel/Create", new pb.VideoChannelCreateRequest(arg))
    );
  }
  public update(
    arg: pb.IVideoChannelUpdateRequest = new pb.VideoChannelUpdateRequest()
  ): Promise<pb.VideoChannelUpdateResponse> {
    return this.host.expectOne(
      this.host.call("VideoChannel/Update", new pb.VideoChannelUpdateRequest(arg))
    );
  }
  public delete(
    arg: pb.IVideoChannelDeleteRequest = new pb.VideoChannelDeleteRequest()
  ): Promise<pb.VideoChannelDeleteResponse> {
    return this.host.expectOne(
      this.host.call("VideoChannel/Delete", new pb.VideoChannelDeleteRequest(arg))
    );
  }
}

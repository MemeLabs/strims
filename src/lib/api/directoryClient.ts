import * as pb from "../pb";
import { RPCHost } from "../rpc/host";
import { Readable as GenericReadable } from "../rpc/stream";

export default class Directory {
  constructor(private readonly host: RPCHost) {}

  public publish(
    arg: pb.IDirectoryPublishRequest = new pb.DirectoryPublishRequest()
  ): Promise<pb.DirectoryPublishResponse> {
    return this.host.expectOne(
      this.host.call("Directory/Publish", new pb.DirectoryPublishRequest(arg))
    );
  }
  public unpublish(
    arg: pb.IDirectoryUnpublishRequest = new pb.DirectoryUnpublishRequest()
  ): Promise<pb.DirectoryUnpublishResponse> {
    return this.host.expectOne(
      this.host.call("Directory/Unpublish", new pb.DirectoryUnpublishRequest(arg))
    );
  }
  public join(
    arg: pb.IDirectoryJoinRequest = new pb.DirectoryJoinRequest()
  ): Promise<pb.DirectoryJoinResponse> {
    return this.host.expectOne(this.host.call("Directory/Join", new pb.DirectoryJoinRequest(arg)));
  }
  public part(
    arg: pb.IDirectoryPartRequest = new pb.DirectoryPartRequest()
  ): Promise<pb.DirectoryPartResponse> {
    return this.host.expectOne(this.host.call("Directory/Part", new pb.DirectoryPartRequest(arg)));
  }
  public ping(
    arg: pb.IDirectoryPingRequest = new pb.DirectoryPingRequest()
  ): Promise<pb.DirectoryPingResponse> {
    return this.host.expectOne(this.host.call("Directory/Ping", new pb.DirectoryPingRequest(arg)));
  }
}

import * as pb from "../pb";
import { RPCHost } from "../rpc/host";
import { Readable as GenericReadable } from "../rpc/stream";

export default class DirectoryFrontend {
  constructor(private readonly host: RPCHost) {}

  public open(
    arg: pb.IDirectoryFrontendOpenRequest = new pb.DirectoryFrontendOpenRequest()
  ): GenericReadable<pb.DirectoryFrontendOpenResponse> {
    return this.host.expectMany(
      this.host.call("DirectoryFrontend/Open", new pb.DirectoryFrontendOpenRequest(arg))
    );
  }
  public test(
    arg: pb.IDirectoryFrontendTestRequest = new pb.DirectoryFrontendTestRequest()
  ): Promise<pb.DirectoryFrontendTestResponse> {
    return this.host.expectOne(
      this.host.call("DirectoryFrontend/Test", new pb.DirectoryFrontendTestRequest(arg))
    );
  }
}

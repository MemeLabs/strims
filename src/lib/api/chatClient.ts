import * as pb from "../pb";
import { RPCHost } from "../rpc/host";
import { Readable as GenericReadable } from "../rpc/stream";

export default class Chat {
  constructor(private readonly host: RPCHost) {}

  public createServer(
    arg: pb.ICreateChatServerRequest = new pb.CreateChatServerRequest()
  ): Promise<pb.CreateChatServerResponse> {
    return this.host.expectOne(
      this.host.call("Chat/CreateServer", new pb.CreateChatServerRequest(arg))
    );
  }
  public updateServer(
    arg: pb.IUpdateChatServerRequest = new pb.UpdateChatServerRequest()
  ): Promise<pb.UpdateChatServerResponse> {
    return this.host.expectOne(
      this.host.call("Chat/UpdateServer", new pb.UpdateChatServerRequest(arg))
    );
  }
  public deleteServer(
    arg: pb.IDeleteChatServerRequest = new pb.DeleteChatServerRequest()
  ): Promise<pb.DeleteChatServerResponse> {
    return this.host.expectOne(
      this.host.call("Chat/DeleteServer", new pb.DeleteChatServerRequest(arg))
    );
  }
  public getServer(
    arg: pb.IGetChatServerRequest = new pb.GetChatServerRequest()
  ): Promise<pb.GetChatServerResponse> {
    return this.host.expectOne(this.host.call("Chat/GetServer", new pb.GetChatServerRequest(arg)));
  }
  public listServers(
    arg: pb.IListChatServersRequest = new pb.ListChatServersRequest()
  ): Promise<pb.ListChatServersResponse> {
    return this.host.expectOne(
      this.host.call("Chat/ListServers", new pb.ListChatServersRequest(arg))
    );
  }
  public openServer(
    arg: pb.IOpenChatServerRequest = new pb.OpenChatServerRequest()
  ): GenericReadable<pb.ChatServerEvent> {
    return this.host.expectMany(
      this.host.call("Chat/OpenServer", new pb.OpenChatServerRequest(arg))
    );
  }
  public openClient(
    arg: pb.IOpenChatClientRequest = new pb.OpenChatClientRequest()
  ): GenericReadable<pb.ChatClientEvent> {
    return this.host.expectMany(
      this.host.call("Chat/OpenClient", new pb.OpenChatClientRequest(arg))
    );
  }
  public callClient(
    arg: pb.ICallChatClientRequest = new pb.CallChatClientRequest()
  ): Promise<pb.CallChatClientResponse> {
    return this.host.expectOne(
      this.host.call("Chat/CallClient", new pb.CallChatClientRequest(arg))
    );
  }
}

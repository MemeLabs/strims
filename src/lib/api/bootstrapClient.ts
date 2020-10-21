import * as pb from "../pb";
import { RPCHost } from "../rpc/host";
import { Readable as GenericReadable } from "../rpc/stream";

export default class Bootstrap {
  constructor(private readonly host: RPCHost) {}

  public createClient(
    arg: pb.ICreateBootstrapClientRequest = new pb.CreateBootstrapClientRequest()
  ): Promise<pb.CreateBootstrapClientResponse> {
    return this.host.expectOne(
      this.host.call("Bootstrap/CreateClient", new pb.CreateBootstrapClientRequest(arg))
    );
  }
  public updateClient(
    arg: pb.IUpdateBootstrapClientRequest = new pb.UpdateBootstrapClientRequest()
  ): Promise<pb.UpdateBootstrapClientResponse> {
    return this.host.expectOne(
      this.host.call("Bootstrap/UpdateClient", new pb.UpdateBootstrapClientRequest(arg))
    );
  }
  public deleteClient(
    arg: pb.IDeleteBootstrapClientRequest = new pb.DeleteBootstrapClientRequest()
  ): Promise<pb.DeleteBootstrapClientResponse> {
    return this.host.expectOne(
      this.host.call("Bootstrap/DeleteClient", new pb.DeleteBootstrapClientRequest(arg))
    );
  }
  public getClient(
    arg: pb.IGetBootstrapClientRequest = new pb.GetBootstrapClientRequest()
  ): Promise<pb.GetBootstrapClientResponse> {
    return this.host.expectOne(
      this.host.call("Bootstrap/GetClient", new pb.GetBootstrapClientRequest(arg))
    );
  }
  public listClients(
    arg: pb.IListBootstrapClientsRequest = new pb.ListBootstrapClientsRequest()
  ): Promise<pb.ListBootstrapClientsResponse> {
    return this.host.expectOne(
      this.host.call("Bootstrap/ListClients", new pb.ListBootstrapClientsRequest(arg))
    );
  }
  public listPeers(
    arg: pb.IListBootstrapPeersRequest = new pb.ListBootstrapPeersRequest()
  ): Promise<pb.ListBootstrapPeersResponse> {
    return this.host.expectOne(
      this.host.call("Bootstrap/ListPeers", new pb.ListBootstrapPeersRequest(arg))
    );
  }
  public publishNetworkToPeer(
    arg: pb.IPublishNetworkToBootstrapPeerRequest = new pb.PublishNetworkToBootstrapPeerRequest()
  ): Promise<pb.PublishNetworkToBootstrapPeerResponse> {
    return this.host.expectOne(
      this.host.call(
        "Bootstrap/PublishNetworkToPeer",
        new pb.PublishNetworkToBootstrapPeerRequest(arg)
      )
    );
  }
}

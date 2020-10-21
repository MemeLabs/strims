import * as pb from "../pb";
import { RPCHost } from "../rpc/host";
import { Readable as GenericReadable } from "../rpc/stream";

export default class Network {
  constructor(private readonly host: RPCHost) {}

  public create(
    arg: pb.ICreateNetworkRequest = new pb.CreateNetworkRequest()
  ): Promise<pb.CreateNetworkResponse> {
    return this.host.expectOne(this.host.call("Network/Create", new pb.CreateNetworkRequest(arg)));
  }
  public update(
    arg: pb.IUpdateNetworkRequest = new pb.UpdateNetworkRequest()
  ): Promise<pb.UpdateNetworkResponse> {
    return this.host.expectOne(this.host.call("Network/Update", new pb.UpdateNetworkRequest(arg)));
  }
  public delete(
    arg: pb.IDeleteNetworkRequest = new pb.DeleteNetworkRequest()
  ): Promise<pb.DeleteNetworkResponse> {
    return this.host.expectOne(this.host.call("Network/Delete", new pb.DeleteNetworkRequest(arg)));
  }
  public get(
    arg: pb.IGetNetworkRequest = new pb.GetNetworkRequest()
  ): Promise<pb.GetNetworkResponse> {
    return this.host.expectOne(this.host.call("Network/Get", new pb.GetNetworkRequest(arg)));
  }
  public list(
    arg: pb.IListNetworksRequest = new pb.ListNetworksRequest()
  ): Promise<pb.ListNetworksResponse> {
    return this.host.expectOne(this.host.call("Network/List", new pb.ListNetworksRequest(arg)));
  }
  public createInvitation(
    arg: pb.ICreateNetworkInvitationRequest = new pb.CreateNetworkInvitationRequest()
  ): Promise<pb.CreateNetworkInvitationResponse> {
    return this.host.expectOne(
      this.host.call("Network/CreateInvitation", new pb.CreateNetworkInvitationRequest(arg))
    );
  }
  public createFromInvitation(
    arg: pb.ICreateNetworkFromInvitationRequest = new pb.CreateNetworkFromInvitationRequest()
  ): Promise<pb.CreateNetworkFromInvitationResponse> {
    return this.host.expectOne(
      this.host.call("Network/CreateFromInvitation", new pb.CreateNetworkFromInvitationRequest(arg))
    );
  }
  public startVPN(
    arg: pb.IStartVPNRequest = new pb.StartVPNRequest()
  ): GenericReadable<pb.NetworkEvent> {
    return this.host.expectMany(this.host.call("Network/StartVPN", new pb.StartVPNRequest(arg)));
  }
  public stopVPN(arg: pb.IStopVPNRequest = new pb.StopVPNRequest()): Promise<pb.StopVPNResponse> {
    return this.host.expectOne(this.host.call("Network/StopVPN", new pb.StopVPNRequest(arg)));
  }
  public getDirectoryEvents(
    arg: pb.IGetDirectoryEventsRequest = new pb.GetDirectoryEventsRequest()
  ): GenericReadable<pb.DirectoryServerEvent> {
    return this.host.expectMany(
      this.host.call("Network/GetDirectoryEvents", new pb.GetDirectoryEventsRequest(arg))
    );
  }
  public testDirectoryPublish(
    arg: pb.ITestDirectoryPublishRequest = new pb.TestDirectoryPublishRequest()
  ): Promise<pb.TestDirectoryPublishResponse> {
    return this.host.expectOne(
      this.host.call("Network/TestDirectoryPublish", new pb.TestDirectoryPublishRequest(arg))
    );
  }
}

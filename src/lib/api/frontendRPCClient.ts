import * as pb from "../pb";
import { RPCHost } from "./rpc_host";
import { Readable as GenericReadable } from "./stream";

export default class FrontendRPC extends RPCHost {
  public createProfile(
    v: pb.ICreateProfileRequest = new pb.CreateProfileRequest()
  ): Promise<pb.CreateProfileResponse> {
    return this.expectOne(this.call("FrontendRPC/CreateProfile", new pb.CreateProfileRequest(v)));
  }
  public loadProfile(
    v: pb.ILoadProfileRequest = new pb.LoadProfileRequest()
  ): Promise<pb.LoadProfileResponse> {
    return this.expectOne(this.call("FrontendRPC/LoadProfile", new pb.LoadProfileRequest(v)));
  }
  public getProfile(
    v: pb.IGetProfileRequest = new pb.GetProfileRequest()
  ): Promise<pb.GetProfileResponse> {
    return this.expectOne(this.call("FrontendRPC/GetProfile", new pb.GetProfileRequest(v)));
  }
  public deleteProfile(
    v: pb.IDeleteProfileRequest = new pb.DeleteProfileRequest()
  ): Promise<pb.DeleteProfileResponse> {
    return this.expectOne(this.call("FrontendRPC/DeleteProfile", new pb.DeleteProfileRequest(v)));
  }
  public getProfiles(
    v: pb.IGetProfilesRequest = new pb.GetProfilesRequest()
  ): Promise<pb.GetProfilesResponse> {
    return this.expectOne(this.call("FrontendRPC/GetProfiles", new pb.GetProfilesRequest(v)));
  }
  public loadSession(
    v: pb.ILoadSessionRequest = new pb.LoadSessionRequest()
  ): Promise<pb.LoadSessionResponse> {
    return this.expectOne(this.call("FrontendRPC/LoadSession", new pb.LoadSessionRequest(v)));
  }
  public createNetwork(
    v: pb.ICreateNetworkRequest = new pb.CreateNetworkRequest()
  ): Promise<pb.CreateNetworkResponse> {
    return this.expectOne(this.call("FrontendRPC/CreateNetwork", new pb.CreateNetworkRequest(v)));
  }
  public deleteNetwork(
    v: pb.IDeleteNetworkRequest = new pb.DeleteNetworkRequest()
  ): Promise<pb.DeleteNetworkResponse> {
    return this.expectOne(this.call("FrontendRPC/DeleteNetwork", new pb.DeleteNetworkRequest(v)));
  }
  public getNetwork(
    v: pb.IGetNetworkRequest = new pb.GetNetworkRequest()
  ): Promise<pb.GetNetworkResponse> {
    return this.expectOne(this.call("FrontendRPC/GetNetwork", new pb.GetNetworkRequest(v)));
  }
  public getNetworks(
    v: pb.IGetNetworksRequest = new pb.GetNetworksRequest()
  ): Promise<pb.GetNetworksResponse> {
    return this.expectOne(this.call("FrontendRPC/GetNetworks", new pb.GetNetworksRequest(v)));
  }
  public getNetworkMemberships(
    v: pb.IGetNetworkMembershipsRequest = new pb.GetNetworkMembershipsRequest()
  ): Promise<pb.GetNetworkMembershipsResponse> {
    return this.expectOne(
      this.call("FrontendRPC/GetNetworkMemberships", new pb.GetNetworkMembershipsRequest(v))
    );
  }
  public deleteNetworkMembership(
    v: pb.IDeleteNetworkMembershipRequest = new pb.DeleteNetworkMembershipRequest()
  ): Promise<pb.DeleteNetworkMembershipResponse> {
    return this.expectOne(
      this.call("FrontendRPC/DeleteNetworkMembership", new pb.DeleteNetworkMembershipRequest(v))
    );
  }
  public createBootstrapClient(
    v: pb.ICreateBootstrapClientRequest = new pb.CreateBootstrapClientRequest()
  ): Promise<pb.CreateBootstrapClientResponse> {
    return this.expectOne(
      this.call("FrontendRPC/CreateBootstrapClient", new pb.CreateBootstrapClientRequest(v))
    );
  }
  public updateBootstrapClient(
    v: pb.IUpdateBootstrapClientRequest = new pb.UpdateBootstrapClientRequest()
  ): Promise<pb.UpdateBootstrapClientResponse> {
    return this.expectOne(
      this.call("FrontendRPC/UpdateBootstrapClient", new pb.UpdateBootstrapClientRequest(v))
    );
  }
  public deleteBootstrapClient(
    v: pb.IDeleteBootstrapClientRequest = new pb.DeleteBootstrapClientRequest()
  ): Promise<pb.DeleteBootstrapClientResponse> {
    return this.expectOne(
      this.call("FrontendRPC/DeleteBootstrapClient", new pb.DeleteBootstrapClientRequest(v))
    );
  }
  public getBootstrapClient(
    v: pb.IGetBootstrapClientRequest = new pb.GetBootstrapClientRequest()
  ): Promise<pb.GetBootstrapClientResponse> {
    return this.expectOne(
      this.call("FrontendRPC/GetBootstrapClient", new pb.GetBootstrapClientRequest(v))
    );
  }
  public getBootstrapClients(
    v: pb.IGetBootstrapClientsRequest = new pb.GetBootstrapClientsRequest()
  ): Promise<pb.GetBootstrapClientsResponse> {
    return this.expectOne(
      this.call("FrontendRPC/GetBootstrapClients", new pb.GetBootstrapClientsRequest(v))
    );
  }
  public createChatServer(
    v: pb.ICreateChatServerRequest = new pb.CreateChatServerRequest()
  ): Promise<pb.CreateChatServerResponse> {
    return this.expectOne(
      this.call("FrontendRPC/CreateChatServer", new pb.CreateChatServerRequest(v))
    );
  }
  public updateChatServer(
    v: pb.IUpdateChatServerRequest = new pb.UpdateChatServerRequest()
  ): Promise<pb.UpdateChatServerResponse> {
    return this.expectOne(
      this.call("FrontendRPC/UpdateChatServer", new pb.UpdateChatServerRequest(v))
    );
  }
  public deleteChatServer(
    v: pb.IDeleteChatServerRequest = new pb.DeleteChatServerRequest()
  ): Promise<pb.DeleteChatServerResponse> {
    return this.expectOne(
      this.call("FrontendRPC/DeleteChatServer", new pb.DeleteChatServerRequest(v))
    );
  }
  public getChatServer(
    v: pb.IGetChatServerRequest = new pb.GetChatServerRequest()
  ): Promise<pb.GetChatServerResponse> {
    return this.expectOne(this.call("FrontendRPC/GetChatServer", new pb.GetChatServerRequest(v)));
  }
  public getChatServers(
    v: pb.IGetChatServersRequest = new pb.GetChatServersRequest()
  ): Promise<pb.GetChatServersResponse> {
    return this.expectOne(this.call("FrontendRPC/GetChatServers", new pb.GetChatServersRequest(v)));
  }
  public startVPN(
    v: pb.IStartVPNRequest = new pb.StartVPNRequest()
  ): GenericReadable<pb.NetworkEvent> {
    return this.expectMany(this.call("FrontendRPC/StartVPN", new pb.StartVPNRequest(v)));
  }
  public stopVPN(v: pb.IStopVPNRequest = new pb.StopVPNRequest()): Promise<pb.StopVPNResponse> {
    return this.expectOne(this.call("FrontendRPC/StopVPN", new pb.StopVPNRequest(v)));
  }
  public joinSwarm(
    v: pb.IJoinSwarmRequest = new pb.JoinSwarmRequest()
  ): Promise<pb.JoinSwarmResponse> {
    return this.expectOne(this.call("FrontendRPC/JoinSwarm", new pb.JoinSwarmRequest(v)));
  }
  public leaveSwarm(
    v: pb.ILeaveSwarmRequest = new pb.LeaveSwarmRequest()
  ): Promise<pb.LeaveSwarmResponse> {
    return this.expectOne(this.call("FrontendRPC/LeaveSwarm", new pb.LeaveSwarmRequest(v)));
  }
  public startRTMPIngress(
    v: pb.IStartRTMPIngressRequest = new pb.StartRTMPIngressRequest()
  ): Promise<pb.StartRTMPIngressResponse> {
    return this.expectOne(
      this.call("FrontendRPC/StartRTMPIngress", new pb.StartRTMPIngressRequest(v))
    );
  }
  public startHLSEgress(
    v: pb.IStartHLSEgressRequest = new pb.StartHLSEgressRequest()
  ): Promise<pb.StartHLSEgressResponse> {
    return this.expectOne(this.call("FrontendRPC/StartHLSEgress", new pb.StartHLSEgressRequest(v)));
  }
  public stopHLSEgress(
    v: pb.IStopHLSEgressRequest = new pb.StopHLSEgressRequest()
  ): Promise<pb.StopHLSEgressResponse> {
    return this.expectOne(this.call("FrontendRPC/StopHLSEgress", new pb.StopHLSEgressRequest(v)));
  }
  public publishSwarm(
    v: pb.IPublishSwarmRequest = new pb.PublishSwarmRequest()
  ): Promise<pb.PublishSwarmResponse> {
    return this.expectOne(this.call("FrontendRPC/PublishSwarm", new pb.PublishSwarmRequest(v)));
  }
  public pProf(v: pb.IPProfRequest = new pb.PProfRequest()): Promise<pb.PProfResponse> {
    return this.expectOne(this.call("FrontendRPC/PProf", new pb.PProfRequest(v)));
  }
  public openChatServer(
    v: pb.IOpenChatServerRequest = new pb.OpenChatServerRequest()
  ): GenericReadable<pb.ChatServerEvent> {
    return this.expectMany(
      this.call("FrontendRPC/OpenChatServer", new pb.OpenChatServerRequest(v))
    );
  }
  public openChatClient(
    v: pb.IOpenChatClientRequest = new pb.OpenChatClientRequest()
  ): GenericReadable<pb.ChatClientEvent> {
    return this.expectMany(
      this.call("FrontendRPC/OpenChatClient", new pb.OpenChatClientRequest(v))
    );
  }
  public callChatClient(
    v: pb.ICallChatClientRequest = new pb.CallChatClientRequest()
  ): Promise<pb.CallChatClientResponse> {
    return this.expectOne(this.call("FrontendRPC/CallChatClient", new pb.CallChatClientRequest(v)));
  }
  public openVideoClient(
    v: pb.IVideoClientOpenRequest = new pb.VideoClientOpenRequest()
  ): GenericReadable<pb.VideoClientEvent> {
    return this.expectMany(
      this.call("FrontendRPC/OpenVideoClient", new pb.VideoClientOpenRequest(v))
    );
  }
  public openVideoServer(
    v: pb.IVideoServerOpenRequest = new pb.VideoServerOpenRequest()
  ): Promise<pb.VideoServerOpenResponse> {
    return this.expectOne(
      this.call("FrontendRPC/OpenVideoServer", new pb.VideoServerOpenRequest(v))
    );
  }
  public writeToVideoServer(
    v: pb.IVideoServerWriteRequest = new pb.VideoServerWriteRequest()
  ): Promise<pb.VideoServerWriteResponse> {
    return this.expectOne(
      this.call("FrontendRPC/WriteToVideoServer", new pb.VideoServerWriteRequest(v))
    );
  }
  public readMetrics(
    v: pb.IReadMetricsRequest = new pb.ReadMetricsRequest()
  ): Promise<pb.ReadMetricsResponse> {
    return this.expectOne(this.call("FrontendRPC/ReadMetrics", new pb.ReadMetricsRequest(v)));
  }
  public createNetworkInvitation(
    v: pb.ICreateNetworkInvitationRequest = new pb.CreateNetworkInvitationRequest()
  ): Promise<pb.CreateNetworkInvitationResponse> {
    return this.expectOne(
      this.call("FrontendRPC/CreateNetworkInvitation", new pb.CreateNetworkInvitationRequest(v))
    );
  }
  public createNetworkMembershipFromInvitation(
    v: pb.ICreateNetworkMembershipFromInvitationRequest = new pb.CreateNetworkMembershipFromInvitationRequest()
  ): Promise<pb.CreateNetworkMembershipFromInvitationResponse> {
    return this.expectOne(
      this.call(
        "FrontendRPC/CreateNetworkMembershipFromInvitation",
        new pb.CreateNetworkMembershipFromInvitationRequest(v)
      )
    );
  }
  public getBootstrapPeers(
    v: pb.IGetBootstrapPeersRequest = new pb.GetBootstrapPeersRequest()
  ): Promise<pb.GetBootstrapPeersResponse> {
    return this.expectOne(
      this.call("FrontendRPC/GetBootstrapPeers", new pb.GetBootstrapPeersRequest(v))
    );
  }
  public publishNetworkToBootstrapPeer(
    v: pb.IPublishNetworkToBootstrapPeerRequest = new pb.PublishNetworkToBootstrapPeerRequest()
  ): Promise<pb.PublishNetworkToBootstrapPeerResponse> {
    return this.expectOne(
      this.call(
        "FrontendRPC/PublishNetworkToBootstrapPeer",
        new pb.PublishNetworkToBootstrapPeerRequest(v)
      )
    );
  }
}

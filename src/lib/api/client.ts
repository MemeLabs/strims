import * as pb from "../pb";
import { RPCHost } from "./rpc_host";
import { Readable as GenericReadable } from "./stream";

interface LoginResponse {
  sessionId: string;
}

// prettier-ignore
export default class Client extends RPCHost {
  public createProfile(v: pb.ICreateProfileRequest = new pb.CreateProfileRequest()): Promise<pb.CreateProfileResponse> {
    return this.expectOne(this.call("createProfile", new pb.CreateProfileRequest(v)));
  }
  public loadProfile(v: pb.ILoadProfileRequest = new pb.LoadProfileRequest()): Promise<pb.LoadProfileResponse> {
    return this.expectOne(this.call("loadProfile", new pb.LoadProfileRequest(v)));
  }
  public getProfile(v: pb.IGetProfileRequest = new pb.GetProfileRequest()): Promise<pb.GetProfileResponse> {
    return this.expectOne(this.call("getProfile", new pb.GetProfileRequest(v)));
  }
  public updateProfile(v: pb.IUpdateProfileRequest = new pb.UpdateProfileRequest()): Promise<pb.UpdateProfileResponse> {
    return this.expectOne(this.call("updateProfile", new pb.UpdateProfileRequest(v)));
  }
  public deleteProfile(v: pb.IDeleteProfileRequest = new pb.DeleteProfileRequest()): Promise<pb.DeleteProfileResponse> {
    return this.expectOne(this.call("deleteProfile", new pb.DeleteProfileRequest(v)));
  }
  public getProfiles(v: pb.IGetProfilesRequest = new pb.GetProfilesRequest()): Promise<pb.GetProfilesResponse> {
    return this.expectOne(this.call("getProfiles", new pb.GetProfilesRequest(v)));
  }
  public loadSession(v: pb.ILoadSessionRequest = new pb.LoadSessionRequest()): Promise<pb.LoadSessionResponse> {
    return this.expectOne(this.call("loadSession", new pb.LoadSessionRequest(v)));
  }
  public createNetwork(v: pb.ICreateNetworkRequest = new pb.CreateNetworkRequest()): Promise<pb.CreateNetworkResponse> {
    return this.expectOne(this.call("createNetwork", new pb.CreateNetworkRequest(v)));
  }
  public updateNetwork(v: pb.IUpdateNetworkRequest = new pb.UpdateNetworkRequest()): Promise<pb.UpdateNetworkResponse> {
    return this.expectOne(this.call("updateNetwork", new pb.UpdateNetworkRequest(v)));
  }
  public deleteNetwork(v: pb.IDeleteNetworkRequest = new pb.DeleteNetworkRequest()): Promise<pb.DeleteNetworkResponse> {
    return this.expectOne(this.call("deleteNetwork", new pb.DeleteNetworkRequest(v)));
  }
  public getNetwork(v: pb.IGetNetworkRequest = new pb.GetNetworkRequest()): Promise<pb.GetNetworkResponse> {
    return this.expectOne(this.call("getNetwork", new pb.GetNetworkRequest(v)));
  }
  public getNetworks(v: pb.IGetNetworksRequest = new pb.GetNetworksRequest()): Promise<pb.GetNetworksResponse> {
    return this.expectOne(this.call("getNetworks", new pb.GetNetworksRequest(v)));
  }
  public getNetworkMemberships(v: pb.IGetNetworkMembershipsRequest = new pb.GetNetworkMembershipsRequest()): Promise<pb.GetNetworkMembershipsResponse> {
    return this.expectOne(this.call("getNetworkMemberships", new pb.GetNetworkMembershipsRequest(v)));
  }
  public deleteNetworkMembership(v: pb.IDeleteNetworkMembershipRequest = new pb.DeleteNetworkMembershipRequest()): Promise<pb.DeleteNetworkMembershipResponse> {
    return this.expectOne(this.call("deleteNetworkMembership", new pb.DeleteNetworkMembershipRequest(v)));
  }
  public createBootstrapClient(v: pb.ICreateBootstrapClientRequest = new pb.CreateBootstrapClientRequest()): Promise<pb.CreateBootstrapClientResponse> {
    return this.expectOne(this.call("createBootstrapClient", new pb.CreateBootstrapClientRequest(v)));
  }
  public updateBootstrapClient(v: pb.IUpdateBootstrapClientRequest = new pb.UpdateBootstrapClientRequest()): Promise<pb.UpdateBootstrapClientResponse> {
    return this.expectOne(this.call("updateBootstrapClient", new pb.UpdateBootstrapClientRequest(v)));
  }
  public deleteBootstrapClient(v: pb.IDeleteBootstrapClientRequest = new pb.DeleteBootstrapClientRequest()): Promise<pb.DeleteBootstrapClientResponse> {
    return this.expectOne(this.call("deleteBootstrapClient", new pb.DeleteBootstrapClientRequest(v)));
  }
  public getBootstrapClient(v: pb.IGetBootstrapClientRequest = new pb.GetBootstrapClientRequest()): Promise<pb.GetBootstrapClientResponse> {
    return this.expectOne(this.call("getBootstrapClient", new pb.GetBootstrapClientRequest(v)));
  }
  public getBootstrapClients(v: pb.IGetBootstrapClientsRequest = new pb.GetBootstrapClientsRequest()): Promise<pb.GetBootstrapClientsResponse> {
    return this.expectOne(this.call("getBootstrapClients", new pb.GetBootstrapClientsRequest(v)));
  }

  public createChatServer(v: pb.ICreateChatServerRequest = new pb.CreateChatServerRequest()): Promise<pb.CreateChatServerResponse> {
    return this.expectOne(this.call("createChatServer", new pb.CreateChatServerRequest(v)));
  }
  public updateChatServer(v: pb.IUpdateChatServerRequest = new pb.UpdateChatServerRequest()): Promise<pb.UpdateChatServerResponse> {
    return this.expectOne(this.call("updateChatServer", new pb.UpdateChatServerRequest(v)));
  }
  public deleteChatServer(v: pb.IDeleteChatServerRequest = new pb.DeleteChatServerRequest()): Promise<pb.DeleteChatServerResponse> {
    return this.expectOne(this.call("deleteChatServer", new pb.DeleteChatServerRequest(v)));
  }
  public getChatServer(v: pb.IGetChatServerRequest = new pb.GetChatServerRequest()): Promise<pb.GetChatServerResponse> {
    return this.expectOne(this.call("getChatServer", new pb.GetChatServerRequest(v)));
  }
  public getChatServers(v: pb.IGetChatServersRequest = new pb.GetChatServersRequest()): Promise<pb.GetChatServersResponse> {
    return this.expectOne(this.call("getChatServers", new pb.GetChatServersRequest(v)));
  }

  public startVPN(v: pb.IStartVPNRequest = new pb.StartVPNRequest()): Promise<pb.StartVPNResponse> {
    return this.expectOne(this.call("startVPN", new pb.StartVPNRequest(v)));
  }
  public stopVPN(v: pb.IStopVPNRequest = new pb.StopVPNRequest()): Promise<pb.StopVPNResponse> {
    return this.expectOne(this.call("stopVPN", new pb.StopVPNRequest(v)));
  }

  public joinSwarm(v: pb.IJoinSwarmRequest = new pb.JoinSwarmRequest()): Promise<pb.JoinSwarmResponse> {
    return this.expectOne(this.call("joinSwarm", new pb.JoinSwarmRequest(v)));
  }
  public leaveSwarm(v: pb.ILeaveSwarmRequest = new pb.LeaveSwarmRequest()): Promise<pb.LeaveSwarmResponse> {
    return this.expectOne(this.call("leaveSwarm", new pb.LeaveSwarmRequest(v)));
  }
  public getIngressStreams(v: pb.IGetIngressStreamsRequest = new pb.GetIngressStreamsRequest()): GenericReadable<pb.GetIngressStreamsResponse> {
    return this.expectMany(this.call("getIngressStreams", new pb.GetIngressStreamsRequest(v)));
  }
  public startHLSIngress(v: pb.IStartHLSIngressRequest = new pb.StartHLSIngressRequest()): Promise<pb.StartHLSIngressResponse> {
    return this.expectOne(this.call("startHLSIngress", new pb.StartHLSIngressRequest(v)));
  }
  public stopHLSIngress(v: pb.IStartHLSIngressRequest = new pb.StartHLSIngressRequest()): Promise<pb.StartHLSIngressResponse> {
    return this.expectOne(this.call("stopHLSIngress", new pb.StartHLSIngressRequest(v)));
  }
  public startHLSEgress(v: pb.IStopHLSEgressRequest = new pb.StopHLSEgressRequest()): Promise<pb.StopHLSEgressResponse> {
    return this.expectOne(this.call("startHLSEgress", new pb.StopHLSEgressRequest(v)));
  }
  public startSwarm(v: pb.IStartSwarmRequest = new pb.StartSwarmRequest()): Promise<pb.StartSwarmResponse> {
    return this.expectOne(this.call("startSwarm", new pb.StartSwarmRequest(v)));
  }
  public writeToSwarm(v: pb.IWriteToSwarmRequest = new pb.WriteToSwarmRequest()): Promise<pb.WriteToSwarmResponse> {
    return this.expectOne(this.call("writeToSwarm", new pb.WriteToSwarmRequest(v)));
  }
  public stopSwarm(v: pb.IStopSwarmRequest = new pb.StopSwarmRequest()): Promise<pb.StopSwarmResponse> {
    return this.expectOne(this.call("stopSwarm", new pb.StopSwarmRequest(v)));
  }
  public publishSwarm(v: pb.IPublishSwarmRequest = new pb.PublishSwarmRequest()): Promise<pb.PublishSwarmResponse> {
    return this.expectOne(this.call("publishSwarm", new pb.PublishSwarmRequest(v)));
  }
  public pprof(v: pb.IPProfRequest = new pb.PProfRequest()): Promise<pb.PProfResponse> {
    return this.expectOne(this.call("pProf", new pb.PProfRequest(v)));
  }
  public openChatClient(v: pb.IChatClientOpenRequest = new pb.ChatClientOpenRequest()): GenericReadable<pb.ChatClientEvent> {
    return this.expectMany(this.call("openChatClient", new pb.ChatClientOpenRequest(v)));
  }
  public callChatClient(v: pb.IChatClientCallRequest = new pb.ChatClientCallRequest()) {
    this.call("callChatClient", new pb.ChatClientCallRequest(v));
  }
  public openVideoClient(v: pb.IVideoClientOpenRequest = new pb.VideoClientOpenRequest()): GenericReadable<pb.VideoClientEvent> {
    return this.expectMany(this.call("openVideoClient", new pb.VideoClientOpenRequest(v)));
  }
  public openVideoServer(v: pb.IVideoServerOpenRequest = new pb.VideoServerOpenRequest()): Promise<pb.VideoServerOpenResponse> {
    return this.expectOne(this.call("openVideoServer", new pb.VideoServerOpenRequest(v)));
  }
  public writeToVideoServer(v: pb.IVideoServerWriteRequest = new pb.VideoServerWriteRequest()): Promise<pb.VideoServerWriteResponse> {
    return this.expectOne(this.call("writeToVideoServer", new pb.VideoServerWriteRequest(v)));
  }
  public readMetrics(v: pb.IReadMetricsRequest = new pb.ReadMetricsRequest()): Promise<pb.ReadMetricsResponse> {
    return this.expectOne(this.call("readMetrics", new pb.ReadMetricsRequest(v)));
  }
  public createNetworkInvitation(v: pb.ICreateNetworkInvitationRequest = new pb.CreateNetworkInvitationRequest()): Promise<pb.CreateNetworkInvitationResponse> {
    return this.expectOne(this.call("createNetworkInvitation", new pb.CreateNetworkInvitationRequest(v)));
  }
  public createNetworkMembershipFromInvitation(v: pb.ICreateNetworkMembershipFromInvitationRequest = new pb.CreateNetworkMembershipFromInvitationRequest()): Promise<pb.CreateNetworkMembershipFromInvitationResponse> {
    return this.expectOne(this.call("createNetworkMembershipFromInvitation", new pb.CreateNetworkMembershipFromInvitationRequest(v)));
  }
}

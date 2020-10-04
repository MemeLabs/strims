// swift-format-ignore-file
//
//  FrontendRPCClient.swift
//  App
//
//  Copyright © 2020 MemeLabs. All rights reserved.
//

import Foundation
import PromiseKit

class FrontendRPCClient: RPCClient {
  public func createProfile(_ arg: PBCreateProfileRequest = PBCreateProfileRequest()) -> Promise<PBCreateProfileResponse> {
    return self.callUnary("FrontendRPC/CreateProfile", arg)
  }
  public func loadProfile(_ arg: PBLoadProfileRequest = PBLoadProfileRequest()) -> Promise<PBLoadProfileResponse> {
    return self.callUnary("FrontendRPC/LoadProfile", arg)
  }
  public func getProfile(_ arg: PBGetProfileRequest = PBGetProfileRequest()) -> Promise<PBGetProfileResponse> {
    return self.callUnary("FrontendRPC/GetProfile", arg)
  }
  public func deleteProfile(_ arg: PBDeleteProfileRequest = PBDeleteProfileRequest()) -> Promise<PBDeleteProfileResponse> {
    return self.callUnary("FrontendRPC/DeleteProfile", arg)
  }
  public func getProfiles(_ arg: PBGetProfilesRequest = PBGetProfilesRequest()) -> Promise<PBGetProfilesResponse> {
    return self.callUnary("FrontendRPC/GetProfiles", arg)
  }
  public func loadSession(_ arg: PBLoadSessionRequest = PBLoadSessionRequest()) -> Promise<PBLoadSessionResponse> {
    return self.callUnary("FrontendRPC/LoadSession", arg)
  }
  public func createNetwork(_ arg: PBCreateNetworkRequest = PBCreateNetworkRequest()) -> Promise<PBCreateNetworkResponse> {
    return self.callUnary("FrontendRPC/CreateNetwork", arg)
  }
  public func deleteNetwork(_ arg: PBDeleteNetworkRequest = PBDeleteNetworkRequest()) -> Promise<PBDeleteNetworkResponse> {
    return self.callUnary("FrontendRPC/DeleteNetwork", arg)
  }
  public func getNetwork(_ arg: PBGetNetworkRequest = PBGetNetworkRequest()) -> Promise<PBGetNetworkResponse> {
    return self.callUnary("FrontendRPC/GetNetwork", arg)
  }
  public func getNetworks(_ arg: PBGetNetworksRequest = PBGetNetworksRequest()) -> Promise<PBGetNetworksResponse> {
    return self.callUnary("FrontendRPC/GetNetworks", arg)
  }
  public func getNetworkMemberships(_ arg: PBGetNetworkMembershipsRequest = PBGetNetworkMembershipsRequest()) -> Promise<PBGetNetworkMembershipsResponse> {
    return self.callUnary("FrontendRPC/GetNetworkMemberships", arg)
  }
  public func deleteNetworkMembership(_ arg: PBDeleteNetworkMembershipRequest = PBDeleteNetworkMembershipRequest()) -> Promise<PBDeleteNetworkMembershipResponse> {
    return self.callUnary("FrontendRPC/DeleteNetworkMembership", arg)
  }
  public func createBootstrapClient(_ arg: PBCreateBootstrapClientRequest = PBCreateBootstrapClientRequest()) -> Promise<PBCreateBootstrapClientResponse> {
    return self.callUnary("FrontendRPC/CreateBootstrapClient", arg)
  }
  public func updateBootstrapClient(_ arg: PBUpdateBootstrapClientRequest = PBUpdateBootstrapClientRequest()) -> Promise<PBUpdateBootstrapClientResponse> {
    return self.callUnary("FrontendRPC/UpdateBootstrapClient", arg)
  }
  public func deleteBootstrapClient(_ arg: PBDeleteBootstrapClientRequest = PBDeleteBootstrapClientRequest()) -> Promise<PBDeleteBootstrapClientResponse> {
    return self.callUnary("FrontendRPC/DeleteBootstrapClient", arg)
  }
  public func getBootstrapClient(_ arg: PBGetBootstrapClientRequest = PBGetBootstrapClientRequest()) -> Promise<PBGetBootstrapClientResponse> {
    return self.callUnary("FrontendRPC/GetBootstrapClient", arg)
  }
  public func getBootstrapClients(_ arg: PBGetBootstrapClientsRequest = PBGetBootstrapClientsRequest()) -> Promise<PBGetBootstrapClientsResponse> {
    return self.callUnary("FrontendRPC/GetBootstrapClients", arg)
  }
  public func createChatServer(_ arg: PBCreateChatServerRequest = PBCreateChatServerRequest()) -> Promise<PBCreateChatServerResponse> {
    return self.callUnary("FrontendRPC/CreateChatServer", arg)
  }
  public func updateChatServer(_ arg: PBUpdateChatServerRequest = PBUpdateChatServerRequest()) -> Promise<PBUpdateChatServerResponse> {
    return self.callUnary("FrontendRPC/UpdateChatServer", arg)
  }
  public func deleteChatServer(_ arg: PBDeleteChatServerRequest = PBDeleteChatServerRequest()) -> Promise<PBDeleteChatServerResponse> {
    return self.callUnary("FrontendRPC/DeleteChatServer", arg)
  }
  public func getChatServer(_ arg: PBGetChatServerRequest = PBGetChatServerRequest()) -> Promise<PBGetChatServerResponse> {
    return self.callUnary("FrontendRPC/GetChatServer", arg)
  }
  public func getChatServers(_ arg: PBGetChatServersRequest = PBGetChatServersRequest()) -> Promise<PBGetChatServersResponse> {
    return self.callUnary("FrontendRPC/GetChatServers", arg)
  }
  public func startVPN(_ arg: PBStartVPNRequest = PBStartVPNRequest()) throws -> RPCResponseStream<PBNetworkEvent> {
    return try self.callStreaming("FrontendRPC/StartVPN", arg)
  }
  public func stopVPN(_ arg: PBStopVPNRequest = PBStopVPNRequest()) -> Promise<PBStopVPNResponse> {
    return self.callUnary("FrontendRPC/StopVPN", arg)
  }
  public func joinSwarm(_ arg: PBJoinSwarmRequest = PBJoinSwarmRequest()) -> Promise<PBJoinSwarmResponse> {
    return self.callUnary("FrontendRPC/JoinSwarm", arg)
  }
  public func leaveSwarm(_ arg: PBLeaveSwarmRequest = PBLeaveSwarmRequest()) -> Promise<PBLeaveSwarmResponse> {
    return self.callUnary("FrontendRPC/LeaveSwarm", arg)
  }
  public func startRTMPIngress(_ arg: PBStartRTMPIngressRequest = PBStartRTMPIngressRequest()) -> Promise<PBStartRTMPIngressResponse> {
    return self.callUnary("FrontendRPC/StartRTMPIngress", arg)
  }
  public func startHLSEgress(_ arg: PBStartHLSEgressRequest = PBStartHLSEgressRequest()) -> Promise<PBStartHLSEgressResponse> {
    return self.callUnary("FrontendRPC/StartHLSEgress", arg)
  }
  public func stopHLSEgress(_ arg: PBStopHLSEgressRequest = PBStopHLSEgressRequest()) -> Promise<PBStopHLSEgressResponse> {
    return self.callUnary("FrontendRPC/StopHLSEgress", arg)
  }
  public func publishSwarm(_ arg: PBPublishSwarmRequest = PBPublishSwarmRequest()) -> Promise<PBPublishSwarmResponse> {
    return self.callUnary("FrontendRPC/PublishSwarm", arg)
  }
  public func pProf(_ arg: PBPProfRequest = PBPProfRequest()) -> Promise<PBPProfResponse> {
    return self.callUnary("FrontendRPC/PProf", arg)
  }
  public func openChatServer(_ arg: PBOpenChatServerRequest = PBOpenChatServerRequest()) throws -> RPCResponseStream<PBChatServerEvent> {
    return try self.callStreaming("FrontendRPC/OpenChatServer", arg)
  }
  public func openChatClient(_ arg: PBOpenChatClientRequest = PBOpenChatClientRequest()) throws -> RPCResponseStream<PBChatClientEvent> {
    return try self.callStreaming("FrontendRPC/OpenChatClient", arg)
  }
  public func callChatClient(_ arg: PBCallChatClientRequest = PBCallChatClientRequest()) -> Promise<PBCallChatClientResponse> {
    return self.callUnary("FrontendRPC/CallChatClient", arg)
  }
  public func openVideoClient(_ arg: PBVideoClientOpenRequest = PBVideoClientOpenRequest()) throws -> RPCResponseStream<PBVideoClientEvent> {
    return try self.callStreaming("FrontendRPC/OpenVideoClient", arg)
  }
  public func openVideoServer(_ arg: PBVideoServerOpenRequest = PBVideoServerOpenRequest()) -> Promise<PBVideoServerOpenResponse> {
    return self.callUnary("FrontendRPC/OpenVideoServer", arg)
  }
  public func writeToVideoServer(_ arg: PBVideoServerWriteRequest = PBVideoServerWriteRequest()) -> Promise<PBVideoServerWriteResponse> {
    return self.callUnary("FrontendRPC/WriteToVideoServer", arg)
  }
  public func readMetrics(_ arg: PBReadMetricsRequest = PBReadMetricsRequest()) -> Promise<PBReadMetricsResponse> {
    return self.callUnary("FrontendRPC/ReadMetrics", arg)
  }
  public func createNetworkInvitation(_ arg: PBCreateNetworkInvitationRequest = PBCreateNetworkInvitationRequest()) -> Promise<PBCreateNetworkInvitationResponse> {
    return self.callUnary("FrontendRPC/CreateNetworkInvitation", arg)
  }
  public func createNetworkMembershipFromInvitation(_ arg: PBCreateNetworkMembershipFromInvitationRequest = PBCreateNetworkMembershipFromInvitationRequest()) -> Promise<PBCreateNetworkMembershipFromInvitationResponse> {
    return self.callUnary("FrontendRPC/CreateNetworkMembershipFromInvitation", arg)
  }
  public func getBootstrapPeers(_ arg: PBGetBootstrapPeersRequest = PBGetBootstrapPeersRequest()) -> Promise<PBGetBootstrapPeersResponse> {
    return self.callUnary("FrontendRPC/GetBootstrapPeers", arg)
  }
  public func publishNetworkToBootstrapPeer(_ arg: PBPublishNetworkToBootstrapPeerRequest = PBPublishNetworkToBootstrapPeerRequest()) -> Promise<PBPublishNetworkToBootstrapPeerResponse> {
    return self.callUnary("FrontendRPC/PublishNetworkToBootstrapPeer", arg)
  }

}

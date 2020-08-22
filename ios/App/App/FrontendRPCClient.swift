//
//  FrontendRPCClient.swift
//  App
//
//  Created by Slugalisk on 8/22/20.
//  Copyright © 2020 MemeLabs. All rights reserved.
//

import Foundation
import PromiseKit

class FrontendRPCClient: RPCClient {
    public func createProfile(_ arg: PBCreateProfileRequest = PBCreateProfileRequest()) -> Promise<PBCreateProfileResponse> {
        return self.callUnary("createProfile", arg)
    }
    public func loadProfile(_ arg: PBLoadProfileRequest = PBLoadProfileRequest()) -> Promise<PBLoadProfileResponse> {
        return self.callUnary("loadProfile", arg)
    }
    public func getProfile(_ arg: PBGetProfileRequest = PBGetProfileRequest()) -> Promise<PBGetProfileResponse> {
        return self.callUnary("getProfile", arg)
    }
    public func updateProfile(_ arg: PBUpdateProfileRequest = PBUpdateProfileRequest()) -> Promise<PBUpdateProfileResponse> {
        return self.callUnary("updateProfile", arg)
    }
    public func deleteProfile(_ arg: PBDeleteProfileRequest = PBDeleteProfileRequest()) -> Promise<PBDeleteProfileResponse> {
        return self.callUnary("deleteProfile", arg)
    }
    public func getProfiles(_ arg: PBGetProfilesRequest = PBGetProfilesRequest()) -> Promise<PBGetProfilesResponse> {
        return self.callUnary("getProfiles", arg)
    }
    public func loadSession(_ arg: PBLoadSessionRequest = PBLoadSessionRequest()) -> Promise<PBLoadSessionResponse> {
        return self.callUnary("loadSession", arg)
    }
    public func createNetwork(_ arg: PBCreateNetworkRequest = PBCreateNetworkRequest()) -> Promise<PBCreateNetworkResponse> {
        return self.callUnary("createNetwork", arg)
    }
    public func updateNetwork(_ arg: PBUpdateNetworkRequest = PBUpdateNetworkRequest()) -> Promise<PBUpdateNetworkResponse> {
        return self.callUnary("updateNetwork", arg)
    }
    public func deleteNetwork(_ arg: PBDeleteNetworkRequest = PBDeleteNetworkRequest()) -> Promise<PBDeleteNetworkResponse> {
        return self.callUnary("deleteNetwork", arg)
    }
    public func getNetwork(_ arg: PBGetNetworkRequest = PBGetNetworkRequest()) -> Promise<PBGetNetworkResponse> {
        return self.callUnary("getNetwork", arg)
    }
    public func getNetworks(_ arg: PBGetNetworksRequest = PBGetNetworksRequest()) -> Promise<PBGetNetworksResponse> {
        return self.callUnary("getNetworks", arg)
    }
    public func getNetworkMemberships(_ arg: PBGetNetworkMembershipsRequest = PBGetNetworkMembershipsRequest()) -> Promise<PBGetNetworkMembershipsResponse> {
        return self.callUnary("getNetworkMemberships", arg)
    }
    public func deleteNetworkMembership(_ arg: PBDeleteNetworkMembershipRequest = PBDeleteNetworkMembershipRequest()) -> Promise<PBDeleteNetworkMembershipResponse> {
        return self.callUnary("deleteNetworkMembership", arg)
    }
    public func createBootstrapClient(_ arg: PBCreateBootstrapClientRequest = PBCreateBootstrapClientRequest()) -> Promise<PBCreateBootstrapClientResponse> {
        return self.callUnary("createBootstrapClient", arg)
    }
    public func updateBootstrapClient(_ arg: PBUpdateBootstrapClientRequest = PBUpdateBootstrapClientRequest()) -> Promise<PBUpdateBootstrapClientResponse> {
        return self.callUnary("updateBootstrapClient", arg)
    }
    public func deleteBootstrapClient(_ arg: PBDeleteBootstrapClientRequest = PBDeleteBootstrapClientRequest()) -> Promise<PBDeleteBootstrapClientResponse> {
        return self.callUnary("deleteBootstrapClient", arg)
    }
    public func getBootstrapClient(_ arg: PBGetBootstrapClientRequest = PBGetBootstrapClientRequest()) -> Promise<PBGetBootstrapClientResponse> {
        return self.callUnary("getBootstrapClient", arg)
    }
    public func getBootstrapClients(_ arg: PBGetBootstrapClientsRequest = PBGetBootstrapClientsRequest()) -> Promise<PBGetBootstrapClientsResponse> {
        return self.callUnary("getBootstrapClients", arg)
    }

    public func createChatServer(_ arg: PBCreateChatServerRequest = PBCreateChatServerRequest()) -> Promise<PBCreateChatServerResponse> {
        return self.callUnary("createChatServer", arg)
    }
    public func updateChatServer(_ arg: PBUpdateChatServerRequest = PBUpdateChatServerRequest()) -> Promise<PBUpdateChatServerResponse> {
        return self.callUnary("updateChatServer", arg)
    }
    public func deleteChatServer(_ arg: PBDeleteChatServerRequest = PBDeleteChatServerRequest()) -> Promise<PBDeleteChatServerResponse> {
        return self.callUnary("deleteChatServer", arg)
    }
    public func getChatServer(_ arg: PBGetChatServerRequest = PBGetChatServerRequest()) -> Promise<PBGetChatServerResponse> {
        return self.callUnary("getChatServer", arg)
    }
    public func getChatServers(_ arg: PBGetChatServersRequest = PBGetChatServersRequest()) -> Promise<PBGetChatServersResponse> {
        return self.callUnary("getChatServers", arg)
    }

    public func startVPN(_ arg: PBStartVPNRequest = PBStartVPNRequest()) -> RPCResponseStream<PBNetworkEvent> {
        return self.callStreaming("startVPN", arg)
    }
    public func stopVPN(_ arg: PBStopVPNRequest = PBStopVPNRequest()) -> Promise<PBStopVPNResponse> {
        return self.callUnary("stopVPN", arg)
    }

    public func joinSwarm(_ arg: PBJoinSwarmRequest = PBJoinSwarmRequest()) -> Promise<PBJoinSwarmResponse> {
        return self.callUnary("joinSwarm", arg)
    }
    public func leaveSwarm(_ arg: PBLeaveSwarmRequest = PBLeaveSwarmRequest()) -> Promise<PBLeaveSwarmResponse> {
        return self.callUnary("leaveSwarm", arg)
    }
    public func getIngressStreams(_ arg: PBGetIngressStreamsRequest = PBGetIngressStreamsRequest()) -> RPCResponseStream<PBGetIngressStreamsResponse> {
        return self.callStreaming("getIngressStreams", arg)
    }
    public func startHLSIngress(_ arg: PBStartHLSIngressRequest = PBStartHLSIngressRequest()) -> Promise<PBStartHLSIngressResponse> {
        return self.callUnary("startHLSIngress", arg)
    }
    public func stopHLSIngress(_ arg: PBStartHLSIngressRequest = PBStartHLSIngressRequest()) -> Promise<PBStartHLSIngressResponse> {
        return self.callUnary("stopHLSIngress", arg)
    }
    public func startHLSEgress(_ arg: PBStopHLSEgressRequest = PBStopHLSEgressRequest()) -> Promise<PBStopHLSEgressResponse> {
        return self.callUnary("startHLSEgress", arg)
    }
    public func startSwarm(_ arg: PBStartSwarmRequest = PBStartSwarmRequest()) -> Promise<PBStartSwarmResponse> {
        return self.callUnary("startSwarm", arg)
    }
    public func writeToSwarm(_ arg: PBWriteToSwarmRequest = PBWriteToSwarmRequest()) -> Promise<PBWriteToSwarmResponse> {
        return self.callUnary("writeToSwarm", arg)
    }
    public func stopSwarm(_ arg: PBStopSwarmRequest = PBStopSwarmRequest()) -> Promise<PBStopSwarmResponse> {
        return self.callUnary("stopSwarm", arg)
    }
    public func publishSwarm(_ arg: PBPublishSwarmRequest = PBPublishSwarmRequest()) -> Promise<PBPublishSwarmResponse> {
        return self.callUnary("publishSwarm", arg)
    }
    public func pprof(_ arg: PBPProfRequest = PBPProfRequest()) -> Promise<PBPProfResponse> {
        return self.callUnary("pProf", arg)
    }
    public func openChatServer(_ arg: PBOpenChatServerRequest = PBOpenChatServerRequest()) -> RPCResponseStream<PBChatServerEvent> {
        return self.callStreaming("openChatServer", arg)
    }
    public func openChatClient(_ arg: PBOpenChatClientRequest = PBOpenChatClientRequest()) -> RPCResponseStream<PBChatClientEvent> {
        return self.callStreaming("openChatClient", arg)
    }
    public func callChatClient(_ arg: PBCallChatClientRequest = PBCallChatClientRequest()) {
        self.call("callChatClient", arg)
    }
    public func openVideoClient(_ arg: PBVideoClientOpenRequest = PBVideoClientOpenRequest()) -> RPCResponseStream<PBVideoClientEvent> {
        return self.callStreaming("openVideoClient", arg)
    }
    public func openVideoServer(_ arg: PBVideoServerOpenRequest = PBVideoServerOpenRequest()) -> Promise<PBVideoServerOpenResponse> {
        return self.callUnary("openVideoServer", arg)
    }
    public func writeToVideoServer(_ arg: PBVideoServerWriteRequest = PBVideoServerWriteRequest()) -> Promise<PBVideoServerWriteResponse> {
        return self.callUnary("writeToVideoServer", arg)
    }
    public func readMetrics(_ arg: PBReadMetricsRequest = PBReadMetricsRequest()) -> Promise<PBReadMetricsResponse> {
        return self.callUnary("readMetrics", arg)
    }
    public func createNetworkInvitation(_ arg: PBCreateNetworkInvitationRequest = PBCreateNetworkInvitationRequest()) -> Promise<PBCreateNetworkInvitationResponse> {
        return self.callUnary("createNetworkInvitation", arg)
    }
    public func createNetworkMembershipFromInvitation(_ arg: PBCreateNetworkMembershipFromInvitationRequest = PBCreateNetworkMembershipFromInvitationRequest()) -> Promise<PBCreateNetworkMembershipFromInvitationResponse> {
        return self.callUnary("createNetworkMembershipFromInvitation", arg)
    }
    public func getBootstrapPeers(_ arg: PBGetBootstrapPeersRequest = PBGetBootstrapPeersRequest()) -> Promise<PBGetBootstrapPeersResponse> {
        return self.callUnary("getBootstrapPeers", arg)
    }
    public func publishNetworkToBootstrapPeer(_ arg: PBPublishNetworkToBootstrapPeerRequest = PBPublishNetworkToBootstrapPeerRequest()) -> Promise<PBPublishNetworkToBootstrapPeerResponse> {
        return self.callUnary("publishNetworkToBootstrapPeer", arg)
    }
}

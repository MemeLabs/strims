//
//  ContentView.swift
//  App
//
//  Created by Slugalisk on 8/20/20.
//  Copyright © 2020 MemeLabs. All rights reserved.
//

import SwiftUI
import PromiseKit

struct ContentView: View {
    var body: some View {
        let client = FrontendRPCClient()

        let handleCreateProfile: () -> Void = {
            firstly {
                client.createProfile(PBCreateProfileRequest.with {
                    $0.name = "test"
                    $0.password = "test"
                })
            }.done { res in
                let json = try res.jsonString()
                print("profile: \(json)")
            }.catch { error in
                print("creating profile failed \(error.localizedDescription)")
            }
        }
        
        let handleLogin: () -> Void = {
            firstly {
                client.getProfiles(PBGetProfilesRequest())
            }.then { res in
                client.loadProfile(PBLoadProfileRequest.with{
                    $0.id = res.profiles[0].id
                    $0.name = "test"
                    $0.password = "test"
                })
            }.done { res in
                let json = try res.jsonString()
                print("profile: \(json)")
            }.catch { error in
                print("loading profile failed \(error.localizedDescription)")
            }
        }
        
        let handleCreateBootstrapClient: () -> Void = {
            firstly {
                client.createBootstrapClient(PBCreateBootstrapClientRequest.with{
                    $0.clientOptions = PBCreateBootstrapClientRequest.OneOf_ClientOptions.websocketOptions(PBBootstrapClientWebSocketOptions.with{
                        $0.url = "wss://192.168.0.111:8080/test-bootstrap"
                        $0.insecureSkipVerifyTls = true
                    })
                })
            }.done { res in
                let json = try res.jsonString()
                print("profile: \(json)")
            }.catch { error in
                print("creating network failed \(error.localizedDescription)")
            }
        }
        
        let handleLoadInviteCert: () -> Void = {
            firstly {
                client.createNetworkMembershipFromInvitation(PBCreateNetworkMembershipFromInvitationRequest.with{
                    $0.invitationB64 = "EoADCmYIARJA3+jPfL6RMfY8aLFZRDYdmzY5s8gsuEvzrLNOM+KQxDtU0VEHnhGkPOp8mryKzh5ISz1dpRr8xD2kcZMIZ+dNRhogVNFRB54RpDzqfJq8is4eSEs9XaUa/MQ9pHGTCGfnTUYSjwIKIFTRUQeeEaQ86nyavIrOHkhLPV2lGvzEPaRxkwhn501GEAEYBiCH8ob6BSiH56v6BTIQTKUyIcq6qpRJYxpU4CVm0zpAIcPsy2/eBc/FLAp62xJka2WVrWqa8JdnYscnOh80REVOPSQbJ5s1uXQRUqJ8hwUUCMw7rPRhP29ZTV8ZGTznCEKGAQogHVmKdL3JUzXvjh3BQ8tFqFCvzPp7Wxe4ak2yWbjSj/cQARgEINb+kfcFKNbMm5UGMhCGDUmvQDYLYehxX3XjVz/EOkAixCMT3+O7tBwyhTEid0bCtxNkpAN6FkrSHdOiIkAv4wWp/OJ3UzlWpYGaA01wO27gUIEb+82rUyDdOGibKKwEIgR0ZXN0"
                })
            }.done { res in
                let json = try res.jsonString()
                print("profile: \(json)")
            }.catch { error in
                print("creating network failed \(error.localizedDescription)")
            }
        }
        
        let handleCreateNetwork: () -> Void = {
            firstly {
                client.createNetwork(PBCreateNetworkRequest.with{
                    $0.name = "test"
                })
            }.done { res in
                let json = try res.jsonString()
                print("profile: \(json)")
            }.catch { error in
                print("creating network failed \(error.localizedDescription)")
            }
        }
        
        let handleStartVPN: () -> Void = {
            do {
                let vpn = try client.startVPN()
                vpn.delegate = { event, eventType in
                    switch (eventType) {
                    case RPCEvent.data:
                        do {
                            let json = try event?.jsonString()
                            print("vpn event: \(json!)")
                        } catch {}
                    case RPCEvent.close:
                        print("vpn event stream closed")
                    default:
                        print("vpn rpc error")
                    }
                }
            } catch {
                print("starting vpn failed \(error)")
            }
        }

        let rootCert: (PBCertificate) -> PBCertificate = { cert in
            var root = cert
            while case .parent(let parent)? = root.parentOneof {
                root = parent
            }
            return root
        }
        
        let publishSwarm: (UInt64) -> Void = { id in
            firstly {
                client.getNetworkMemberships()
            }.then { memberships in
                when(fulfilled: memberships.networkMemberships.map { membership in
                    client.publishSwarm(PBPublishSwarmRequest.with{
                        $0.id = id
                        $0.networkKey = rootCert(membership.certificate).key
                    })
                } as [Promise<PBPublishSwarmResponse>])
            }.catch(only: RPCClientError.self) { error in
                print("publishing swarm failed \(error.message)")
            }.catch { error in
                print("publishing swarm failed \(error.localizedDescription)")
            }
        }
        
        let handleJoinVideoSwarm: () -> Void = {
            do {
                let client = try client.openVideoClient()
                client.delegate = { event, eventType in
                    switch (eventType) {
                    case RPCEvent.data:
                        if let body = event?.body {
                            switch (body) {
                            case .open(let open):
                                print("open: \(open.id)")
                                publishSwarm(open.id)
                            case .data(let data):
                                print("video data: \(data.data.count) bytes")
                            case .close:
                                print("close")
                            }
                        }
                    case RPCEvent.close:
                        print("vpn event stream closed")
                    default:
                        print("vpn rpc error")
                    }
                }
            } catch {
                print("joining video swarm failed \(error)")
            }
        }
        
        return VStack {
            Button(action: handleCreateProfile) {
                Text("create profile")
            }
            Button(action: handleLogin) {
                Text("log in")
            }
            Button(action: handleCreateBootstrapClient) {
                Text("create bootstrap client")
            }
            Button(action: handleLoadInviteCert) {
                Text("load invite cert")
            }
            Button(action: handleCreateNetwork) {
                Text("create network")
            }
            Button(action: handleStartVPN) {
                Text("start vpn")
            }
            Button(action: handleJoinVideoSwarm) {
                Text("join video swarm")
            }
        }
    }
}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView()
    }
}

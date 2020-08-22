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

        let handleCreate: () -> Void = {
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
                let vpn = client.startVPN()
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
            }.catch { error in
                print("profile loading failed \(error.localizedDescription)")
            }
        }
        
        return VStack {
            Button(action: handleCreate) {
                Text("create profile")
            }
            Button(action: handleLogin) {
                Text("log in")
            }
        }
    }
}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView()
    }
}

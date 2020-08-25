//
//  ContentView.swift
//  App
//
//  Created by Slugalisk on 8/20/20.
//  Copyright © 2020 MemeLabs. All rights reserved.
//

import AVFoundation
import CryptoKit
import PromiseKit
import SwiftUI

class SegmentPlaylist {
  public var index: Int = 0
  public var queue: [SegmentBuffer] = []
  public var current: SegmentBuffer = SegmentBuffer(0)

  func append(data: Data) {
    current.append(data)
  }

  func flush() {
    queue.append(current)
    index += 1
    current = SegmentBuffer(index)
  }

  func next() -> SegmentBuffer? {
    return queue[0]
  }

  func advance() {
    if !queue.isEmpty {
      queue.remove(at: 0)
    }
  }

  func indexRange() -> Range<Int> {
    return index - queue.count..<index
  }
}

class SegmentBuffer {
  public let index: Int
  public var data: Data = Data()

  init(_ index: Int) {
    self.index = index
  }

  func append(_ data: Data) {
    self.data.append(data)
  }

  private func headerCount() -> Int {
    print(Int(data[0]) << 8 + Int(data[1]))
    return Int(data[0]) << 8 + Int(data[1])
  }

  func initData() -> Data {
    return data.subdata(in: 2..<headerCount() + 2)
  }

  func segmentData() -> Data {
    return data.subdata(in: headerCount() + 2..<data.count)
  }
}

class PlayerUIView: UIView, AVAssetResourceLoaderDelegate {
  private var player: AVPlayer?
  private let playerLayer = AVPlayerLayer()
  private var playlist: SegmentPlaylist = SegmentPlaylist()

  override init(frame: CGRect) {
    super.init(frame: frame)
    setupPlayer()
  }

  required init?(coder aDecoder: NSCoder) {
    super.init(coder: aDecoder)
  }

  public func setupDelegateThing(_ test: RPCResponseStream<PBVideoClientEvent>) {
    test.delegate = { event, eventType in
      switch eventType {
      case RPCEvent.data:
        if let body = event?.body {
          switch (body) {
          case .open(let open):
            print("open: \(open.id)")
          case .data(let data):
            DispatchQueue.main.sync {
              self.playlist.append(data: data.data)
              if data.flush {
                print("flush segment...")
                self.playlist.flush()
                self.setupPlayer()
                print(self.player!.error)
                self.player!.play()
              }
            }
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
  }

  func setupPlayer() {
    if let _ = player {
      return
    }

    //    let url = URL(string: "asdf://strims.gg/index.m3u8")!
    //    let url = URL(string: "http://192.168.0.111:8000/index.m3u8")!
    let url = URL(string: "http://127.0.0.1:8003/index.m3u8")!
    let asset = AVURLAsset(url: url, options: nil)
    asset.resourceLoader.setDelegate(self, queue: DispatchQueue.global(qos: .default))
    let item = AVPlayerItem(asset: asset)
    player = AVPlayer(playerItem: item)
    player!.play()
    playerLayer.player = player
    layer.addSublayer(playerLayer)
  }

  override func layoutSubviews() {
    super.layoutSubviews()
    playerLayer.frame = bounds
  }

  func resourceLoader(
    _ resourceLoader: AVAssetResourceLoader,
    shouldWaitForLoadingOfRequestedResource loadingRequest: AVAssetResourceLoadingRequest
  ) -> Bool {
    print("resourceLoader")
    guard let url = loadingRequest.request.url else { return false }

    switch url.path {
    case "/index.m3u8":
      return handlePlaylistRequest(loadingRequest)
    case "/init.mp4":
      return handleInitRequest(loadingRequest)
    default:
      return handleSegmentRequest(loadingRequest)
    }
  }

  func handlePlaylistRequest(_ loadingRequest: AVAssetResourceLoadingRequest) -> Bool {
    var playlist = ""

    playlist += "#EXTM3U\n"
    playlist += "#EXT-X-VERSION:7\n"
    playlist += "#EXT-X-TARGETDURATION:1\n"
    playlist += "#EXT-X-MEDIA-SEQUENCE:\(self.playlist.indexRange().min()!)\n"
    playlist += "#EXT-X-MAP:URI=\"init.mp4\"\n"

    for i in self.playlist.indexRange() {
      playlist += "#EXTINF:1\n"
      playlist += "\(i).m4s\n"
    }

    print(playlist)

    if let infoRequest = loadingRequest.contentInformationRequest {
      print("read info request thing...")
      infoRequest.contentType = "public.m3u-playlist"
      infoRequest.contentLength = Int64(playlist.count)
      infoRequest.isByteRangeAccessSupported = false
      //      infoRequest.renewalDate = Date()
    }

    if let dataRequest = loadingRequest.dataRequest {
      print("read data request")
      dataRequest.respond(with: playlist.data(using: .ascii)!)
    }
    loadingRequest.finishLoading()

    return true
  }

  func handleInitRequest(_ loadingRequest: AVAssetResourceLoadingRequest) -> Bool {
    print("handleInitRequest")
    guard let segment = playlist.next() else { return false }
    print("segment: \(segment.data.count)")
    return handleDataRequest(loadingRequest, segment.initData(), "public.mpeg-4")
  }

  //  func handleSegmentRequest(_ loadingRequest: AVAssetResourceLoadingRequest) -> Bool {
  //    print("handleSegmentRequest")
  //    guard let segment = playlist.next() else { return false }
  //    print("segment: \(segment.data.count)")
  //    playlist.advance()
  //    return handleDataRequest(loadingRequest, segment.segmentData(), "public.mpeg-4")
  //  }

  func handleSegmentRequest(_ loadingRequest: AVAssetResourceLoadingRequest) -> Bool {
    let url = URL(string: "http://192.168.0.111:8000/2.m4s")!

    //          if let infoRequest = loadingRequest.contentInformationRequest {
    //            print("read info request thing...")
    //            print(infoRequest)
    //            infoRequest.contentType = "public.mpeg-4"
    //            infoRequest.contentLength = Int64(data!.count)
    //            infoRequest.isByteRangeAccessSupported = false
    //            print(infoRequest)
    //          }

    print("setting redirect thing")
    loadingRequest.redirect = URLRequest(url: url)

    //          if let dataRequest = loadingRequest.dataRequest {
    //            print("read data request \(data!.count)")
    //            print(dataRequest)
    //            dataRequest.respond(with: data!)
    //          }
    //        loadingRequest.finishLoading()
    print(loadingRequest)

    return true
  }

  func handleDataRequest(
    _ loadingRequest: AVAssetResourceLoadingRequest,
    _ data: Data,
    _ contentType: String
  ) -> Bool {
    let hash = SHA256.hash(data: data)
    let hex = hash.map { String(format: "%02hhx", $0) }.joined()
    print("handleDataRequest: \(data.count) \(hex)")

    if let infoRequest = loadingRequest.contentInformationRequest {
      print("read info request thing...")
      print(infoRequest)
      infoRequest.contentType = contentType
      infoRequest.contentLength = Int64(data.count)
      infoRequest.isByteRangeAccessSupported = false
      print(infoRequest)
    }

    if let dataRequest = loadingRequest.dataRequest {
      print("read data request \(data.count)")
      print(dataRequest)
      dataRequest.respond(with: data)
    }
    //    loadingRequest.finishLoading()
    print(loadingRequest)

    return true
  }
}

struct PlayerView: UIViewRepresentable {
  @Binding var test: RPCResponseStream<PBVideoClientEvent>?

  func updateUIView(_ uiView: PlayerUIView, context: UIViewRepresentableContext<PlayerView>) {
    if let thing = self.test {
      uiView.setupDelegateThing(thing)
    }
  }

  func makeUIView(context: Context) -> PlayerUIView {
    return PlayerUIView(frame: .zero)
  }
}

struct ContentView: View {
  @State private var test: RPCResponseStream<PBVideoClientEvent>?

  var body: some View {
    let client = FrontendRPCClient()

    let handleCreateProfile: () -> Void = {
      firstly {
        client.createProfile(
          PBCreateProfileRequest.with {
            $0.name = "test"
            $0.password = "test"
          }
        )
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
        client.loadProfile(
          PBLoadProfileRequest.with {
            $0.id = res.profiles[0].id
            $0.name = "test"
            $0.password = "test"
          }
        )
      }.done { res in
        let json = try res.jsonString()
        print("profile: \(json)")
      }.catch { error in
        print("loading profile failed \(error.localizedDescription)")
      }
    }

    let handleCreateBootstrapClient: () -> Void = {
      firstly {
        client.createBootstrapClient(
          PBCreateBootstrapClientRequest.with {
            $0.clientOptions = PBCreateBootstrapClientRequest.OneOf_ClientOptions.websocketOptions(
              PBBootstrapClientWebSocketOptions.with {
                $0.url = "wss://192.168.0.111:8080/test-bootstrap"
                $0.insecureSkipVerifyTls = true
              }
            )
          }
        )
      }.done { res in
        let json = try res.jsonString()
        print("profile: \(json)")
      }.catch { error in
        print("creating network failed \(error.localizedDescription)")
      }
    }

    let handleLoadInviteCert: () -> Void = {
      firstly {
        client.createNetworkMembershipFromInvitation(
          PBCreateNetworkMembershipFromInvitationRequest.with {
            $0.invitationB64 =
              "EoADCmYIARJA3+jPfL6RMfY8aLFZRDYdmzY5s8gsuEvzrLNOM+KQxDtU0VEHnhGkPOp8mryKzh5ISz1dpRr8xD2kcZMIZ+dNRhogVNFRB54RpDzqfJq8is4eSEs9XaUa/MQ9pHGTCGfnTUYSjwIKIFTRUQeeEaQ86nyavIrOHkhLPV2lGvzEPaRxkwhn501GEAEYBiCH8ob6BSiH56v6BTIQTKUyIcq6qpRJYxpU4CVm0zpAIcPsy2/eBc/FLAp62xJka2WVrWqa8JdnYscnOh80REVOPSQbJ5s1uXQRUqJ8hwUUCMw7rPRhP29ZTV8ZGTznCEKGAQogHVmKdL3JUzXvjh3BQ8tFqFCvzPp7Wxe4ak2yWbjSj/cQARgEINb+kfcFKNbMm5UGMhCGDUmvQDYLYehxX3XjVz/EOkAixCMT3+O7tBwyhTEid0bCtxNkpAN6FkrSHdOiIkAv4wWp/OJ3UzlWpYGaA01wO27gUIEb+82rUyDdOGibKKwEIgR0ZXN0"
          }
        )
      }.done { res in
        let json = try res.jsonString()
        print("profile: \(json)")
      }.catch { error in
        print("creating network failed \(error.localizedDescription)")
      }
    }

    let handleCreateNetwork: () -> Void = {
      firstly {
        client.createNetwork(
          PBCreateNetworkRequest.with {
            $0.name = "test"
          }
        )
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
          switch eventType {
          case RPCEvent.data:
            do {
              let json = try event?.jsonString()
              print("vpn event: \(json!)")
            }
            catch {}
          case RPCEvent.close:
            print("vpn event stream closed")
          default:
            print("vpn rpc error")
          }
        }
      }
      catch {
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
        when(
          fulfilled: memberships.networkMemberships.map { membership in
            client.publishSwarm(
              PBPublishSwarmRequest.with {
                $0.id = id
                $0.networkKey = rootCert(membership.certificate).key
              }
            )
          } as [Promise<PBPublishSwarmResponse>]
        )
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
          switch eventType {
          case RPCEvent.data:
            if let body = event?.body {
              switch (body) {
              case .open(let open):
                print("open: \(open.id)")
                publishSwarm(open.id)
                self.test = client
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
      }
      catch {
        print("joining video swarm failed \(error)")
      }
    }

    return VStack {
      PlayerView(test: $test)
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

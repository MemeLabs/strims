// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The video control on fake nodes (see `strims_testworld`), through a
//! UI's RPC connection: channel records and their checks, RTMP URLs and
//! stream keys, what is supported, the ingress and HLS configs, change
//! events, and a network's channels going with it.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fmt::Debug;
use std::future::Future;

use futures_util::StreamExt;
use strims_base::rng::det_rng;
use strims_crypto::{
    DEFAULT_CERT_TTL, Issuer, generate_key, network_certificate, new_certificate_request,
    sign_certificate_request,
};
use strims_dao::schema::chat::ChatServers;
use strims_dao::schema::network::Networks;
use strims_dao::schema::video::{
    VideoChannels, channel_by_stream_key, new_channel, parse_stream_key,
};
use strims_dao::{Recv, Subscription, Table};
use strims_directory::snippet::MAX_TEXT_BYTES;
use strims_kv::MemoryKv;
use strims_proto::chat::v1::Server;
use strims_proto::network::v1::directory::ListingSnippet;
use strims_proto::service::CallError;
use strims_proto::types::KeyUsage;
use strims_proto::video::v1::video_channel::{Local, LocalShare, Owner, RemoteShare};
use strims_proto::video::v1::{
    HlsEgressClient, HlsEgressCloseStreamRequest, HlsEgressConfig, HlsEgressGetConfigRequest,
    HlsEgressIsSupportedRequest, HlsEgressOpenStreamRequest, HlsEgressSetConfigRequest,
    VideoChannel, VideoChannelCreateRequest, VideoChannelDeleteRequest, VideoChannelFrontendClient,
    VideoChannelGetRequest, VideoChannelListRequest, VideoChannelUpdateRequest, VideoIngressClient,
    VideoIngressConfig, VideoIngressGetChannelUrlRequest, VideoIngressGetChannelUrlResponse,
    VideoIngressGetConfigRequest, VideoIngressIsSupportedRequest, VideoIngressListStreamsRequest,
    VideoIngressSetConfigRequest,
};
use strims_rpc::{Client, Router, Timer};
use strims_testworld::*;
use strims_video::hls::HlsRoutes;
use strims_video::host::{NoConnection, NoListener, NoRtmp};
use strims_video::{
    Config, HlsSettings, MAX_CHANNELS, NoVideoHost, VideoControl, VideoEvent, VideoHost,
    bind_frontend,
};

/// A host whose RTMP listener is supported, though no listen succeeds.
#[derive(Clone)]
struct ListeningHost;

impl VideoHost for ListeningHost {
    type Error = NoRtmp;
    type Reader = NoConnection;
    type Writer = NoConnection;
    type Listener = NoListener;

    fn rtmp_supported(&self) -> bool {
        true
    }

    fn listen(&self, _: &str) -> impl Future<Output = Result<NoListener, NoRtmp>> {
        std::future::ready(Err(NoRtmp))
    }

    type Codec = strims_video::codec::NoCodec;

    fn codec(&self) -> Option<Self::Codec> {
        None
    }
}

type Control<V> = VideoControl<MemoryKv, FakeHost, V, TokioTimer>;

/// A node's video control, and a UI connected to its frontend.
struct Side<V: VideoHost> {
    video: Control<V>,
    channels: VideoChannelFrontendClient<Client>,
    ingress: VideoIngressClient<Client>,
    hls: HlsEgressClient<Client>,
}

fn start(n: &Node) -> Side<NoVideoHost> {
    start_with(n, NoVideoHost, Config::default())
}

fn start_with<V: VideoHost>(n: &Node, host: V, config: Config) -> Side<V> {
    let video = VideoControl::new(
        n.host.clone(),
        host,
        n.store.clone(),
        n.profile.clone(),
        n.network.clone(),
        n.transfers.clone(),
        n.directory.clone(),
        HlsRoutes::new(),
        n.timer.clone(),
        n.rng(),
        config,
    );
    let v = video.clone();
    n.spawn(async move { v.run().await });
    let router = Router::new();
    bind_frontend(&router, &video);
    let client = frontend_client(router);
    Side {
        video,
        channels: VideoChannelFrontendClient::new(client.clone()),
        ingress: VideoIngressClient::new(client.clone()),
        hls: HlsEgressClient::new(client),
    }
}

/// The message of a call's remote error.
fn refusal<T: Debug>(r: Result<T, CallError>) -> String {
    match r {
        Err(CallError::Remote(e)) => e.message,
        r => panic!("not refused: {r:?}"),
    }
}

fn snippet(title: &str) -> ListingSnippet {
    ListingSnippet {
        title: title.into(),
        description: "desc".into(),
        channel_name: "chan".into(),
        tags: vec!["a".into(), "b".into()],
        ..Default::default()
    }
}

impl<V: VideoHost> Side<V> {
    async fn create(&self, network_key: &[u8], title: &str) -> Result<VideoChannel, CallError> {
        let res = self
            .channels
            .create(&VideoChannelCreateRequest {
                directory_listing_snippet: Some(snippet(title)),
                network_key: network_key.to_vec(),
            })
            .await?;
        Ok(res.channel.unwrap())
    }

    async fn update(
        &self,
        id: u64,
        network_key: &[u8],
        s: ListingSnippet,
    ) -> Result<VideoChannel, CallError> {
        let res = self
            .channels
            .update(&VideoChannelUpdateRequest {
                id,
                directory_listing_snippet: Some(s),
                network_key: network_key.to_vec(),
            })
            .await?;
        Ok(res.channel.unwrap())
    }

    async fn get(&self, id: u64) -> Result<VideoChannel, CallError> {
        let res = self.channels.get(&VideoChannelGetRequest { id }).await?;
        Ok(res.channel.unwrap())
    }

    async fn list(&self) -> Vec<VideoChannel> {
        let res = self.channels.list(&VideoChannelListRequest {}).await;
        res.unwrap().channels
    }

    async fn url(&self, id: u64) -> Result<VideoIngressGetChannelUrlResponse, CallError> {
        self.ingress
            .get_channel_url(&VideoIngressGetChannelUrlRequest { id })
            .await
    }

    async fn set_ingress(&self, c: VideoIngressConfig) -> Result<VideoIngressConfig, CallError> {
        let res = self
            .ingress
            .set_config(&VideoIngressSetConfigRequest { config: Some(c) })
            .await?;
        Ok(res.config.unwrap())
    }

    async fn ingress_config(&self) -> VideoIngressConfig {
        let res = self.ingress.get_config(&VideoIngressGetConfigRequest {});
        res.await.unwrap().config.unwrap()
    }

    async fn set_hls(&self, c: HlsEgressConfig) -> Result<HlsEgressConfig, CallError> {
        let res = self
            .hls
            .set_config(&HlsEgressSetConfigRequest { config: Some(c) })
            .await?;
        Ok(res.config.unwrap())
    }

    async fn hls_config(&self) -> HlsEgressConfig {
        let res = self.hls.get_config(&HlsEgressGetConfigRequest {});
        res.await.unwrap().config.unwrap()
    }

    async fn supported(&self) -> (bool, bool) {
        let ingress = self
            .ingress
            .is_supported(&VideoIngressIsSupportedRequest {})
            .await
            .unwrap();
        let hls = self
            .hls
            .is_supported(&HlsEgressIsSupportedRequest {})
            .await
            .unwrap();
        (ingress.supported, hls.supported)
    }
}

fn ingress(server_addr: &str, public_server_addr: &str) -> VideoIngressConfig {
    VideoIngressConfig {
        enabled: true,
        server_addr: server_addr.into(),
        public_server_addr: public_server_addr.into(),
        service_network_keys: Vec::new(),
    }
}

/// Stores a channel of `owner`, as a share would.
async fn seed_channel(n: &Node, owner: Owner) -> VideoChannel {
    let mut tx = n.store.write(n.timer.now()).await.unwrap();
    let c = new_channel(&mut tx, owner, snippet("seeded"))
        .await
        .unwrap();
    let c = VideoChannels::insert(&mut tx, c).await.unwrap();
    tx.commit().await.unwrap();
    c
}

/// The channel stream key `key` names on `n`.
async fn by_stream_key(n: &Node, key: &str) -> Option<VideoChannel> {
    let mut tx = n.store.read().await.unwrap();
    channel_by_stream_key(&mut tx, key).await.unwrap()
}

/// The next event within 5 seconds.
async fn next_event(events: &mut Subscription<VideoEvent>) -> VideoEvent {
    let wait = std::time::Duration::from_secs(5);
    match tokio::time::timeout(wait, events.next()).await {
        Ok(Some(Recv::Event(e))) => (*e).clone(),
        r => panic!("no event: {r:?}"),
    }
}

/// Channels are created as the profile's own on one of its networks with
/// a new key and token, and keep them through updates; update and delete
/// refuse channels that don't exist.
#[test]
fn channels_round_trip_through_the_frontend() {
    run(async {
        let world = World::default();
        let n = node_in(&world, 1).await;
        let (net, _) = owned_network(&n, &[], 10).await;
        let (other_net, _) = owned_network(&n, &[], 11).await;
        let (net, other_net) = (net.public, other_net.public);
        let s = start(&n);

        let a = s.create(&net, "first").await.unwrap();
        let owner = Owner::Local(Local {
            auth_key: n.profile_key().public.clone(),
            network_key: net.clone(),
        });
        assert_eq!(a.owner, Some(owner));
        let key = a.key.clone().unwrap();
        assert_eq!((key.public.len(), key.private.len()), (32, 64));
        assert_eq!(a.token.len(), 32);
        assert_eq!(a.directory_listing_snippet, Some(snippet("first")));
        assert_eq!(s.get(a.id).await.unwrap(), a);

        let b = s.create(&net, "second").await.unwrap();
        assert_ne!(b.id, a.id);
        assert_ne!(b.key, a.key);
        assert_ne!(b.token, a.token);
        assert_eq!(s.list().await, [a.clone(), b.clone()]);

        let moved = s.update(a.id, &other_net, snippet("moved")).await.unwrap();
        assert_eq!(
            (moved.id, &moved.key, &moved.token),
            (a.id, &a.key, &a.token)
        );
        assert_eq!(moved.directory_listing_snippet, Some(snippet("moved")));
        let Some(Owner::Local(l)) = &moved.owner else {
            panic!("not local: {moved:?}");
        };
        assert_eq!(l.network_key, other_net);
        assert_eq!(s.get(a.id).await.unwrap(), moved);

        let missing = 999_999;
        assert_eq!(
            refusal(s.update(missing, &net, snippet("x")).await),
            "video channel not found"
        );
        assert_eq!(refusal(s.get(missing).await), "video channel not found");
        let delete = |id| s.channels.delete(&VideoChannelDeleteRequest { id });
        delete(a.id).await.unwrap();
        assert_eq!(refusal(delete(a.id).await), "video channel not found");
        assert_eq!(refusal(s.get(a.id).await), "video channel not found");
        assert_eq!(s.list().await, [b]);
    });
}

/// A channel's snippet passes the directory's bounds and its network is
/// one of the profile's; a refused create or update stores nothing.
#[test]
fn channels_refuse_large_snippets_unknown_networks_and_past_their_bound() {
    run(async {
        let world = World::default();
        let n = node_in(&world, 1).await;
        let (net, _) = owned_network(&n, &[], 10).await;
        let net = net.public;
        let s = start(&n);

        let long = "x".repeat(MAX_TEXT_BYTES + 1);
        assert_eq!(refusal(s.create(&net, &long).await), "snippet too large");
        for bad in [Vec::new(), vec![1; 32], vec![1; 1024], net[..16].to_vec()] {
            assert_eq!(refusal(s.create(&bad, "x").await), "network not found");
        }
        assert!(s.list().await.is_empty());

        let c = s.create(&net, "kept").await.unwrap();
        assert_eq!(
            refusal(s.update(c.id, &net, snippet(&long)).await),
            "snippet too large"
        );
        assert_eq!(
            refusal(s.update(c.id, &[2; 32], snippet("x")).await),
            "network not found"
        );
        assert_eq!(s.get(c.id).await.unwrap(), c);

        for i in 1..MAX_CHANNELS {
            s.create(&net, &format!("c{i}")).await.unwrap();
        }
        assert_eq!(
            refusal(s.create(&net, "one too many").await),
            "too many video channels"
        );
        assert_eq!(s.list().await.len(), MAX_CHANNELS);
    });
}

/// A local channel streams to this device's ingress at its public address,
/// or its listen address without one; a remote share to the sharing
/// device's ingress under the share's id; a local share has no URL here.
/// Each key is the token signed by the profile's key.
#[test]
fn channel_urls_by_owner() {
    run(async {
        let world = World::default();
        let n = node_in(&world, 1).await;
        let (net_key, _) = owned_network(&n, &[], 10).await;
        let net = net_key.public.clone();
        let s = start(&n);
        let profile_key = n.profile_key().public.clone();

        let local = s.create(&net, "local").await.unwrap();
        let u = s.url(local.id).await.unwrap();
        assert_eq!(u.server_addr, "127.0.0.1:1935");
        assert_eq!(
            u.url,
            format!("rtmp://127.0.0.1:1935/live/{}", u.stream_key)
        );
        assert_eq!(by_stream_key(&n, &u.stream_key).await, Some(local.clone()));
        let (id, signature) = parse_stream_key(&u.stream_key).unwrap();
        assert_eq!(id, local.id);
        assert!(strims_crypto::sign::verify(
            strims_crypto::Domain::VideoStreamKey,
            &profile_key,
            &[&local.token],
            &signature
        ));

        s.set_ingress(ingress("0.0.0.0:1935", "")).await.unwrap();
        let listen = s.url(local.id).await.unwrap();
        assert_eq!(listen.server_addr, "0.0.0.0:1935");
        assert_eq!(listen.stream_key, u.stream_key);
        s.set_ingress(ingress("0.0.0.0:1935", "ingress.example.com"))
            .await
            .unwrap();
        let public = s.url(local.id).await.unwrap();
        assert_eq!(public.server_addr, "ingress.example.com");
        assert_eq!(
            public.url,
            format!("rtmp://ingress.example.com/live/{}", u.stream_key)
        );

        let remote = seed_channel(
            &n,
            Owner::RemoteShare(RemoteShare {
                id: 4242,
                network_key: net.clone(),
                service_key: vec![7; 32],
                service_salt: b"salt".to_vec(),
                server_addr: "remote.example.com:1935".into(),
            }),
        )
        .await;
        let r = s.url(remote.id).await.unwrap();
        assert_eq!(r.server_addr, "remote.example.com:1935");
        assert_eq!(
            r.url,
            format!("rtmp://remote.example.com:1935/live/{}", r.stream_key)
        );
        let (id, signature) = parse_stream_key(&r.stream_key).unwrap();
        assert_eq!(id, 4242);
        assert!(strims_crypto::sign::verify(
            strims_crypto::Domain::VideoStreamKey,
            &profile_key,
            &[&remote.token],
            &signature
        ));
        assert_eq!(by_stream_key(&n, &r.stream_key).await, None);

        let rng = &mut det_rng(9);
        let root = network_certificate(&net_key, "net", DEFAULT_CERT_TTL, T0, rng).unwrap();
        let sharer = generate_key(rng);
        let csr = new_certificate_request(&sharer, KeyUsage::Peer as i32, "").unwrap();
        let issuer = Issuer::Certificate {
            key: &net_key,
            certificate: &root,
        };
        let certificate =
            sign_certificate_request(&csr, DEFAULT_CERT_TTL, issuer, T0, rng).unwrap();
        let share = seed_channel(
            &n,
            Owner::LocalShare(LocalShare {
                certificate: Some(certificate),
            }),
        )
        .await;
        assert_eq!(
            refusal(s.url(share.id).await),
            "cannot generate stream key for channel"
        );
        assert_eq!(refusal(s.url(999_999).await), "video channel not found");

        let claimed = s.update(remote.id, &net, snippet("mine")).await.unwrap();
        assert!(matches!(claimed.owner, Some(Owner::Local(_))));
        let mine = s.url(remote.id).await.unwrap();
        assert_eq!(mine.server_addr, "ingress.example.com");
        assert_eq!(by_stream_key(&n, &mine.stream_key).await, Some(claimed));
    });
}

/// Ingress is supported where the host listens, and HLS where the driver
/// serves it.
#[test]
fn support_follows_the_host_and_config() {
    run(async {
        let world = World::default();
        let plain = node_in(&world, 1).await;
        let listening = node_in(&world, 2).await;
        let serving = node_in(&world, 3).await;
        let config = Config {
            hls: HlsSettings {
                serve: true,
                base_url: "http://127.0.0.1:8082".into(),
                ..HlsSettings::default()
            },
            ..Config::default()
        };

        assert_eq!(start(&plain).supported().await, (false, false));
        let s = start_with(&listening, ListeningHost, Config::default());
        assert_eq!(s.supported().await, (true, false));
        assert!(s.video.ingress_supported());
        let s = start_with(&serving, ListeningHost, config.clone());
        assert_eq!(s.supported().await, (true, true));
        assert_eq!(
            start_with(&serving, NoVideoHost, config).supported().await,
            (false, true)
        );
    });
}

/// The configs start at their defaults, take what passes their checks and
/// refuse the rest unchanged. The ingress has no streams yet, and HLS
/// streams don't open where the driver doesn't serve HLS.
#[test]
fn configs_are_checked_and_round_trip() {
    run(async {
        let world = World::default();
        let n = node_in(&world, 1).await;
        let s = start(&n);

        assert_eq!(
            s.ingress_config().await,
            VideoIngressConfig {
                server_addr: "127.0.0.1:1935".into(),
                ..Default::default()
            }
        );
        let c = VideoIngressConfig {
            service_network_keys: vec![vec![1; 32], vec![2; 64]],
            ..ingress("[::]:1935", "ingress.example.com:1935")
        };
        assert_eq!(s.set_ingress(c.clone()).await.unwrap(), c);
        assert_eq!(s.ingress_config().await, c);
        let refused = [
            (ingress("1935", ""), "server address must be host:port"),
            (ingress("", ""), "server address must be host:port"),
            (
                ingress(&format!("{}:1935", "h".repeat(256)), ""),
                "server address too long",
            ),
            (
                ingress("a:1", &"p".repeat(257)),
                "public server address too long",
            ),
            (
                VideoIngressConfig {
                    service_network_keys: vec![vec![1; 32]; 65],
                    ..ingress("a:1", "")
                },
                "too many service networks",
            ),
            (
                VideoIngressConfig {
                    service_network_keys: vec![vec![1; 65]],
                    ..ingress("a:1", "")
                },
                "service network key too long",
            ),
        ];
        for (bad, message) in refused {
            assert_eq!(refusal(s.set_ingress(bad).await), message);
        }
        let empty = s
            .ingress
            .set_config(&VideoIngressSetConfigRequest { config: None });
        assert_eq!(refusal(empty.await), "server address must be host:port");
        assert_eq!(s.ingress_config().await, c);

        assert_eq!(s.hls_config().await, HlsEgressConfig::default());
        let h = HlsEgressConfig {
            enabled: true,
            public_server_addr: "https://cdn.example.com".into(),
        };
        assert_eq!(s.set_hls(h.clone()).await.unwrap(), h);
        assert_eq!(s.hls_config().await, h);
        let bad = |addr: &str| HlsEgressConfig {
            enabled: true,
            public_server_addr: addr.into(),
        };
        assert_eq!(
            refusal(s.set_hls(bad("cdn.example.com")).await),
            "public server address must start with http:// or https://"
        );
        let long = format!("https://{}", "a".repeat(256));
        assert_eq!(
            refusal(s.set_hls(bad(&long)).await),
            "public server address too long"
        );
        assert_eq!(s.hls_config().await, h);
        s.hls
            .set_config(&HlsEgressSetConfigRequest { config: None })
            .await
            .unwrap();
        assert_eq!(s.hls_config().await, HlsEgressConfig::default());

        let streams = s.ingress.list_streams(&VideoIngressListStreamsRequest {});
        assert!(streams.await.unwrap().streams.is_empty());
        let open = s.hls.open_stream(&HlsEgressOpenStreamRequest {
            swarm_uri: "magnet:?xt=urn:ppspp:x".into(),
            ..Default::default()
        });
        assert_eq!(refusal(open.await), "HLS egress is not supported");
        let close = s.hls.close_stream(&HlsEgressCloseStreamRequest {
            transfer_id: vec![0; 32],
        });
        assert_eq!(refusal(close.await), "HLS stream not found");
    });
}

/// Each write of a channel or config is reported once, and a config
/// written unchanged isn't. What the store held when the control started
/// isn't reported.
#[test]
fn changes_are_reported() {
    run(async {
        let world = World::default();
        let n = node_in(&world, 1).await;
        let (net, _) = owned_network(&n, &[], 10).await;
        let net = net.public;
        let before = seed_channel(&n, Owner::Local(Local::default())).await;
        let s = start(&n);
        let mut events = s.video.subscribe();
        settle().await;
        assert_eq!(drain(&mut events), []);

        let c = s.create(&net, "new").await.unwrap();
        assert_eq!(
            next_event(&mut events).await,
            VideoEvent::ChannelChanged(c.clone())
        );
        let c = s.update(c.id, &net, snippet("changed")).await.unwrap();
        assert_eq!(
            next_event(&mut events).await,
            VideoEvent::ChannelChanged(c.clone())
        );
        s.channels
            .delete(&VideoChannelDeleteRequest { id: before.id })
            .await
            .unwrap();
        assert_eq!(
            next_event(&mut events).await,
            VideoEvent::ChannelDeleted(before)
        );

        let i = ingress("0.0.0.0:1935", "");
        s.set_ingress(i.clone()).await.unwrap();
        assert_eq!(
            next_event(&mut events).await,
            VideoEvent::IngressConfigChanged(i.clone())
        );
        let h = HlsEgressConfig {
            enabled: true,
            public_server_addr: String::new(),
        };
        s.set_hls(h.clone()).await.unwrap();
        assert_eq!(
            next_event(&mut events).await,
            VideoEvent::HlsConfigChanged(h.clone())
        );
        s.set_ingress(i).await.unwrap();
        s.set_hls(h).await.unwrap();
        settle().await;
        assert_eq!(drain(&mut events), []);
        assert_eq!(s.video.metrics().store_lags, 0);
    });
}

/// After the control's store events lag, it reads the records again and
/// reports what differs from what it last reported.
#[test]
fn a_lagged_control_reports_what_changed_meanwhile() {
    run(async {
        let world = World::default();
        let n = node_in(&world, 1).await;
        let (net, _) = owned_network(&n, &[], 10).await;
        let net = net.public;
        let s = start(&n);
        let mut events = s.video.subscribe();
        let gone = s.create(&net, "gone").await.unwrap();
        assert_eq!(
            next_event(&mut events).await,
            VideoEvent::ChannelChanged(gone.clone())
        );

        let mut tx = n.store.write(n.timer.now()).await.unwrap();
        VideoChannels::delete(&mut tx, gone.id).await.unwrap();
        let owner = Owner::Local(Local {
            auth_key: n.profile_key().public.clone(),
            network_key: net.clone(),
        });
        let added = new_channel(&mut tx, owner, snippet("added")).await.unwrap();
        let added = VideoChannels::insert(&mut tx, added).await.unwrap();
        for _ in 0..300 {
            let id = tx.generate_id().await.unwrap();
            let server = Server {
                id,
                ..Default::default()
            };
            ChatServers::insert(&mut tx, server).await.unwrap();
        }
        tx.commit().await.unwrap();

        assert_eq!(
            next_event(&mut events).await,
            VideoEvent::ChannelDeleted(gone)
        );
        assert_eq!(
            next_event(&mut events).await,
            VideoEvent::ChannelChanged(added)
        );
        settle().await;
        assert_eq!(drain(&mut events), []);
        assert_eq!(s.video.metrics().store_lags, 1);
    });
}

/// Deleting a network deletes the channels on it and drops its key from
/// the ingress config, and both are reported.
#[test]
fn a_deleted_networks_channels_go_with_it() {
    run(async {
        let world = World::default();
        let n = node_in(&world, 1).await;
        let (net, _) = owned_network(&n, &[], 10).await;
        let (kept_net, _) = owned_network(&n, &[], 11).await;
        let (net, kept_net) = (net.public, kept_net.public);
        let s = start(&n);
        let doomed = s.create(&net, "doomed").await.unwrap();
        let kept = s.create(&kept_net, "kept").await.unwrap();
        let config = VideoIngressConfig {
            service_network_keys: vec![net.clone(), kept_net.clone()],
            ..ingress("0.0.0.0:1935", "")
        };
        s.set_ingress(config.clone()).await.unwrap();
        let mut events = s.video.subscribe();
        settle().await;
        drain(&mut events);

        let mut tx = n.store.write(n.timer.now()).await.unwrap();
        assert!(Networks::delete(&mut tx, 10).await.unwrap());
        tx.commit().await.unwrap();

        let dropped = VideoIngressConfig {
            service_network_keys: vec![kept_net.clone()],
            ..config
        };
        assert_eq!(
            next_event(&mut events).await,
            VideoEvent::ChannelDeleted(doomed.clone())
        );
        assert_eq!(
            next_event(&mut events).await,
            VideoEvent::IngressConfigChanged(dropped.clone())
        );
        assert_eq!(s.list().await, [kept]);
        assert_eq!(s.ingress_config().await, dropped);
        assert_eq!(refusal(s.url(doomed.id).await), "video channel not found");
        assert_eq!(refusal(s.create(&net, "x").await), "network not found");
    });
}

// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The network directory between a Go and a Rust node over a direct link:
//! Go's internal/directory on the helper (`-directory`, the `dir-*` verbs
//! and events), and `strims_directory` through `strims_node::app` on a
//! Rust node with a profile store, network control and transfer control.
//! The owner of the network serves its directory; the other reads it from
//! the event and asset swarms, which bind by announce once the network
//! binds.
//!
//! - `go_owner_rust_member`: Go publishes before Rust joins, and Rust's
//!   view, read from Go's event swarm from its window tail, shows the
//!   listing. Rust publishes, joins and parts at Go's server, and both
//!   views show the listings, their users and presence. Go's server sends
//!   a listing's user count once the listing changes after its first
//!   broadcast, so the counts follow Rust's join. Rust's moderation is
//!   refused until Go, the owner and so an admin, makes Rust a moderator;
//!   then Rust's ban removes the listing from both views, and Go can't
//!   publish it again. Go's server subscribes to the snippet of Rust's
//!   media listing on Rust's snippet port, and both views show it. A
//!   server config change rewrites Go's asset swarm, and Rust reads the
//!   new bundle. Rust renews its alias at Go's CA, and its ping takes the
//!   alias to Go's server.
//! - `rust_owner_go_member`: the reverse. Go reads Rust's early listing
//!   from the window tail of Rust's event swarm, publishes, and decodes
//!   Rust's broadcasts record by record. Rust's server subscribes to the
//!   snippet of Go's media listing on Go's snippet port, verifies Go's
//!   signature and takes an update. Go's moderation is refused until Rust
//!   makes it a moderator. Rust's asset bundle, with its icon, reaches Go,
//!   and again once a server config change rewrites it. Go renews its
//!   alias at Rust's CA, and its ping takes the alias to Rust's server.
//!
//! The snippets carry both images, so their first delta splits and their
//! events span several frames.
//!
//! On a loaded machine Go's own view of the network it owns can stay
//! empty, and `go_owner_rust_member` then times out waiting for it: Go's
//! directory reader opens a client before the server takes the lease, and
//! internal/servicemanager loses a stop that comes before the client's
//! `Run` starts, so the reader stays on the client's empty swarm.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration as StdDuration;

use prost::Message;
use strims_base::rng::det_rng;
use strims_crypto::{SignableMessage, key_from_seed, membership_trusted};
use strims_dao::schema::profile::Profile as ProfileRecord;
use strims_dao::seal::StorageKey;
use strims_dao::{ProfileStore, ReplicatedStore, Singleton};
use strims_directory::{CallFailure, DirectoryControl, DirectoryError};
use strims_kv::MemoryKv;
use strims_network::{NetworkControl, NetworkFrontend};
use strims_node::app::{NodeHost, NodeTimer, PeerServices};
use strims_node::{
    Clock, DeliveryMode, HostId, NetworkId, Node, NodeConfig, NodeEvent, SwarmOptions, SystemClock,
    TokioNet, WriterOptions,
};
use strims_ppspp::{Uri, UriOptions};
use strims_proto::network::v1::create_network_from_invitation_request::Invitation as InvitationField;
use strims_proto::network::v1::directory::event::Body;
use strims_proto::network::v1::directory::listing::{Chat, Content, Media};
use strims_proto::network::v1::directory::listing_query::Query;
use strims_proto::network::v1::directory::listing_snippet_image::SourceOneof;
use strims_proto::network::v1::directory::server_config::Integrations;
use strims_proto::network::v1::directory::server_config::integrations::Swarm;
use strims_proto::network::v1::directory::{
    EventBroadcast, Listing, ListingModeration, ListingQuery, ListingSnippet, ListingSnippetImage,
    NetworkListings, NetworkListingsItem, ServerConfig as DirectoryConfig, UserModeration,
};
use strims_proto::network::v1::{
    CreateInvitationRequest, CreateNetworkFromInvitationRequest, CreateServerRequest, Network,
    NetworkFrontendService, UpdateAliasRequest, UpdateServerConfigRequest,
};
use strims_proto::profile::v1::Profile;
use strims_proto::service::Context;
use strims_proto::types::{Image, ImageType};
use strims_transfer::TransferControl;
use strims_vpn::VerifyMode;
use tokio::task::{LocalSet, spawn_local};
use tokio::time::Instant;

use crate::harness::{GoEvent, GoNode, GoOptions, helper};

pub(crate) type Timer = NodeTimer<SystemClock>;
type Control = NetworkControl<MemoryKv, NodeHost, Timer>;
type Frontend = NetworkFrontend<MemoryKv, NodeHost, Timer>;
type Transfers = TransferControl<NodeHost, Timer>;
type Directory = DirectoryControl<MemoryKv, NodeHost, Timer>;
type Bound = Rc<RefCell<BTreeMap<HostId, bool>>>;

/// How long a directory step may take: a broadcast interval, a swarm
/// round trip and a call's retries, with room to spare.
pub(crate) const DIR_STEP: StdDuration = StdDuration::from_secs(60);

/// A Rust node running its profile's networks and their directories.
pub(crate) struct RustMember {
    pub(crate) node: Node,
    pub(crate) host: NodeHost,
    pub(crate) store: ReplicatedStore<MemoryKv>,
    pub(crate) profile: Profile,
    pub(crate) control: Control,
    pub(crate) frontend: Frontend,
    pub(crate) transfers: Transfers,
    pub(crate) directory: Directory,
    /// Peers bound to a network.
    bound: Bound,
}

pub(crate) async fn rust_member(seed: u8) -> RustMember {
    let key = key_from_seed(&[seed; 32]);
    let profile = Profile {
        id: u64::from(seed),
        name: format!("rust-{seed}"),
        key: Some(key.clone()),
        device_id: 1,
        ..Default::default()
    };
    let mut config = NodeConfig::new(key, u64::from(seed));
    config.host_key_seed = Some([seed.wrapping_add(0x80); 32]);
    config.label = format!("rust-{seed}");
    config.tcp_listen = Some("127.0.0.1:0".to_owned());
    config.max_peers = 16;
    config.vpn.verify = VerifyMode::Audit;
    let (node, mut events) = Node::spawn(config, TokioNet, SystemClock).await.unwrap();

    let now = SystemClock.now();
    let store = ProfileStore::create(
        MemoryKv::new(),
        profile.id,
        StorageKey::from_bytes([seed; 32]),
        det_rng(u64::from(seed)),
        now,
    )
    .await
    .unwrap();
    let mut tx = store.write(now).await.unwrap();
    ProfileRecord::set(&mut tx, profile.clone()).await.unwrap();
    tx.commit().await.unwrap();
    let store = ReplicatedStore::new(store).await.unwrap();

    let host = NodeHost::new(node.clone());
    let control = NetworkControl::new(
        host.clone(),
        store.clone(),
        profile.clone(),
        NodeTimer(SystemClock),
        det_rng(u64::from(seed) + 1),
        strims_network::Config::default(),
    );
    let transfers = TransferControl::new(
        host.clone(),
        NodeTimer(SystemClock),
        strims_transfer::Config::default(),
    );
    let directory = DirectoryControl::new(
        host.clone(),
        store.clone(),
        control.clone(),
        transfers.clone(),
        NodeTimer(SystemClock),
        det_rng(u64::from(seed) + 2),
        strims_directory::Config::default(),
    );
    spawn_local({
        let control = control.clone();
        async move { control.run().await }
    });
    spawn_local({
        let transfers = transfers.clone();
        async move { transfers.run().await }
    });
    spawn_local({
        let directory = directory.clone();
        async move { directory.run().await }
    });
    let services = PeerServices::new(node.clone())
        .with(Rc::new(control.clone()))
        .with(Rc::new(transfers.clone()));
    let bound: Bound = Rc::default();
    let b2 = bound.clone();
    spawn_local(async move {
        while let Some(e) = events.recv().await {
            match &e {
                NodeEvent::NetworkPeerAdded { host_id, .. } => {
                    b2.borrow_mut().insert(*host_id, true);
                }
                NodeEvent::NetworkPeerRemoved { host_id, .. } => {
                    b2.borrow_mut().insert(*host_id, false);
                }
                NodeEvent::TransferError { .. } => eprintln!("rust {seed}: {e:?}"),
                _ => {}
            }
            services.handle_event(&e).await;
        }
    });
    let frontend = NetworkFrontend::new(control.clone());
    RustMember {
        node,
        host,
        store,
        profile,
        control,
        frontend,
        transfers,
        directory,
        bound,
    }
}

impl RustMember {
    pub(crate) fn peer_key(&self) -> Vec<u8> {
        self.profile.key.as_ref().unwrap().public.clone()
    }

    fn alias(&self) -> &str {
        &self.profile.name
    }

    pub(crate) async fn create(&self, name: &str, icon: Option<Image>) -> Network {
        self.frontend
            .create_server(
                &Context::default(),
                CreateServerRequest {
                    name: name.into(),
                    icon,
                    ..Default::default()
                },
            )
            .await
            .unwrap()
            .network
            .unwrap()
    }

    pub(crate) async fn join(&self, invitation: Vec<u8>) -> Network {
        self.frontend
            .create_network_from_invitation(
                &Context::default(),
                CreateNetworkFromInvitationRequest {
                    alias: String::new(),
                    invitation: Some(InvitationField::InvitationBytes(invitation)),
                },
            )
            .await
            .unwrap()
            .network
            .unwrap()
    }

    pub(crate) fn is_bound(&self, peer: HostId) -> bool {
        self.bound.borrow().get(&peer).copied().unwrap_or(false)
    }

    pub(crate) fn trusted(&self, id: u64) -> bool {
        let now = SystemClock.now();
        self.control.networks().iter().any(|n| {
            n.network.id == id
                && n.network
                    .certificate
                    .as_ref()
                    .is_some_and(|c| membership_trusted(c, now))
        })
    }

    fn net(&self, id: u64) -> NetworkId {
        self.control
            .networks()
            .iter()
            .find(|n| n.network.id == id)
            .and_then(|n| n.net)
            .unwrap()
    }

    /// Listing `id` of the view of network `network_id`.
    fn listing(&self, network_id: u64, id: u64) -> Option<strims_directory::view::Listing> {
        self.directory
            .listings(network_id)
            .into_iter()
            .find(|l| l.id == id)
    }

    /// The alias of the user with `peer_key` in listing `id`.
    fn alias_in(&self, network_id: u64, id: u64, peer_key: &[u8]) -> Option<String> {
        self.directory
            .listing_users(network_id, id)
            .into_iter()
            .find(|u| u.peer_key == peer_key)
            .map(|u| u.alias)
    }

    /// The peer keys of the users in listing `id`.
    fn users(&self, network_id: u64, id: u64) -> Vec<Vec<u8>> {
        let mut keys: Vec<Vec<u8>> = self
            .directory
            .listing_users(network_id, id)
            .into_iter()
            .map(|u| u.peer_key)
            .collect();
        keys.sort();
        keys
    }

    /// Creates a media swarm whose transfer under the empty salt is
    /// published on network `network_id`, serving `snippet`: a media
    /// listing of the swarm.
    async fn media(&self, network_id: u64, key_seed: u8, snippet: &ListingSnippet) -> Listing {
        let (swarm, id, _writer) = self
            .node
            .create_swarm(WriterOptions {
                swarm_options: SwarmOptions {
                    live_window: 1 << 10,
                    delivery_mode: DeliveryMode::MANDATORY,
                    ..Default::default()
                },
                key: key_from_seed(&[key_seed; 32]),
            })
            .await
            .unwrap();
        let t = self.transfers.add(swarm, &id, Vec::new());
        assert!(self.transfers.publish(t, self.net(network_id)));
        self.directory.push_snippet(&id, snippet.clone()).unwrap();
        media_listing(&Uri::new(id, UriOptions::default()).to_string())
    }
}

/// Polls `f` every 100 ms until it has a value.
pub(crate) async fn until<T>(what: &str, mut f: impl FnMut() -> Option<T>) -> T {
    let deadline = Instant::now() + DIR_STEP;
    loop {
        if let Some(v) = f() {
            return v;
        }
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        tokio::time::sleep(StdDuration::from_millis(100)).await;
    }
}

pub(crate) fn go_options(seed: u8) -> GoOptions {
    let mut o = GoOptions::new(seed);
    o.peer_rpc = true;
    o.network_control = true;
    o.directory = true;
    o.max_peers = 16;
    o
}

/// A message as a helper argument.
pub(crate) fn arg(m: &impl Message) -> String {
    let b = m.encode_to_vec();
    if b.is_empty() {
        "-".to_owned()
    } else {
        hex::encode(b)
    }
}

/// `<id> <key hex>` from a net-create or net-join reply.
pub(crate) fn id_key(reply: &str) -> (u64, String) {
    let mut f = reply.split(' ');
    let id = f.next().unwrap().parse().unwrap();
    (id, f.next().unwrap().to_owned())
}

fn chat(name: &str, key_seed: u8) -> Listing {
    Listing {
        content: Some(Content::Chat(Chat {
            key: key_from_seed(&[key_seed; 32]).public,
            name: name.into(),
        })),
    }
}

fn media_listing(uri: &str) -> Listing {
    Listing {
        content: Some(Content::Media(Media {
            mime_type: "video/mp4".into(),
            swarm_uri: uri.into(),
        })),
    }
}

/// A directory config with the swarm integration and `publish_quota`.
fn directory_config(publish_quota: u32) -> DirectoryConfig {
    DirectoryConfig {
        integrations: Some(Integrations {
            swarm: Some(Swarm { enable: true }),
            ..Default::default()
        }),
        publish_quota,
        ..Default::default()
    }
}

/// The publish quota of the asset bundle of the member's one network.
fn publish_quota(r: &RustMember) -> Option<u32> {
    let n = r.directory.networks();
    Some(
        n.first()?
            .asset_bundle
            .as_ref()?
            .directory
            .as_ref()?
            .publish_quota,
    )
}

fn by_id(id: u64) -> ListingQuery {
    ListingQuery {
        query: Some(Query::Id(id)),
    }
}

fn ban() -> Option<ListingModeration> {
    Some(ListingModeration {
        is_banned: Some(true),
        ..Default::default()
    })
}

fn moderator() -> Option<UserModeration> {
    Some(UserModeration {
        is_moderator: Some(true),
        ..Default::default()
    })
}

/// A PNG of `len` bytes of `fill`.
pub(crate) fn image(len: usize, fill: u8) -> Image {
    Image {
        r#type: ImageType::Png.into(),
        height: 16,
        width: 16,
        data: vec![fill; len],
    }
}

/// A snippet with both images, so that its first delta is split and its
/// events span several frames.
fn snippet(title: &str) -> ListingSnippet {
    let snippet_image = |len, fill| ListingSnippetImage {
        source_oneof: Some(SourceOneof::Image(image(len, fill))),
    };
    ListingSnippet {
        title: title.into(),
        description: "interop".into(),
        live: true,
        channel_name: "chan".into(),
        tags: vec!["a".into(), "b".into()],
        thumbnail: Some(snippet_image(12 * 1024, 0x11)),
        channel_logo: Some(snippet_image(3 * 1024, 0x22)),
        ..Default::default()
    }
}

/// The listings of Go's view of network `id`.
pub(crate) async fn go_listings(go: &mut GoNode, id: u64) -> Vec<NetworkListingsItem> {
    let reply = go.cmd(&format!("dir-listings {id}")).await.unwrap();
    if reply == "-" {
        return Vec::new();
    }
    NetworkListings::decode(hex::decode(reply).unwrap().as_slice())
        .unwrap()
        .listings
}

/// The peer keys of the users of Go's view of network `id`.
async fn go_users(go: &mut GoNode, id: u64) -> Vec<Vec<u8>> {
    let reply = go.cmd(&format!("dir-users {id}")).await.unwrap();
    let mut keys: Vec<Vec<u8>> = reply
        .split(',')
        .filter(|u| *u != "-")
        .map(|u| hex::decode(u.rsplit(':').next().unwrap()).unwrap())
        .collect();
    keys.sort();
    keys
}

/// Polls Go's view of network `id` every 200 ms until `f` holds.
pub(crate) async fn go_until(
    go: &mut GoNode,
    id: u64,
    what: &str,
    mut f: impl FnMut(&[NetworkListingsItem]) -> bool,
) {
    let deadline = Instant::now() + DIR_STEP;
    loop {
        let ls = go_listings(go, id).await;
        if f(&ls) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for go's view: {what}: {ls:?}"
        );
        tokio::time::sleep(StdDuration::from_millis(200)).await;
    }
}

/// Waits for a broadcast Go read on network `key` with an event matching
/// `f`.
async fn go_event(go: &mut GoNode, key: &[u8], what: &str, mut f: impl FnMut(&Body) -> bool) {
    let deadline = Instant::now() + DIR_STEP;
    let matches = |b: &EventBroadcast, f: &mut dyn FnMut(&Body) -> bool| {
        b.events.iter().filter_map(|e| e.body.as_ref()).any(f)
    };
    go.wait_until(
        what,
        deadline,
        |e| matches!(e, GoEvent::DirEvent(k, b) if k == key && matches(b, &mut f)),
    )
    .await;
}

/// Calls `f` every second until it works, while the member port opens and
/// the server's address resolves.
pub(crate) async fn retry<T>(what: &str, mut f: impl AsyncFnMut() -> Result<T, String>) -> T {
    let deadline = Instant::now() + DIR_STEP;
    loop {
        match f().await {
            Ok(v) => return v,
            Err(e) => {
                assert!(Instant::now() < deadline, "{what} never worked: {e}");
                tokio::time::sleep(StdDuration::from_secs(1)).await;
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn go_owner_rust_member() {
    let Some(bin) = helper() else {
        return;
    };
    crate::bounded(
        420,
        LocalSet::new().run_until(async {
            let mut go = GoNode::spawn(&bin, "gd0", go_options(0xe0)).await;
            let (go_net, key_hex) = id_key(
                &go.cmd(&format!("net-create {}", hex::encode("dir")))
                    .await
                    .unwrap(),
            );
            let key = hex::decode(&key_hex).unwrap();
            let early = chat("go's early chat", 0xe4);
            let early_id: u64 = retry("go's early publish", async || {
                go.cmd(&format!("dir-publish {key_hex} {}", arg(&early)))
                    .await
            })
            .await
            .parse()
            .unwrap();
            go_until(&mut go, go_net, "go's early chat", |ls| {
                ls.iter().any(|l| l.id == early_id)
            })
            .await;
            let invitation = go.cmd(&format!("net-invite {go_net}")).await.unwrap();

            let r = rust_member(0xe1).await;
            let joined = r.join(hex::decode(&invitation).unwrap()).await;
            let net_id = joined.id;
            assert_eq!(r.node.dial(&go.tcp_uri).await.unwrap(), go.host_id);
            until("rust to bind go and renew", || {
                (r.is_bound(go.host_id) && r.trusted(net_id)).then_some(())
            })
            .await;
            let bundle = until("go's asset bundle in rust", || {
                r.directory.networks().first()?.asset_bundle.clone()
            })
            .await;
            eprintln!("go's asset bundle: {bundle:?}");
            // the reader starts at the event swarm's window tail
            until("go's early chat in rust's view", || {
                let l = r.listing(net_id, early_id)?;
                (l.listing == early).then_some(())
            })
            .await;
            let client = bundle.directory.unwrap_or_default();
            assert!(client.integrations.is_some_and(|i| i.swarm));
            assert!(!r.directory.networks()[0].serving);

            // calls from the member, and its view of the event swarm
            let go_chat = chat("go's chat", 0xe5);
            let go_id: u64 = retry("go's publish", async || {
                go.cmd(&format!("dir-publish {key_hex} {}", arg(&go_chat)))
                    .await
            })
            .await
            .parse()
            .unwrap();
            let rust_chat = chat("rust's chat", 0xe6);
            let rust_id = retry("rust's publish", async || {
                r.directory
                    .publish(&key, rust_chat.clone())
                    .await
                    .map_err(|e| e.to_string())
            })
            .await;
            until("rust's view of both chats", || {
                let g = r.listing(net_id, go_id)?;
                let m = r.listing(net_id, rust_id)?;
                (g.listing == go_chat
                    && m.listing == rust_chat
                    && r.users(net_id, go_id) == [go.profile_key.clone()]
                    && r.users(net_id, rust_id) == [r.peer_key()])
                .then_some(())
            })
            .await;
            // Go's server sends a listing's user count when the listing
            // changes after its first broadcast: here the join.
            assert_eq!(r.directory.join(&key, by_id(go_id)).await.unwrap(), go_id);
            let mut both = vec![go.profile_key.clone(), r.peer_key()];
            both.sort();
            until("rust's join in its view", || {
                let g = r.listing(net_id, go_id)?;
                (g.user_count == 2 && r.users(net_id, go_id) == both).then_some(())
            })
            .await;
            let aliases: Vec<String> = r
                .directory
                .listing_users(net_id, go_id)
                .into_iter()
                .map(|u| u.alias)
                .collect();
            assert!(aliases.iter().any(|a| a == r.alias()), "{aliases:?}");
            assert!(aliases.iter().any(|a| a == "go-gd0"), "{aliases:?}");
            go_until(&mut go, go_net, "rust's join", |ls| {
                let count = |id| ls.iter().find(|l| l.id == id).map(|l| l.user_count);
                count(go_id) == Some(2) && count(rust_id).is_some()
            })
            .await;
            assert_eq!(go_users(&mut go, go_net).await, both);

            r.directory.part(&key, go_id).await.unwrap();
            until("rust's part in its view", || {
                let g = r.listing(net_id, go_id)?;
                (g.user_count == 1 && r.users(net_id, go_id) == [go.profile_key.clone()])
                    .then_some(())
            })
            .await;
            go_until(&mut go, go_net, "rust's part", |ls| {
                ls.iter().any(|l| l.id == go_id && l.user_count == 1)
            })
            .await;

            // moderation: refused, until the owner makes rust a moderator
            assert_eq!(
                r.directory.moderate_listing(&key, go_id, ban()).await,
                Err(CallFailure::Refused(DirectoryError::PermissionDenied))
            );
            assert_eq!(
                r.directory
                    .moderate_user(&key, &go.profile_key, moderator())
                    .await,
                Err(CallFailure::Refused(DirectoryError::PermissionDenied))
            );
            go.cmd(&format!(
                "dir-moderate-user {key_hex} {} {}",
                hex::encode(r.peer_key()),
                arg(&moderator().unwrap())
            ))
            .await
            .unwrap();
            r.directory
                .moderate_listing(&key, go_id, ban())
                .await
                .unwrap();
            until("the ban in rust's view", || {
                r.listing(net_id, go_id).is_none().then_some(())
            })
            .await;
            go_until(&mut go, go_net, "the ban", |ls| {
                !ls.iter().any(|l| l.id == go_id)
            })
            .await;
            let refused = go
                .cmd(&format!("dir-publish {key_hex} {}", arg(&go_chat)))
                .await;
            assert_eq!(refused, Err("listing banned".to_owned()));

            // rust's snippet, from its snippet port to go's server
            let mut s = snippet("rust live");
            s.sign_message(&key_from_seed(&[0xe7; 32])).unwrap();
            let media = r.media(net_id, 0xe8, &s).await;
            let media_id = r.directory.publish(&key, media).await.unwrap();
            go_event(&mut go, &key, "rust's snippet in go's events", |b| {
                matches!(b, Body::ListingChange(c)
                    if c.id == media_id && c.snippet.as_ref() == Some(&s))
            })
            .await;
            until("rust's snippet in rust's view", || {
                let l = r.listing(net_id, media_id)?;
                (l.snippet.as_ref() == Some(&s)).then_some(())
            })
            .await;
            let m = r.directory.metrics(&key).unwrap();
            assert_eq!(m.snippets_served, 1, "{m:?}");

            // go's server rewrites its asset swarm from the start
            go.cmd(&format!(
                "dir-config {go_net} {}",
                arg(&directory_config(5))
            ))
            .await
            .unwrap();
            until("go's new asset bundle in rust's view", || {
                (publish_quota(&r) == Some(5)).then_some(())
            })
            .await;

            // rust renews at go's CA, then pings go's server as renamed
            r.frontend
                .update_alias(
                    &Context::default(),
                    UpdateAliasRequest {
                        id: net_id,
                        alias: "renamed".into(),
                    },
                )
                .await
                .unwrap();
            until("rust's new alias in rust's view", || {
                let alias = r.alias_in(net_id, rust_id, &r.peer_key())?;
                (alias == "renamed").then_some(())
            })
            .await;

            r.directory.unpublish(&key, rust_id).await.unwrap();
            r.directory.unpublish(&key, media_id).await.unwrap();
            let ids = |ls: Vec<u64>| ls == [early_id];
            until("rust's unpublish in its view", || {
                ids(r.directory.listings(net_id).iter().map(|l| l.id).collect()).then_some(())
            })
            .await;
            go_until(&mut go, go_net, "rust's unpublish", |ls| {
                ids(ls.iter().map(|l| l.id).collect())
            })
            .await;
            let m = r.directory.metrics(&key).unwrap();
            eprintln!("rust's directory: {m:?}");
            assert_eq!(
                (m.discontinuities, m.view.malformed_records),
                (0, 0),
                "{m:?}"
            );
        }),
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rust_owner_go_member() {
    let Some(bin) = helper() else {
        return;
    };
    crate::bounded(
        420,
        LocalSet::new().run_until(async {
            let r = rust_member(0xf1).await;
            let icon = image(20 * 1024, 0x33);
            let owned = r.create("dir", Some(icon.clone())).await;
            let net_id = owned.id;
            let key = strims_dao::schema::network::network_key(&owned);
            let key_hex = hex::encode(&key);
            until("rust to serve", || {
                let n = r.directory.networks();
                (n.first()?.serving && n[0].asset_bundle.is_some()).then_some(())
            })
            .await;
            let early = chat("rust's early chat", 0xf4);
            let early_id = r.directory.publish(&key, early.clone()).await.unwrap();
            until("rust's early chat", || {
                r.listing(net_id, early_id).is_some().then_some(())
            })
            .await;
            let invitation = r
                .frontend
                .create_invitation(
                    &Context::default(),
                    CreateInvitationRequest {
                        network_id: net_id,
                        bootstrap_client_id: 0,
                    },
                )
                .await
                .unwrap()
                .invitation
                .unwrap();

            let mut go = GoNode::spawn(&bin, "gd1", go_options(0xf0)).await;
            let (go_net, joined_key) = id_key(
                &go.cmd(&format!(
                    "net-join {}",
                    hex::encode(invitation.encode_to_vec())
                ))
                .await
                .unwrap(),
            );
            assert_eq!(joined_key, key_hex);
            assert_eq!(r.node.dial(&go.tcp_uri).await.unwrap(), go.host_id);
            until("rust to bind go", || r.is_bound(go.host_id).then_some(())).await;
            // go's reader starts at the event swarm's window tail
            go_until(&mut go, go_net, "rust's early chat", |ls| {
                ls.iter()
                    .any(|l| l.id == early_id && l.listing.as_ref() == Some(&early))
            })
            .await;

            // go's calls, and its reader of rust's event swarm
            let go_chat = chat("go's chat", 0xf5);
            let go_id: u64 = retry("go's publish", async || {
                go.cmd(&format!("dir-publish {key_hex} {}", arg(&go_chat)))
                    .await
            })
            .await
            .parse()
            .unwrap();
            go_event(&mut go, &key, "go's chat in go's events", |b| {
                matches!(b, Body::ListingChange(c)
                    if c.id == go_id && c.listing.as_ref() == Some(&go_chat))
            })
            .await;
            assert_eq!(r.directory.join(&key, by_id(go_id)).await.unwrap(), go_id);
            let mut both = vec![go.profile_key.clone(), r.peer_key()];
            both.sort();
            until("rust's view of go's chat", || {
                let l = r.listing(net_id, go_id)?;
                (l.user_count == 2 && r.users(net_id, go_id) == both).then_some(())
            })
            .await;
            go_until(&mut go, go_net, "go's chat with both users", |ls| {
                ls.iter().any(|l| {
                    l.id == go_id && l.user_count == 2 && l.listing == Some(go_chat.clone())
                })
            })
            .await;
            assert_eq!(go_users(&mut go, go_net).await, both);

            // go's snippet, from its snippet port to rust's server
            let uri = go
                .cmd(&format!(
                    "dir-snippet {key_hex} {}",
                    arg(&snippet("go live"))
                ))
                .await
                .unwrap();
            let media = media_listing(&uri);
            let media_id: u64 = go
                .cmd(&format!("dir-publish {key_hex} {}", arg(&media)))
                .await
                .unwrap()
                .parse()
                .unwrap();
            let shown = until("go's snippet in rust's view", || {
                let s = r.listing(net_id, media_id)?.snippet?;
                (s.title == "go live").then_some(s)
            })
            .await;
            shown.verify_message().expect("go's signature verifies");
            assert_eq!(shown.tags, ["a", "b"]);
            go_event(&mut go, &key, "go's snippet in go's events", |b| {
                matches!(b, Body::ListingChange(c)
                    if c.id == media_id && c.snippet.as_ref() == Some(&shown))
            })
            .await;
            go.cmd(&format!(
                "dir-snippet-update {uri} {}",
                arg(&snippet("go still live"))
            ))
            .await
            .unwrap();
            until("go's snippet update in rust's view", || {
                let s = r.listing(net_id, media_id)?.snippet?;
                (s.title == "go still live" && s.verify_message().is_ok()).then_some(())
            })
            .await;
            let server = r.directory.metrics(&key).unwrap().server.unwrap();
            eprintln!("rust's server: {server:?}");
            assert_eq!(server.subscriptions, 1, "{server:?}");
            assert_eq!(server.snippets_rejected, 0, "{server:?}");

            // go's moderation: refused, until rust, the owner, makes it a
            // moderator
            let ban_early = format!(
                "dir-moderate-listing {key_hex} {early_id} {}",
                arg(&ban().unwrap())
            );
            assert_eq!(
                go.cmd(&ban_early).await,
                Err("permission denied".to_owned())
            );
            r.directory
                .moderate_user(&key, &go.profile_key, moderator())
                .await
                .unwrap();
            go.cmd(&ban_early).await.unwrap();
            until("go's ban in rust's view", || {
                r.listing(net_id, early_id).is_none().then_some(())
            })
            .await;
            go_until(&mut go, go_net, "go's ban", |ls| {
                !ls.iter().any(|l| l.id == early_id)
            })
            .await;
            assert_eq!(
                r.directory.publish(&key, early.clone()).await,
                Err(CallFailure::Refused(DirectoryError::ListingBanned))
            );

            // rust's asset bundle
            let deadline = Instant::now() + DIR_STEP;
            let GoEvent::DirAssets(_, bundle) = go
                .wait_until(
                    "rust's asset bundle in go",
                    deadline,
                    |e| matches!(e, GoEvent::DirAssets(k, _) if *k == key),
                )
                .await
            else {
                unreachable!()
            };
            assert_eq!(
                Some(&bundle),
                r.directory.networks()[0].asset_bundle.as_ref()
            );
            assert_eq!(bundle.icon.as_ref(), Some(&icon));
            let client = bundle.directory.unwrap();
            assert!(client.integrations.is_some_and(|i| i.swarm), "{client:?}");

            // rust's server rewrites its asset swarm from the start
            let mut config = r.control.networks()[0]
                .network
                .server_config
                .clone()
                .unwrap();
            config.directory = Some(directory_config(5));
            r.frontend
                .update_server_config(
                    &Context::default(),
                    UpdateServerConfigRequest {
                        network_id: net_id,
                        server_config: Some(config),
                    },
                )
                .await
                .unwrap();
            let deadline = Instant::now() + DIR_STEP;
            go.wait_until("rust's new asset bundle in go", deadline, |e| {
                matches!(e, GoEvent::DirAssets(k, b)
                    if *k == key && b.directory.as_ref().is_some_and(|c| c.publish_quota == 5)
                        && b.icon.as_ref() == Some(&icon))
            })
            .await;

            // go renews at rust's CA, then pings rust's server as renamed
            go.cmd(&format!("net-alias {go_net} {}", hex::encode("renamed")))
                .await
                .unwrap();
            until("go's new alias in rust's view", || {
                let alias = r.alias_in(net_id, go_id, &go.profile_key)?;
                (alias == "renamed").then_some(())
            })
            .await;

            go.cmd(&format!("dir-unpublish {key_hex} {go_id}"))
                .await
                .unwrap();
            until("go's unpublish in rust's view", || {
                r.listing(net_id, go_id).is_none().then_some(())
            })
            .await;
            go_event(
                &mut go,
                &key,
                "go's unpublish in go's events",
                |b| matches!(b, Body::Unpublish(u) if u.id == go_id),
            )
            .await;
        }),
    )
    .await;
}

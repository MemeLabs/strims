// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The network directory on whole nodes, under turmoil. Every node runs a
//! profile store, a network control, a transfer control and a directory
//! control, and serves its peers' RPC links with the network and transfer
//! controls. Nobody binds peers or swarm channels by hand.
//!
//! - o creates a network, shortens its directory's sessions to two
//!   minutes with pings every minute, serves the directory and invites m.
//!   m joins from the invitation, links to o and renews.
//! - m publishes a chat listing and joins it, which leaves m its publisher
//!   only; o joins it too. o's view (the server's own) and m's (read from
//!   the event swarm) converge on the listing, its two users and their
//!   presence in it.
//! - m creates a media swarm, publishes its transfer under the empty salt,
//!   pushes a signed snippet for it and publishes a media listing of it.
//!   o's server subscribes to the snippet on m's snippet port, and both
//!   views show it.
//! - o parts the chat, and m unpublishes it: both views see it go. m's
//!   part of the chat it publishes is refused, as it never viewed it.
//! - o changes its server config, which rewrites the asset swarm from its
//!   start, and m reads the new asset bundle.
//! - m renames itself on the network, renewing at o's CA over vpn; its
//!   new certificate reaches the server, and both views show the alias.
//! - m's pings keep its session and its media listing past the session
//!   timeout. Then m leaves the network, and its directory stops with it,
//!   removing the directory's swarms: its pings stop, the session
//!   expires, and the listing goes from o's view.
//! - o's record loses its server config, and o reads the directory; it
//!   gains it again, and o serves again, from a new server.
//!
//! Every host appends what it sees, stamped with simulated time, to one
//! trace. The same seed must give the same trace, and another seed a
//! different one. `STRIMS_TURMOIL_SEED=<s>` runs seed `s` in place of 7.

#![cfg(feature = "turmoil")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::net::IpAddr;
use std::rc::Rc;
use std::time::Duration as StdDuration;

use futures_util::StreamExt;
use prost::Message;
use sha2::{Digest, Sha256};
use strims_base::rng::det_rng;
use strims_base::time::{Duration, Time};
use strims_crypto::{Signer, key_from_seed, membership_trusted};
use strims_dao::schema::network::Networks;
use strims_dao::schema::profile::Profile as ProfileRecord;
use strims_dao::seal::StorageKey;
use strims_dao::{ProfileStore, Recv, ReplicatedStore, Singleton, Table};
use strims_directory::{
    CallFailure, DirectoryControl, DirectoryError, DirectoryEvent, ListingEvent, SignedSnippet,
};
use strims_kv::MemoryKv;
use strims_network::{NetworkControl, NetworkFrontend};
use strims_node::app::{NodeHost, NodeTimer, PeerServices};
use strims_node::{
    Clock, DeliveryMode, HostId, Node, NodeConfig, NodeEvent, SimClock, SwarmOptions, TurmoilNet,
    WriterOptions,
};
use strims_ppspp::{Uri, UriOptions};
use strims_proto::network::v1::Network as NetworkRecord;
use strims_proto::network::v1::create_network_from_invitation_request::Invitation as InvitationField;
use strims_proto::network::v1::directory::listing::{Chat, Content, Media};
use strims_proto::network::v1::directory::listing_query::Query;
use strims_proto::network::v1::directory::{Listing, ListingQuery, ListingSnippet};
use strims_proto::network::v1::{
    CreateInvitationRequest, CreateNetworkFromInvitationRequest, CreateServerRequest,
    DeleteNetworkRequest, Invitation, NetworkFrontendService, UpdateAliasRequest,
    UpdateServerConfigRequest,
};
use strims_proto::profile::v1::Profile;
use strims_proto::service::Context;
use strims_transfer::TransferControl;
use tokio::task::spawn_local;

const TCP_PORT: u16 = 7000;
/// The directory's session timeout and ping interval, in seconds.
const SESSION_TIMEOUT: u32 = 120;
const PING_INTERVAL: u32 = 60;
/// The publish quota o's server config changes to.
const PUBLISH_QUOTA: u32 = 5;

fn epoch() -> Time {
    Time::from_unix_millis(1_700_000_000_000)
}

type Trace = Rc<RefCell<Vec<String>>>;
type Timer = NodeTimer<SimClock>;
type Network = NetworkControl<MemoryKv, NodeHost, Timer>;
type Transfers = TransferControl<NodeHost, Timer>;
type Directory = DirectoryControl<MemoryKv, NodeHost, Timer>;

#[derive(Clone)]
struct World {
    seed: u64,
    trace: Trace,
    ids: Rc<RefCell<BTreeMap<String, HostId>>>,
    invitation: Rc<RefCell<Option<Invitation>>>,
    /// Listing ids by name.
    listings: Rc<RefCell<BTreeMap<String, u64>>>,
    /// Steps done.
    done: Rc<RefCell<BTreeMap<String, bool>>>,
}

impl World {
    fn new(seed: u64) -> World {
        World {
            seed,
            trace: Rc::default(),
            ids: Rc::default(),
            invitation: Rc::default(),
            listings: Rc::default(),
            done: Rc::default(),
        }
    }

    fn log(&self, clock: &SimClock, name: &str, line: impl Into<String>) {
        let t = (clock.now() - epoch()).as_millis();
        self.trace
            .borrow_mut()
            .push(format!("{t} {name} {}", line.into()));
    }

    fn profile(&self, name: &str) -> Profile {
        Profile {
            id: u64::from(name.as_bytes()[0]),
            name: name.to_owned(),
            key: Some(self.key(name)),
            device_id: 1,
            ..Default::default()
        }
    }

    fn key(&self, name: &str) -> strims_proto::types::Key {
        let mut s = [name.as_bytes()[0]; 32];
        s[..8].copy_from_slice(&self.seed.to_le_bytes());
        key_from_seed(&s)
    }

    fn uri(&self, name: &str) -> String {
        let ip = turmoil::lookup(name);
        format!(
            "tcp://{ip}:{TCP_PORT}/{}",
            hex::encode(&self.key(name).public)
        )
    }

    fn name(&self, id: &HostId) -> String {
        self.ids
            .borrow()
            .iter()
            .find(|(_, i)| *i == id)
            .map(|(n, _)| n.clone())
            .unwrap_or_default()
    }

    fn listing(&self, name: &str) -> Option<u64> {
        self.listings.borrow().get(name).copied()
    }

    fn set(&self, step: &str) {
        self.done.borrow_mut().insert(step.to_owned(), true);
    }

    fn is(&self, step: &str) -> bool {
        self.done.borrow().get(step).copied().unwrap_or(false)
    }

    /// Polls `f` every 100 ms of simulated time until it has a value.
    async fn wait<T>(&self, what: &str, secs: u64, mut f: impl FnMut() -> Option<T>) -> T {
        let wait = async {
            loop {
                if let Some(v) = f() {
                    return v;
                }
                tokio::time::sleep(StdDuration::from_millis(100)).await;
            }
        };
        tokio::time::timeout(StdDuration::from_secs(secs), wait)
            .await
            .unwrap_or_else(|_| panic!("timed out waiting for {what}"))
    }
}

/// The publish quota of the node's asset bundle.
fn publish_quota(h: &Started) -> Option<u32> {
    let n = h.directory.networks();
    Some(
        n.first()?
            .asset_bundle
            .as_ref()?
            .directory
            .as_ref()?
            .publish_quota,
    )
}

/// The snippet m pushes for its media swarm.
fn snippet() -> ListingSnippet {
    ListingSnippet {
        title: "live now".into(),
        live: true,
        channel_name: "m's channel".into(),
        ..Default::default()
    }
}

fn chat(w: &World) -> Listing {
    Listing {
        content: Some(Content::Chat(Chat {
            key: w.key("chat").public,
            name: "m's chat".into(),
        })),
    }
}

fn by_id(id: u64) -> ListingQuery {
    ListingQuery {
        query: Some(Query::Id(id)),
    }
}

/// A directory event, for the trace.
fn describe(e: &DirectoryEvent) -> String {
    match e {
        DirectoryEvent::Started { .. } => "dir started".into(),
        DirectoryEvent::Stopped { .. } => "dir stopped".into(),
        DirectoryEvent::Listing { event, .. } => match event {
            ListingEvent::Changed(l) => format!(
                "dir listing {} changed users={} snippet={}",
                l.id,
                l.user_count,
                l.snippet.as_ref().map_or("-", |s| s.title.as_str())
            ),
            ListingEvent::Unpublished(l) => format!("dir listing {} unpublished", l.id),
            ListingEvent::UserCountChanged {
                id,
                user_count,
                recent_user_count,
            } => format!("dir listing {id} users={user_count} recent={recent_user_count}"),
        },
        DirectoryEvent::User {
            listing_id,
            kind,
            user,
            ..
        } => format!("dir user {kind:?} {listing_id} {}", user.alias),
        DirectoryEvent::AssetBundle { bundle, .. } => {
            let c = bundle.directory.unwrap_or_default();
            format!(
                "dir assets ping={}..{} swarm={}",
                c.min_ping_interval,
                c.max_ping_interval,
                c.integrations.is_some_and(|i| i.swarm)
            )
        }
        DirectoryEvent::Lagged { .. } => "dir lagged".into(),
    }
}

struct Started {
    node: Node,
    clock: SimClock,
    network: Network,
    frontend: NetworkFrontend<MemoryKv, NodeHost, Timer>,
    transfers: Transfers,
    directory: Directory,
    store: ReplicatedStore<MemoryKv>,
    /// Peers bound to the network, by name.
    bound: Rc<RefCell<BTreeMap<String, bool>>>,
}

async fn start(w: &World, name: &str, index: u64) -> Started {
    let ip: IpAddr = turmoil::lookup(name);
    let profile = w.profile(name);
    let mut config = NodeConfig::new(profile.key.clone().unwrap(), w.seed.wrapping_add(index));
    config.label = name.to_owned();
    config.tcp_listen = Some(format!("0.0.0.0:{TCP_PORT}"));
    config.public_ip = Some(ip);
    config.max_peers = 16;
    let clock = SimClock::turmoil(epoch());
    let (node, mut events) = Node::spawn(config, TurmoilNet, clock).await.unwrap();
    w.ids.borrow_mut().insert(name.to_owned(), node.host_id());

    let store = ProfileStore::create(
        MemoryKv::new(),
        profile.id,
        StorageKey::from_bytes([index as u8; 32]),
        det_rng(w.seed ^ index),
        clock.now(),
    )
    .await
    .unwrap();
    let mut tx = store.write(clock.now()).await.unwrap();
    ProfileRecord::set(&mut tx, profile.clone()).await.unwrap();
    tx.commit().await.unwrap();
    let store = ReplicatedStore::new(store).await.unwrap();

    let host = NodeHost::new(node.clone());
    let network = NetworkControl::new(
        host.clone(),
        store.clone(),
        profile,
        NodeTimer(clock),
        det_rng(w.seed.wrapping_mul(31) ^ index),
        strims_network::Config::default(),
    );
    let transfers = TransferControl::new(
        host.clone(),
        NodeTimer(clock),
        strims_transfer::Config {
            search_interval: Duration::from_secs(5),
            ..strims_transfer::Config::default()
        },
    );
    let directory = DirectoryControl::new(
        host,
        store.clone(),
        network.clone(),
        transfers.clone(),
        NodeTimer(clock),
        det_rng(w.seed.wrapping_mul(37) ^ index),
        strims_directory::Config::default(),
    );
    spawn_local({
        let network = network.clone();
        async move { network.run().await }
    });
    spawn_local({
        let transfers = transfers.clone();
        async move { transfers.run().await }
    });
    let mut dir_events = directory.subscribe();
    spawn_local({
        let directory = directory.clone();
        async move { directory.run().await }
    });
    let (w2, me) = (w.clone(), name.to_owned());
    spawn_local(async move {
        while let Some(e) = dir_events.next().await {
            let line = match e {
                Recv::Event(e) => describe(&e),
                Recv::Lagged => "dir events lagged".into(),
            };
            w2.log(&clock, &me, line);
        }
    });

    let services = PeerServices::new(node.clone())
        .with(Rc::new(network.clone()))
        .with(Rc::new(transfers.clone()));
    let bound: Rc<RefCell<BTreeMap<String, bool>>> = Rc::default();
    let (w2, me, b2) = (w.clone(), name.to_owned(), bound.clone());
    spawn_local(async move {
        while let Some(e) = events.recv().await {
            match &e {
                NodeEvent::PeerAdded(p) => {
                    w2.log(&clock, &me, format!("peer+ {}", w2.name(&p.host_id)))
                }
                NodeEvent::NetworkPeerAdded { host_id, .. } => {
                    let peer = w2.name(host_id);
                    w2.log(&clock, &me, format!("net+ {peer}"));
                    b2.borrow_mut().insert(peer, true);
                }
                NodeEvent::NetworkPeerRemoved { host_id, .. } => {
                    let peer = w2.name(host_id);
                    w2.log(&clock, &me, format!("net- {peer}"));
                    b2.borrow_mut().insert(peer, false);
                }
                NodeEvent::TransferError { host_id, error } => {
                    w2.log(
                        &clock,
                        &me,
                        format!("transfer error {} {error}", w2.name(host_id)),
                    );
                }
                _ => {}
            }
            services.handle_event(&e).await;
        }
    });
    let frontend = NetworkFrontend::new(network.clone());
    Started {
        node,
        clock,
        network,
        frontend,
        transfers,
        directory,
        store,
        bound,
    }
}

impl Started {
    /// Changes the profile's record of network `id`.
    async fn change_network(&self, id: u64, f: impl FnOnce(&mut NetworkRecord)) {
        let mut tx = self.store.write(self.clock.now()).await.unwrap();
        let mut n = Networks::get(&mut tx, id).await.unwrap().unwrap();
        f(&mut n);
        Networks::upsert(&mut tx, n).await.unwrap();
        tx.commit().await.unwrap();
    }

    fn serving(&self) -> bool {
        self.directory.networks().first().is_some_and(|n| n.serving)
    }

    fn is_bound(&self, peer: &str) -> bool {
        self.bound.borrow().get(peer).copied().unwrap_or(false)
    }

    /// This side's id and key of its one network.
    fn network(&self) -> (u64, Vec<u8>) {
        let n = self.network.networks()[0].network.clone();
        let key = strims_dao::schema::network::network_key(&n);
        (n.id, key)
    }

    fn trusted(&self) -> bool {
        let now = self.clock.now();
        self.network.networks().first().is_some_and(|n| {
            n.network
                .certificate
                .as_ref()
                .is_some_and(|c| membership_trusted(c, now))
        })
    }

    /// The listing `id` of this side's view.
    fn listing(&self, id: u64) -> Option<strims_directory::view::Listing> {
        let (network_id, _) = self.network();
        self.directory
            .listings(network_id)
            .into_iter()
            .find(|l| l.id == id)
    }

    /// Whether the user with `peer_key` is in listing `id` of this side's
    /// view, with this alias.
    fn present(&self, id: u64, peer_key: &[u8], alias: &str) -> bool {
        let (network_id, _) = self.network();
        self.directory
            .listing_users(network_id, id)
            .iter()
            .any(|u| u.peer_key == peer_key && u.alias == alias)
    }

    /// Whether this side's view shows `snippet` for listing `id`.
    fn shows(&self, id: u64, snippet: &ListingSnippet) -> bool {
        self.listing(id)
            .is_some_and(|l| l.snippet.as_ref() == Some(snippet))
    }
}

async fn owner(w: World) -> turmoil::Result {
    let h = start(&w, "o", 1).await;
    let created = h
        .frontend
        .create_server(
            &Context::default(),
            CreateServerRequest {
                name: "net".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .network
        .unwrap();
    let mut config = created.server_config.clone().unwrap();
    let d = config.directory.get_or_insert_default();
    d.session_timeout = SESSION_TIMEOUT;
    d.min_ping_interval = PING_INTERVAL;
    d.max_ping_interval = PING_INTERVAL;
    h.frontend
        .update_server_config(
            &Context::default(),
            UpdateServerConfigRequest {
                network_id: created.id,
                server_config: Some(config),
            },
        )
        .await
        .unwrap();
    w.wait("o to serve the directory with the new config", 60, || {
        let n = h.directory.networks();
        let n = n.first()?;
        let c = n.asset_bundle.as_ref()?.directory.as_ref()?;
        (n.serving && c.min_ping_interval == PING_INTERVAL).then_some(())
    })
    .await;
    w.log(&h.clock, "o", "serving");
    let invitation = h
        .frontend
        .create_invitation(
            &Context::default(),
            CreateInvitationRequest {
                network_id: created.id,
                bootstrap_client_id: 0,
            },
        )
        .await
        .unwrap()
        .invitation
        .unwrap();
    *w.invitation.borrow_mut() = Some(invitation);

    let (m_key, o_key) = (w.key("m").public, w.key("o").public);
    let chat_id = w.wait("m's chat", 240, || w.listing("chat")).await;
    let (_, key) = h.network();
    assert_eq!(
        h.directory.join(&key, by_id(chat_id)).await.unwrap(),
        chat_id
    );
    w.wait("o's view of the chat", 60, || {
        let l = h.listing(chat_id)?;
        (l.listing == chat(&w)
            && l.user_count == 2
            && h.present(chat_id, &m_key, "m")
            && h.present(chat_id, &o_key, "o"))
        .then_some(())
    })
    .await;
    w.log(&h.clock, "o", "sees the chat with m and o in it");

    let media_id = w
        .wait("m's media listing", 120, || w.listing("media"))
        .await;
    let s = snippet();
    w.wait("o's view of the snippet", 60, || {
        h.shows(media_id, &s).then_some(())
    })
    .await;
    w.log(&h.clock, "o", "shows m's snippet");

    h.directory.part(&key, chat_id).await.unwrap();
    w.set("o parted the chat");
    w.wait("the chat to go", 120, || {
        (w.is("m left the chat") && h.listing(chat_id).is_none()).then_some(())
    })
    .await;
    w.log(&h.clock, "o", "the chat went");

    // a new asset bundle, rewritten from the asset swarm's start
    let (network_id, _) = h.network();
    let mut config = h.network.networks()[0]
        .network
        .server_config
        .clone()
        .unwrap();
    config.directory.get_or_insert_default().publish_quota = PUBLISH_QUOTA;
    h.frontend
        .update_server_config(
            &Context::default(),
            UpdateServerConfigRequest {
                network_id,
                server_config: Some(config),
            },
        )
        .await
        .unwrap();
    w.wait("o's new asset bundle", 60, || {
        (publish_quota(&h) == Some(PUBLISH_QUOTA)).then_some(())
    })
    .await;
    w.log(&h.clock, "o", "changed the publish quota");
    w.set("o changed the quota");

    w.wait("m's new alias", 120, || {
        h.present(media_id, &m_key, "m2").then_some(())
    })
    .await;
    w.log(&h.clock, "o", "sees m as m2");

    w.wait("m to stop", 600, || w.is("m stopped").then_some(()))
        .await;
    assert!(h.listing(media_id).is_some(), "m's pings keep its session");
    w.wait(
        "m's session to expire",
        2 * u64::from(SESSION_TIMEOUT),
        || h.listing(media_id).is_none().then_some(()),
    )
    .await;
    let (_, key) = h.network();
    let m = h.directory.metrics(&key).unwrap();
    let server = m.server.clone().unwrap();
    w.log(
        &h.clock,
        "o",
        format!(
            "m's session expired: sessions created={} expired={} subscriptions={} snippets shown={}",
            server.sessions_created,
            server.sessions_expired,
            server.subscriptions,
            server.snippets_shown
        ),
    );
    assert_eq!(server.sessions_expired, 1, "{m:?}");
    assert!(server.snippets_shown >= 1, "{m:?}");

    // the server config lost, then gained again, switches the mode
    let (network_id, key) = h.network();
    let config = h.network.networks()[0].network.server_config.clone();
    h.change_network(network_id, |n| n.server_config = None)
        .await;
    w.wait("o to read the directory", 60, || {
        (!h.serving()).then_some(())
    })
    .await;
    w.log(&h.clock, "o", "reads");
    h.change_network(network_id, |n| n.server_config = config)
        .await;
    w.wait("o to serve again", 120, || h.serving().then_some(()))
        .await;
    let id = h.directory.publish(&key, chat(&w)).await.unwrap();
    w.wait("o's view of its chat", 60, || {
        h.listing(id).is_some().then_some(())
    })
    .await;
    let m = h.directory.metrics(&key).unwrap();
    w.log(
        &h.clock,
        "o",
        format!(
            "serves again: serves={} host errors={}",
            m.serves, m.host_errors
        ),
    );
    assert_eq!((m.serves, m.host_errors), (2, 0), "{m:?}");
    w.set("o done");
    Ok(())
}

async fn member(w: World) -> turmoil::Result {
    let h = start(&w, "m", 2).await;
    let invitation = w
        .wait("an invitation", 60, || w.invitation.borrow().clone())
        .await;
    h.frontend
        .create_network_from_invitation(
            &Context::default(),
            CreateNetworkFromInvitationRequest {
                alias: String::new(),
                invitation: Some(InvitationField::InvitationBytes(invitation.encode_to_vec())),
            },
        )
        .await
        .unwrap();
    h.node.dial(&w.uri("o")).await.unwrap();
    w.wait("m to bind with o", 120, || {
        (h.is_bound("o") && h.trusted()).then_some(())
    })
    .await;
    w.log(&h.clock, "m", "renewed and bound with o");
    let (_, key) = h.network();
    let (m_key, o_key) = (w.key("m").public, w.key("o").public);

    // calls fail until the member port opens and the address resolves
    let chat_id = loop {
        match h.directory.publish(&key, chat(&w)).await {
            Ok(id) => break id,
            Err(e) => {
                assert!(
                    h.clock.now() - epoch() < Duration::from_secs(300),
                    "publishing never worked: {e}"
                );
                tokio::time::sleep(StdDuration::from_secs(1)).await;
            }
        }
    };
    w.log(&h.clock, "m", format!("published the chat as {chat_id}"));
    // a host joining what it publishes stays a publisher only
    assert_eq!(
        h.directory.join(&key, by_id(chat_id)).await.unwrap(),
        chat_id
    );
    w.listings.borrow_mut().insert("chat".into(), chat_id);
    w.wait("m's view of the chat", 120, || {
        let l = h.listing(chat_id)?;
        (l.listing == chat(&w)
            && l.user_count == 2
            && h.present(chat_id, &m_key, "m")
            && h.present(chat_id, &o_key, "o"))
        .then_some(())
    })
    .await;
    w.log(&h.clock, "m", "sees the chat with m and o in it");

    let net = h.network.networks()[0].net.unwrap();
    let (swarm, swarm_id, _writer) = h
        .node
        .create_swarm(WriterOptions {
            swarm_options: SwarmOptions {
                live_window: 1 << 10,
                delivery_mode: DeliveryMode::MANDATORY,
                ..Default::default()
            },
            key: w.key("media"),
        })
        .await
        .unwrap();
    let t = h.transfers.add(swarm, &swarm_id, Vec::new());
    assert!(h.transfers.publish(t, net));
    let s = snippet();
    let signer = Signer::new(&w.key("media")).unwrap();
    let signed = SignedSnippet::sign(&signer, &s, h.clock.now());
    h.directory.push_snippet(&swarm_id, signed).unwrap();
    let media = Listing {
        content: Some(Content::Media(Media {
            mime_type: "video/mp4".into(),
            swarm_uri: Uri::new(swarm_id.clone(), UriOptions::default()).to_string(),
        })),
    };
    let media_id = h.directory.publish(&key, media).await.unwrap();
    w.log(&h.clock, "m", format!("published the media as {media_id}"));
    w.listings.borrow_mut().insert("media".into(), media_id);
    w.wait("m's view of the snippet", 60, || {
        h.shows(media_id, &s).then_some(())
    })
    .await;
    w.log(&h.clock, "m", "shows its snippet");

    w.wait("o to part the chat", 60, || {
        let l = h.listing(chat_id)?;
        (w.is("o parted the chat") && l.user_count == 1 && !h.present(chat_id, &o_key, "o"))
            .then_some(())
    })
    .await;
    w.log(&h.clock, "m", "sees o part the chat");
    assert_eq!(
        h.directory.part(&key, chat_id).await,
        Err(CallFailure::Refused(DirectoryError::NotViewing))
    );
    h.directory.unpublish(&key, chat_id).await.unwrap();
    w.set("m left the chat");
    w.wait("the chat to go from m's view", 60, || {
        h.listing(chat_id).is_none().then_some(())
    })
    .await;
    w.log(&h.clock, "m", "the chat went");

    w.wait("m's new asset bundle", 60, || {
        (w.is("o changed the quota") && publish_quota(&h) == Some(PUBLISH_QUOTA)).then_some(())
    })
    .await;
    w.log(&h.clock, "m", "took the new asset bundle");

    // a renewed certificate reaches the server, and the views
    let (network_id, _) = h.network();
    h.frontend
        .update_alias(
            &Context::default(),
            UpdateAliasRequest {
                id: network_id,
                alias: "m2".into(),
            },
        )
        .await
        .unwrap();
    w.wait("m's new alias in its view", 120, || {
        h.present(media_id, &m_key, "m2").then_some(())
    })
    .await;
    w.log(&h.clock, "m", "renamed to m2");

    // pings keep the session past its timeout
    let before = h.directory.metrics(&key).unwrap();
    tokio::time::sleep(StdDuration::from_secs(2 * u64::from(SESSION_TIMEOUT))).await;
    let m = h.directory.metrics(&key).unwrap();
    assert!(m.pings >= before.pings + 2, "{m:?}");
    assert_eq!(m.ping_errors, before.ping_errors, "{m:?}");
    assert!(h.listing(media_id).is_some(), "the session lasts");
    w.log(
        &h.clock,
        "m",
        format!("pinged {} times; leaving", m.pings - before.pings),
    );
    let swarms = h.node.metrics().await.unwrap().runner.swarms.len();
    h.frontend
        .delete(&Context::default(), DeleteNetworkRequest { id: network_id })
        .await
        .unwrap();
    let node = h.node.clone();
    let left = async || {
        let n = node.metrics().await.unwrap().runner.swarms.len();
        h.directory.networks().is_empty() && n == swarms - 2
    };
    let wait = async {
        while !left().await {
            tokio::time::sleep(StdDuration::from_millis(100)).await;
        }
    };
    tokio::time::timeout(StdDuration::from_secs(60), wait)
        .await
        .expect("the directory stops with the network");
    w.log(&h.clock, "m", "left the network");
    w.set("m stopped");
    w.wait("the end", 600, || w.is("o done").then_some(()))
        .await;
    Ok(())
}

fn run(seed: u64) -> Vec<String> {
    let w = World::new(seed);
    let mut sim = turmoil::Builder::new()
        .rng_seed(seed)
        .epoch(std::time::UNIX_EPOCH + StdDuration::from_secs(1_700_000_000))
        .simulation_duration(StdDuration::from_secs(1800))
        .min_message_latency(StdDuration::from_millis(1))
        .max_message_latency(StdDuration::from_millis(15))
        .build();
    sim.client("o", owner(w.clone()));
    sim.client("m", member(w.clone()));
    let res = sim.run();
    let trace = w.trace.borrow().clone();
    if let Err(e) = res {
        for l in &trace {
            eprintln!("{l}");
        }
        panic!("seed {seed}: {e}\n{}", replay(seed));
    }
    trace
}

fn digest(trace: &[String]) -> String {
    let mut h = Sha256::new();
    for l in trace {
        h.update(l.as_bytes());
        h.update(b"\n");
    }
    hex::encode(h.finalize())
}

fn replay(seed: u64) -> String {
    format!(
        "replay: STRIMS_TURMOIL_SEED={seed} cargo test -p strims-node --features turmoil \
         --test turmoil_directory members_publish_view_and_expire_on_whole_nodes -- --exact --nocapture"
    )
}

#[test]
fn members_publish_view_and_expire_on_whole_nodes() {
    // a seed to replay, from the replay command
    #[allow(clippy::disallowed_methods)]
    let seed = std::env::var("STRIMS_TURMOIL_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(7);
    let a = run(seed);
    for l in &a {
        eprintln!("{l}");
    }
    eprintln!("seed {seed}: {} lines, {}", a.len(), digest(&a));
    let b = run(seed);
    assert_eq!(
        digest(&a),
        digest(&b),
        "same seed, different traces\n{}",
        replay(seed)
    );
    assert_ne!(
        digest(&a),
        digest(&run(seed + 1)),
        "different seeds, same trace\n{}",
        replay(seed)
    );
}

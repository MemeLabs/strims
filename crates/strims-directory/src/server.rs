// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The directory server's state: its listings, the users and sessions
//! that publish and view them, the snippet subscriptions of media
//! listings, and what each broadcast sends. Sans-IO: the control feeds it
//! the Directory service's calls with the moderation records it read, the
//! deltas of its snippet subscriptions and the time, and carries out its
//! [`ServerAction`]s, draining them after every call.
//!
//! # Model
//!
//! - A session is a host, named by its certificate key; a user is a peer
//!   key, and takes the alias of its latest call. A user exists while it
//!   has sessions.
//! - A session publishes or views a listing, not both: publishing a listing
//!   the session views ends the view, and a join of a listing the session
//!   publishes adds nothing.
//! - A listing exists while it has publishers and isn't banned. Once either
//!   fails, the next broadcast removes it and sends `Unpublish`.
//! - A user's sessions publish at most the publish quota of listings and
//!   view at most the join quota, counted per session. Repeating a publish
//!   or join that already holds succeeds without counting again.
//! - A session ends when it has been idle for the session timeout, when
//!   its user opens one more than [`Limits::max_sessions_per_user`] (the
//!   least recently active ends), or when its host calls as another user.
//!   A call that finds the caller's session keeps it alive, whatever its
//!   outcome; a call that fails creates nothing, though a session of
//!   another user on the caller's host still ends.
//! - A listing's user count is the number of users with a session that
//!   publishes or views it; its recent user count, the number of distinct
//!   users that joined, published or pinged it within [`RECENT_USER_TTL`].
//!
//! # Broadcasts
//!
//! Every listing and user carries the stamp of its latest change, from one
//! counter; `mark` is the counter's value at the last broadcast. A
//! broadcast, every broadcast interval:
//!
//! 1. starts a refresh when one is due and none is running: it queues
//!    every listing, then every user, least recently changed first;
//! 2. sends queued refresh items, up to [`REFRESH_BYTES_PER_TICK`] (at
//!    least one): a listing's `ListingChange` and `UserCountChange`, a
//!    user's presence. Items that step 3 or 4 covers are skipped;
//! 3. for each listing changed since `mark`, newest first: removes it if it
//!    has no publishers or is banned, ending its users' memberships, and
//!    sends `Unpublish`; else sends `ListingChange` and `UserCountChange`
//!    if it was modified since `mark`, else `UserCountChange`;
//! 4. sends the presence of each user changed since `mark`, newest first;
//! 5. ends the sessions idle for the session timeout, and removes the
//!    users left without sessions, sending their offline presence. Changes
//!    the endings make go out with the next broadcast;
//! 6. packs the events, in order, into records of at most
//!    [`MAX_BROADCAST_RECORD`] bytes, or of one larger event.
//!
//! # Snippets
//!
//! A media listing has at most one snippet source: the session that first
//! published it, then, when the source stops publishing, the newest
//! publisher. The server subscribes to the source's host
//! ([`ServerAction::Subscribe`]) and merges each delta into the
//! subscription's next snippet, which starts empty. When the next snippet
//! passes [`snippet::check`] and verifies, the listing shows it and is
//! modified. A subscription ends when the publisher ends it, when no delta
//! verifies within the snippet timeout of subscribing or of a failed
//! verification, after [`MAX_UNVERIFIED`] failed verifications in a row,
//! after more than [`MAX_DELTAS_PER_MINUTE`] deltas in a minute, or when
//! the next snippet is over a bound. It's retried after
//! [`MIN_SNIPPET_BACKOFF`], doubling up to [`MAX_SNIPPET_BACKOFF`], plus up
//! to a quarter of jitter; by then a source that stopped publishing has
//! moved to the newest publisher. A verified delta resets the backoff.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use prost::Message;
use rand_core::Rng;
use slotmap::{SlotMap, new_key_type};
use strims_base::rng::DetRng;
use strims_base::time::{Duration, Time};
use strims_crypto::{SignableMessage, certificate_parent};
use strims_kademlia::ID;
use strims_ppspp::SwarmId;
use strims_proto::network::v1::directory::event::{
    Body, ListingChange, Unpublish, UserCountChange, UserPresenceChange,
};
use strims_proto::network::v1::directory::listing_query::Query;
use strims_proto::network::v1::directory::{
    AssetBundle, ClientConfig, Event, EventBroadcast, Listing, ListingModeration, ListingQuery,
    ListingSnippet, ListingSnippetDelta, ServerConfig, UserModeration, client_config,
};
use strims_proto::types::{Certificate, Image};

use crate::{DirectoryError, Limits, MAX_ALIAS_BYTES, MAX_PEER_KEY_BYTES, listing, snippet};

/// How long a user counts towards a listing's recent users.
pub const RECENT_USER_TTL: Duration = Duration::from_secs(30 * 60);
/// The highest publish or join quota.
pub const MAX_QUOTA: u32 = 64;
/// The most bytes of events a broadcast record packs together.
pub const MAX_BROADCAST_RECORD: usize = 64 * 1024;
/// The longest encoded event: a `ListingChange` of the largest listing,
/// snippet and moderation, with its framing.
pub const MAX_EVENT_BYTES: usize = listing::MAX_LISTING_BYTES + snippet::MAX_SNIPPET_BYTES + 1024;
/// The most refresh bytes one broadcast sends, unless a single item is
/// larger.
pub const REFRESH_BYTES_PER_TICK: usize = 256 * 1024;
/// The most deltas a snippet subscription may send in a minute.
pub const MAX_DELTAS_PER_MINUTE: u32 = 120;
/// Failed verifications in a row that end a snippet subscription.
pub const MAX_UNVERIFIED: u32 = 3;
/// The first wait before a failed snippet subscription is retried.
pub const MIN_SNIPPET_BACKOFF: Duration = Duration::from_secs(30);
/// The longest wait before a failed snippet subscription is retried.
pub const MAX_SNIPPET_BACKOFF: Duration = Duration::from_secs(10 * 60);
/// The longest encoded asset bundle; a bundle with a larger icon goes
/// without it.
pub const MAX_ASSET_BUNDLE: usize = 1024 * 1024;

const RATE_WINDOW: Duration = Duration::MINUTE;

new_key_type! {
    struct ListingKey;
    struct UserKey;
    struct SessionKey;
}

/// A snippet subscription's id, unique for the server's life.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubId(pub u64);

/// The host behind a call, from its vpn certificate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Caller {
    /// The host certificate's key.
    pub host_key: Vec<u8>,
    /// The host's id: its key.
    pub host: ID,
    /// The membership certificate's key: the user.
    pub peer_key: Vec<u8>,
    /// The membership certificate's subject.
    pub alias: String,
}

impl Caller {
    /// The caller presenting host certificate `c`: a 32-byte host key
    /// under a membership certificate. The vpn connection verified the
    /// chain.
    pub fn from_certificate(c: &Certificate) -> Option<Caller> {
        let parent = certificate_parent(c)?;
        let host = ID::try_from(c.key.as_slice()).ok()?;
        Some(Caller {
            host_key: c.key.clone(),
            host,
            peer_key: parent.key.clone(),
            alias: parent.subject.clone(),
        })
    }

    fn check(&self) -> Result<(), DirectoryError> {
        if self.host_key.len() > MAX_PEER_KEY_BYTES
            || self.peer_key.len() > MAX_PEER_KEY_BYTES
            || self.alias.len() > MAX_ALIAS_BYTES
        {
            return Err(DirectoryError::TooLarge);
        }
        Ok(())
    }
}

/// The server's settings from a network's `ServerConfig`. Fields given in
/// seconds; 0, or no config, takes the default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tunables {
    /// Default 1 s; 1 s to 60 s.
    pub broadcast_interval: Duration,
    /// Default 5 min; from the broadcast interval to 1 h.
    pub refresh_interval: Duration,
    /// Default 15 min; 2 min to 24 h.
    pub session_timeout: Duration,
    /// The clients' ping interval range. Defaults 10 and 14 min; both at
    /// least 1 min and at most a minute under the session timeout, the
    /// maximum at least the minimum.
    pub min_ping_interval: Duration,
    pub max_ping_interval: Duration,
    /// How long a snippet subscription may go without a verified delta
    /// after subscribing or a failed verification. Default 30 s; 1 s to
    /// 10 min.
    pub snippet_timeout: Duration,
    /// Default 10; 1 to [`MAX_QUOTA`].
    pub publish_quota: u32,
    pub join_quota: u32,
    /// Whether the swarm integration is enabled, for clients.
    pub swarm: bool,
}

impl Tunables {
    pub fn from_config(c: Option<&ServerConfig>) -> Tunables {
        let field = |f: fn(&ServerConfig) -> u32| c.map_or(0, f);
        let secs =
            |v: u32, default: i64| Duration::from_secs(if v == 0 { default } else { i64::from(v) });
        let quota = |v: u32| if v == 0 { 10 } else { v.min(MAX_QUOTA) };

        let broadcast_interval =
            secs(field(|c| c.broadcast_interval), 1).clamp(Duration::SECOND, Duration::MINUTE);
        let session_timeout = secs(field(|c| c.session_timeout), 15 * 60)
            .clamp(Duration::MINUTE * 2, Duration::HOUR * 24);
        let refresh_interval =
            secs(field(|c| c.refresh_interval), 5 * 60).clamp(broadcast_interval, Duration::HOUR);
        let ping_ceiling = session_timeout - Duration::MINUTE;
        let min_ping_interval =
            secs(field(|c| c.min_ping_interval), 10 * 60).clamp(Duration::MINUTE, ping_ceiling);
        let max_ping_interval =
            secs(field(|c| c.max_ping_interval), 14 * 60).clamp(min_ping_interval, ping_ceiling);
        let snippet_timeout = secs(field(|c| c.load_media_embed_timeout), 30)
            .clamp(Duration::SECOND, Duration::MINUTE * 10);
        let swarm = c
            .and_then(|c| c.integrations.as_ref())
            .and_then(|i| i.swarm.as_ref())
            .is_some_and(|s| s.enable);
        Tunables {
            broadcast_interval,
            refresh_interval,
            session_timeout,
            min_ping_interval,
            max_ping_interval,
            snippet_timeout,
            publish_quota: quota(field(|c| c.publish_quota)),
            join_quota: quota(field(|c| c.join_quota)),
            swarm,
        }
    }
}

impl Default for Tunables {
    fn default() -> Tunables {
        Tunables::from_config(None)
    }
}

/// What the control must do.
#[derive(Clone, Debug, PartialEq)]
pub enum ServerAction {
    /// Write each record to the event swarm, in order.
    Broadcast(Vec<EventBroadcast>),
    /// Subscribe to `host`'s snippet of `swarm_id` on the snippet port, and
    /// feed its deltas to [`Server::snippet_delta`] and its end to
    /// [`Server::snippet_ended`].
    Subscribe {
        sub: SubId,
        listing_id: u64,
        host: ID,
        swarm_id: SwarmId,
    },
    /// End subscription `sub`, without reporting it.
    Unsubscribe { sub: SubId },
}

/// What a [`Server`] has done.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ServerMetrics {
    /// Broadcasts that sent events, their records and events.
    pub broadcasts: u64,
    pub records: u64,
    pub events: u64,
    /// Refreshes started, and items they sent.
    pub refreshes: u64,
    pub refreshed: u64,
    pub listings_created: u64,
    pub listings_removed: u64,
    pub users_created: u64,
    pub users_removed: u64,
    pub sessions_created: u64,
    pub sessions_expired: u64,
    /// Sessions ended for their user's session limit.
    pub sessions_evicted: u64,
    /// Sessions ended because their host called as another user.
    pub sessions_displaced: u64,
    /// Calls that failed.
    pub refused: u64,
    pub subscriptions: u64,
    pub snippet_deltas: u64,
    /// Deltas for subscriptions that had ended.
    pub stale_deltas: u64,
    /// Verified snippets that changed a listing's snippet.
    pub snippets_shown: u64,
    /// Deltas after which the next snippet didn't verify.
    pub snippets_unverified: u64,
    /// Subscriptions ended by the publisher, by timeout, by failed
    /// verifications, by their delta rate and by size.
    pub snippets_ended: u64,
    pub snippet_timeouts: u64,
    pub snippets_rejected: u64,
    pub snippet_floods: u64,
    pub snippets_oversized: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Roles {
    publishing: u32,
    viewing: u32,
}

/// Distinct users by their latest activity on a listing.
#[derive(Debug, Default)]
struct Recent {
    by_key: BTreeMap<Vec<u8>, Time>,
    by_time: BTreeSet<(Time, Vec<u8>)>,
}

impl Recent {
    fn touch(&mut self, key: &[u8], now: Time, max: usize) {
        if let Some(t) = self.by_key.get_mut(key) {
            self.by_time.remove(&(*t, key.to_vec()));
            *t = now;
        } else {
            while self.by_key.len() >= max.max(1) {
                let Some((_, k)) = self.by_time.pop_first() else {
                    break;
                };
                self.by_key.remove(&k);
            }
            self.by_key.insert(key.to_vec(), now);
        }
        self.by_time.insert((now, key.to_vec()));
    }

    /// The users active after `now - ttl`, forgetting the others.
    fn count(&mut self, now: Time, ttl: Duration) -> u32 {
        let cutoff = now - ttl;
        while let Some((t, _)) = self.by_time.first() {
            if *t > cutoff {
                break;
            }
            if let Some((_, k)) = self.by_time.pop_first() {
                self.by_key.remove(&k);
            }
        }
        u32::try_from(self.by_key.len()).unwrap_or(u32::MAX)
    }
}

#[derive(Debug)]
struct Subscription {
    sub: SubId,
    next: ListingSnippet,
    unverified: u32,
    window: Time,
    deltas: u32,
}

#[derive(Debug)]
enum SourceState {
    Subscribed(Box<Subscription>),
    /// Backing off, until the timer.
    Waiting,
}

#[derive(Debug)]
struct Source {
    session: SessionKey,
    host: ID,
    swarm_id: SwarmId,
    backoff: Duration,
    timer: Option<Time>,
    state: SourceState,
}

#[derive(Debug)]
struct ListingEntry {
    id: u64,
    key: Vec<u8>,
    listing: Listing,
    moderation: Option<ListingModeration>,
    snippet: ListingSnippet,
    swarm_id: Option<SwarmId>,
    evicted: bool,
    touched: u64,
    modified: u64,
    /// Publisher sessions, with the stamp each started publishing at.
    publishers: BTreeMap<SessionKey, u64>,
    viewers: BTreeSet<SessionKey>,
    members: BTreeMap<UserKey, Roles>,
    recent: Recent,
    /// The recent user count last broadcast.
    broadcast_recent: u32,
    source: Option<Source>,
}

impl ListingEntry {
    fn release(&mut self, user: UserKey, publishing: bool) {
        if let Some(r) = self.members.get_mut(&user) {
            if publishing {
                r.publishing = r.publishing.saturating_sub(1);
            } else {
                r.viewing = r.viewing.saturating_sub(1);
            }
            if *r == Roles::default() {
                self.members.remove(&user);
            }
        }
    }

    fn change_event(&self) -> Event {
        Event {
            body: Some(Body::ListingChange(ListingChange {
                id: self.id,
                listing: Some(self.listing.clone()),
                snippet: Some(self.snippet.clone()),
                moderation: self.moderation.clone(),
            })),
        }
    }

    fn change_len(&self) -> usize {
        self.listing.encoded_len()
            + self.snippet.encoded_len()
            + self.moderation.as_ref().map_or(0, Message::encoded_len)
            + 64
    }

    fn count_event(&mut self, now: Time, ttl: Duration) -> Event {
        self.broadcast_recent = self.recent.count(now, ttl);
        Event {
            body: Some(Body::UserCountChange(UserCountChange {
                id: self.id,
                user_count: u32::try_from(self.members.len()).unwrap_or(u32::MAX),
                recent_user_count: self.broadcast_recent,
            })),
        }
    }
}

#[derive(Debug)]
struct UserEntry {
    id: u64,
    peer_key: Vec<u8>,
    alias: String,
    sessions: BTreeSet<SessionKey>,
    touched: u64,
}

#[derive(Debug)]
struct SessionEntry {
    host_key: Vec<u8>,
    host: ID,
    user: UserKey,
    published: BTreeSet<ListingKey>,
    viewed: BTreeSet<ListingKey>,
    /// The latest call.
    active: Time,
}

#[derive(Clone, Copy, Debug)]
enum RefreshItem {
    Listing(ListingKey),
    User(UserKey),
}

/// What a delta did to a subscription.
enum DeltaOutcome {
    Shown(bool),
    Unverified,
    Fail(Failure),
}

enum Failure {
    Flood,
    Oversized,
    Rejected,
}

/// A network's directory server; see the module docs.
#[derive(Debug)]
pub struct Server {
    limits: Limits,
    tunables: Tunables,
    rng: DetRng,
    listings: SlotMap<ListingKey, ListingEntry>,
    listing_ids: BTreeMap<u64, ListingKey>,
    listing_keys: BTreeMap<Vec<u8>, ListingKey>,
    /// Listings by the stamp of their latest change.
    listing_order: BTreeMap<u64, ListingKey>,
    users: SlotMap<UserKey, UserEntry>,
    user_peers: BTreeMap<Vec<u8>, UserKey>,
    user_order: BTreeMap<u64, UserKey>,
    sessions: SlotMap<SessionKey, SessionEntry>,
    session_hosts: BTreeMap<Vec<u8>, SessionKey>,
    session_expiry: BTreeSet<(Time, SessionKey)>,
    /// Users a call left without sessions; the next broadcast removes them.
    /// A subset of `users`.
    orphans: BTreeSet<UserKey>,
    /// Live snippet subscriptions; at most one per listing.
    subs: BTreeMap<SubId, ListingKey>,
    /// Snippet source retries and deadlines; at most one per listing.
    source_timers: BTreeSet<(Time, ListingKey)>,
    /// At most one item per listing and user.
    refresh: VecDeque<RefreshItem>,
    last_refresh: Time,
    next_broadcast: Time,
    stamp: u64,
    mark: u64,
    next_listing_id: u64,
    next_user_id: u64,
    next_sub: u64,
    /// The actions of the latest calls; see the module docs.
    actions: VecDeque<ServerAction>,
    metrics: ServerMetrics,
}

impl Server {
    /// A server with no listings, settings from `config`.
    pub fn new(limits: Limits, config: Option<&ServerConfig>, rng: DetRng, now: Time) -> Server {
        let tunables = Tunables::from_config(config);
        Server {
            limits,
            tunables,
            rng,
            listings: SlotMap::with_key(),
            listing_ids: BTreeMap::new(),
            listing_keys: BTreeMap::new(),
            listing_order: BTreeMap::new(),
            users: SlotMap::with_key(),
            user_peers: BTreeMap::new(),
            user_order: BTreeMap::new(),
            sessions: SlotMap::with_key(),
            session_hosts: BTreeMap::new(),
            session_expiry: BTreeSet::new(),
            orphans: BTreeSet::new(),
            subs: BTreeMap::new(),
            source_timers: BTreeSet::new(),
            refresh: VecDeque::new(),
            last_refresh: now,
            next_broadcast: now + tunables.broadcast_interval,
            stamp: 0,
            mark: 0,
            next_listing_id: 1,
            next_user_id: 1,
            next_sub: 1,
            actions: VecDeque::new(),
            metrics: ServerMetrics::default(),
        }
    }

    /// Takes the settings of a changed `ServerConfig`. Memberships beyond
    /// lowered quotas stay.
    pub fn set_config(&mut self, config: Option<&ServerConfig>) {
        self.tunables = Tunables::from_config(config);
    }

    pub fn tunables(&self) -> &Tunables {
        &self.tunables
    }

    /// The client config of the asset bundle: the effective quotas and
    /// ping intervals, the swarm integration, and no embed integrations.
    pub fn client_config(&self) -> ClientConfig {
        let t = &self.tunables;
        let secs = |d: Duration| u32::try_from(d.as_secs()).unwrap_or(u32::MAX);
        ClientConfig {
            integrations: Some(client_config::Integrations {
                angelthump: false,
                twitch: false,
                youtube: false,
                swarm: t.swarm,
            }),
            publish_quota: t.publish_quota,
            join_quota: t.join_quota,
            min_ping_interval: secs(t.min_ping_interval),
            max_ping_interval: secs(t.max_ping_interval),
        }
    }

    /// The asset bundle with `icon` and [`Server::client_config`], without
    /// the icon if the bundle would exceed [`MAX_ASSET_BUNDLE`].
    pub fn asset_bundle(&self, icon: Option<Image>) -> AssetBundle {
        let mut b = AssetBundle {
            icon,
            directory: Some(self.client_config()),
        };
        if b.encoded_len() > MAX_ASSET_BUNDLE {
            b.icon = None;
        }
        b
    }

    /// `Publish`: the caller's host publishes `l`. `user` is the caller's
    /// user record moderation, `record` the listing record's; a new listing
    /// takes `record` as its moderation. Returns the listing's id.
    pub fn publish(
        &mut self,
        now: Time,
        c: &Caller,
        l: Listing,
        user: Option<&UserModeration>,
        record: Option<&ListingModeration>,
    ) -> Result<u64, DirectoryError> {
        let r = self.try_publish(now, c, l, user, record);
        self.count(&r);
        r
    }

    /// `Unpublish`: the caller's host stops publishing listing `id`.
    pub fn unpublish(&mut self, now: Time, c: &Caller, id: u64) -> Result<(), DirectoryError> {
        let r = self.try_leave(now, c, id, true);
        self.count(&r);
        r
    }

    /// `Join`: the caller's host views the listing `q` names. `user` is the
    /// caller's user record moderation. Returns the listing's id.
    pub fn join(
        &mut self,
        now: Time,
        c: &Caller,
        q: &ListingQuery,
        user: Option<&UserModeration>,
    ) -> Result<u64, DirectoryError> {
        let r = self.try_join(now, c, q, user);
        self.count(&r);
        r
    }

    /// `Part`: the caller's host stops viewing listing `id`.
    pub fn part(&mut self, now: Time, c: &Caller, id: u64) -> Result<(), DirectoryError> {
        let r = self.try_leave(now, c, id, false);
        self.count(&r);
        r
    }

    /// `Ping`: keeps the caller's session alive, and counts its user as
    /// recent on the listings the session publishes and views.
    pub fn ping(&mut self, now: Time, c: &Caller) -> Result<(), DirectoryError> {
        let r = self.try_ping(now, c);
        self.count(&r);
        r
    }

    fn count<T>(&mut self, r: &Result<T, DirectoryError>) {
        if r.is_err() {
            self.metrics.refused += 1;
        }
    }

    /// The listing with id `id`.
    pub fn listing(&self, id: u64) -> Option<&Listing> {
        let k = self.listing_ids.get(&id)?;
        self.listings.get(*k).map(|l| &l.listing)
    }

    /// Takes listing `id`'s moderation after the control stored it: a ban
    /// removes the listing at the next broadcast, and any change goes out
    /// with it. A category over [`listing::MAX_CATEGORY_BYTES`] isn't
    /// kept.
    pub fn listing_moderated(&mut self, _now: Time, id: u64, m: Option<ListingModeration>) {
        let Some(&lk) = self.listing_ids.get(&id) else {
            return;
        };
        let Some(l) = self.listings.get_mut(lk) else {
            return;
        };
        l.evicted = m.as_ref().and_then(|m| m.is_banned) == Some(true);
        l.moderation = bounded(m);
        self.modify_listing(lk);
    }

    /// Takes the moderation of the user with `peer_key` after the control
    /// stored it: disabled joins end its sessions' views, disabled
    /// publishing their publishing.
    pub fn user_moderated(&mut self, now: Time, peer_key: &[u8], m: &UserModeration) {
        let Some(&uk) = self.user_peers.get(peer_key) else {
            return;
        };
        let (no_join, no_publish) = (
            m.disable_join == Some(true),
            m.disable_publish == Some(true),
        );
        let sessions: Vec<SessionKey> = self
            .users
            .get(uk)
            .map(|u| u.sessions.iter().copied().collect())
            .unwrap_or_default();
        let mut changed = false;
        for sk in sessions {
            let Some(s) = self.sessions.get(sk) else {
                continue;
            };
            let viewed: Vec<ListingKey> = if no_join {
                s.viewed.iter().copied().collect()
            } else {
                Vec::new()
            };
            let published: Vec<ListingKey> = if no_publish {
                s.published.iter().copied().collect()
            } else {
                Vec::new()
            };
            for lk in viewed {
                changed |= self.remove_viewer(lk, sk);
                self.touch_listing(lk);
            }
            for lk in published {
                changed |= self.remove_publisher(now, lk, sk);
                self.touch_listing(lk);
            }
        }
        if changed {
            self.touch_user(uk);
        }
    }

    /// A delta of snippet subscription `sub`.
    pub fn snippet_delta(&mut self, now: Time, sub: SubId, d: &ListingSnippetDelta) {
        self.metrics.snippet_deltas += 1;
        let Some(&lk) = self.subs.get(&sub) else {
            self.metrics.stale_deltas += 1;
            return;
        };
        let Some(l) = self.listings.get_mut(lk) else {
            return;
        };
        let Some(src) = l.source.as_mut() else {
            return;
        };
        let SourceState::Subscribed(s) = &mut src.state else {
            return;
        };
        if s.sub != sub {
            self.metrics.stale_deltas += 1;
            return;
        }
        if now >= s.window + RATE_WINDOW {
            s.window = now;
            s.deltas = 0;
        }
        s.deltas = s.deltas.saturating_add(1);
        let outcome = if s.deltas > MAX_DELTAS_PER_MINUTE {
            DeltaOutcome::Fail(Failure::Flood)
        } else {
            snippet::merge(&mut s.next, d);
            if snippet::check(&s.next).is_err() {
                DeltaOutcome::Fail(Failure::Oversized)
            } else if s.next.verify_message().is_ok() {
                s.unverified = 0;
                src.backoff = MIN_SNIPPET_BACKOFF;
                let changed = l.snippet != s.next;
                if changed {
                    l.snippet.clone_from(&s.next);
                }
                DeltaOutcome::Shown(changed)
            } else {
                s.unverified = s.unverified.saturating_add(1);
                if s.unverified >= MAX_UNVERIFIED {
                    DeltaOutcome::Fail(Failure::Rejected)
                } else {
                    DeltaOutcome::Unverified
                }
            }
        };
        let timer = src.timer;
        match outcome {
            DeltaOutcome::Shown(changed) => {
                self.set_source_timer(lk, None);
                if changed {
                    self.metrics.snippets_shown += 1;
                    self.modify_listing(lk);
                }
            }
            DeltaOutcome::Unverified => {
                self.metrics.snippets_unverified += 1;
                if timer.is_none() {
                    self.set_source_timer(lk, Some(now + self.tunables.snippet_timeout));
                }
            }
            DeltaOutcome::Fail(f) => {
                match f {
                    Failure::Flood => self.metrics.snippet_floods += 1,
                    Failure::Oversized => self.metrics.snippets_oversized += 1,
                    Failure::Rejected => {
                        self.metrics.snippets_unverified += 1;
                        self.metrics.snippets_rejected += 1;
                    }
                }
                self.fail_source(now, lk, true);
            }
        }
    }

    /// Snippet subscription `sub` ended at the publisher.
    pub fn snippet_ended(&mut self, now: Time, sub: SubId) {
        let Some(&lk) = self.subs.get(&sub) else {
            return;
        };
        self.metrics.snippets_ended += 1;
        self.fail_source(now, lk, false);
    }

    /// When [`Server::handle_timeout`] is next due.
    pub fn poll_timeout(&self) -> Option<Time> {
        let timer = self.source_timers.first().map(|(t, _)| *t);
        Some(timer.map_or(self.next_broadcast, |t| t.min(self.next_broadcast)))
    }

    /// Runs the snippet retries and timeouts due at `now`, and the
    /// broadcast if it's due.
    pub fn handle_timeout(&mut self, now: Time) {
        while let Some(&(t, lk)) = self.source_timers.first() {
            if t > now {
                break;
            }
            self.source_timers.pop_first();
            self.source_timer(now, lk);
        }
        if now >= self.next_broadcast {
            self.broadcast(now);
            self.next_broadcast += self.tunables.broadcast_interval;
            if self.next_broadcast <= now {
                self.next_broadcast = now + self.tunables.broadcast_interval;
            }
        }
    }

    pub fn poll_action(&mut self) -> Option<ServerAction> {
        self.actions.pop_front()
    }

    pub fn metrics(&self) -> &ServerMetrics {
        &self.metrics
    }

    // --- calls ----------------------------------------------------------

    fn try_publish(
        &mut self,
        now: Time,
        c: &Caller,
        l: Listing,
        user: Option<&UserModeration>,
        record: Option<&ListingModeration>,
    ) -> Result<u64, DirectoryError> {
        c.check()?;
        if user.and_then(|m| m.disable_publish) == Some(true) {
            return Err(DirectoryError::PublishDisabled);
        }
        let v = listing::validate(&l)?;
        if record.and_then(|m| m.is_banned) == Some(true) {
            return Err(DirectoryError::ListingBanned);
        }
        let session = self.caller_session(now, c);
        let existing = self.listing_keys.get(&v.key).copied();
        if let (Some(sk), Some(lk)) = (session, existing)
            && self
                .sessions
                .get(sk)
                .is_some_and(|s| s.published.contains(&lk))
        {
            self.keep_alive(now, c, sk);
            self.touch_listing(lk);
            return self.listing_id(lk);
        }
        let uk = self.user_peers.get(&c.peer_key).copied();
        if let Some(uk) = uk
            && self.role_count(uk, true) >= self.tunables.publish_quota as usize
        {
            return Err(DirectoryError::PublishQuota);
        }
        if existing.is_none() && self.listings.len() >= self.limits.max_listings {
            return Err(DirectoryError::Full);
        }
        self.check_capacity(uk, session)?;

        let sk = self.admit(now, c, session);
        let lk = match existing {
            Some(lk) => lk,
            None => self.insert_listing(l, v, record.cloned()),
        };
        self.remove_viewer(lk, sk);
        self.add_publisher(now, lk, sk);
        self.touch_listing(lk);
        if self
            .listings
            .get(lk)
            .is_some_and(|l| l.swarm_id.is_some() && l.source.is_none())
        {
            self.start_source(now, lk, sk);
        }
        self.listing_id(lk)
    }

    fn try_join(
        &mut self,
        now: Time,
        c: &Caller,
        q: &ListingQuery,
        user: Option<&UserModeration>,
    ) -> Result<u64, DirectoryError> {
        c.check()?;
        if user.and_then(|m| m.disable_join) == Some(true) {
            return Err(DirectoryError::JoinDisabled);
        }
        let session = self.caller_session(now, c);
        let found = self.find_listing(q);
        if let (Some(sk), Some(lk)) = (session, found)
            && self
                .sessions
                .get(sk)
                .is_some_and(|s| s.viewed.contains(&lk) || s.published.contains(&lk))
        {
            self.keep_alive(now, c, sk);
            self.touch_listing(lk);
            return self.listing_id(lk);
        }
        let uk = self.user_peers.get(&c.peer_key).copied();
        if let Some(uk) = uk
            && self.role_count(uk, false) >= self.tunables.join_quota as usize
        {
            return Err(DirectoryError::ViewQuota);
        }
        let lk = found.ok_or(DirectoryError::ListingNotFound)?;
        self.check_capacity(uk, session)?;

        let sk = self.admit(now, c, session);
        self.add_viewer(now, lk, sk);
        self.touch_listing(lk);
        self.listing_id(lk)
    }

    /// Unpublish (`publishing`) or Part.
    fn try_leave(
        &mut self,
        now: Time,
        c: &Caller,
        id: u64,
        publishing: bool,
    ) -> Result<(), DirectoryError> {
        c.check()?;
        let lk = *self
            .listing_ids
            .get(&id)
            .ok_or(DirectoryError::ListingNotFound)?;
        let sk = self
            .caller_session(now, c)
            .ok_or(DirectoryError::SessionNotFound)?;
        self.keep_alive(now, c, sk);
        let left = if publishing {
            self.remove_publisher(now, lk, sk)
        } else {
            self.remove_viewer(lk, sk)
        };
        if !left {
            return Err(if publishing {
                DirectoryError::NotPublishing
            } else {
                DirectoryError::NotViewing
            });
        }
        self.touch_listing(lk);
        Ok(())
    }

    fn try_ping(&mut self, now: Time, c: &Caller) -> Result<(), DirectoryError> {
        c.check()?;
        let sk = self
            .caller_session(now, c)
            .ok_or(DirectoryError::SessionNotFound)?;
        self.keep_alive(now, c, sk);
        let Some(s) = self.sessions.get(sk) else {
            return Ok(());
        };
        let listings: Vec<ListingKey> = s.viewed.iter().chain(&s.published).copied().collect();
        let max = self.limits.max_recent_users;
        for lk in listings {
            if let Some(l) = self.listings.get_mut(lk) {
                l.recent.touch(&c.peer_key, now, max);
            }
        }
        Ok(())
    }

    fn listing_id(&self, lk: ListingKey) -> Result<u64, DirectoryError> {
        self.listings
            .get(lk)
            .map(|l| l.id)
            .ok_or(DirectoryError::ListingNotFound)
    }

    fn find_listing(&self, q: &ListingQuery) -> Option<ListingKey> {
        match q.query.as_ref()? {
            Query::Id(id) => self.listing_ids.get(id).copied(),
            Query::Listing(l) => self.listing_keys.get(&listing::key(l)).copied(),
        }
    }

    /// The caller's session. A session of another user on the caller's
    /// host ends first.
    fn caller_session(&mut self, now: Time, c: &Caller) -> Option<SessionKey> {
        let sk = *self.session_hosts.get(&c.host_key)?;
        let user = self.sessions.get(sk).map(|s| s.user)?;
        if self
            .users
            .get(user)
            .is_some_and(|u| u.peer_key == c.peer_key)
        {
            return Some(sk);
        }
        self.metrics.sessions_displaced += 1;
        if let Some((uk, true)) = self.remove_session(now, sk) {
            self.orphans.insert(uk);
        }
        None
    }

    /// Listings that the user's sessions publish or view, counted per
    /// session.
    fn role_count(&self, uk: UserKey, publishing: bool) -> usize {
        let Some(u) = self.users.get(uk) else {
            return 0;
        };
        u.sessions
            .iter()
            .filter_map(|sk| self.sessions.get(*sk))
            .map(|s| {
                if publishing {
                    s.published.len()
                } else {
                    s.viewed.len()
                }
            })
            .sum()
    }

    /// Whether a call can add the caller's user (if `uk` is none) and
    /// session (if `session` is none).
    fn check_capacity(
        &self,
        uk: Option<UserKey>,
        session: Option<SessionKey>,
    ) -> Result<(), DirectoryError> {
        if uk.is_none() && self.users.len() >= self.limits.max_users {
            return Err(DirectoryError::Full);
        }
        if session.is_none() {
            let evicts = uk
                .and_then(|uk| self.users.get(uk))
                .is_some_and(|u| u.sessions.len() >= self.max_sessions_per_user());
            if !evicts && self.sessions.len() >= self.limits.max_sessions {
                return Err(DirectoryError::Full);
            }
        }
        Ok(())
    }

    fn max_sessions_per_user(&self) -> usize {
        self.limits.max_sessions_per_user.max(1)
    }

    /// The caller's user and session, made or refreshed.
    fn admit(&mut self, now: Time, c: &Caller, session: Option<SessionKey>) -> SessionKey {
        match session {
            Some(sk) => {
                self.keep_alive(now, c, sk);
                sk
            }
            None => {
                let uk = self.upsert_user(c);
                self.insert_session(now, c, uk)
            }
        }
    }

    fn keep_alive(&mut self, now: Time, c: &Caller, sk: SessionKey) {
        self.upsert_user(c);
        if let Some(s) = self.sessions.get_mut(sk) {
            self.session_expiry.remove(&(s.active, sk));
            s.active = now;
            self.session_expiry.insert((now, sk));
        }
    }

    /// The caller's user, made if new, touched, with the caller's alias.
    fn upsert_user(&mut self, c: &Caller) -> UserKey {
        let uk = match self.user_peers.get(&c.peer_key) {
            Some(&uk) => {
                if let Some(u) = self.users.get_mut(uk)
                    && u.alias != c.alias
                {
                    u.alias.clone_from(&c.alias);
                }
                uk
            }
            None => {
                let id = self.next_user_id;
                self.next_user_id += 1;
                self.metrics.users_created += 1;
                let uk = self.users.insert(UserEntry {
                    id,
                    peer_key: c.peer_key.clone(),
                    alias: c.alias.clone(),
                    sessions: BTreeSet::new(),
                    touched: 0,
                });
                self.user_peers.insert(c.peer_key.clone(), uk);
                uk
            }
        };
        self.touch_user(uk);
        uk
    }

    fn insert_session(&mut self, now: Time, c: &Caller, uk: UserKey) -> SessionKey {
        let oldest = self.users.get(uk).and_then(|u| {
            if u.sessions.len() < self.max_sessions_per_user() {
                return None;
            }
            u.sessions
                .iter()
                .filter_map(|sk| self.sessions.get(*sk).map(|s| (s.active, *sk)))
                .min()
        });
        if let Some((_, sk)) = oldest {
            self.metrics.sessions_evicted += 1;
            self.remove_session(now, sk);
        }
        self.metrics.sessions_created += 1;
        let sk = self.sessions.insert(SessionEntry {
            host_key: c.host_key.clone(),
            host: c.host,
            user: uk,
            published: BTreeSet::new(),
            viewed: BTreeSet::new(),
            active: now,
        });
        self.session_hosts.insert(c.host_key.clone(), sk);
        self.session_expiry.insert((now, sk));
        if let Some(u) = self.users.get_mut(uk) {
            u.sessions.insert(sk);
        }
        self.orphans.remove(&uk);
        sk
    }

    fn insert_listing(
        &mut self,
        l: Listing,
        v: listing::Validated,
        moderation: Option<ListingModeration>,
    ) -> ListingKey {
        let id = self.next_listing_id;
        self.next_listing_id += 1;
        self.metrics.listings_created += 1;
        let lk = self.listings.insert(ListingEntry {
            id,
            key: v.key.clone(),
            listing: l,
            evicted: false,
            moderation: bounded(moderation),
            snippet: ListingSnippet::default(),
            swarm_id: v.swarm_id,
            touched: 0,
            modified: 0,
            publishers: BTreeMap::new(),
            viewers: BTreeSet::new(),
            members: BTreeMap::new(),
            recent: Recent::default(),
            broadcast_recent: 0,
            source: None,
        });
        self.listing_ids.insert(id, lk);
        self.listing_keys.insert(v.key, lk);
        self.modify_listing(lk);
        lk
    }

    // --- memberships ----------------------------------------------------

    fn add_viewer(&mut self, now: Time, lk: ListingKey, sk: SessionKey) {
        let Some(s) = self.sessions.get_mut(sk) else {
            return;
        };
        if s.published.contains(&lk) || !s.viewed.insert(lk) {
            return;
        }
        let uk = s.user;
        let peer_key = self.users.get(uk).map(|u| u.peer_key.clone());
        let max = self.limits.max_recent_users;
        if let Some(l) = self.listings.get_mut(lk) {
            l.viewers.insert(sk);
            l.members.entry(uk).or_default().viewing += 1;
            if let Some(k) = peer_key {
                l.recent.touch(&k, now, max);
            }
        }
    }

    fn remove_viewer(&mut self, lk: ListingKey, sk: SessionKey) -> bool {
        let Some(s) = self.sessions.get_mut(sk) else {
            return false;
        };
        if !s.viewed.remove(&lk) {
            return false;
        }
        let uk = s.user;
        if let Some(l) = self.listings.get_mut(lk) {
            l.viewers.remove(&sk);
            l.release(uk, false);
        }
        true
    }

    fn add_publisher(&mut self, now: Time, lk: ListingKey, sk: SessionKey) {
        let Some(s) = self.sessions.get_mut(sk) else {
            return;
        };
        if !s.published.insert(lk) {
            return;
        }
        let uk = s.user;
        let peer_key = self.users.get(uk).map(|u| u.peer_key.clone());
        self.stamp += 1;
        let order = self.stamp;
        let max = self.limits.max_recent_users;
        if let Some(l) = self.listings.get_mut(lk) {
            l.publishers.insert(sk, order);
            l.members.entry(uk).or_default().publishing += 1;
            if let Some(k) = peer_key {
                l.recent.touch(&k, now, max);
            }
        }
    }

    fn remove_publisher(&mut self, now: Time, lk: ListingKey, sk: SessionKey) -> bool {
        let Some(s) = self.sessions.get_mut(sk) else {
            return false;
        };
        if !s.published.remove(&lk) {
            return false;
        }
        let uk = s.user;
        let Some(l) = self.listings.get_mut(lk) else {
            return true;
        };
        l.publishers.remove(&sk);
        l.release(uk, true);
        if l.source.as_ref().is_some_and(|src| src.session == sk) {
            self.replace_source(now, lk);
        }
        true
    }

    /// Ends session `sk` and its memberships. Returns its user, and whether
    /// the user has no sessions left.
    fn remove_session(&mut self, now: Time, sk: SessionKey) -> Option<(UserKey, bool)> {
        let s = self.sessions.get(sk)?;
        let viewed: Vec<ListingKey> = s.viewed.iter().copied().collect();
        let published: Vec<ListingKey> = s.published.iter().copied().collect();
        for lk in viewed {
            self.remove_viewer(lk, sk);
            self.touch_listing(lk);
        }
        for lk in published {
            self.remove_publisher(now, lk, sk);
            self.touch_listing(lk);
        }
        let s = self.sessions.remove(sk)?;
        if self.session_hosts.get(&s.host_key) == Some(&sk) {
            self.session_hosts.remove(&s.host_key);
        }
        self.session_expiry.remove(&(s.active, sk));
        let empty = match self.users.get_mut(s.user) {
            Some(u) => {
                u.sessions.remove(&sk);
                u.sessions.is_empty()
            }
            None => false,
        };
        self.touch_user(s.user);
        Some((s.user, empty))
    }

    /// Removes user `uk`, returning its offline presence.
    fn remove_user(&mut self, uk: UserKey) -> Option<Event> {
        let u = self.users.remove(uk)?;
        self.metrics.users_removed += 1;
        if self.user_peers.get(&u.peer_key) == Some(&uk) {
            self.user_peers.remove(&u.peer_key);
        }
        self.user_order.remove(&u.touched);
        self.orphans.remove(&uk);
        Some(Event {
            body: Some(Body::UserPresenceChange(UserPresenceChange {
                id: u.id,
                alias: u.alias,
                peer_key: u.peer_key,
                online: false,
                listing_ids: Vec::new(),
            })),
        })
    }

    /// Removes listing `lk`, ending its memberships, and returns its id.
    fn remove_listing(&mut self, lk: ListingKey) -> Option<u64> {
        self.drop_source(lk);
        let l = self.listings.remove(lk)?;
        self.metrics.listings_removed += 1;
        self.listing_ids.remove(&l.id);
        if self.listing_keys.get(&l.key) == Some(&lk) {
            self.listing_keys.remove(&l.key);
        }
        self.listing_order.remove(&l.touched);
        let mut users = Vec::new();
        for sk in &l.viewers {
            if let Some(s) = self.sessions.get_mut(*sk) {
                s.viewed.remove(&lk);
                users.push(s.user);
            }
        }
        for sk in l.publishers.keys() {
            if let Some(s) = self.sessions.get_mut(*sk) {
                s.published.remove(&lk);
                users.push(s.user);
            }
        }
        for uk in users {
            self.touch_user(uk);
        }
        Some(l.id)
    }

    // --- stamps ---------------------------------------------------------

    fn touch_listing(&mut self, lk: ListingKey) {
        if let Some(l) = self.listings.get_mut(lk) {
            self.listing_order.remove(&l.touched);
            self.stamp += 1;
            l.touched = self.stamp;
            self.listing_order.insert(self.stamp, lk);
        }
    }

    fn modify_listing(&mut self, lk: ListingKey) {
        self.touch_listing(lk);
        if let Some(l) = self.listings.get_mut(lk) {
            l.modified = l.touched;
        }
    }

    fn touch_user(&mut self, uk: UserKey) {
        if let Some(u) = self.users.get_mut(uk) {
            self.user_order.remove(&u.touched);
            self.stamp += 1;
            u.touched = self.stamp;
            self.user_order.insert(self.stamp, uk);
        }
    }

    // --- snippet sources ------------------------------------------------

    fn start_source(&mut self, now: Time, lk: ListingKey, sk: SessionKey) {
        let Some(host) = self.sessions.get(sk).map(|s| s.host) else {
            return;
        };
        let Some(l) = self.listings.get_mut(lk) else {
            return;
        };
        let Some(swarm_id) = l.swarm_id.clone() else {
            return;
        };
        l.source = Some(Source {
            session: sk,
            host,
            swarm_id,
            backoff: MIN_SNIPPET_BACKOFF,
            timer: None,
            state: SourceState::Waiting,
        });
        self.subscribe(now, lk);
    }

    fn subscribe(&mut self, now: Time, lk: ListingKey) {
        let sub = SubId(self.next_sub);
        let Some(l) = self.listings.get_mut(lk) else {
            return;
        };
        let Some(src) = l.source.as_mut() else {
            return;
        };
        self.next_sub += 1;
        src.state = SourceState::Subscribed(Box::new(Subscription {
            sub,
            next: ListingSnippet::default(),
            unverified: 0,
            window: now,
            deltas: 0,
        }));
        let action = ServerAction::Subscribe {
            sub,
            listing_id: l.id,
            host: src.host,
            swarm_id: src.swarm_id.clone(),
        };
        self.subs.insert(sub, lk);
        self.metrics.subscriptions += 1;
        self.set_source_timer(lk, Some(now + self.tunables.snippet_timeout));
        self.actions.push_back(action);
    }

    fn set_source_timer(&mut self, lk: ListingKey, at: Option<Time>) {
        let Some(src) = self.listings.get_mut(lk).and_then(|l| l.source.as_mut()) else {
            return;
        };
        if let Some(t) = src.timer.take() {
            self.source_timers.remove(&(t, lk));
        }
        if let Some(t) = at {
            src.timer = Some(t);
            self.source_timers.insert((t, lk));
        }
    }

    /// Ends the source's subscription, if any, asking the control to end
    /// it too when `unsubscribe`.
    fn stop_subscription(&mut self, lk: ListingKey, unsubscribe: bool) {
        let Some(src) = self.listings.get_mut(lk).and_then(|l| l.source.as_mut()) else {
            return;
        };
        if let SourceState::Subscribed(s) = &src.state {
            let sub = s.sub;
            src.state = SourceState::Waiting;
            self.subs.remove(&sub);
            if unsubscribe {
                self.actions.push_back(ServerAction::Unsubscribe { sub });
            }
        }
    }

    fn fail_source(&mut self, now: Time, lk: ListingKey, unsubscribe: bool) {
        self.stop_subscription(lk, unsubscribe);
        let Some(src) = self.listings.get_mut(lk).and_then(|l| l.source.as_mut()) else {
            return;
        };
        let delay = src.backoff;
        src.backoff = (delay * 2).min(MAX_SNIPPET_BACKOFF);
        let quarter = u64::try_from((delay / 4).as_millis()).unwrap_or(0);
        let jitter = self.rng.next_u64() % quarter.saturating_add(1);
        let jitter = Duration::from_millis(i64::try_from(jitter).unwrap_or(0));
        self.set_source_timer(lk, Some(now + delay + jitter));
    }

    fn source_timer(&mut self, now: Time, lk: ListingKey) {
        let Some(l) = self.listings.get_mut(lk) else {
            return;
        };
        let Some(src) = l.source.as_mut() else {
            return;
        };
        src.timer = None;
        let subscribed = matches!(src.state, SourceState::Subscribed(_));
        let publishing = l.publishers.contains_key(&src.session);
        if subscribed {
            self.metrics.snippet_timeouts += 1;
            self.fail_source(now, lk, true);
        } else if publishing {
            self.subscribe(now, lk);
        } else {
            self.replace_source(now, lk);
        }
    }

    /// Moves the listing's source to its newest publisher, or drops it.
    fn replace_source(&mut self, now: Time, lk: ListingKey) {
        self.drop_source(lk);
        let newest = self.listings.get(lk).and_then(|l| {
            l.publishers
                .iter()
                .max_by_key(|(_, order)| **order)
                .map(|(sk, _)| *sk)
        });
        if let Some(sk) = newest {
            self.start_source(now, lk, sk);
        }
    }

    fn drop_source(&mut self, lk: ListingKey) {
        self.stop_subscription(lk, true);
        self.set_source_timer(lk, None);
        if let Some(l) = self.listings.get_mut(lk) {
            l.source = None;
        }
    }

    // --- broadcasts -----------------------------------------------------

    fn broadcast(&mut self, now: Time) {
        let ttl = RECENT_USER_TTL;
        let mut events = Vec::new();

        if self.refresh.is_empty() && now >= self.last_refresh + self.tunables.refresh_interval {
            self.last_refresh = now;
            self.metrics.refreshes += 1;
            let listings = self
                .listing_order
                .values()
                .map(|k| RefreshItem::Listing(*k));
            let users = self.user_order.values().map(|k| RefreshItem::User(*k));
            self.refresh.extend(listings.chain(users));
        }

        let mut spent = 0;
        while let Some(&item) = self.refresh.front() {
            let cost = match item {
                RefreshItem::Listing(lk) => self
                    .listings
                    .get(lk)
                    .filter(|l| l.modified <= self.mark && !l.publishers.is_empty() && !l.evicted)
                    .map(ListingEntry::change_len),
                RefreshItem::User(uk) => self
                    .users
                    .get(uk)
                    .filter(|u| u.touched <= self.mark && !u.sessions.is_empty())
                    .map(|u| u.alias.len() + u.peer_key.len() + 64),
            };
            if let Some(cost) = cost {
                if spent > 0 && spent + cost > REFRESH_BYTES_PER_TICK {
                    break;
                }
                spent += cost;
                self.metrics.refreshed += 1;
                match item {
                    RefreshItem::Listing(lk) => {
                        if let Some(l) = self.listings.get_mut(lk) {
                            events.push(l.change_event());
                            events.push(l.count_event(now, ttl));
                        }
                    }
                    RefreshItem::User(uk) => events.extend(self.presence(uk)),
                }
            }
            self.refresh.pop_front();
        }

        let changed: Vec<ListingKey> = self
            .listing_order
            .range(self.mark + 1..)
            .rev()
            .map(|(_, k)| *k)
            .collect();
        for lk in changed {
            let Some(l) = self.listings.get_mut(lk) else {
                continue;
            };
            if l.publishers.is_empty() || l.evicted {
                if let Some(id) = self.remove_listing(lk) {
                    events.push(Event {
                        body: Some(Body::Unpublish(Unpublish { id })),
                    });
                }
            } else {
                if l.modified > self.mark {
                    events.push(l.change_event());
                }
                events.push(l.count_event(now, ttl));
            }
        }

        let users: Vec<UserKey> = self
            .user_order
            .range(self.mark + 1..)
            .rev()
            .map(|(_, k)| *k)
            .collect();
        for uk in users {
            events.extend(self.presence(uk));
        }

        let new_mark = self.stamp;
        let cutoff = now - self.tunables.session_timeout;
        while let Some(&(t, sk)) = self.session_expiry.first() {
            if t > cutoff {
                break;
            }
            self.session_expiry.pop_first();
            self.metrics.sessions_expired += 1;
            if let Some((uk, true)) = self.remove_session(now, sk) {
                events.extend(self.remove_user(uk));
            }
        }
        for uk in std::mem::take(&mut self.orphans) {
            if self.users.get(uk).is_some_and(|u| u.sessions.is_empty()) {
                events.extend(self.remove_user(uk));
            }
        }
        self.mark = new_mark;

        if !events.is_empty() {
            self.metrics.broadcasts += 1;
            self.metrics.events += events.len() as u64;
            let records = pack(events);
            self.metrics.records += records.len() as u64;
            self.actions.push_back(ServerAction::Broadcast(records));
        }
    }

    /// User `uk`'s online presence, unless it has no sessions.
    fn presence(&self, uk: UserKey) -> Option<Event> {
        let u = self.users.get(uk)?;
        if u.sessions.is_empty() {
            return None;
        }
        let mut ids: Vec<u64> = u
            .sessions
            .iter()
            .filter_map(|sk| self.sessions.get(*sk))
            .flat_map(|s| s.viewed.iter().chain(&s.published))
            .filter_map(|lk| self.listings.get(*lk).map(|l| l.id))
            .collect();
        ids.sort_unstable();
        ids.dedup();
        Some(Event {
            body: Some(Body::UserPresenceChange(UserPresenceChange {
                id: u.id,
                alias: u.alias.clone(),
                peer_key: u.peer_key.clone(),
                online: true,
                listing_ids: ids,
            })),
        })
    }
}

/// `m` without a category over [`listing::MAX_CATEGORY_BYTES`].
fn bounded(m: Option<ListingModeration>) -> Option<ListingModeration> {
    m.map(|mut m| {
        if listing::validate_moderation(&m).is_err() {
            m.category = None;
        }
        m
    })
}

/// `events` in order, in records of at most [`MAX_BROADCAST_RECORD`]
/// bytes, or of one larger event.
fn pack(events: Vec<Event>) -> Vec<EventBroadcast> {
    let mut records = Vec::new();
    let mut record = EventBroadcast::default();
    let mut len = 0;
    for e in events {
        let n = prost::encoding::message::encoded_len(1, &e);
        if !record.events.is_empty() && len + n > MAX_BROADCAST_RECORD {
            records.push(std::mem::take(&mut record));
            len = 0;
        }
        len += n;
        record.events.push(e);
    }
    if !record.events.is_empty() {
        records.push(record);
    }
    records
}

#[cfg(test)]
impl Server {
    /// The state views converge to.
    pub(crate) fn projection(&self) -> crate::view::Projection {
        use crate::view::{ProjectedListing, ProjectedUser, Projection};
        let mut p = Projection::default();
        for l in self.listings.values() {
            p.listings.insert(
                l.id,
                ProjectedListing {
                    listing: l.listing.clone(),
                    snippet: Some(l.snippet.clone()),
                    moderation: l.moderation.clone(),
                    user_count: u32::try_from(l.members.len()).unwrap_or(u32::MAX),
                    recent_user_count: l.broadcast_recent,
                },
            );
        }
        for (uk, u) in &self.users {
            if let Some(Event {
                body: Some(Body::UserPresenceChange(e)),
            }) = self.presence(uk)
            {
                p.users.insert(
                    u.id,
                    ProjectedUser {
                        alias: e.alias,
                        peer_key: e.peer_key,
                        listing_ids: e.listing_ids,
                    },
                );
            }
        }
        p
    }

    /// Whether every listing and user change has been broadcast.
    pub(crate) fn settled(&self) -> bool {
        self.stamp == self.mark && self.orphans.is_empty() && self.refresh.is_empty()
    }

    pub(crate) fn listing_ids(&self) -> Vec<u64> {
        self.listing_ids.keys().copied().collect()
    }

    /// The listings each session of the user with `peer_key` publishes and
    /// views, by host key.
    #[allow(clippy::type_complexity)]
    pub(crate) fn user_sessions(&self, peer_key: &[u8]) -> BTreeMap<Vec<u8>, (Vec<u64>, Vec<u64>)> {
        let mut out = BTreeMap::new();
        let Some(u) = self
            .user_peers
            .get(peer_key)
            .and_then(|k| self.users.get(*k))
        else {
            return out;
        };
        for sk in &u.sessions {
            if let Some(s) = self.sessions.get(*sk) {
                let ids = |set: &BTreeSet<ListingKey>| -> Vec<u64> {
                    set.iter()
                        .filter_map(|lk| self.listings.get(*lk).map(|l| l.id))
                        .collect()
                };
                out.insert(s.host_key.clone(), (ids(&s.published), ids(&s.viewed)));
            }
        }
        out
    }

    /// The host subscribed to for listing `id`'s snippet.
    pub(crate) fn snippet_source(&self, id: u64) -> Option<(ID, Option<SubId>)> {
        let l = self
            .listing_ids
            .get(&id)
            .and_then(|k| self.listings.get(*k))?;
        let src = l.source.as_ref()?;
        let sub = match &src.state {
            SourceState::Subscribed(s) => Some(s.sub),
            SourceState::Waiting => None,
        };
        Some((src.host, sub))
    }

    pub(crate) fn snippet(&self, id: u64) -> Option<&ListingSnippet> {
        let l = self
            .listing_ids
            .get(&id)
            .and_then(|k| self.listings.get(*k))?;
        Some(&l.snippet)
    }

    /// Checks the internal invariants.
    pub(crate) fn check(&self) -> Result<(), String> {
        let t = &self.tunables;
        let lim = &self.limits;
        if self.listings.len() > lim.max_listings
            || self.users.len() > lim.max_users
            || self.sessions.len() > lim.max_sessions
        {
            return Err("over a limit".into());
        }
        if self.listing_ids.len() != self.listings.len()
            || self.listing_keys.len() != self.listings.len()
            || self.listing_order.len() != self.listings.len()
        {
            return Err("listing indexes".into());
        }
        if self.user_peers.len() != self.users.len() || self.user_order.len() != self.users.len() {
            return Err("user indexes".into());
        }
        if self.session_hosts.len() != self.sessions.len()
            || self.session_expiry.len() != self.sessions.len()
        {
            return Err("session indexes".into());
        }
        let mut subs = 0;
        let mut timers = 0;
        for (lk, l) in &self.listings {
            if self.listing_ids.get(&l.id) != Some(&lk)
                || self.listing_keys.get(&l.key) != Some(&lk)
                || self.listing_order.get(&l.touched) != Some(&lk)
                || l.modified > l.touched
            {
                return Err(format!("listing {} indexes", l.id));
            }
            let mut members: BTreeMap<UserKey, Roles> = BTreeMap::new();
            for sk in &l.viewers {
                let s = self.sessions.get(*sk).ok_or("viewer session")?;
                if !s.viewed.contains(&lk) || l.publishers.contains_key(sk) {
                    return Err(format!("listing {} viewer", l.id));
                }
                members.entry(s.user).or_default().viewing += 1;
            }
            for sk in l.publishers.keys() {
                let s = self.sessions.get(*sk).ok_or("publisher session")?;
                if !s.published.contains(&lk) {
                    return Err(format!("listing {} publisher", l.id));
                }
                members.entry(s.user).or_default().publishing += 1;
            }
            if members != l.members {
                return Err(format!("listing {} members", l.id));
            }
            if l.recent.by_key.len() != l.recent.by_time.len()
                || l.recent.by_key.len() > lim.max_recent_users
            {
                return Err(format!("listing {} recent users", l.id));
            }
            if let Some(src) = &l.source {
                if l.swarm_id.is_none() || !l.publishers.contains_key(&src.session) {
                    return Err(format!("listing {} source", l.id));
                }
                if let SourceState::Subscribed(s) = &src.state {
                    subs += 1;
                    if self.subs.get(&s.sub) != Some(&lk) {
                        return Err(format!("listing {} subscription", l.id));
                    }
                }
                if let Some(t) = src.timer {
                    timers += 1;
                    if !self.source_timers.contains(&(t, lk)) {
                        return Err(format!("listing {} timer", l.id));
                    }
                }
            } else if l.swarm_id.is_some() && !l.publishers.is_empty() {
                return Err(format!("listing {} has publishers but no source", l.id));
            }
        }
        if subs != self.subs.len() || timers != self.source_timers.len() {
            return Err("subscriptions or timers".into());
        }
        for (uk, u) in &self.users {
            if self.user_peers.get(&u.peer_key) != Some(&uk)
                || self.user_order.get(&u.touched) != Some(&uk)
            {
                return Err(format!("user {} indexes", u.id));
            }
            if u.sessions.is_empty() != self.orphans.contains(&uk) {
                return Err(format!("user {} orphaned", u.id));
            }
            if u.sessions.len() > self.max_sessions_per_user() {
                return Err(format!("user {} sessions", u.id));
            }
            if self.role_count(uk, true) > t.publish_quota as usize
                || self.role_count(uk, false) > t.join_quota as usize
            {
                return Err(format!("user {} quota", u.id));
            }
        }
        for (sk, s) in &self.sessions {
            if self.session_hosts.get(&s.host_key) != Some(&sk)
                || !self.session_expiry.contains(&(s.active, sk))
            {
                return Err("session indexes".into());
            }
            let u = self.users.get(s.user).ok_or("session user")?;
            if !u.sessions.contains(&sk) {
                return Err("session user".into());
            }
            if !s.viewed.is_disjoint(&s.published) {
                return Err("session publishes and views a listing".into());
            }
            for lk in s.viewed.iter().chain(&s.published) {
                if self.listings.get(*lk).is_none() {
                    return Err("session listing".into());
                }
            }
        }
        if self.refresh.len() > lim.max_listings + lim.max_users {
            return Err("refresh queue".into());
        }
        Ok(())
    }
}

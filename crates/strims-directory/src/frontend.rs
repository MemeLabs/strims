// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The UI's directory calls: `strims.network.v1.directory.DirectoryFrontend`.
//!
//! Calls to a network's directory server go through the
//! [`DirectoryControl`]; reads come from its views. The UI names networks
//! by key, and a key that isn't one of the profile's networks is refused
//! with "network not found". A network's name is its root certificate's
//! subject.
//!
//! # Watches
//!
//! A watch subscribes to the control's events before it reads the views,
//! so what it sends converges on the views. It may repeat what it sent:
//! a `Change` carries listings as they are and a join a user as it is, so
//! a repeat changes nothing.
//!
//! A watch takes events only as the UI takes its responses, so what it
//! queues is one reading of the views at most, plus what one batch of
//! events makes. A UI that falls behind by more than the subscription
//! holds gets the views again, as the difference between what it was sent
//! and what the views hold. A response carries at most
//! [`MAX_RESPONSE_ITEMS`] events or users, and grows past
//! [`MAX_RESPONSE_BYTES`] only by its first one.
//!
//! A watch runs until the UI cancels the call.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context as TaskContext, Poll};

use futures_core::Stream;
use prost::Message;
use strims_crypto::{certificate_body, certificate_root};
use strims_dao::schema::network::network_key;
use strims_dao::{Recv, Subscription};
use strims_kv::Kv;
use strims_proto::network::v1::Network as NetworkRecord;
use strims_proto::network::v1::ca::ca_find_request::Query;
use strims_proto::network::v1::directory::frontend_get_users_response::{
    Alias, User as AliasedUser,
};
use strims_proto::network::v1::directory::frontend_watch_listing_users_response::{
    User as ListingUser, UserEventType,
};
use strims_proto::network::v1::directory::frontend_watch_listings_response::{
    Change, Event, Unpublish, UserCountChange, event::Event as EventBody,
};
use strims_proto::network::v1::directory::{
    DirectoryFrontendService, FrontendGetListingRequest, FrontendGetListingResponse,
    FrontendGetListingsRequest, FrontendGetListingsResponse, FrontendGetUsersRequest,
    FrontendGetUsersResponse, FrontendJoinRequest, FrontendJoinResponse,
    FrontendModerateListingRequest, FrontendModerateListingResponse, FrontendModerateUserRequest,
    FrontendModerateUserResponse, FrontendPartRequest, FrontendPartResponse,
    FrontendPublishRequest, FrontendPublishResponse, FrontendTestRequest, FrontendTestResponse,
    FrontendUnpublishRequest, FrontendUnpublishResponse, FrontendWatchAssetBundlesRequest,
    FrontendWatchAssetBundlesResponse, FrontendWatchListingUsersRequest,
    FrontendWatchListingUsersResponse, FrontendWatchListingsRequest, FrontendWatchListingsResponse,
    Listing, ListingContentType, ListingQuery, Network, NetworkListings, NetworkListingsItem,
    register_directory_frontend_service,
};
use strims_proto::service::{Context, Error as RpcError, ServerStream};
use strims_rpc::{Router, Timer};

use crate::control::{CallFailure, DirectoryControl};
use crate::embed::{self, EmbedService, EmbedServices};
use crate::events::{DirectoryEvent, ListingEvent, UserEventKind};
use crate::host::Host;
use crate::listing::content_type;
use crate::{DirectoryError, view};

/// The most events, or users, one response of a watch carries.
pub const MAX_RESPONSE_ITEMS: usize = 256;

/// The encoded size a watch's response grows past only by its first event
/// or user.
pub const MAX_RESPONSE_BYTES: usize = 1024 * 1024;

/// What an item costs in a response beyond its encoding: its tag and
/// length prefix, rounded up.
const ITEM_OVERHEAD: usize = 16;

/// The embeds `Test` publishes, by service.
const TEST_EMBEDS: [(EmbedService, &[&str]); 4] = [
    (
        EmbedService::YouTube,
        &[
            "DDTGlyJVNVI",
            "SocSlBubzwA",
            "-IO6fpjDJY8",
            "nKrznsPB5t8",
            "PMesD2l6viA",
            "GTYJd2qfx5g",
            "i6zaVYWLTkU",
            "9VE7afYWzYo",
            "JQ4Jx8XfP_w",
            "oEgZeTr3Vs4",
            "pRpeEdMmmQ0",
        ],
    ),
    (
        EmbedService::AngelThump,
        &[
            "psrngafk",
            "bliutwo",
            "t4tv",
            "somuchforsubtlety",
            "erik",
            "windowsmoviehouse",
            "keyno",
            "eastmancolor",
            "feenamabob",
            "harkdan",
        ],
    ),
    (
        EmbedService::TwitchStream,
        &[
            "namemannen",
            "not0like0this",
            "buddha",
            "shroud",
            "purgegamers",
            "sweetdreams",
            "destiny",
            "hannapig_",
            "ibabyrainbow",
            "tastelesstv",
        ],
    ),
    (EmbedService::TwitchVod, &["1157956746", "1160400711"]),
];

/// The embeds of [`TEST_EMBEDS`] whose services `services` has, taking
/// each service's next in turn, so that a small quota still shows every
/// service.
fn test_embeds(services: EmbedServices) -> Vec<Listing> {
    let sets: Vec<&[&str]> = TEST_EMBEDS
        .iter()
        .filter(|(s, _)| services.supports(*s))
        .map(|(_, ids)| *ids)
        .collect();
    let services: Vec<EmbedService> = TEST_EMBEDS
        .iter()
        .map(|(s, _)| *s)
        .filter(|s| services.supports(*s))
        .collect();
    let longest = sets.iter().map(|ids| ids.len()).max().unwrap_or(0);
    let mut out = Vec::new();
    for i in 0..longest {
        for (s, ids) in services.iter().zip(&sets) {
            if let Some(id) = ids.get(i) {
                out.push(embed::listing(*s, id));
            }
        }
    }
    out
}

fn not_found(what: &str) -> RpcError {
    RpcError::new(0, format!("{what} not found"))
}

fn call_error(e: CallFailure) -> RpcError {
    e.to_rpc()
}

/// The directory's name for the profile's network `n`.
fn describe(n: &NetworkRecord) -> Network {
    Network {
        id: n.id,
        name: n
            .certificate
            .as_ref()
            .and_then(|c| certificate_body(certificate_root(c)).ok())
            .map(|b| b.subject)
            .unwrap_or_default(),
        key: network_key(n),
    }
}

fn item(l: view::Listing) -> NetworkListingsItem {
    NetworkListingsItem {
        id: l.id,
        listing: Some(l.listing),
        snippet: l.snippet,
        moderation: l.moderation,
        user_count: l.user_count,
        recent_user_count: l.recent_user_count,
    }
}

fn listing_user(u: &view::User) -> ListingUser {
    ListingUser {
        id: u.id,
        alias: u.alias.clone(),
        peer_key: u.peer_key.clone(),
    }
}

/// Which listings a read or watch reports.
#[derive(Clone, Debug)]
struct ListingFilter {
    /// The content types reported, `None` for every type. It holds only
    /// known types, so a request naming only unknown ones matches nothing.
    content_types: Option<BTreeSet<i32>>,
    /// The one listing reported, 0 for every listing.
    listing_id: u64,
}

impl ListingFilter {
    fn new(content_types: &[i32], listing_id: u64) -> ListingFilter {
        let content_types = (!content_types.is_empty()).then(|| {
            content_types
                .iter()
                .copied()
                .filter(|t| ListingContentType::try_from(*t).is_ok())
                .collect()
        });
        ListingFilter {
            content_types,
            listing_id,
        }
    }

    fn matches(&self, l: &view::Listing) -> bool {
        let ty = i32::from(content_type(&l.listing));
        (self.listing_id == 0 || self.listing_id == l.id)
            && self
                .content_types
                .as_ref()
                .is_none_or(|ts| ts.contains(&ty))
    }
}

/// Serves `strims.network.v1.directory.DirectoryFrontend` for a node's
/// directory control; see the [module docs](self).
pub struct DirectoryFrontend<K: Kv, H: Host, T: Timer> {
    control: DirectoryControl<K, H, T>,
}

impl<K: Kv, H: Host, T: Timer> DirectoryFrontend<K, H, T> {
    /// The frontend of `control`.
    pub fn new(control: DirectoryControl<K, H, T>) -> Self {
        DirectoryFrontend { control }
    }

    /// The profile's networks, in id order.
    fn networks(&self) -> Vec<Network> {
        let mut ns: Vec<Network> = self
            .control
            .network()
            .networks()
            .iter()
            .map(|i| describe(&i.network))
            .collect();
        ns.sort_by_key(|n| n.id);
        ns
    }

    /// The profile's network whose key is `key`.
    fn network(&self, key: &[u8]) -> Result<Network, RpcError> {
        self.control
            .network()
            .network(key)
            .map(|i| describe(&i.network))
            .ok_or_else(|| not_found("network"))
    }

    /// The keys a request names, each one of the profile's networks;
    /// `None` when it names none.
    fn named(&self, keys: &[Vec<u8>]) -> Result<Option<BTreeSet<Vec<u8>>>, RpcError> {
        if keys.is_empty() {
            return Ok(None);
        }
        keys.iter()
            .map(|k| self.network(k).map(|n| n.key))
            .collect::<Result<_, _>>()
            .map(Some)
    }
}

/// Serves the directory frontend of `control` on `router`.
pub fn bind_frontend<K: Kv, H: Host, T: Timer>(
    router: &Router,
    control: &DirectoryControl<K, H, T>,
) {
    let mut router = router.clone();
    register_directory_frontend_service(
        &mut router,
        Rc::new(DirectoryFrontend::new(control.clone())),
    );
}

impl<K: Kv, H: Host, T: Timer> DirectoryFrontendService for DirectoryFrontend<K, H, T> {
    async fn publish(
        &self,
        _: &Context,
        req: FrontendPublishRequest,
    ) -> Result<FrontendPublishResponse, RpcError> {
        let listing = req.listing.unwrap_or_default();
        let id = self
            .control
            .publish(&req.network_key, listing)
            .await
            .map_err(call_error)?;
        Ok(FrontendPublishResponse { id })
    }

    async fn unpublish(
        &self,
        _: &Context,
        req: FrontendUnpublishRequest,
    ) -> Result<FrontendUnpublishResponse, RpcError> {
        self.control
            .unpublish(&req.network_key, req.id)
            .await
            .map_err(call_error)?;
        Ok(FrontendUnpublishResponse {})
    }

    async fn join(
        &self,
        _: &Context,
        req: FrontendJoinRequest,
    ) -> Result<FrontendJoinResponse, RpcError> {
        let query = req.query.unwrap_or_default();
        let id = self
            .control
            .join(&req.network_key, query)
            .await
            .map_err(call_error)?;
        Ok(FrontendJoinResponse { id })
    }

    async fn part(
        &self,
        _: &Context,
        req: FrontendPartRequest,
    ) -> Result<FrontendPartResponse, RpcError> {
        self.control
            .part(&req.network_key, req.id)
            .await
            .map_err(call_error)?;
        Ok(FrontendPartResponse {})
    }

    /// Publishes a fixed set of embed listings from this host, of the
    /// services the network's client config enables, until the publish
    /// quota refuses one. With no service enabled, it's refused with
    /// "unsupported embed service".
    async fn test(
        &self,
        _: &Context,
        req: FrontendTestRequest,
    ) -> Result<FrontendTestResponse, RpcError> {
        self.network(&req.network_key)?;
        let enabled = self
            .control
            .client_config(&req.network_key)
            .and_then(|c| c.integrations)
            .unwrap_or_default();
        let listings = test_embeds(EmbedServices {
            angelthump: enabled.angelthump,
            twitch: enabled.twitch,
            youtube: enabled.youtube,
        });
        if listings.is_empty() {
            return Err(CallFailure::Refused(DirectoryError::UnsupportedEmbed).to_rpc());
        }
        for l in listings {
            match self.control.publish(&req.network_key, l).await {
                Ok(_) => {}
                Err(CallFailure::Refused(DirectoryError::PublishQuota)) => break,
                Err(e) => return Err(call_error(e)),
            }
        }
        Ok(FrontendTestResponse {})
    }

    async fn moderate_listing(
        &self,
        _: &Context,
        req: FrontendModerateListingRequest,
    ) -> Result<FrontendModerateListingResponse, RpcError> {
        self.control
            .moderate_listing(&req.network_key, req.id, req.moderation)
            .await
            .map_err(call_error)?;
        Ok(FrontendModerateListingResponse {})
    }

    /// The user is the holder of the network CA's latest certificate for
    /// the alias.
    async fn moderate_user(
        &self,
        _: &Context,
        req: FrontendModerateUserRequest,
    ) -> Result<FrontendModerateUserResponse, RpcError> {
        let cert = self
            .control
            .network()
            .find_certificate(&req.network_key, Query::Subject(req.alias), false)
            .await?;
        let peer_key = certificate_body(&cert)
            .map_err(|e| RpcError::new(0, e.to_string()))?
            .key;
        self.control
            .moderate_user(&req.network_key, &peer_key, req.moderation)
            .await
            .map_err(call_error)?;
        Ok(FrontendModerateUserResponse {})
    }

    /// The users of every network's view, one per peer key, with the
    /// networks each alias is used on.
    async fn get_users(
        &self,
        _: &Context,
        _: FrontendGetUsersRequest,
    ) -> Result<FrontendGetUsersResponse, RpcError> {
        let mut networks = BTreeMap::new();
        // bound: the views' users
        let mut users: Vec<AliasedUser> = Vec::new();
        let mut by_peer: BTreeMap<Vec<u8>, usize> = BTreeMap::new();
        for n in self.networks() {
            for u in self.control.users(n.id) {
                let i = *by_peer.entry(u.peer_key.clone()).or_insert_with(|| {
                    users.push(AliasedUser {
                        aliases: Vec::new(),
                        peer_key: u.peer_key.clone(),
                    });
                    users.len() - 1
                });
                let Some(user) = users.get_mut(i) else {
                    continue;
                };
                match user.aliases.iter_mut().find(|a| a.alias == u.alias) {
                    Some(a) => a.network_ids.push(n.id),
                    None => user.aliases.push(Alias {
                        alias: u.alias,
                        network_ids: vec![n.id],
                    }),
                }
            }
            networks.insert(n.id, n);
        }
        Ok(FrontendGetUsersResponse { users, networks })
    }

    async fn get_listing(
        &self,
        _: &Context,
        req: FrontendGetListingRequest,
    ) -> Result<FrontendGetListingResponse, RpcError> {
        let n = self.network(&req.network_key)?;
        let l = req
            .query
            .as_ref()
            .and_then(|q| self.control.listing(n.id, q))
            .ok_or_else(|| not_found("listing"))?;
        Ok(FrontendGetListingResponse {
            id: l.id,
            listing: Some(l.listing),
            snippet: l.snippet,
            moderation: l.moderation,
            user_count: l.user_count,
            recent_user_count: l.recent_user_count,
        })
    }

    /// Every named network, or every network of the profile, with the
    /// listings of its view of the requested content types.
    async fn get_listings(
        &self,
        _: &Context,
        req: FrontendGetListingsRequest,
    ) -> Result<FrontendGetListingsResponse, RpcError> {
        let keys = self.named(&req.network_keys)?;
        let filter = ListingFilter::new(&req.content_types, 0);
        let listings = self
            .networks()
            .into_iter()
            .filter(|n| keys.as_ref().is_none_or(|k| k.contains(&n.key)))
            .map(|n| {
                let listings = self.control.listings(n.id);
                NetworkListings {
                    network: Some(n),
                    listings: listings
                        .into_iter()
                        .filter(|l| filter.matches(l))
                        .map(item)
                        .collect(),
                }
            })
            .collect();
        Ok(FrontendGetListingsResponse { listings })
    }

    /// The listings of the named networks, or of every network, of the
    /// requested content types, or the one requested: a `Change` per
    /// network with its listings, then their changes. A named network, or
    /// with none named any network, whose directory starts later is added
    /// the same way, and one that stops has its listings unpublished.
    async fn watch_listings(
        &self,
        _: &Context,
        req: FrontendWatchListingsRequest,
    ) -> Result<ServerStream<FrontendWatchListingsResponse>, RpcError> {
        let keys = self.named(&req.network_keys)?;
        let events = self.control.subscribe();
        let mut feed = ListingsFeed {
            control: self.control.clone(),
            filter: ListingFilter::new(&req.content_types, req.listing_id),
            keys,
            watched: BTreeMap::new(),
            pending: VecDeque::new(),
            max_listings: self.control.shared.config.view.max_listings,
        };
        feed.handle(None);
        Ok(Box::pin(Watch::new(events, feed)))
    }

    /// The users of the listing the query names: a `JOIN` of those it has,
    /// then runs of joins, parts and renames. The watch follows the query:
    /// a listing the view doesn't hold yet, or holds again after an
    /// unpublish or a restart of the directory, is reported once it does,
    /// and its users part when it goes.
    async fn watch_listing_users(
        &self,
        _: &Context,
        req: FrontendWatchListingUsersRequest,
    ) -> Result<ServerStream<FrontendWatchListingUsersResponse>, RpcError> {
        self.network(&req.network_key)?;
        let events = self.control.subscribe();
        let mut feed = UsersFeed {
            control: self.control.clone(),
            network_key: req.network_key,
            query: req.query.unwrap_or_default(),
            network_id: None,
            listing_id: None,
            reported: BTreeMap::new(),
            pending: VecDeque::new(),
            greeted: false,
            max_users: self.control.shared.config.view.max_users,
        };
        feed.handle(None);
        Ok(Box::pin(Watch::new(events, feed)))
    }

    /// The latest asset bundle of each network, then each new one.
    async fn watch_asset_bundles(
        &self,
        _: &Context,
        _: FrontendWatchAssetBundlesRequest,
    ) -> Result<ServerStream<FrontendWatchAssetBundlesResponse>, RpcError> {
        let events = self.control.subscribe();
        let mut feed = BundlesFeed {
            control: self.control.clone(),
            pending: BTreeMap::new(),
        };
        feed.handle(None);
        Ok(Box::pin(Watch::new(events, feed)))
    }
}

// --- watches ----------------------------------------------------------------

/// What a watch makes of the control's events.
trait Feed {
    type Response;

    /// Takes the control's next event, or `None` once the subscription
    /// lagged, which reads the views again.
    fn handle(&mut self, e: Option<&DirectoryEvent>);

    /// Whether a response is ready.
    fn ready(&self) -> bool;

    /// Whether as much is queued as the watch sends before it takes more
    /// events.
    fn full(&self) -> bool;

    /// The next response, while one is ready.
    fn response(&mut self) -> Self::Response;
}

/// A watch's stream: its feed's responses, from the control's events taken
/// as the UI takes the responses. Dropping it unsubscribes.
struct Watch<F> {
    events: Subscription<DirectoryEvent>,
    feed: F,
    /// The control's events ended.
    closed: bool,
}

impl<F: Feed> Watch<F> {
    fn new(events: Subscription<DirectoryEvent>, feed: F) -> Watch<F> {
        Watch {
            events,
            feed,
            closed: false,
        }
    }
}

impl<F: Feed + Unpin> Stream for Watch<F> {
    type Item = Result<F::Response, RpcError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<Self::Item>> {
        let this = &mut *self;
        loop {
            if this.feed.ready() {
                return Poll::Ready(Some(Ok(this.feed.response())));
            }
            if this.closed {
                return Poll::Ready(None);
            }
            loop {
                match Pin::new(&mut this.events).poll_next(cx) {
                    Poll::Ready(Some(Recv::Event(e))) => this.feed.handle(Some(&e)),
                    Poll::Ready(Some(Recv::Lagged)) => this.feed.handle(None),
                    Poll::Ready(None) => {
                        this.closed = true;
                        break;
                    }
                    Poll::Pending => break,
                }
                if this.feed.full() {
                    break;
                }
            }
            if !this.feed.ready() && !this.closed {
                return Poll::Pending;
            }
        }
    }
}

/// Takes the run at the front of `pending` that `same` accepts, within a
/// response's bounds: at least its first item.
fn take_run<I>(
    pending: &mut VecDeque<I>,
    size: impl Fn(&I) -> usize,
    same: impl Fn(&I) -> bool,
) -> Vec<I> {
    let mut out = Vec::new();
    let mut bytes = 0usize;
    while let Some(i) = pending.front() {
        if !same(i) {
            break;
        }
        let len = size(i).saturating_add(ITEM_OVERHEAD);
        let over =
            out.len() >= MAX_RESPONSE_ITEMS || bytes.saturating_add(len) > MAX_RESPONSE_BYTES;
        if !out.is_empty() && over {
            break;
        }
        bytes = bytes.saturating_add(len);
        out.extend(pending.pop_front());
    }
    out
}

/// A network a listings watch follows.
struct Watched {
    network: Network,
    /// The listings the UI was sent and holds. Bound: the view's listings
    /// ([`ViewLimits::max_listings`](crate::ViewLimits)); a listing beyond
    /// it reads the view again.
    reported: BTreeSet<u64>,
}

fn change(network: &Network, listings: Vec<NetworkListingsItem>) -> Event {
    Event {
        event: Some(EventBody::Change(Change {
            listings: Some(NetworkListings {
                network: Some(network.clone()),
                listings,
            }),
        })),
    }
}

fn unpublish(network_id: u64, listing_id: u64) -> Event {
    Event {
        event: Some(EventBody::Unpublish(Unpublish {
            network_id,
            listing_id,
        })),
    }
}

/// Queues `listings` of `network` as `Change` events, each within a
/// response's size bound but for its first listing. No listings make one
/// empty `Change`, which names the network to the UI.
fn push_changes(
    pending: &mut VecDeque<Event>,
    network: &Network,
    listings: Vec<NetworkListingsItem>,
) {
    let budget = MAX_RESPONSE_BYTES.saturating_sub(network.encoded_len() + 2 * ITEM_OVERHEAD);
    let mut items = Vec::new();
    let mut bytes = 0usize;
    for l in listings {
        let len = l.encoded_len().saturating_add(ITEM_OVERHEAD);
        if !items.is_empty() && bytes.saturating_add(len) > budget {
            pending.push_back(change(network, std::mem::take(&mut items)));
            bytes = 0;
        }
        bytes = bytes.saturating_add(len);
        items.push(l);
    }
    pending.push_back(change(network, items));
}

/// The feed of a `WatchListings` call.
struct ListingsFeed<K: Kv, H: Host, T: Timer> {
    control: DirectoryControl<K, H, T>,
    filter: ListingFilter,
    /// The networks the request names, `None` for every network.
    keys: Option<BTreeSet<Vec<u8>>>,
    /// The networks followed, by id: those whose directories run. Bound:
    /// the profile's networks.
    watched: BTreeMap<u64, Watched>,
    /// Bound: one reading of the followed views, plus what one batch of
    /// events makes.
    pending: VecDeque<Event>,
    max_listings: usize,
}

impl<K: Kv, H: Host, T: Timer> ListingsFeed<K, H, T> {
    fn wants(&self, key: &[u8]) -> bool {
        self.keys.as_ref().is_none_or(|k| k.contains(key))
    }

    /// Reads the view of every network followed, and of every network
    /// wanted whose directory runs, again.
    fn snapshot_all(&mut self) {
        let running: Vec<(u64, Vec<u8>)> = self
            .control
            .running()
            .into_iter()
            .filter(|(_, k)| self.wants(k))
            .collect();
        let gone: Vec<u64> = self
            .watched
            .keys()
            .copied()
            .filter(|id| !running.iter().any(|(r, _)| r == id))
            .collect();
        for id in gone {
            self.unwatch(id);
        }
        for (id, key) in running {
            self.snapshot(id, &key);
        }
    }

    /// Follows network `id` and reads its view again: unpublishes what the
    /// UI holds that the view doesn't, then sends the view's listings.
    fn snapshot(&mut self, id: u64, key: &[u8]) {
        let network = self
            .control
            .network()
            .network(key)
            .map(|i| describe(&i.network))
            .filter(|n| n.id == id)
            .unwrap_or_else(|| Network {
                id,
                name: String::new(),
                key: key.to_vec(),
            });
        let listings: Vec<view::Listing> = self
            .control
            .listings(id)
            .into_iter()
            .filter(|l| self.filter.matches(l))
            .collect();
        let ids: BTreeSet<u64> = listings.iter().map(|l| l.id).collect();
        let prev = self.watched.insert(
            id,
            Watched {
                network: network.clone(),
                reported: ids.clone(),
            },
        );
        if let Some(prev) = prev {
            for gone in prev.reported.difference(&ids) {
                self.pending.push_back(unpublish(id, *gone));
            }
        }
        push_changes(
            &mut self.pending,
            &network,
            listings.into_iter().map(item).collect(),
        );
    }

    /// Stops following network `id`, unpublishing what the UI holds of it.
    fn unwatch(&mut self, id: u64) {
        if let Some(w) = self.watched.remove(&id) {
            for l in w.reported {
                self.pending.push_back(unpublish(id, l));
            }
        }
    }

    fn listing_event(&mut self, network_id: u64, event: &ListingEvent) {
        let Some(w) = self.watched.get_mut(&network_id) else {
            return;
        };
        match event {
            ListingEvent::Changed(l) => {
                if !self.filter.matches(l) {
                    if w.reported.remove(&l.id) {
                        self.pending.push_back(unpublish(network_id, l.id));
                    }
                    return;
                }
                if !w.reported.contains(&l.id) && w.reported.len() >= self.max_listings {
                    let key = w.network.key.clone();
                    self.snapshot(network_id, &key);
                    return;
                }
                w.reported.insert(l.id);
                push_changes(&mut self.pending, &w.network, vec![item(l.clone())]);
            }
            ListingEvent::Unpublished(l) => {
                if w.reported.remove(&l.id) {
                    self.pending.push_back(unpublish(network_id, l.id));
                }
            }
            ListingEvent::UserCountChanged {
                id,
                user_count,
                recent_user_count,
            } => {
                if w.reported.contains(id) {
                    self.pending.push_back(Event {
                        event: Some(EventBody::UserCountChange(UserCountChange {
                            network_id,
                            listing_id: *id,
                            user_count: *user_count,
                            recent_user_count: *recent_user_count,
                        })),
                    });
                }
            }
        }
    }
}

impl<K: Kv, H: Host, T: Timer> Feed for ListingsFeed<K, H, T> {
    type Response = FrontendWatchListingsResponse;

    fn handle(&mut self, e: Option<&DirectoryEvent>) {
        let Some(e) = e else {
            self.snapshot_all();
            return;
        };
        match e {
            DirectoryEvent::Started {
                network_id,
                network_key,
            } => {
                if self.wants(network_key) {
                    self.snapshot(*network_id, network_key);
                }
            }
            DirectoryEvent::Stopped { network_id, .. } => self.unwatch(*network_id),
            DirectoryEvent::Lagged { network_id } => {
                let key = self.watched.get(network_id).map(|w| w.network.key.clone());
                if let Some(key) = key {
                    self.snapshot(*network_id, &key);
                }
            }
            DirectoryEvent::Listing { network_id, event } => self.listing_event(*network_id, event),
            DirectoryEvent::User { .. } | DirectoryEvent::AssetBundle { .. } => {}
        }
    }

    fn ready(&self) -> bool {
        !self.pending.is_empty()
    }

    fn full(&self) -> bool {
        self.pending.len() >= MAX_RESPONSE_ITEMS
    }

    fn response(&mut self) -> FrontendWatchListingsResponse {
        FrontendWatchListingsResponse {
            events: take_run(&mut self.pending, Message::encoded_len, |_| true),
        }
    }
}

/// The feed of a `WatchListingUsers` call.
struct UsersFeed<K: Kv, H: Host, T: Timer> {
    control: DirectoryControl<K, H, T>,
    network_key: Vec<u8>,
    query: ListingQuery,
    /// The id of the profile's network with the key.
    network_id: Option<u64>,
    /// The listing the query names, while the view holds it.
    listing_id: Option<u64>,
    /// The users the UI was sent and holds, by id. Bound: the view's users
    /// ([`ViewLimits::max_users`](crate::ViewLimits)); a join beyond it
    /// reads the view again.
    reported: BTreeMap<u64, view::User>,
    /// Bound: twice the reported users, plus one batch of events.
    pending: VecDeque<(UserEventType, ListingUser)>,
    /// Whether the first response, the `JOIN` of the listing's users, was
    /// taken.
    greeted: bool,
    max_users: usize,
}

impl<K: Kv, H: Host, T: Timer> UsersFeed<K, H, T> {
    /// Reads the listing's users again: parts those the UI holds that the
    /// view doesn't, then joins those the view holds that the UI doesn't,
    /// or holds under another alias.
    fn sync(&mut self) {
        self.network_id = self
            .control
            .network()
            .network(&self.network_key)
            .map(|i| i.network.id);
        let listing = self
            .network_id
            .and_then(|n| self.control.listing(n, &self.query).map(|l| (n, l.id)));
        self.listing_id = listing.map(|(_, id)| id);
        let users: BTreeMap<u64, view::User> = listing
            .map(|(n, id)| self.control.listing_users(n, id))
            .unwrap_or_default()
            .into_iter()
            .take(self.max_users)
            .map(|u| (u.id, u))
            .collect();
        let prev = std::mem::replace(&mut self.reported, users);
        for (id, u) in &prev {
            if !self.reported.contains_key(id) {
                self.pending
                    .push_back((UserEventType::Part, listing_user(u)));
            }
        }
        for (id, u) in &self.reported {
            if prev.get(id) != Some(u) {
                self.pending
                    .push_back((UserEventType::Join, listing_user(u)));
            }
        }
    }

    /// Parts every user the UI holds: the listing went.
    fn part_all(&mut self) {
        self.listing_id = None;
        for u in std::mem::take(&mut self.reported).values() {
            self.pending
                .push_back((UserEventType::Part, listing_user(u)));
        }
    }

    fn user_event(&mut self, kind: UserEventKind, user: &view::User) {
        let ty = match kind {
            UserEventKind::Parted => {
                if self.reported.remove(&user.id).is_some() {
                    self.pending
                        .push_back((UserEventType::Part, listing_user(user)));
                }
                return;
            }
            UserEventKind::Joined => UserEventType::Join,
            UserEventKind::Renamed => UserEventType::Rename,
        };
        match self.reported.get(&user.id) {
            Some(u) if u == user => return,
            None if self.reported.len() >= self.max_users => {
                self.sync();
                return;
            }
            _ => {}
        }
        self.reported.insert(user.id, user.clone());
        self.pending.push_back((ty, listing_user(user)));
    }
}

impl<K: Kv, H: Host, T: Timer> Feed for UsersFeed<K, H, T> {
    type Response = FrontendWatchListingUsersResponse;

    fn handle(&mut self, e: Option<&DirectoryEvent>) {
        let Some(e) = e else {
            self.sync();
            return;
        };
        let ours = |id: &u64| Some(*id) == self.network_id;
        match e {
            DirectoryEvent::Started { network_key, .. }
            | DirectoryEvent::Stopped { network_key, .. } => {
                if *network_key == self.network_key {
                    self.sync();
                }
            }
            DirectoryEvent::Lagged { network_id } if ours(network_id) => self.sync(),
            DirectoryEvent::Listing { network_id, event } if ours(network_id) => match event {
                ListingEvent::Changed(_) if self.listing_id.is_none() => self.sync(),
                ListingEvent::Unpublished(l) if Some(l.id) == self.listing_id => self.part_all(),
                _ => {}
            },
            DirectoryEvent::User {
                network_id,
                listing_id,
                kind,
                user,
            } if ours(network_id) && Some(*listing_id) == self.listing_id => {
                self.user_event(*kind, user);
            }
            _ => {}
        }
    }

    fn ready(&self) -> bool {
        !self.greeted || !self.pending.is_empty()
    }

    fn full(&self) -> bool {
        self.pending.len() >= MAX_RESPONSE_ITEMS
    }

    fn response(&mut self) -> FrontendWatchListingUsersResponse {
        self.greeted = true;
        let ty = self
            .pending
            .front()
            .map_or(UserEventType::Join, |(ty, _)| *ty);
        let users = take_run(
            &mut self.pending,
            |(_, u)| u.encoded_len(),
            |(t, _)| *t == ty,
        );
        FrontendWatchListingUsersResponse {
            r#type: i32::from(ty),
            users: users.into_iter().map(|(_, u)| u).collect(),
        }
    }
}

/// The feed of a `WatchAssetBundles` call.
struct BundlesFeed<K: Kv, H: Host, T: Timer> {
    control: DirectoryControl<K, H, T>,
    /// The latest bundle of each network not sent yet, by network id.
    /// Bound: one per network.
    pending: BTreeMap<u64, FrontendWatchAssetBundlesResponse>,
}

impl<K: Kv, H: Host, T: Timer> Feed for BundlesFeed<K, H, T> {
    type Response = FrontendWatchAssetBundlesResponse;

    fn handle(&mut self, e: Option<&DirectoryEvent>) {
        let response = |network_id, network_key, bundle| FrontendWatchAssetBundlesResponse {
            network_id,
            network_key,
            asset_bundle: Some(bundle),
        };
        match e {
            None => {
                for (id, key, bundle) in self.control.asset_bundles() {
                    self.pending.insert(id, response(id, key, bundle));
                }
            }
            Some(DirectoryEvent::AssetBundle {
                network_id,
                network_key,
                bundle,
            }) => {
                let r = response(*network_id, network_key.clone(), bundle.clone());
                self.pending.insert(*network_id, r);
            }
            Some(_) => {}
        }
    }

    fn ready(&self) -> bool {
        !self.pending.is_empty()
    }

    fn full(&self) -> bool {
        false
    }

    fn response(&mut self) -> FrontendWatchAssetBundlesResponse {
        self.pending.pop_first().map(|(_, r)| r).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::indexing_slicing)]

    use super::*;

    #[test]
    fn test_embeds_take_each_enabled_service_in_turn() {
        let ids = |services| -> Vec<(EmbedService, String)> {
            test_embeds(services)
                .iter()
                .filter_map(|l| embed::of(l).map(|(s, id)| (s, id.to_owned())))
                .collect()
        };
        let both = ids(EmbedServices {
            angelthump: true,
            youtube: true,
            twitch: false,
        });
        assert_eq!(both.len(), 21);
        assert_eq!(
            both[..3],
            [
                (EmbedService::YouTube, "DDTGlyJVNVI".to_owned()),
                (EmbedService::AngelThump, "psrngafk".to_owned()),
                (EmbedService::YouTube, "SocSlBubzwA".to_owned()),
            ]
        );
        assert_eq!(both[20], (EmbedService::YouTube, "pRpeEdMmmQ0".to_owned()));
        let twitch = ids(EmbedServices {
            twitch: true,
            ..Default::default()
        });
        assert_eq!(twitch.len(), 12);
        assert_eq!(
            twitch[1],
            (EmbedService::TwitchVod, "1157956746".to_owned())
        );
        assert!(ids(EmbedServices::default()).is_empty());
        for l in test_embeds(EmbedServices {
            angelthump: true,
            twitch: true,
            youtube: true,
        }) {
            assert!(crate::listing::validate(&l).is_ok(), "{l:?}");
        }
    }
}

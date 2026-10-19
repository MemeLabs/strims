// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A member's view of a directory: the listings and users the server's
//! event stream describes. Sans-IO: the control feeds it each record of
//! the event stream in order ([`View::handle_record`]), resets it when the
//! stream skips ([`View::reset`]), and drains its [`ViewEvent`]s after
//! each call.
//!
//! # Semantics
//!
//! - `ListingChange` adds a listing or replaces its listing, snippet and
//!   moderation, keeping its user counts. The snippet's body is read
//!   without verifying its signature: the server verified it, and the
//!   network's root key signs the event swarm. Its images come from the
//!   event's images by digest; a snippet that doesn't decode or names an
//!   image the event lacks is dropped and counted. `UserCountChange` sets
//!   the counts. `Unpublish` removes the listing; its users' memberships
//!   end with it, without part events.
//! - An online `UserPresenceChange` adds a user or sets its alias and
//!   listings. A user is in the listings its latest presence names that the
//!   view holds; it joins a listing named earlier once a `ListingChange`
//!   adds it. An offline presence parts the user from its listings and
//!   removes it.
//! - A malformed event is skipped and counted: a listing change without a
//!   listing, a presence whose peer key or alias is over its bound, an
//!   event without a body. So is a record that doesn't decode.
//!
//! What events can make the view hold is bounded by [`ViewLimits`]; what
//! an event adds beyond a bound is dropped and counted in [`ViewMetrics`].

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use prost::Message;
use strims_proto::network::v1::directory::event::{
    Body, ListingChange, UserCountChange, UserPresenceChange,
};
use strims_proto::network::v1::directory::listing_query::Query;
use strims_proto::network::v1::directory::{
    EventBroadcast, ListingModeration, ListingQuery, ListingSnippet,
};

use crate::{MAX_ALIAS_BYTES, MAX_PEER_KEY_BYTES, ViewLimits, listing, snippet};

/// A listing as a view holds it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Listing {
    pub id: u64,
    pub listing: strims_proto::network::v1::directory::Listing,
    pub snippet: Option<ListingSnippet>,
    pub moderation: Option<ListingModeration>,
    pub user_count: u32,
    pub recent_user_count: u32,
}

/// A user as a view holds it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct User {
    pub id: u64,
    pub alias: String,
    pub peer_key: Vec<u8>,
}

/// What changed in a [`View`].
#[derive(Clone, Debug, PartialEq)]
pub enum ViewEvent {
    /// A listing was added or changed.
    ListingChanged(Listing),
    /// A listing was removed.
    ListingUnpublished(Listing),
    UserCountChanged {
        id: u64,
        user_count: u32,
        recent_user_count: u32,
    },
    UserJoined {
        listing: u64,
        user: User,
    },
    UserParted {
        listing: u64,
        user: User,
    },
    /// A user in the listing took a new alias.
    UserRenamed {
        listing: u64,
        user: User,
    },
    /// Events were dropped for the queue's bound: read the view again.
    Lagged,
}

/// What a [`View`] has read and dropped.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ViewMetrics {
    pub records: u64,
    pub events: u64,
    pub resets: u64,
    /// Records that didn't decode, and malformed events.
    pub malformed_records: u64,
    pub malformed_events: u64,
    /// Events for listings the view doesn't hold.
    pub unknown_listings: u64,
    /// Listings, users and listing ids dropped for a [`ViewLimits`] bound.
    pub listings_dropped: u64,
    pub users_dropped: u64,
    pub ids_dropped: u64,
    /// Snippets that didn't decode or named an image their event lacked.
    pub malformed_snippets: u64,
    /// Listings stored without their snippet's images, without their
    /// snippet, or not at all, for their size.
    pub images_dropped: u64,
    pub snippets_dropped: u64,
    pub oversized_listings: u64,
    /// Events dropped for the queue's bound.
    pub events_dropped: u64,
}

#[derive(Debug)]
struct Entry {
    listing: Listing,
    key: Vec<u8>,
    /// The users in the listing.
    users: BTreeSet<u64>,
}

#[derive(Debug)]
struct UserEntry {
    user: User,
    /// The listing ids of the user's latest presence.
    ids: BTreeSet<u64>,
}

/// A directory's listings and users; see the module docs.
#[derive(Debug)]
pub struct View {
    limits: ViewLimits,
    listings: BTreeMap<u64, Entry>,
    keys: BTreeMap<Vec<u8>, u64>,
    users: BTreeMap<u64, UserEntry>,
    peers: BTreeMap<Vec<u8>, u64>,
    /// Listing ids across users.
    memberships: usize,
    events: VecDeque<ViewEvent>,
    queued_bytes: usize,
    lagged: bool,
    metrics: ViewMetrics,
}

impl View {
    pub fn new(limits: ViewLimits) -> View {
        View {
            limits,
            listings: BTreeMap::new(),
            keys: BTreeMap::new(),
            users: BTreeMap::new(),
            peers: BTreeMap::new(),
            memberships: 0,
            events: VecDeque::new(),
            queued_bytes: 0,
            lagged: false,
            metrics: ViewMetrics::default(),
        }
    }

    /// Reads one record of the event stream. A record that doesn't decode
    /// is skipped and counted; returns whether it decoded.
    pub fn handle_record(&mut self, record: &[u8]) -> bool {
        match EventBroadcast::decode(record) {
            Ok(b) => {
                self.metrics.records += 1;
                self.handle_broadcast(&b);
                true
            }
            Err(_) => {
                self.metrics.malformed_records += 1;
                false
            }
        }
    }

    /// Applies the events of `b` in order.
    pub fn handle_broadcast(&mut self, b: &EventBroadcast) {
        for e in &b.events {
            self.metrics.events += 1;
            match &e.body {
                Some(Body::ListingChange(c)) => self.listing_change(c),
                Some(Body::Unpublish(u)) => self.unpublish(u.id),
                Some(Body::UserCountChange(c)) => self.user_count_change(c),
                Some(Body::UserPresenceChange(p)) => self.presence(p),
                Some(Body::Ping(_)) => {}
                None => self.metrics.malformed_events += 1,
            }
        }
    }

    /// Forgets everything, parting every user from its listings and
    /// unpublishing every listing, as events.
    pub fn reset(&mut self) {
        self.metrics.resets += 1;
        let listings = std::mem::take(&mut self.listings);
        for (id, entry) in listings {
            for uid in &entry.users {
                if let Some(user) = self.users.get(uid).map(|u| u.user.clone()) {
                    self.emit(ViewEvent::UserParted { listing: id, user });
                }
            }
            self.emit(ViewEvent::ListingUnpublished(entry.listing));
        }
        self.keys.clear();
        self.users.clear();
        self.peers.clear();
        self.memberships = 0;
    }

    pub fn poll_event(&mut self) -> Option<ViewEvent> {
        if let Some(e) = self.events.pop_front() {
            self.queued_bytes = self.queued_bytes.saturating_sub(event_cost(&e));
            return Some(e);
        }
        if self.lagged {
            self.lagged = false;
            return Some(ViewEvent::Lagged);
        }
        None
    }

    pub fn listings(&self) -> impl Iterator<Item = &Listing> {
        self.listings.values().map(|e| &e.listing)
    }

    pub fn listing(&self, id: u64) -> Option<&Listing> {
        self.listings.get(&id).map(|e| &e.listing)
    }

    /// The listing `q` names: by id, or by the listing's [`listing::key`].
    pub fn listing_by_query(&self, q: &ListingQuery) -> Option<&Listing> {
        match q.query.as_ref()? {
            Query::Id(id) => self.listing(*id),
            Query::Listing(l) => self.listing(*self.keys.get(&listing::key(l))?),
        }
    }

    pub fn users(&self) -> impl Iterator<Item = &User> {
        self.users.values().map(|u| &u.user)
    }

    /// The users in listing `id`.
    pub fn listing_users(&self, id: u64) -> Vec<User> {
        let Some(e) = self.listings.get(&id) else {
            return Vec::new();
        };
        e.users
            .iter()
            .filter_map(|uid| self.users.get(uid))
            .map(|u| u.user.clone())
            .collect()
    }

    /// The listings the user with `peer_key` is in.
    pub fn listings_of_peer(&self, peer_key: &[u8]) -> Vec<Listing> {
        let Some(u) = self.peers.get(peer_key).and_then(|id| self.users.get(id)) else {
            return Vec::new();
        };
        u.ids
            .iter()
            .filter_map(|id| self.listings.get(id))
            .map(|e| e.listing.clone())
            .collect()
    }

    pub fn metrics(&self) -> &ViewMetrics {
        &self.metrics
    }

    fn emit(&mut self, e: ViewEvent) {
        if self.lagged {
            self.metrics.events_dropped += 1;
            return;
        }
        let cost = event_cost(&e);
        if self.events.len() >= self.limits.max_queued_events
            || self.queued_bytes.saturating_add(cost) > self.limits.max_queued_bytes
        {
            self.lagged = true;
            self.metrics.events_dropped += 1;
            return;
        }
        self.queued_bytes += cost;
        self.events.push_back(e);
    }

    fn listing_change(&mut self, c: &ListingChange) {
        let Some(l) = &c.listing else {
            self.metrics.malformed_events += 1;
            return;
        };
        let max = self.limits.max_listing_bytes;
        let mut snippet = match &c.snippet {
            Some(s) => match snippet::decode(s, &c.snippet_images) {
                Ok(s) => Some(s),
                Err(_) => {
                    self.metrics.malformed_snippets += 1;
                    None
                }
            },
            None => None,
        };
        let moderation = c.moderation.clone();
        let size = |s: &Option<ListingSnippet>| {
            l.encoded_len()
                + s.as_ref().map_or(0, Message::encoded_len)
                + moderation.as_ref().map_or(0, Message::encoded_len)
        };
        if size(&snippet) > max {
            if let Some(s) = snippet.as_mut() {
                s.thumbnail = None;
                s.channel_logo = None;
            }
            self.metrics.images_dropped += 1;
            if size(&snippet) > max {
                snippet = None;
                self.metrics.snippets_dropped += 1;
                if size(&snippet) > max {
                    self.metrics.oversized_listings += 1;
                    return;
                }
            }
        }
        let key = listing::key(l);
        let id = c.id;
        if let Some(e) = self.listings.get_mut(&id) {
            e.listing.listing.clone_from(l);
            e.listing.snippet = snippet;
            e.listing.moderation = moderation;
            let old = std::mem::replace(&mut e.key, key.clone());
            let changed = e.listing.clone();
            if old != key {
                if self.keys.get(&old) == Some(&id) {
                    self.keys.remove(&old);
                }
                self.keys.insert(key, id);
            }
            self.emit(ViewEvent::ListingChanged(changed));
            return;
        }
        if self.listings.len() >= self.limits.max_listings {
            self.metrics.listings_dropped += 1;
            return;
        }
        let listing = Listing {
            id,
            listing: l.clone(),
            snippet,
            moderation,
            user_count: 0,
            recent_user_count: 0,
        };
        let users: BTreeSet<u64> = self
            .users
            .iter()
            .filter(|(_, u)| u.ids.contains(&id))
            .map(|(uid, _)| *uid)
            .collect();
        self.keys.insert(key.clone(), id);
        self.listings.insert(
            id,
            Entry {
                listing: listing.clone(),
                key,
                users: users.clone(),
            },
        );
        self.emit(ViewEvent::ListingChanged(listing));
        for uid in users {
            if let Some(user) = self.users.get(&uid).map(|u| u.user.clone()) {
                self.emit(ViewEvent::UserJoined { listing: id, user });
            }
        }
    }

    fn unpublish(&mut self, id: u64) {
        let Some(e) = self.listings.remove(&id) else {
            self.metrics.unknown_listings += 1;
            return;
        };
        if self.keys.get(&e.key) == Some(&id) {
            self.keys.remove(&e.key);
        }
        for uid in &e.users {
            if let Some(u) = self.users.get_mut(uid)
                && u.ids.remove(&id)
            {
                self.memberships = self.memberships.saturating_sub(1);
            }
        }
        self.emit(ViewEvent::ListingUnpublished(e.listing));
    }

    fn user_count_change(&mut self, c: &UserCountChange) {
        let Some(e) = self.listings.get_mut(&c.id) else {
            self.metrics.unknown_listings += 1;
            return;
        };
        e.listing.user_count = c.user_count;
        e.listing.recent_user_count = c.recent_user_count;
        self.emit(ViewEvent::UserCountChanged {
            id: c.id,
            user_count: c.user_count,
            recent_user_count: c.recent_user_count,
        });
    }

    fn presence(&mut self, p: &UserPresenceChange) {
        if p.peer_key.len() > MAX_PEER_KEY_BYTES || p.alias.len() > MAX_ALIAS_BYTES {
            self.metrics.malformed_events += 1;
            return;
        }
        if !p.online {
            self.remove_user(p.id);
            return;
        }
        let uid = p.id;
        if !self.users.contains_key(&uid) {
            if self.users.len() >= self.limits.max_users {
                self.metrics.users_dropped += 1;
                return;
            }
            self.users.insert(
                uid,
                UserEntry {
                    user: User {
                        id: uid,
                        alias: p.alias.clone(),
                        peer_key: p.peer_key.clone(),
                    },
                    ids: BTreeSet::new(),
                },
            );
            self.peers.insert(p.peer_key.clone(), uid);
        }
        let Some(u) = self.users.get_mut(&uid) else {
            return;
        };
        if u.user.peer_key != p.peer_key {
            let old = std::mem::replace(&mut u.user.peer_key, p.peer_key.clone());
            if self.peers.get(&old) == Some(&uid) {
                self.peers.remove(&old);
            }
            self.peers.insert(p.peer_key.clone(), uid);
        }

        let mut renamed = Vec::new();
        if u.user.alias != p.alias {
            u.user.alias.clone_from(&p.alias);
            renamed = u
                .ids
                .iter()
                .copied()
                .filter(|id| self.listings.contains_key(id))
                .collect();
        }
        let mut ids: BTreeSet<u64> = p.listing_ids.iter().copied().collect();
        while ids.len() > self.limits.max_listing_ids {
            ids.pop_last();
            self.metrics.ids_dropped += 1;
        }
        let removed: Vec<u64> = u.ids.difference(&ids).copied().collect();
        let mut added: Vec<u64> = ids.difference(&u.ids).copied().collect();
        let room = self
            .limits
            .max_memberships
            .saturating_add(removed.len())
            .saturating_sub(self.memberships);
        if added.len() > room {
            self.metrics.ids_dropped += (added.len() - room) as u64;
            added.truncate(room);
        }
        let user = u.user.clone();
        for id in &removed {
            u.ids.remove(id);
        }
        u.ids.extend(added.iter().copied());
        self.memberships = self.memberships.saturating_sub(removed.len()) + added.len();

        for id in renamed {
            let user = user.clone();
            self.emit(ViewEvent::UserRenamed { listing: id, user });
        }
        for id in added {
            if self.link(id, uid, true) {
                let user = user.clone();
                self.emit(ViewEvent::UserJoined { listing: id, user });
            }
        }
        for id in removed {
            if self.link(id, uid, false) {
                let user = user.clone();
                self.emit(ViewEvent::UserParted { listing: id, user });
            }
        }
    }

    /// Adds user `uid` to listing `id`, or removes it; returns whether the
    /// view holds the listing.
    fn link(&mut self, id: u64, uid: u64, add: bool) -> bool {
        let Some(e) = self.listings.get_mut(&id) else {
            return false;
        };
        if add {
            e.users.insert(uid);
        } else {
            e.users.remove(&uid);
        }
        true
    }

    fn remove_user(&mut self, uid: u64) {
        let Some(u) = self.users.remove(&uid) else {
            return;
        };
        if self.peers.get(&u.user.peer_key) == Some(&uid) {
            self.peers.remove(&u.user.peer_key);
        }
        self.memberships = self.memberships.saturating_sub(u.ids.len());
        for id in u.ids {
            if self.link(id, uid, false) {
                let user = u.user.clone();
                self.emit(ViewEvent::UserParted { listing: id, user });
            }
        }
    }
}

/// The bytes an event holds while queued, roughly.
fn event_cost(e: &ViewEvent) -> usize {
    const OVERHEAD: usize = 64;
    OVERHEAD
        + match e {
            ViewEvent::ListingChanged(l) | ViewEvent::ListingUnpublished(l) => {
                l.listing.encoded_len()
                    + l.snippet.as_ref().map_or(0, Message::encoded_len)
                    + l.moderation.as_ref().map_or(0, Message::encoded_len)
            }
            ViewEvent::UserJoined { user, .. }
            | ViewEvent::UserParted { user, .. }
            | ViewEvent::UserRenamed { user, .. } => user.alias.len() + user.peer_key.len(),
            ViewEvent::UserCountChanged { .. } | ViewEvent::Lagged => 0,
        }
}

/// A directory's state as views should hold it.
#[cfg(test)]
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Projection {
    pub listings: BTreeMap<u64, ProjectedListing>,
    pub users: BTreeMap<u64, ProjectedUser>,
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProjectedListing {
    pub listing: strims_proto::network::v1::directory::Listing,
    pub snippet: Option<ListingSnippet>,
    pub moderation: Option<ListingModeration>,
    pub user_count: u32,
    pub recent_user_count: u32,
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProjectedUser {
    pub alias: String,
    pub peer_key: Vec<u8>,
    pub listing_ids: Vec<u64>,
}

#[cfg(test)]
impl View {
    pub(crate) fn projection(&self) -> Projection {
        let mut p = Projection::default();
        for (id, e) in &self.listings {
            let l = &e.listing;
            p.listings.insert(
                *id,
                ProjectedListing {
                    listing: l.listing.clone(),
                    snippet: l.snippet.clone(),
                    moderation: l.moderation.clone(),
                    user_count: l.user_count,
                    recent_user_count: l.recent_user_count,
                },
            );
        }
        for (id, u) in &self.users {
            p.users.insert(
                *id,
                ProjectedUser {
                    alias: u.user.alias.clone(),
                    peer_key: u.user.peer_key.clone(),
                    listing_ids: u
                        .ids
                        .iter()
                        .copied()
                        .filter(|id| self.listings.contains_key(id))
                        .collect(),
                },
            );
        }
        p
    }

    /// Checks the internal invariants.
    pub(crate) fn check(&self) -> Result<(), String> {
        let lim = &self.limits;
        if self.listings.len() > lim.max_listings || self.users.len() > lim.max_users {
            return Err("over a limit".into());
        }
        let memberships: usize = self.users.values().map(|u| u.ids.len()).sum();
        if memberships != self.memberships || memberships > lim.max_memberships {
            return Err(format!("memberships {memberships} vs {}", self.memberships));
        }
        if self.events.len() > lim.max_queued_events || self.queued_bytes > lim.max_queued_bytes {
            return Err("event queue".into());
        }
        for (key, id) in &self.keys {
            if self.listings.get(id).is_none_or(|e| &e.key != key) {
                return Err(format!("key of listing {id}"));
            }
        }
        for (key, uid) in &self.peers {
            if self.users.get(uid).is_none_or(|u| &u.user.peer_key != key) {
                return Err(format!("peer key of user {uid}"));
            }
        }
        for (id, e) in &self.listings {
            if e.listing.id != *id || !self.keys.contains_key(&e.key) {
                return Err(format!("listing {id} indexes"));
            }
            for uid in &e.users {
                if !self.users.get(uid).is_some_and(|u| u.ids.contains(id)) {
                    return Err(format!("listing {id} user {uid}"));
                }
            }
        }
        for (uid, u) in &self.users {
            if u.user.id != *uid || !self.peers.contains_key(&u.user.peer_key) {
                return Err(format!("user {uid} indexes"));
            }
            if u.ids.len() > lim.max_listing_ids {
                return Err(format!("user {uid} ids"));
            }
            for id in &u.ids {
                if let Some(e) = self.listings.get(id)
                    && !e.users.contains(uid)
                {
                    return Err(format!("user {uid} not in listing {id}"));
                }
            }
        }
        Ok(())
    }
}

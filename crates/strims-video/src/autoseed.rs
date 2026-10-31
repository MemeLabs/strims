// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Autoseed: the device seeds the swarms of the directory listings its
//! rules name.
//!
//! # Rules
//!
//! The device's autoseed config and rules are records in its store
//! ([`VideoControl::set_autoseed_config`],
//! [`VideoControl::create_autoseed_rule`] and the rest). A rule names a
//! network key of 1 to [`MAX_NETWORK_KEY_BYTES`] bytes, a swarm id of 1
//! to [`MAX_SWARM_ID_BYTES`] and a salt, with a label; the salt, the
//! label and the number of rules the frontend creates are bounded by
//! [`AutoseedSettings`]. Deleting a network deletes its rules.
//!
//! # Seeds
//!
//! While the config is enabled, a rule matches a media listing in the
//! view of its network whose swarm URI names its swarm id, with options
//! within the egress bounds ([`EgressSettings`]); the salt takes no part.
//! A rule that matches listings seeds the swarm with the options of the
//! one of lowest listing id, and a live window of
//! [`EgressSettings::window_bytes`]: a swarm the node adds or seeds with
//! other options is reopened with those, and so is a seeded swarm when its
//! listing's options change ([`egress`](crate::egress#options)). At most
//! [`AutoseedSettings::max_seeds`] rules seed at once, the lowest rule
//! ids first.
//!
//! A seed's transfer is published on its rule's network once the node
//! joins it:
//!
//! - Under the empty salt, the transfer is a video swarm's
//!   ([crate docs](crate#video-swarms)), and the seed holds its slot. A
//!   swarm the node reads for streams is seeded where it is, and streams
//!   read a seeded swarm where it is; it goes with the last of its seeds
//!   and streams. A capture's or an ingest session's swarm is seeded
//!   where it is too: the seed publishes it on the rule's network, and
//!   it goes with its writer. An ingest session that takes over a seeded
//!   swarm's slot keeps its seeds, which publish the session's swarm. A
//!   seed's end withdraws the transfer from its rule's network unless its
//!   writer, a stream or another seed names that network too.
//! - Under another salt, autoseed seeds only a transfer it added: while
//!   another component has a transfer of the swarm id under the salt, the
//!   rule seeds nothing ([`Metrics::autoseed_skipped`](crate::Metrics::autoseed_skipped)).
//!   The transfer and its swarm go with the last rule seeding them.
//!
//! # Updates
//!
//! Autoseed keeps an index of the listings that match some rule, and
//! plans the seeds from it again whenever it changes: as the directory
//! reports listings that change or go, and networks whose directories
//! start or stop. It reads the views of the networks rules name
//! ([`DirectoryControl::listings`](strims_directory::DirectoryControl::listings))
//! when the config or a rule changes, and after the store's or the
//! directory's events lag.

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use futures_util::FutureExt;
use strims_dao::schema::autoseed::{AutoseedConfig, AutoseedRules, new_rule};
use strims_dao::{Singleton, Table};
use strims_directory::listing::MAX_SWARM_ID_BYTES;
use strims_directory::{DirectoryEvent, Host, ListingEvent};
use strims_kv::Kv;
use strims_ppspp::{SwarmId, SwarmOptions};
use strims_proto::autoseed::v1::{Config as SeedConfig, Rule};
use strims_proto::network::v1::directory::Listing;
use strims_proto::network::v1::directory::listing::Content;
use strims_rpc::Timer;
use strims_transfer::TransferId;

use crate::config::{AutoseedSettings, EgressSettings, MAX_NETWORK_KEY_BYTES};
use crate::control::Shared;
use crate::egress::check_stream;
use crate::registry::{Seeded, same_options};
use crate::{VideoControl, VideoError, VideoHost};

// --- planning -----------------------------------------------------------------

/// The swarm of a listing, as a seed adds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Candidate {
    pub(crate) swarm_id: SwarmId,
    /// Its options, with the egress live window.
    pub(crate) options: SwarmOptions,
}

/// The swarm of `l`, if it is a media listing whose swarm URI passes the
/// egress checks of `egress`.
pub(crate) fn candidate(egress: &EgressSettings, l: &Listing) -> Option<Candidate> {
    let Some(Content::Media(m)) = &l.content else {
        return None;
    };
    let (uri, options) = check_stream(egress, &m.swarm_uri, &[]).ok()?;
    Some(Candidate {
        swarm_id: uri.id,
        options,
    })
}

/// The listings that match some rule, by network key and listing id.
#[derive(Default)]
pub(crate) struct Index {
    /// The swarm ids rules name, by network key. Bound: the rules.
    named: BTreeMap<Vec<u8>, BTreeSet<SwarmId>>,
    /// Bound: the listings of the views of the networks rules name
    /// ([`ViewLimits::max_listings`](strims_directory::ViewLimits::max_listings)
    /// each).
    listings: BTreeMap<Vec<u8>, BTreeMap<u64, Candidate>>,
}

impl Index {
    /// An empty index of the listings that match `rules`.
    pub(crate) fn new<'a>(rules: impl IntoIterator<Item = &'a Rule>) -> Index {
        let mut named: BTreeMap<Vec<u8>, BTreeSet<SwarmId>> = BTreeMap::new();
        for r in rules {
            named
                .entry(r.network_key.clone())
                .or_default()
                .insert(SwarmId::new(&r.swarm_id));
        }
        Index {
            named,
            listings: BTreeMap::new(),
        }
    }

    /// Whether a rule names the network `key`.
    pub(crate) fn names(&self, key: &[u8]) -> bool {
        self.named.contains_key(key)
    }

    /// Takes listing `id` of the network `key` as `l`, checked against
    /// `egress`. Returns whether the index changed.
    pub(crate) fn update(
        &mut self,
        egress: &EgressSettings,
        key: &[u8],
        id: u64,
        l: &Listing,
    ) -> bool {
        let Some(swarms) = self.named.get(key) else {
            return false;
        };
        match candidate(egress, l).filter(|c| swarms.contains(&c.swarm_id)) {
            Some(c) => {
                let listings = self.listings.entry(key.to_vec()).or_default();
                listings.insert(id, c.clone()) != Some(c)
            }
            None => self.remove(key, id),
        }
    }

    /// Drops listing `id` of the network `key`. Returns whether the index
    /// held it.
    pub(crate) fn remove(&mut self, key: &[u8], id: u64) -> bool {
        let Some(listings) = self.listings.get_mut(key) else {
            return false;
        };
        let held = listings.remove(&id).is_some();
        if listings.is_empty() {
            self.listings.remove(key);
        }
        held
    }

    /// Drops the listings of the network `key`. Returns whether there were
    /// any.
    pub(crate) fn clear(&mut self, key: &[u8]) -> bool {
        self.listings.remove(key).is_some()
    }
}

/// The seed a rule wants: the swarm of a listing, under the rule's salt,
/// published on the rule's network.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Want {
    pub(crate) network_key: Vec<u8>,
    pub(crate) listing_id: u64,
    pub(crate) swarm_id: SwarmId,
    pub(crate) salt: Vec<u8>,
    pub(crate) options: SwarmOptions,
}

impl Want {
    /// The transfer it seeds.
    pub(crate) fn transfer_id(&self) -> TransferId {
        TransferId::new(&self.swarm_id, &self.salt)
    }

    /// Whether it seeds what `o` seeds: the same transfer on the same
    /// network.
    fn same_seed(&self, o: &Want) -> bool {
        self.transfer_id() == o.transfer_id() && self.network_key == o.network_key
    }
}

/// The seeds `rules` want of the listings in `index`, by rule id: none
/// unless `enabled`, and at most `max_seeds`, from the lowest rule id
/// up. A rule takes the lowest id of the listings that match it.
pub(crate) fn plan(
    enabled: bool,
    rules: &BTreeMap<u64, Rule>,
    index: &Index,
    max_seeds: usize,
) -> BTreeMap<u64, Want> {
    let mut wants = BTreeMap::new();
    if !enabled {
        return wants;
    }
    for (id, r) in rules {
        if wants.len() >= max_seeds {
            break;
        }
        let Some(listings) = index.listings.get(&r.network_key) else {
            continue;
        };
        let swarm_id = SwarmId::new(&r.swarm_id);
        let Some((listing_id, c)) = listings.iter().find(|(_, c)| c.swarm_id == swarm_id) else {
            continue;
        };
        wants.insert(
            *id,
            Want {
                network_key: r.network_key.clone(),
                listing_id: *listing_id,
                swarm_id,
                salt: r.salt.clone(),
                options: c.options.clone(),
            },
        );
    }
    wants
}

// --- state --------------------------------------------------------------------

/// What a rule's seed holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Hold {
    /// A seed on registry slot `.0` of its transfer.
    Slot(u64),
    /// A hold on the salted transfer that add `.0` made.
    Salted(u64),
    /// Nothing: the salted transfer is another component's.
    Skipped,
}

/// A rule's seed.
struct Seed {
    want: Want,
    hold: Hold,
}

/// A transfer under a non-empty salt that autoseed added.
struct Salted<S> {
    /// Its swarm, once added; the transfer exists while it is set.
    swarm: Option<S>,
    /// The add that made it, which its task matches.
    add: u64,
    /// The keys of the networks it is published on, each once the node
    /// joins it. Bound: the networks rules name.
    networks: BTreeSet<Vec<u8>>,
    /// Rules seeding it. At 0 while its swarm is being added, it goes when
    /// the add ends.
    rules: usize,
}

/// Autoseed's index and seeds; see the [module docs](self).
pub(crate) struct Autoseed<S> {
    /// The key of each network whose directory runs, by network id.
    /// Bound: the profile's networks.
    networks: BTreeMap<u64, Vec<u8>>,
    index: Index,
    /// The seeds, by rule id. Bound:
    /// [`AutoseedSettings::max_seeds`].
    seeds: BTreeMap<u64, Seed>,
    /// The salted transfers autoseed added, by transfer id. Bound: a
    /// transfer per seed, and those whose seeds ended while their swarm
    /// was being added, until the add ends.
    salted: BTreeMap<TransferId, Salted<S>>,
    next_add: u64,
    /// The views are to be read again and the seeds planned from them.
    reconcile: bool,
}

impl<S> Autoseed<S> {
    /// The id and key of each network whose directory runs.
    pub(crate) fn directory_networks(&self) -> impl Iterator<Item = (u64, &Vec<u8>)> {
        self.networks.iter().map(|(id, key)| (*id, key))
    }
}

impl<S> Default for Autoseed<S> {
    fn default() -> Self {
        Autoseed {
            networks: BTreeMap::new(),
            index: Index::default(),
            seeds: BTreeMap::new(),
            salted: BTreeMap::new(),
            next_add: 1,
            reconcile: false,
        }
    }
}

/// What a salted transfer's add found once its swarm was added.
enum Landed {
    /// Its entry holds the swarm: add the transfer.
    Kept,
    /// Its entry is gone, or no rule seeds it any more: remove the swarm.
    Unwanted,
    /// Another component added the transfer meanwhile: remove the swarm.
    Taken,
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Shared<K, H, V, T> {
    /// Has `run` read the views again and plan the seeds from them, once
    /// the store's events at hand are taken.
    pub(crate) fn schedule_autoseed(&self) {
        self.autoseed.borrow_mut().reconcile = true;
        self.wake_run();
    }

    /// Whether a [`reconcile_autoseed`](Self::reconcile_autoseed) is due.
    pub(crate) fn autoseed_due(&self) -> bool {
        self.autoseed.borrow().reconcile
    }

    /// Reads the networks whose directories run.
    pub(crate) fn read_autoseed_networks(&self) {
        let networks = self
            .directory
            .networks()
            .into_iter()
            .map(|n| (n.network_id, n.network_key))
            .collect();
        self.autoseed.borrow_mut().networks = networks;
    }

    /// Indexes the listings of the views of the networks rules name, and
    /// plans the seeds from them.
    pub(crate) fn reconcile_autoseed(self: &Rc<Self>) {
        let mut index = self.with_autoseed_records(|enabled, rules| {
            if enabled {
                Index::new(rules.values())
            } else {
                Index::default()
            }
        });
        let named: Vec<(u64, Vec<u8>)> = {
            let mut a = self.autoseed.borrow_mut();
            a.reconcile = false;
            a.networks
                .iter()
                .filter(|(_, key)| index.names(key))
                .map(|(id, key)| (*id, key.clone()))
                .collect()
        };
        for (id, key) in named {
            for l in self.directory.listings(id) {
                index.update(&self.config.egress, &key, l.id, &l.listing);
            }
        }
        self.autoseed.borrow_mut().index = index;
        self.replan_autoseed();
    }

    /// Takes a directory event into the index, and plans the seeds again
    /// if it changed.
    pub(crate) fn autoseed_directory_event(self: &Rc<Self>, e: &DirectoryEvent) {
        let changed = {
            let a = &mut *self.autoseed.borrow_mut();
            match e {
                DirectoryEvent::Started {
                    network_id,
                    network_key,
                } => {
                    a.networks.insert(*network_id, network_key.clone());
                    a.index.clear(network_key)
                }
                DirectoryEvent::Stopped {
                    network_id,
                    network_key,
                } => {
                    a.networks.remove(network_id);
                    a.index.clear(network_key)
                }
                DirectoryEvent::Listing { network_id, event } => {
                    let Some(key) = a.networks.get(network_id) else {
                        return;
                    };
                    match event {
                        ListingEvent::Changed(l) => {
                            a.index.update(&self.config.egress, key, l.id, &l.listing)
                        }
                        ListingEvent::Unpublished(l) => a.index.remove(key, l.id),
                        ListingEvent::UserCountChanged { .. } => false,
                    }
                }
                DirectoryEvent::Lagged { network_id } => {
                    if a.networks.get(network_id).is_some_and(|k| a.index.names(k)) {
                        a.reconcile = true;
                    }
                    false
                }
                DirectoryEvent::User { .. } | DirectoryEvent::AssetBundle { .. } => false,
            }
        };
        if changed {
            self.replan_autoseed();
        }
    }

    /// Plans the seeds from the index, then ends and starts seeds to
    /// match.
    fn replan_autoseed(self: &Rc<Self>) {
        let wants = {
            let a = self.autoseed.borrow();
            self.with_autoseed_records(|enabled, rules| {
                plan(enabled, rules, &a.index, self.config.autoseed.max_seeds)
            })
        };
        let held = std::mem::take(&mut self.autoseed.borrow_mut().seeds);
        let mut seeds = BTreeMap::new();
        let mut skipped = BTreeSet::new();
        for (rule, seed) in held {
            match wants.get(&rule) {
                Some(w) if w.same_seed(&seed.want) && self.seed_held(&seed) => {
                    let want = w.clone();
                    self.follow_listing(&want);
                    seeds.insert(rule, Seed { want, ..seed });
                    continue;
                }
                Some(w) if w.same_seed(&seed.want) && seed.hold == Hold::Skipped => {
                    skipped.insert(rule);
                }
                _ => {}
            }
            self.end_seed(seed);
        }
        for (rule, want) in wants {
            if seeds.contains_key(&rule) {
                continue;
            }
            let hold = self.start_seed(&want);
            if hold == Hold::Skipped && !skipped.contains(&rule) {
                self.metrics.borrow_mut().autoseed_skipped += 1;
            }
            seeds.insert(rule, Seed { want, hold });
        }
        self.autoseed.borrow_mut().seeds = seeds;
    }

    /// Reopens the slot a held seed of `w` holds with the options of `w`'s
    /// listing, when it has others.
    fn follow_listing(self: &Rc<Self>, w: &Want) {
        let t = w.transfer_id();
        let now = self.timer.now();
        let reopened = self.with_swarms(|r| {
            r.get(&t)
                .filter(|e| !same_options(&e.options, &w.options))?;
            r.reopen(&t, w.options.clone(), now)
        });
        if let Some((slot, old)) = reopened {
            tracing::debug!(swarm = %w.swarm_id, "a seed reopens its swarm with its listing's options");
            self.reopen_swarm(t, slot, old, w.swarm_id.clone(), w.options.clone());
        }
    }

    /// Whether `seed` still holds what it seeds.
    fn seed_held(&self, seed: &Seed) -> bool {
        let t = seed.want.transfer_id();
        match seed.hold {
            Hold::Slot(slot) => self.with_swarms(|r| r.seeds_slot(&t, slot)),
            Hold::Salted(add) => self
                .autoseed
                .borrow()
                .salted
                .get(&t)
                .is_some_and(|e| e.add == add),
            Hold::Skipped => false,
        }
    }

    /// Starts the seed `w` wants: what it holds.
    fn start_seed(self: &Rc<Self>, w: &Want) -> Hold {
        let t = w.transfer_id();
        if w.salt.is_empty() {
            let now = self.timer.now();
            let seeded = self.with_swarms(|r| {
                r.seed(
                    t,
                    w.network_key.clone(),
                    w.swarm_id.clone(),
                    w.options.clone(),
                    now,
                )
            });
            return match seeded {
                Seeded::Held(slot) => {
                    self.publish_joined(&t);
                    Hold::Slot(slot)
                }
                Seeded::Reopened { slot, old } => {
                    self.reopen_swarm(t, slot, old, w.swarm_id.clone(), w.options.clone());
                    Hold::Slot(slot)
                }
                Seeded::Added(slot) => {
                    let add = self.clone().add_stream_swarm(
                        t,
                        slot,
                        w.swarm_id.clone(),
                        w.options.clone(),
                    );
                    self.spawn(add.boxed_local());
                    Hold::Slot(slot)
                }
            };
        }
        let joined = {
            let mut a = self.autoseed.borrow_mut();
            match a.salted.get_mut(&t) {
                Some(e) => {
                    e.rules += 1;
                    e.networks.insert(w.network_key.clone());
                    Some(e.add)
                }
                None => None,
            }
        };
        if let Some(add) = joined {
            self.publish_salted(&t);
            return Hold::Salted(add);
        }
        if self.transfers.find(&w.swarm_id, &w.salt).is_some() {
            return Hold::Skipped;
        }
        let add = {
            let mut a = self.autoseed.borrow_mut();
            let add = a.next_add;
            a.next_add += 1;
            a.salted.insert(
                t,
                Salted {
                    swarm: None,
                    add,
                    networks: BTreeSet::from([w.network_key.clone()]),
                    rules: 1,
                },
            );
            add
        };
        let task = self.clone().add_salted_swarm(
            t,
            add,
            w.swarm_id.clone(),
            w.salt.clone(),
            w.options.clone(),
        );
        self.spawn(task.boxed_local());
        Hold::Salted(add)
    }

    /// Ends `seed`, taking down what no seed or stream holds any more.
    fn end_seed(self: &Rc<Self>, seed: Seed) {
        let t = seed.want.transfer_id();
        match seed.hold {
            Hold::Slot(slot) => {
                let network = &seed.want.network_key;
                let released = self.with_swarms(|r| r.unseed(&t, slot, network));
                self.released(t, released);
            }
            Hold::Salted(add) => {
                let gone = {
                    let mut a = self.autoseed.borrow_mut();
                    let last = a
                        .salted
                        .get_mut(&t)
                        .filter(|e| e.add == add)
                        .is_some_and(|e| {
                            e.rules = e.rules.saturating_sub(1);
                            e.rules == 0 && e.swarm.is_some()
                        });
                    if last { a.salted.remove(&t) } else { None }
                };
                if let Some(swarm) = gone.and_then(|e| e.swarm) {
                    self.transfers.remove(t);
                    let s = self.clone();
                    self.spawn(async move { s.remove_host_swarm(swarm).await }.boxed_local());
                }
            }
            Hold::Skipped => {}
        }
    }

    /// Adds the swarm of salted transfer `t`, made by add `add`, then the
    /// transfer, unless another component added it meanwhile.
    async fn add_salted_swarm(
        self: Rc<Self>,
        t: TransferId,
        add: u64,
        id: SwarmId,
        salt: Vec<u8>,
        options: SwarmOptions,
    ) {
        let swarm = match self.host.add_swarm(id.clone(), options).await {
            Ok(swarm) => swarm,
            Err(e) => {
                self.host_failed("adding an autoseed swarm", &e);
                let mut a = self.autoseed.borrow_mut();
                if a.salted.get(&t).is_some_and(|e| e.add == add) {
                    a.salted.remove(&t);
                }
                return;
            }
        };
        let taken = self.transfers.find(&id, &salt).is_some();
        let landed = {
            let mut a = self.autoseed.borrow_mut();
            let state = a
                .salted
                .get(&t)
                .filter(|e| e.add == add && e.swarm.is_none())
                .map(|e| e.rules);
            match state {
                None => Landed::Unwanted,
                Some(0) => {
                    a.salted.remove(&t);
                    Landed::Unwanted
                }
                Some(_) if taken => {
                    a.salted.remove(&t);
                    Landed::Taken
                }
                Some(_) => {
                    if let Some(e) = a.salted.get_mut(&t) {
                        e.swarm = Some(swarm);
                    }
                    Landed::Kept
                }
            }
        };
        match landed {
            Landed::Kept => {
                self.transfers.add(swarm, &id, salt);
                self.publish_salted(&t);
            }
            Landed::Unwanted => self.remove_host_swarm(swarm).await,
            Landed::Taken => {
                self.replan_autoseed();
                self.remove_host_swarm(swarm).await;
            }
        }
    }

    /// Publishes salted transfer `t` on its networks the node has joined.
    fn publish_salted(&self, t: &TransferId) {
        let keys: Vec<Vec<u8>> = self
            .autoseed
            .borrow()
            .salted
            .get(t)
            .filter(|e| e.swarm.is_some())
            .map(|e| e.networks.iter().cloned().collect())
            .unwrap_or_default();
        self.publish_on_joined(t, keys);
    }

    /// [`publish_salted`](Self::publish_salted) for each salted transfer
    /// on the network `key`, or on any network.
    pub(crate) fn publish_salted_on(&self, key: Option<&[u8]>) {
        let ids: Vec<TransferId> = self
            .autoseed
            .borrow()
            .salted
            .iter()
            .filter(|(_, e)| key.is_none_or(|k| e.networks.contains(k)))
            .map(|(t, _)| *t)
            .collect();
        for t in ids {
            self.publish_salted(&t);
        }
    }
}

// --- records ------------------------------------------------------------------

/// Checks a rule's fields against `s`; see the [module docs](self#rules).
fn check_rule(s: &AutoseedSettings, r: &Rule) -> Result<(), VideoError> {
    let within = |n: usize, max: usize| (1..=max).contains(&n);
    if !within(r.network_key.len(), MAX_NETWORK_KEY_BYTES) {
        return Err(VideoError::AutoseedRule("network key"));
    }
    if !within(r.swarm_id.len(), MAX_SWARM_ID_BYTES) {
        return Err(VideoError::AutoseedRule("swarm id"));
    }
    if r.salt.len() > s.max_salt_bytes {
        return Err(VideoError::AutoseedRule("salt"));
    }
    if r.label.len() > s.max_label_bytes {
        return Err(VideoError::AutoseedRule("label"));
    }
    Ok(())
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> VideoControl<K, H, V, T> {
    /// The device's autoseed config.
    pub async fn autoseed_config(&self) -> Result<SeedConfig, VideoError> {
        let mut tx = self.shared.store.read().await?;
        Ok(AutoseedConfig::get(&mut tx).await?.unwrap_or_default())
    }

    /// Stores `c` as the device's autoseed config.
    pub async fn set_autoseed_config(&self, c: SeedConfig) -> Result<SeedConfig, VideoError> {
        let s = &self.shared;
        let mut tx = s.store.write(s.timer.now()).await?;
        let c = AutoseedConfig::set(&mut tx, c).await?;
        tx.commit().await?;
        Ok(c)
    }

    /// The device's autoseed rules, in id order.
    pub async fn list_autoseed_rules(&self) -> Result<Vec<Rule>, VideoError> {
        let mut tx = self.shared.store.read().await?;
        Ok(AutoseedRules::get_all(&mut tx).await?)
    }

    /// The autoseed rule `id`.
    pub async fn get_autoseed_rule(&self, id: u64) -> Result<Rule, VideoError> {
        let mut tx = self.shared.store.read().await?;
        AutoseedRules::get(&mut tx, id)
            .await?
            .ok_or(VideoError::AutoseedRuleNotFound)
    }

    /// Creates an autoseed rule of `rule`'s network key, swarm id, salt
    /// and label, under a new id.
    ///
    /// # Errors
    ///
    /// A field is out of its bounds (see the [module docs](self#rules)),
    /// or the device has
    /// [`AutoseedSettings::max_rules`](crate::AutoseedSettings::max_rules)
    /// rules.
    pub async fn create_autoseed_rule(&self, rule: Rule) -> Result<Rule, VideoError> {
        let s = &self.shared;
        check_rule(&s.config.autoseed, &rule)?;
        let mut tx = s.store.write(s.timer.now()).await?;
        if AutoseedRules::get_all(&mut tx).await?.len() >= s.config.autoseed.max_rules {
            return Err(VideoError::TooManyAutoseedRules);
        }
        let r = new_rule(&mut tx, Rule { id: 0, ..rule }).await?;
        let r = AutoseedRules::insert(&mut tx, r).await?;
        tx.commit().await?;
        Ok(r)
    }

    /// Gives autoseed rule `id` the network key, swarm id, salt and label
    /// of `rule`.
    ///
    /// # Errors
    ///
    /// As [`VideoControl::create_autoseed_rule`], or there's no rule `id`.
    pub async fn update_autoseed_rule(&self, id: u64, rule: Rule) -> Result<Rule, VideoError> {
        let s = &self.shared;
        check_rule(&s.config.autoseed, &rule)?;
        let mut tx = s.store.write(s.timer.now()).await?;
        if AutoseedRules::get(&mut tx, id).await?.is_none() {
            return Err(VideoError::AutoseedRuleNotFound);
        }
        let r = AutoseedRules::update(&mut tx, Rule { id, ..rule }).await?;
        tx.commit().await?;
        Ok(r)
    }

    /// Deletes autoseed rule `id`, if there is one.
    pub async fn delete_autoseed_rule(&self, id: u64) -> Result<(), VideoError> {
        let s = &self.shared;
        let mut tx = s.store.write(s.timer.now()).await?;
        if AutoseedRules::delete(&mut tx, id).await? {
            tx.commit().await?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "autoseed_test.rs"]
mod tests;

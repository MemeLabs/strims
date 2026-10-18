// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A keeper for one listing this host publishes on a network's directory,
//! for as long as its owner runs it.
//!
//! [`ListingKeeper::run`] publishes the listing on the directory of the
//! network the keeper names by key, once the node runs that directory,
//! and publishes it again:
//!
//! - when the directory starts over ([`DirectoryEvent::Started`]);
//! - when the view drops the listing ([`ListingEvent::Unpublished`] of its
//!   id);
//! - after a failed publish;
//! - when a check finds the view without it: every
//!   [`KeeperConfig::listing_check`], and on [`ListingKeeper::check`] or a
//!   [`DirectoryEvent::Lagged`] of its network.
//!
//! Its publishes and unpublishes are at least
//! [`KeeperConfig::min_republish`] apart, so the reasons for a publish
//! that come meanwhile make one. Publishing is idempotent and answers the
//! listing's id; `run`'s `on_listed` gets each id other than the last.
//!
//! The directory names a listing by its [key](crate::listing::key) (a
//! chat's key, a media listing's swarm id) and keeps its content while it
//! lists it. So a keeper given changed content
//! ([`ListingKeeper::set_listing`]) unpublishes the listing, waits until
//! the view drops it (or a check interval passes), and publishes the new
//! content.
//!
//! The owner feeds the keeper the directory's events
//! ([`ListingKeeper::handle_event`], and [`ListingKeeper::check`] after its
//! subscription lagged) and ends it with [`ListingKeeper::stop`]: `run`
//! drops any call in flight, unpublishes the listing, waiting at most
//! [`KeeperConfig::unpublish_timeout`], and returns.
//!
//! # Bounds
//!
//! A keeper holds two listings (the one to keep, and the one last
//! published), the last listing id and one waker. The directory's events
//! leave it the size it is. A listing over
//! [`MAX_LISTING_BYTES`](crate::listing::MAX_LISTING_BYTES) fails every
//! publish.

use std::cell::{Cell, RefCell};
use std::future::{Future, poll_fn};
use std::pin::pin;
use std::rc::Rc;
use std::task::{Poll, Waker};

use futures_util::future::{Either, select};
use strims_base::time::{Duration, Time};
use strims_kv::Kv;
use strims_proto::network::v1::directory::Listing;
use strims_rpc::Timer;

use crate::control::{CallFailure, DirectoryControl};
use crate::events::{DirectoryEvent, ListingEvent};
use crate::host::Host;

/// How a [`ListingKeeper`] paces its calls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeeperConfig {
    /// The shortest wait between two of its publishes or unpublishes.
    pub min_republish: Duration,
    /// How often it checks that the view holds the listing, and the
    /// longest it waits for the view to drop a listing it unpublished for
    /// changed content.
    pub listing_check: Duration,
    /// The longest a stopped keeper waits for its listing's unpublish.
    pub unpublish_timeout: Duration,
}

impl Default for KeeperConfig {
    fn default() -> KeeperConfig {
        KeeperConfig {
            min_republish: Duration::from_secs(5),
            listing_check: Duration::from_secs(60),
            unpublish_timeout: Duration::from_secs(5),
        }
    }
}

/// What a [`ListingKeeper`] has done.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeeperMetrics {
    /// Publishes that succeeded.
    pub publishes: u64,
    /// Publishes, and unpublishes of changed content, that failed.
    pub errors: u64,
}

struct State {
    network_key: Vec<u8>,
    /// The listing to keep published.
    listing: RefCell<Listing>,
    /// The id the directory last answered a publish with, until `run`
    /// withdraws the listing.
    listing_id: Cell<Option<u64>>,
    /// The id of the network's directory on this node, as `run` last
    /// found it.
    network_id: Cell<Option<u64>>,
    /// `run` must publish the listing again.
    republish: Cell<bool>,
    /// `run` must check that the view holds the listing.
    check: Cell<bool>,
    /// Something `run` looks at changed since it last looked.
    poked: Cell<bool>,
    stopped: Cell<bool>,
    /// Wakes `run`.
    waker: RefCell<Option<Waker>>,
    metrics: Cell<KeeperMetrics>,
}

impl State {
    fn poke(&self) {
        self.poked.set(true);
        let w = self.waker.take();
        if let Some(w) = w {
            w.wake();
        }
    }

    fn register(&self, w: &Waker) {
        let prev = self.waker.replace(Some(w.clone()));
        drop(prev);
    }

    fn count(&self, f: impl FnOnce(&mut KeeperMetrics)) {
        let mut m = self.metrics.get();
        f(&mut m);
        self.metrics.set(m);
    }
}

/// What the directory holds of the listing, as `run` last saw it.
enum Published {
    /// Nothing, or nothing known.
    No,
    /// Listing `id`, as `listing`.
    Listed { id: u64, listing: Listing },
    /// Listing `id`, unpublished at `since` for changed content, until the
    /// view drops it.
    Retiring { id: u64, since: Time },
}

/// Keeps one listing published on a network's directory; see the
/// [module docs](self).
///
/// Clones share the keeper.
#[derive(Clone)]
pub struct ListingKeeper {
    state: Rc<State>,
}

impl ListingKeeper {
    /// A keeper of `listing` on the directory of the network whose key is
    /// `network_key`.
    pub fn new(network_key: Vec<u8>, listing: Listing) -> ListingKeeper {
        ListingKeeper {
            state: Rc::new(State {
                network_key,
                listing: RefCell::new(listing),
                listing_id: Cell::new(None),
                network_id: Cell::new(None),
                republish: Cell::new(false),
                check: Cell::new(false),
                poked: Cell::new(false),
                stopped: Cell::new(false),
                waker: RefCell::new(None),
                metrics: Cell::new(KeeperMetrics::default()),
            }),
        }
    }

    /// Has `run` keep `listing` published from now on. Changed content
    /// replaces the published listing once the view drops it; see the
    /// [module docs](self).
    pub fn set_listing(&self, listing: Listing) {
        let s = &self.state;
        let changed = *s.listing.borrow() != listing;
        if changed {
            let prev = s.listing.replace(listing);
            drop(prev);
            s.poke();
        }
    }

    /// The id the directory last answered a publish with, until `run`
    /// withdraws the listing.
    pub fn listing_id(&self) -> Option<u64> {
        self.state.listing_id.get()
    }

    /// What the keeper has done.
    pub fn metrics(&self) -> KeeperMetrics {
        self.state.metrics.get()
    }

    /// Takes a directory event: the keeper's network starting over, or its
    /// view dropping the listing, has `run` publish it again, and its view
    /// lagging has `run` check it.
    pub fn handle_event(&self, e: &DirectoryEvent) {
        let s = &self.state;
        let own = |id: u64| s.network_id.get() == Some(id);
        match e {
            DirectoryEvent::Started { network_key, .. } if *network_key == s.network_key => {
                s.republish.set(true);
                s.poke();
            }
            DirectoryEvent::Listing {
                network_id,
                event: ListingEvent::Unpublished(l),
            } if own(*network_id) && s.listing_id.get() == Some(l.id) => {
                s.republish.set(true);
                s.poke();
            }
            DirectoryEvent::Lagged { network_id } if own(*network_id) => self.check(),
            _ => {}
        }
    }

    /// Has `run` check that the view holds the listing, and publish it
    /// again if not: what an owner whose directory subscription lagged
    /// calls.
    pub fn check(&self) {
        self.state.check.set(true);
        self.state.poke();
    }

    /// Has `run` unpublish the listing and return.
    pub fn stop(&self) {
        self.state.stopped.set(true);
        self.state.poke();
    }

    /// Keeps the listing published on `directory` until [`stop`](Self::stop);
    /// see the [module docs](self). `on_listed` gets each listing id the
    /// directory answers a publish with that differs from the last. Run
    /// it once.
    pub async fn run<K: Kv, H: Host, T: Timer>(
        &self,
        directory: &DirectoryControl<K, H, T>,
        config: KeeperConfig,
        mut on_listed: impl FnMut(u64),
    ) {
        let s = &*self.state;
        let timer = &directory.shared.timer;
        let mut published = Published::No;
        let mut last: Option<Time> = None;
        let mut check_at = timer.now() + config.listing_check;
        while !s.stopped.get() {
            let now = timer.now();
            let Some(network_id) = directory.network_id(&s.network_key) else {
                s.network_id.set(None);
                if now >= check_at {
                    check_at = now + config.listing_check;
                }
                self.wait(timer, check_at).await;
                continue;
            };
            s.network_id.set(Some(network_id));
            let publish_at = last.map_or(now, |t| t + config.min_republish);
            match &published {
                Published::Retiring { id, since } => {
                    let give_up = *since + config.listing_check;
                    if !directory.view_holds(network_id, *id, None) || now >= give_up {
                        published = Published::No;
                        s.republish.set(true);
                    } else {
                        self.wait(timer, give_up).await;
                    }
                    continue;
                }
                Published::Listed { id, listing } if *listing != *s.listing.borrow() => {
                    if publish_at > now {
                        self.wait(timer, publish_at).await;
                        continue;
                    }
                    let id = *id;
                    last = Some(now);
                    let Some(res) = self
                        .until_stopped(directory.unpublish(&s.network_key, id))
                        .await
                    else {
                        break;
                    };
                    match res {
                        Ok(()) => published = Published::Retiring { id, since: now },
                        Err(e) => {
                            s.count(|m| m.errors += 1);
                            tracing::debug!(error = %e, "unpublishing a changed listing failed");
                            // a refusal means the directory holds no
                            // listing `id` of this host's
                            if let CallFailure::Refused(_) = e {
                                published = Published::Retiring { id, since: now };
                            }
                        }
                    }
                    continue;
                }
                _ => {}
            }
            let due = s.republish.get() || matches!(published, Published::No);
            if due && publish_at <= now {
                s.republish.set(false);
                last = Some(now);
                let listing = s.listing.borrow().clone();
                let Some(res) = self
                    .until_stopped(directory.publish(&s.network_key, listing.clone()))
                    .await
                else {
                    break;
                };
                match res {
                    Ok(id) => {
                        s.count(|m| m.publishes += 1);
                        let new = s.listing_id.replace(Some(id)) != Some(id);
                        published = Published::Listed { id, listing };
                        if new {
                            on_listed(id);
                        }
                    }
                    Err(e) => {
                        s.count(|m| m.errors += 1);
                        tracing::debug!(error = %e, "publishing a listing failed");
                        s.republish.set(true);
                    }
                }
                continue;
            }
            let check_due = now >= check_at;
            if check_due || s.check.get() {
                s.check.set(false);
                if check_due {
                    check_at = now + config.listing_check;
                }
                if let Published::Listed { id, listing } = &published
                    && !directory.view_holds(network_id, *id, Some(listing))
                {
                    s.republish.set(true);
                }
                continue;
            }
            let at = if due {
                publish_at.min(check_at)
            } else {
                check_at
            };
            self.wait(timer, at).await;
        }
        s.listing_id.set(None);
        if let Published::Listed { id, .. } = published {
            let deadline = timer.now() + config.unpublish_timeout;
            let unpublish = pin!(directory.unpublish(&s.network_key, id));
            let timeout = pin!(timer.sleep_until(deadline));
            match select(unpublish, timeout).await {
                Either::Left((Ok(()), _)) => {}
                Either::Left((Err(e), _)) => {
                    tracing::debug!(error = %e, "unpublishing a listing failed");
                }
                Either::Right(_) => tracing::debug!("unpublishing a listing timed out"),
            }
        }
    }

    /// Waits for the keeper's next poke, or until `at`.
    async fn wait<T: Timer>(&self, timer: &T, at: Time) {
        let s = &*self.state;
        let mut sleep = pin!(timer.sleep_until(at));
        poll_fn(|cx| {
            if s.poked.replace(false) || s.stopped.get() {
                return Poll::Ready(());
            }
            s.register(cx.waker());
            if sleep.as_mut().poll(cx).is_ready() {
                return Poll::Ready(());
            }
            Poll::Pending
        })
        .await;
    }

    /// `f`'s output, or `None` once the keeper stops first.
    async fn until_stopped<F: Future>(&self, f: F) -> Option<F::Output> {
        let s = &*self.state;
        let stopped = pin!(poll_fn(|cx| {
            if s.stopped.get() {
                return Poll::Ready(());
            }
            s.register(cx.waker());
            Poll::Pending
        }));
        match select(pin!(f), stopped).await {
            Either::Left((out, _)) => Some(out),
            Either::Right(((), _)) => None,
        }
    }
}

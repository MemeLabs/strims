// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! When to dial each bootstrap client: a synchronous state machine the
//! control drives with the store's clients, dial results and the node's
//! peers.
//!
//! A client is dialed once it's due. A dial's host is linked once the
//! node reports it as a peer; a dial that resolves first waits for that
//! report until `confirm_timeout`. A link that goes is redialed after the
//! relink grace or the client's backoff, whichever is longer, unless the
//! host links again first, as it does when a link is replaced. Failed
//! dials back off, doubling from `initial_backoff` up to `max_backoff`
//! with equal jitter, for as long as the client exists; a link that lasts
//! `stable_after` resets the backoff. A permanent failure, or a URL of
//! this node, parks the client until its URL changes.

use std::collections::{BTreeMap, BTreeSet};

use rand_core::Rng;
use strims_base::rng::DetRng;
use strims_base::time::{Duration, Time};
use strims_kademlia::ID;

use super::{Config, Failure};

/// A dial for the control to make, and report with
/// [`Schedule::dial_done`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Dial {
    pub(crate) client: u64,
    pub(crate) generation: u64,
    pub(crate) url: String,
}

/// Why a client isn't dialed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Park {
    /// Its URL is this node's.
    Own,
    /// Dialing it can't succeed.
    Permanent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Phase {
    Due,
    Dialing,
    /// Dialed `host`, which the node hasn't reported as a peer yet.
    Confirming {
        host: ID,
        until: Time,
    },
    Linked {
        host: ID,
        since: Time,
    },
    /// Waiting to dial again. `host` is the one whose link went, if any:
    /// it linking again makes the client linked.
    Waiting {
        at: Time,
        host: Option<ID>,
    },
    Parked(Park),
}

#[derive(Clone, Debug)]
struct Client {
    url: String,
    generation: u64,
    /// Failed dials, or short links, in a row.
    attempt: u32,
    phase: Phase,
}

/// The dial schedule of a profile's bootstrap clients.
pub(crate) struct Schedule {
    initial_backoff: Duration,
    max_backoff: Duration,
    relink_grace: Duration,
    stable_after: Duration,
    confirm_timeout: Duration,
    max_clients: usize,
    max_dials: usize,
    rng: DetRng,
    /// Bound: `max_clients`.
    clients: BTreeMap<u64, Client>,
    /// The node's peers. Bound: the node's peers.
    linked: BTreeSet<ID>,
    /// Dials handed out and not yet reported. At most `max_dials`.
    in_flight: usize,
    next_generation: u64,
}

impl Schedule {
    pub(crate) fn new(config: &Config, rng: DetRng) -> Schedule {
        Schedule {
            initial_backoff: config.initial_backoff,
            max_backoff: config.max_backoff,
            relink_grace: config.relink_grace,
            stable_after: config.stable_after,
            confirm_timeout: config.confirm_timeout,
            max_clients: config.max_clients,
            max_dials: config.max_dials,
            rng,
            clients: BTreeMap::new(),
            linked: BTreeSet::new(),
            in_flight: 0,
            next_generation: 0,
        }
    }

    /// Takes the store's clients, `(id, url)`, and this node's own link
    /// URIs. Clients beyond `max_clients`, by id, are left out; returns
    /// how many.
    pub(crate) fn set_clients(&mut self, clients: Vec<(u64, String)>, own: &[String]) -> usize {
        let mut wanted: BTreeMap<u64, String> = clients.into_iter().collect();
        let skipped = wanted.len().saturating_sub(self.max_clients);
        while wanted.len() > self.max_clients {
            wanted.pop_last();
        }
        self.clients.retain(|id, _| wanted.contains_key(id));
        for (id, url) in wanted {
            let own = is_own(&url, own);
            let generation = self.next_generation;
            let fresh = |url: String| Client {
                url,
                generation,
                attempt: 0,
                phase: if own {
                    Phase::Parked(Park::Own)
                } else {
                    Phase::Due
                },
            };
            match self.clients.get_mut(&id) {
                None => {
                    self.clients.insert(id, fresh(url));
                }
                Some(c) if c.url != url || own != (c.phase == Phase::Parked(Park::Own)) => {
                    *c = fresh(url);
                }
                Some(_) => continue,
            }
            self.next_generation += 1;
        }
        skipped
    }

    /// The next dial to make, if a client is due and fewer than
    /// `max_dials` are in flight.
    pub(crate) fn poll_dial(&mut self, now: Time) -> Option<Dial> {
        if self.in_flight >= self.max_dials {
            return None;
        }
        let (&id, c) = self.clients.iter_mut().find(|(_, c)| match c.phase {
            Phase::Due => true,
            Phase::Waiting { at, .. } => !at.after(now),
            _ => false,
        })?;
        c.phase = Phase::Dialing;
        self.in_flight += 1;
        Some(Dial {
            client: id,
            generation: c.generation,
            url: c.url.clone(),
        })
    }

    /// Takes a dial's result. Every dial from [`Schedule::poll_dial`] is
    /// reported once; a client that changed or went since ignores it.
    pub(crate) fn dial_done(
        &mut self,
        now: Time,
        client: u64,
        generation: u64,
        result: Result<ID, Failure>,
    ) {
        self.in_flight = self.in_flight.saturating_sub(1);
        let delay = self.backoff(client);
        let Some(c) = self.clients.get_mut(&client) else {
            return;
        };
        if c.generation != generation || c.phase != Phase::Dialing {
            return;
        }
        c.phase = match result {
            Ok(host) if self.linked.contains(&host) => Phase::Linked { host, since: now },
            Ok(host) => Phase::Confirming {
                host,
                until: now + self.confirm_timeout,
            },
            Err(Failure::Retry(_)) => {
                c.attempt = c.attempt.saturating_add(1);
                Phase::Waiting {
                    at: now + delay,
                    host: None,
                }
            }
            Err(Failure::Permanent(_)) => Phase::Parked(Park::Permanent),
        };
    }

    /// The node linked `host`.
    pub(crate) fn peer_added(&mut self, now: Time, host: ID) {
        self.linked.insert(host);
        for c in self.clients.values_mut() {
            match c.phase {
                Phase::Confirming { host: h, .. } | Phase::Waiting { host: Some(h), .. }
                    if h == host =>
                {
                    c.phase = Phase::Linked { host, since: now };
                }
                _ => {}
            }
        }
    }

    /// The node's link to `host` went.
    pub(crate) fn peer_removed(&mut self, now: Time, host: ID) {
        self.linked.remove(&host);
        let ids: Vec<u64> = self
            .clients
            .iter()
            .filter(|(_, c)| match c.phase {
                Phase::Linked { host: h, .. } | Phase::Confirming { host: h, .. } => h == host,
                _ => false,
            })
            .map(|(id, _)| *id)
            .collect();
        for id in ids {
            if let Some(c) = self.clients.get_mut(&id)
                && let Phase::Linked { since, .. } = c.phase
                && now - since >= self.stable_after
            {
                c.attempt = 0;
            }
            let delay = self.backoff(id).max(self.relink_grace);
            if let Some(c) = self.clients.get_mut(&id) {
                c.attempt = c.attempt.saturating_add(1);
                c.phase = Phase::Waiting {
                    at: now + delay,
                    host: Some(host),
                };
            }
        }
    }

    /// When [`Schedule::handle_timeout`] has work.
    pub(crate) fn poll_timeout(&self) -> Option<Time> {
        self.clients
            .values()
            .filter_map(|c| match c.phase {
                Phase::Waiting { at, .. } => Some(at),
                Phase::Confirming { until, .. } => Some(until),
                _ => None,
            })
            .min()
    }

    /// Makes waiting clients due, and retries dials whose host the node
    /// never reported.
    pub(crate) fn handle_timeout(&mut self, now: Time) {
        let ids: Vec<u64> = self.clients.keys().copied().collect();
        for id in ids {
            let delay = self.backoff(id);
            let Some(c) = self.clients.get_mut(&id) else {
                continue;
            };
            match c.phase {
                Phase::Waiting { at, .. } if !at.after(now) => c.phase = Phase::Due,
                Phase::Confirming { until, .. } if !until.after(now) => {
                    c.attempt = c.attempt.saturating_add(1);
                    c.phase = Phase::Waiting {
                        at: now + delay,
                        host: None,
                    };
                }
                _ => {}
            }
        }
    }

    /// A client's phase.
    #[cfg(test)]
    pub(crate) fn phase(&self, client: u64) -> Option<Phase> {
        self.clients.get(&client).map(|c| c.phase)
    }

    /// The wait before redialing `client` at its attempt: doubling from
    /// `initial_backoff` up to `max_backoff`, then between half and all of
    /// it.
    fn backoff(&mut self, client: u64) -> Duration {
        let attempt = self.clients.get(&client).map_or(0, |c| c.attempt);
        let full = (self.initial_backoff * (1i64 << attempt.min(30))).min(self.max_backoff);
        let half = full.as_nanos() / 2;
        let span = u64::try_from(half).unwrap_or(0).saturating_add(1);
        let extra = i64::try_from(self.rng.next_u64() % span).unwrap_or(0);
        Duration::from_nanos(half.saturating_add(extra))
    }
}

/// Whether `url`, without its fragment, is one of `own`.
fn is_own(url: &str, own: &[String]) -> bool {
    let url = url.split_once('#').map_or(url, |(u, _)| u);
    own.iter().any(|o| o == url)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use strims_base::rng::det_rng;

    use super::*;

    const T0: Time = Time::from_unix_millis(1_700_000_000_000);
    const OWN: &str = "ws://127.0.0.1:1/own";

    fn host(n: u8) -> ID {
        ID::from_bytes(&[n; 32])
    }

    fn schedule() -> Schedule {
        Schedule::new(&Config::default(), det_rng(1))
    }

    fn clients(urls: &[(u64, &str)]) -> Vec<(u64, String)> {
        urls.iter().map(|(id, u)| (*id, (*u).to_owned())).collect()
    }

    fn own() -> Vec<String> {
        vec![OWN.to_owned()]
    }

    /// Fails `client`'s dials until its wait passes `limit`; returns each
    /// wait.
    fn failing_waits(s: &mut Schedule, client: u64, n: usize) -> Vec<Duration> {
        let mut now = T0;
        let mut waits = Vec::new();
        for _ in 0..n {
            let d = s.poll_dial(now).unwrap();
            assert_eq!(d.client, client);
            s.dial_done(now, d.client, d.generation, Err(Failure::Retry("x".into())));
            let at = s.poll_timeout().unwrap();
            waits.push(at - now);
            now = at;
            s.handle_timeout(now);
        }
        waits
    }

    #[test]
    fn failed_dials_back_off_with_jitter_up_to_the_cap() {
        let mut s = schedule();
        s.set_clients(clients(&[(1, "ws://a/x")]), &own());
        let waits = failing_waits(&mut s, 1, 14);
        let c = Config::default();
        for (i, w) in waits.iter().enumerate() {
            let full = (c.initial_backoff * (1i64 << i)).min(c.max_backoff);
            assert!(*w >= full / 2 && *w <= full, "attempt {i}: {w} of {full}");
        }
        assert!(waits[13] >= c.max_backoff / 2);
    }

    #[test]
    fn the_same_seed_waits_the_same() {
        let mut a = schedule();
        let mut b = schedule();
        a.set_clients(clients(&[(1, "ws://a/x")]), &own());
        b.set_clients(clients(&[(1, "ws://a/x")]), &own());
        assert_eq!(failing_waits(&mut a, 1, 8), failing_waits(&mut b, 1, 8));
    }

    #[test]
    fn a_replaced_link_is_not_redialed() {
        let mut s = schedule();
        s.set_clients(clients(&[(1, "ws://a/x")]), &own());
        let d = s.poll_dial(T0).unwrap();
        s.peer_added(T0, host(2));
        s.dial_done(T0, d.client, d.generation, Ok(host(2)));
        assert!(matches!(s.phase(1), Some(Phase::Linked { .. })));
        let t = T0 + Duration::from_secs(5);
        s.peer_removed(t, host(2));
        s.peer_added(t, host(2));
        assert!(matches!(s.phase(1), Some(Phase::Linked { .. })));
        assert_eq!(s.poll_dial(t + Duration::from_secs(600)), None);
    }

    #[test]
    fn a_lost_link_is_redialed_after_the_grace() {
        let mut s = schedule();
        s.set_clients(clients(&[(1, "ws://a/x")]), &own());
        let d = s.poll_dial(T0).unwrap();
        s.dial_done(T0, d.client, d.generation, Ok(host(2)));
        assert!(matches!(s.phase(1), Some(Phase::Confirming { .. })));
        s.peer_added(T0, host(2));
        let t = T0 + Duration::from_secs(120);
        s.peer_removed(t, host(2));
        let at = s.poll_timeout().unwrap();
        assert!(at - t >= Config::default().relink_grace);
        assert_eq!(s.poll_dial(t), None);
        s.handle_timeout(at);
        assert_eq!(s.poll_dial(at).unwrap().client, 1);
    }

    #[test]
    fn a_flapping_link_backs_off() {
        let mut s = schedule();
        s.set_clients(clients(&[(1, "ws://a/x")]), &own());
        let mut now = T0;
        let mut waits = Vec::new();
        for _ in 0..6 {
            let d = s.poll_dial(now).unwrap();
            s.peer_added(now, host(2));
            s.dial_done(now, d.client, d.generation, Ok(host(2)));
            now += Duration::SECOND;
            s.peer_removed(now, host(2));
            let at = s.poll_timeout().unwrap();
            waits.push(at - now);
            now = at;
            s.handle_timeout(now);
        }
        assert!(waits[5] >= Duration::from_secs(16), "{waits:?}");

        // A link that lasts resets the backoff.
        let d = s.poll_dial(now).unwrap();
        s.peer_added(now, host(2));
        s.dial_done(now, d.client, d.generation, Ok(host(2)));
        now += Config::default().stable_after;
        s.peer_removed(now, host(2));
        let wait = s.poll_timeout().unwrap() - now;
        assert!(wait <= Config::default().relink_grace, "{wait}");
    }

    #[test]
    fn permanent_failures_park_until_the_url_changes() {
        let mut s = schedule();
        s.set_clients(clients(&[(1, "wss://a/x")]), &own());
        let d = s.poll_dial(T0).unwrap();
        s.dial_done(
            T0,
            d.client,
            d.generation,
            Err(Failure::Permanent("x".into())),
        );
        assert_eq!(s.phase(1), Some(Phase::Parked(Park::Permanent)));
        assert_eq!(s.poll_timeout(), None);
        assert_eq!(s.poll_dial(T0 + Duration::HOUR), None);
        s.set_clients(clients(&[(1, "wss://a/x")]), &own());
        assert_eq!(s.phase(1), Some(Phase::Parked(Park::Permanent)));
        s.set_clients(clients(&[(1, "ws://a/y")]), &own());
        assert_eq!(s.poll_dial(T0).unwrap().url, "ws://a/y");
    }

    #[test]
    fn a_changed_url_drops_the_old_dials_result() {
        let mut s = schedule();
        s.set_clients(clients(&[(1, "ws://a/x")]), &own());
        let old = s.poll_dial(T0).unwrap();
        s.set_clients(clients(&[(1, "ws://b/x")]), &own());
        let new = s.poll_dial(T0).unwrap();
        assert_eq!(new.url, "ws://b/x");
        s.dial_done(T0, old.client, old.generation, Ok(host(2)));
        assert_eq!(s.phase(1), Some(Phase::Dialing));
        s.dial_done(T0, new.client, new.generation, Ok(host(3)));
        assert!(matches!(
            s.phase(1),
            Some(Phase::Confirming { host: h, .. }) if h == host(3)
        ));
    }

    #[test]
    fn a_deleted_client_ignores_its_dial() {
        let mut s = schedule();
        s.set_clients(clients(&[(1, "ws://a/x")]), &own());
        let d = s.poll_dial(T0).unwrap();
        s.set_clients(Vec::new(), &own());
        s.dial_done(T0, d.client, d.generation, Err(Failure::Retry("x".into())));
        assert_eq!(s.phase(1), None);
        assert_eq!(s.poll_timeout(), None);
        assert_eq!(s.in_flight, 0);
    }

    #[test]
    fn clients_and_dials_are_bounded() {
        let c = Config::default();
        let mut s = schedule();
        let many: Vec<(u64, String)> = (1..=c.max_clients as u64 + 3)
            .map(|i| (i, format!("ws://a/{i}")))
            .collect();
        assert_eq!(s.set_clients(many, &own()), 3);
        assert_eq!(s.clients.len(), c.max_clients);
        assert!(s.phase(c.max_clients as u64 + 1).is_none());
        let mut dials = Vec::new();
        while let Some(d) = s.poll_dial(T0) {
            dials.push(d);
        }
        assert_eq!(dials.len(), c.max_dials);
        let d = &dials[0];
        s.dial_done(T0, d.client, d.generation, Err(Failure::Retry("x".into())));
        assert!(s.poll_dial(T0).is_some());
    }

    #[test]
    fn this_nodes_own_url_is_never_dialed() {
        let mut s = schedule();
        let mine = format!("{OWN}#insecure");
        s.set_clients(clients(&[(1, &mine), (2, "ws://a/x")]), &own());
        assert_eq!(s.phase(1), Some(Phase::Parked(Park::Own)));
        assert_eq!(s.poll_dial(T0).unwrap().client, 2);
        assert_eq!(s.poll_dial(T0), None);
        // Once the node's URIs change, the client is dialed.
        s.set_clients(clients(&[(1, &mine), (2, "ws://a/x")]), &[]);
        assert_eq!(s.poll_dial(T0).unwrap().client, 1);
    }

    #[test]
    fn a_dial_never_reported_as_a_peer_is_retried() {
        let mut s = schedule();
        s.set_clients(clients(&[(1, "ws://a/x")]), &own());
        let d = s.poll_dial(T0).unwrap();
        s.dial_done(T0, d.client, d.generation, Ok(host(2)));
        let until = s.poll_timeout().unwrap();
        assert_eq!(until - T0, Config::default().confirm_timeout);
        s.handle_timeout(until);
        assert!(matches!(
            s.phase(1),
            Some(Phase::Waiting { host: None, .. })
        ));
        let at = s.poll_timeout().unwrap();
        s.handle_timeout(at);
        assert_eq!(s.poll_dial(at).unwrap().client, 1);
    }

    #[test]
    fn a_link_that_goes_before_its_dial_resolves_is_retried() {
        let mut s = schedule();
        s.set_clients(clients(&[(1, "ws://a/x")]), &own());
        let d = s.poll_dial(T0).unwrap();
        s.peer_added(T0, host(2));
        s.peer_removed(T0, host(2));
        s.dial_done(T0, d.client, d.generation, Ok(host(2)));
        assert!(matches!(s.phase(1), Some(Phase::Confirming { .. })));
    }
}

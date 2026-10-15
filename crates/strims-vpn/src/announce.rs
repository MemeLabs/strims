// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Link announcements on port [`LINK_ANNOUNCE_PORT`], for
//! [`Routing::Learned`](crate::Routing::Learned).
//!
//! A host tells its direct links which hosts it is linked to, and passes on
//! the lists its own links announced to it. Every host so knows the links
//! within three hops of it before any traffic has passed: a message to a
//! destination nobody has heard from travels blind for at most two hops
//! before it reaches a host that knows a path.
//!
//! - **When:** [`ANNOUNCE_DELAY`] after this host's links or a neighbour's
//!   announced list change (so a burst of changes sends one announcement),
//!   and every [`ANNOUNCE_INTERVAL`], well inside the learned-link ttl.
//! - **To links that announce:** a broadcast flagged [`M_NORELAY`].
//! - **To the other links**, with [`Config::announce_reach`] 2 (the
//!   default): a broadcast without `M_NORELAY` whose trailer already holds
//!   [`MAX_MESSAGE_HOPS`] - 1 entries, all written and signed by this host.
//!   The padding keeps it to at most one hop beyond the first relay, the
//!   one link closest to this host's id, so a relay forwards at most one
//!   frame per announcement it gets. Hosts on the far side of a link that
//!   doesn't announce so learn this host's links too. Learned hosts never
//!   relay an announcement.
//! - **Body:**
//!
//!   ```text
//!   version (1) | flags (1; bit 0: the own list is complete)
//!   count (1) | count host ids          this host's routed links, in id order
//!   then, until the end, per link:      the list that link last announced
//!     host id | count (1) | count host ids
//!   ```
//!
//!   At most [`MAX_ANNOUNCED_LINKS`] own links and [`MAX_ANNOUNCED_IDS`]
//!   ids in all (8 KiB). A complete own list replaces the links learned
//!   for the sender; everything else only adds learned links. A host passes
//!   on only lists it received first-hand (in one hop) within the
//!   learned-link ttl, so stale links can't circulate.
//! - **Trust:** an announcement whose trailer holds only its sender came in
//!   one hop, from the vnic peer the frame came from. One relayed once
//!   names its sender in the first trailer entry, which only
//!   [`VerifyMode::Audit`](crate::VerifyMode)/`Enforce` check. A peer can
//!   claim links it doesn't have, as it can forge trailer entries.
//!
//! [`Config::announce_reach`]: crate::Config::announce_reach

use std::collections::BTreeMap;

use strims_base::time::{Duration, Time};
use strims_kademlia::{ID, ID_LENGTH};

use strims_vnic::PeerKey;

use crate::host::{Host, Inbound};
use crate::message::{M_BROADCAST, M_NORELAY, MAX_MESSAGE_HOPS, append_trailer_entry};
use crate::{Error, NetworkId};

/// The port. It must stay unassigned in the v2 protocol (services use
/// 10–12 and 14, apps reserve ports from 1000 up), so that hosts that don't
/// announce have no handler on it.
pub const LINK_ANNOUNCE_PORT: u16 = 20;
/// How often a host repeats its announcement.
pub const ANNOUNCE_INTERVAL: Duration = Duration::from_secs(2 * 60);
/// How long a host waits after a change before announcing.
pub const ANNOUNCE_DELAY: Duration = Duration::SECOND;
/// The most own links one announcement lists.
pub const MAX_ANNOUNCED_LINKS: usize = 64;
/// The most host ids one announcement carries (8 KiB).
pub const MAX_ANNOUNCED_IDS: usize = 256;

const VERSION: u8 = 1;
const COMPLETE: u8 = 1;

/// The lists this host's links announced to it, with when they last did.
/// Bounded by the links, each list by [`MAX_ANNOUNCED_LINKS`].
#[derive(Debug, Default)]
pub(crate) struct NeighbourLists {
    lists: BTreeMap<ID, (Time, Vec<ID>)>,
}

impl NeighbourLists {
    /// Stores `id`'s list; returns whether it changed.
    fn set(&mut self, now: Time, id: ID, links: &[ID]) -> bool {
        let (seen, list) = self.lists.entry(id).or_default();
        *seen = now;
        if list.as_slice() == links {
            return false;
        }
        list.clear();
        list.extend_from_slice(links);
        true
    }

    pub(crate) fn remove(&mut self, id: &ID) {
        self.lists.remove(id);
    }

    /// Whether `id` announced its links since `since`, so routes with
    /// [`Routing::Learned`](crate::Routing::Learned).
    fn announced_since(&self, id: &ID, since: Time) -> bool {
        self.lists
            .get(id)
            .is_some_and(|(seen, _)| !seen.before(since))
    }
}

/// Encodes this host's announcement: `own` links (sorted), then the lists
/// in `neighbours` of those links received since `fresh`, within the size
/// bounds.
fn encode(out: &mut Vec<u8>, own: &[ID], neighbours: &NeighbourLists, fresh: Time) {
    let complete = own.len() <= MAX_ANNOUNCED_LINKS;
    let own = own.get(..MAX_ANNOUNCED_LINKS).unwrap_or(own);
    out.push(VERSION);
    out.push(if complete { COMPLETE } else { 0 });
    // invariant: at most MAX_ANNOUNCED_LINKS < 256.
    out.push(own.len() as u8);
    for id in own {
        out.extend_from_slice(&id.to_bytes());
    }
    let mut budget = MAX_ANNOUNCED_IDS.saturating_sub(own.len());
    for id in own {
        let Some((seen, list)) = neighbours.lists.get(id) else {
            continue;
        };
        if seen.before(fresh) || list.is_empty() || budget < 1 + list.len() {
            continue;
        }
        budget -= 1 + list.len();
        out.extend_from_slice(&id.to_bytes());
        // invariant: lists are at most MAX_ANNOUNCED_LINKS long.
        out.push(list.len() as u8);
        for m in list {
            out.extend_from_slice(&m.to_bytes());
        }
    }
}

/// A decoded announcement.
struct Announcement<'a> {
    complete: bool,
    own: &'a [[u8; ID_LENGTH]],
    /// The rest: neighbour lists.
    rest: &'a [u8],
}

fn decode(b: &[u8]) -> Result<Announcement<'_>, Error> {
    let bad = || Error::Decode("link announcement".to_owned());
    let (&[version, flags, count], rest) = b.split_first_chunk::<3>().ok_or_else(bad)?;
    if version != VERSION {
        return Err(bad());
    }
    let n = usize::from(count) * ID_LENGTH;
    let own = rest.get(..n).ok_or_else(bad)?.as_chunks::<ID_LENGTH>().0;
    Ok(Announcement {
        complete: flags & COMPLETE != 0,
        own,
        rest: rest.get(n..).unwrap_or_default(),
    })
}

/// The next neighbour list in `rest`: (neighbour, its links, the rest).
#[allow(clippy::type_complexity)]
fn next_list(rest: &[u8]) -> Option<(ID, &[[u8; ID_LENGTH]], &[u8])> {
    let (id, rest) = rest.split_first_chunk::<ID_LENGTH>()?;
    let (&count, rest) = rest.split_first()?;
    let n = usize::from(count) * ID_LENGTH;
    let links = rest.get(..n)?.as_chunks::<ID_LENGTH>().0;
    Some((ID::from_bytes(id), links, rest.get(n..)?))
}

impl<H: Copy> Host<H> {
    /// Sends this host's announcement to every routed link of `net`: in
    /// one hop to the links that announce themselves, and with a reach of
    /// 2 ([`Config::announce_reach`]) to the others.
    ///
    /// [`Config::announce_reach`]: crate::Config::announce_reach
    pub(crate) fn link_announce(&mut self, now: Time, net: NetworkId) -> Result<(), Error> {
        let n = self.networks.get(net).ok_or(Error::NetworkNotFound)?;
        let mut own: Vec<ID> = n.routes.iter().map(|r| r.id).collect();
        if own.is_empty() {
            return Ok(());
        }
        own.sort();
        let fresh = now - self.config.topology_ttl;
        let reach = self.config.announce_reach.clamp(1, 2);
        let (mut near, mut far) = (Vec::new(), Vec::new());
        for r in n.routes.iter() {
            if reach == 1 || n.neighbour_lists.announced_since(&r.id, fresh) {
                near.push(r.peer);
            } else {
                far.push(r.peer);
            }
        }
        let mut body = self.pool.take();
        encode(&mut body, &own, &n.neighbour_lists, fresh);
        let mut res = Ok(());
        for (reach, peers) in [(1, &near), (reach, &far)] {
            if peers.is_empty() {
                continue;
            }
            match self.send_announcement(now, net, &body, reach, peers) {
                Ok(()) => self.metrics.link_announcements_sent += 1,
                Err(e) => res = Err(e),
            }
        }
        self.pool.put(body);
        res
    }

    /// Queues an announcement on `peers`. With a reach of 1 it is flagged
    /// [`M_NORELAY`]. With a reach of 2 it isn't, and this host writes
    /// [`MAX_MESSAGE_HOPS`] - 1 trailer entries: arriving with four, it
    /// may be relayed to `MAX_MESSAGE_REPLICAS - 4 = 1` link, where the
    /// trailer is full.
    fn send_announcement(
        &mut self,
        now: Time,
        net: NetworkId,
        body: &[u8],
        reach: usize,
        peers: &[PeerKey],
    ) -> Result<(), Error> {
        let (flags, entries) = match reach {
            1 => (M_BROADCAST | M_NORELAY, 1),
            _ => (M_BROADCAST, MAX_MESSAGE_HOPS + 1 - reach),
        };
        let id = self.id;
        let header = self.new_header(
            net,
            id,
            LINK_ANNOUNCE_PORT,
            LINK_ANNOUNCE_PORT,
            body.len(),
            flags,
            entries,
        )?;
        let mut wire = self.encode_new(now, &header, body)?;
        for _ in 1..entries {
            append_trailer_entry(&mut wire, now, &id, &self.signer);
        }
        self.enqueue(now, net, wire, peers);
        Ok(())
    }

    /// Port [`LINK_ANNOUNCE_PORT`]: learn a neighbour's links and the
    /// links of its neighbours.
    pub(crate) fn link_announce_handle(
        &mut self,
        now: Time,
        net: NetworkId,
        msg: &Inbound<'_>,
    ) -> Result<(), Error> {
        // Hops past the sender's own trailer entries: 1 from the
        // neighbour itself (every entry is the frame's sender), 2 through
        // one relay.
        let distance = msg.hops.saturating_sub(msg.lead) + 1;
        if msg.lead == 0 || distance > 2 {
            return Ok(());
        }
        let a = decode(msg.body)?;
        let mut own = [ID::MIN; MAX_ANNOUNCED_LINKS];
        let mut len = 0;
        for (slot, id) in own.iter_mut().zip(a.own) {
            *slot = ID::from_bytes(id);
            len += 1;
        }
        let own = own.get(..len).unwrap_or_default();
        let n = self.networks.get_mut(net).ok_or(Error::NetworkNotFound)?;
        n.topology.set_links(now, msg.src, own, a.complete);
        // A changed list goes into this host's next announcement; only
        // first-hand lists are passed on.
        if distance == 1
            && n.by_host.contains_key(&msg.src)
            && n.neighbour_lists.set(now, msg.src, own)
        {
            n.links_changed = true;
        }
        let mut rest = a.rest;
        let mut ids = 0;
        while let Some((id, links, r)) = next_list(rest) {
            rest = r;
            ids += 1 + links.len();
            if ids > MAX_ANNOUNCED_IDS {
                break;
            }
            // Only lists of the sender's own links.
            if !own.contains(&id) {
                continue;
            }
            for m in links {
                n.topology.learn_edge(now, id, ID::from_bytes(m));
            }
        }
        self.metrics.link_announcements_received += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;

    fn id(i: u64) -> ID {
        ID([i, 0, 0, 0])
    }

    #[test]
    fn codec() {
        let own = [id(1), id(2)];
        let t = Time::from_unix(1000, 0);
        let mut nl = NeighbourLists::default();
        assert!(nl.set(t, id(1), &[id(5), id(6)]));
        assert!(!nl.set(t, id(1), &[id(5), id(6)]));
        nl.set(t, id(9), &[id(7)]);
        // Stale lists aren't passed on.
        let mut b = Vec::new();
        encode(&mut b, &own, &nl, t + Duration::SECOND);
        assert_eq!(b.len(), 3 + 2 * ID_LENGTH);
        let mut b = Vec::new();
        encode(&mut b, &own, &nl, t);
        assert_eq!(b.len(), 3 + 2 * ID_LENGTH + ID_LENGTH + 1 + 2 * ID_LENGTH);
        let a = decode(&b).unwrap();
        assert!(a.complete);
        let got: Vec<ID> = a.own.iter().map(ID::from_bytes).collect();
        assert_eq!(got, own);
        // Only lists of own links are passed on.
        let (n, links, rest) = next_list(a.rest).unwrap();
        assert_eq!(n, id(1));
        let links: Vec<ID> = links.iter().map(ID::from_bytes).collect();
        assert_eq!(links, [id(5), id(6)]);
        assert!(rest.is_empty());
        // Truncated, wrong version, empty.
        assert!(decode(&b[..3 + ID_LENGTH]).is_err());
        assert!(next_list(&a.rest[..a.rest.len() - 1]).is_none());
        b[0] = 2;
        assert!(decode(&b).is_err());
        assert!(decode(&[]).is_err());
    }

    #[test]
    fn size_is_bounded() {
        let own: Vec<ID> = (0..100).map(id).collect();
        let t = Time::from_unix(1000, 0);
        let mut nl = NeighbourLists::default();
        let list: Vec<ID> = (1000..1064).map(id).collect();
        for i in 0..100 {
            nl.set(t, id(i), &list);
        }
        let mut b = Vec::new();
        encode(&mut b, &own, &nl, t);
        let a = decode(&b).unwrap();
        assert!(!a.complete);
        assert_eq!(a.own.len(), MAX_ANNOUNCED_LINKS);
        let mut ids = a.own.len();
        let mut rest = a.rest;
        while let Some((_, links, r)) = next_list(rest) {
            ids += 1 + links.len();
            rest = r;
        }
        assert!(ids <= MAX_ANNOUNCED_IDS);
        assert!(b.len() <= 3 + MAX_ANNOUNCED_IDS * ID_LENGTH + MAX_ANNOUNCED_LINKS);
    }
}

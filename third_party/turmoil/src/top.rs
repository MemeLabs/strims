use crate::envelope::{Envelope, Protocol};
use crate::hash::IndexMap;
use crate::host::Host;
use crate::rt::Rt;
use crate::{config, TRACING_TARGET};

use rand::{Rng, RngCore};
use rand_distr::{Distribution, Exp};
use std::collections::{BTreeSet, VecDeque};
use std::io::{Error, ErrorKind, Result};
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tokio::time::Instant;

/// Describes the network topology.
pub(crate) struct Topology {
    config: config::Link,

    /// Specific configuration overrides between specific hosts.
    links: IndexMap<Pair, Link>,

    /// The positions in `links` of the links whose `sent` queue holds a
    /// message: the links a tick visits.
    busy: BTreeSet<usize>,

    /// Per host, the positions in `links` of its links holding messages
    /// ready for it: the links a delivery to it visits, in `links` order.
    ready: IndexMap<IpAddr, BTreeSet<usize>>,

    /// The network's time: `rt`'s as of the last [`Topology::tick_by`],
    /// which every link shares.
    now: Instant,

    /// We don't use a Rt for async. Right now, we just use it to tick time
    /// forward in the same way we do it elsewhere. We'd like to represent
    /// network state with async in the future.
    rt: Rt<'static>,
}

/// This type is used as the key in the [`Topology::links`] map. See [`new`]
/// which orders the addrs, such that this type uniquely identifies the link
/// between two hosts on the network.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct Pair(IpAddr, IpAddr);

impl Pair {
    fn new(a: IpAddr, b: IpAddr) -> Pair {
        assert_ne!(a, b);

        if a < b {
            Pair(a, b)
        } else {
            Pair(b, a)
        }
    }
}

/// An iterator for the network topology, providing access to all active links
/// in the simulated network.
pub struct LinksIter<'a> {
    iter: std::iter::Enumerate<indexmap::map::IterMut<'a, Pair, Link>>,
    now: Instant,
}

/// An iterator over the links whose in-flight queue holds a message, in the
/// order [`LinksIter`] yields links.
pub struct BusyLinksIter<'a> {
    iter: indexmap::map::IterMut<'a, Pair, Link>,
    /// The position of `iter`'s next link.
    next: usize,
    busy: std::collections::btree_set::Iter<'a, usize>,
    now: Instant,
}

/// An iterator for the link, providing access to sent messages that have not
/// yet been delivered.
pub struct LinkIter<'a> {
    a: IpAddr,
    b: IpAddr,
    index: usize,
    now: Instant,
    iter: std::collections::vec_deque::IterMut<'a, Sent>,
}

impl LinkIter<'_> {
    /// The [`IpAddr`] pair for the link. Always ordered to uniquely identify
    /// the link.
    pub fn pair(&self) -> (IpAddr, IpAddr) {
        (self.a, self.b)
    }

    /// The link's position in the order [`LinksIter`] yields links: the
    /// order they were registered in, which never changes.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Schedule all messages on the link for delivery the next time the
    /// simulation steps, consuming the iterator.
    pub fn deliver_all(self) {
        for sent in self {
            sent.deliver();
        }
    }
}

/// Provides a reference to a message that is currently inflight on the network
/// from one host to another.
pub struct SentRef<'a> {
    src: SocketAddr,
    dst: SocketAddr,
    now: Instant,
    sent: &'a mut Sent,
}

impl SentRef<'_> {
    /// The (src, dst) [`SocketAddr`] pair for the message.
    pub fn pair(&self) -> (SocketAddr, SocketAddr) {
        (self.src, self.dst)
    }

    /// The message [`Protocol`].
    pub fn protocol(&self) -> &Protocol {
        &self.sent.protocol
    }

    /// Schedule the message for delivery the next time the simulation steps,
    /// consuming the item.
    pub fn deliver(self) {
        self.sent.deliver(self.now);
    }
}

impl<'a> Iterator for LinksIter<'a> {
    type Item = LinkIter<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (index, (pair, link)) = self.iter.next()?;

        Some(LinkIter {
            a: pair.0,
            b: pair.1,
            index,
            now: self.now,
            iter: link.sent.iter_mut(),
        })
    }
}

impl<'a> Iterator for BusyLinksIter<'a> {
    type Item = LinkIter<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let index = *self.busy.next()?;
        // `busy` ascends, so the link is at or after `iter`'s next.
        let (pair, link) = self.iter.nth(index.checked_sub(self.next)?)?;
        self.next = index + 1;

        Some(LinkIter {
            a: pair.0,
            b: pair.1,
            index,
            now: self.now,
            iter: link.sent.iter_mut(),
        })
    }
}

impl<'a> Iterator for LinkIter<'a> {
    type Item = SentRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let sent = self.iter.next()?;

        Some(SentRef {
            src: sent.src,
            dst: sent.dst,
            now: self.now,
            sent,
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl ExactSizeIterator for LinkIter<'_> {}

/// A two-way link between two hosts on the network.
struct Link {
    /// The state of the link from node 'a' to node 'b'.
    /// Of the two nodes attached to a link, 'a' is the
    /// one whose ip address sorts smallest.
    state_a_b: State,
    /// The state of the link from node 'b' to node 'a'.
    /// Of the two nodes attached to a link, 'b' is the
    /// one whose ip address sorts greatest.
    state_b_a: State,

    /// Optional, per-link configuration.
    config: config::Link,

    /// Sent messages that are either scheduled for delivery in the future
    /// or are on hold.
    sent: VecDeque<Sent>,

    /// Messages that are ready to be delivered.
    deliverable: IndexMap<IpAddr, VecDeque<Envelope>>,
}

/// States that a link between two nodes can be in.
#[derive(Clone, Copy)]
enum State {
    /// The link is healthy.
    Healthy,

    /// The link was explicitly partitioned.
    ExplicitPartition,

    /// The link was randomly partitioned.
    RandPartition,

    /// Messages are being held indefinitely.
    Hold,
}

impl Topology {
    pub(crate) fn new(config: config::Link) -> Topology {
        let rt = Rt::no_software();
        Topology {
            config,
            links: IndexMap::default(),
            busy: BTreeSet::new(),
            ready: IndexMap::default(),
            now: rt.now(),
            rt,
        }
    }

    /// Register a link between two hosts
    pub(crate) fn register(&mut self, a: IpAddr, b: IpAddr) {
        let pair = Pair::new(a, b);
        assert!(self.links.insert(pair, Link::new()).is_none());
    }

    pub(crate) fn set_max_message_latency(&mut self, value: Duration) {
        self.config.latency_mut().max_message_latency = value;
    }

    pub(crate) fn set_link_message_latency(&mut self, a: IpAddr, b: IpAddr, value: Duration) {
        let pair = Pair::new(a, b);
        let latency = self
            .links
            .get_mut(&pair)
            .unwrap_or_else(|| panic!("unable to find link between {pair:?}"))
            .latency(self.config.latency());
        latency.min_message_latency = value;
        latency.max_message_latency = value;
    }

    pub(crate) fn set_link_max_message_latency(&mut self, a: IpAddr, b: IpAddr, value: Duration) {
        let pair = Pair::new(a, b);
        self.links
            .get_mut(&pair)
            .unwrap_or_else(|| panic!("unable to find link between {pair:?}"))
            .latency(self.config.latency())
            .max_message_latency = value;
    }

    pub(crate) fn set_message_latency_curve(&mut self, value: f64) {
        self.config.latency_mut().latency_distribution = Exp::new(value).unwrap();
    }

    pub(crate) fn set_fail_rate(&mut self, value: f64) {
        self.config.message_loss_mut().fail_rate = value;
    }

    pub(crate) fn set_link_fail_rate(&mut self, a: IpAddr, b: IpAddr, value: f64) {
        let pair = Pair::new(a, b);
        self.links
            .get_mut(&pair)
            .unwrap_or_else(|| panic!("unable to find link between {pair:?}"))
            .message_loss(self.config.message_loss())
            .fail_rate = value;
    }

    // Send a `message` from `src` to `dst`. This method returns immediately,
    // and message delivery happens at a later time (or never, if the link is
    // broken).
    pub(crate) fn enqueue_message(
        &mut self,
        rand: &mut dyn RngCore,
        src: SocketAddr,
        dst: SocketAddr,
        message: Protocol,
    ) -> Result<()> {
        let Some((index, pair, link)) = self.links.get_full_mut(&Pair::new(src.ip(), dst.ip()))
        else {
            return Err(Error::new(
                ErrorKind::ConnectionRefused,
                "Connection refused",
            ));
        };
        let result = link.enqueue_message(&self.config, rand, self.now, src, dst, message);
        track(&mut self.busy, &mut self.ready, index, pair, link);
        result
    }

    // Move messages from any network links to the `dst` host: those of its
    // links that hold messages ready for it, in `links` order. A message the
    // host refuses goes back on its link, so it reaches the other end no
    // sooner than the next step.
    pub(crate) fn deliver_messages(&mut self, rand: &mut dyn RngCore, dst: &mut Host) {
        let Some(ready) = self.ready.get_mut(&dst.addr) else {
            return;
        };
        for index in std::mem::take(ready) {
            let Some((pair, link)) = self.links.get_index_mut(index) else {
                continue;
            };
            link.deliver_messages(&self.config, rand, self.now, dst);
            track(&mut self.busy, &mut self.ready, index, pair, link);
        }
    }

    pub(crate) fn hold(&mut self, a: IpAddr, b: IpAddr) {
        self.with_link(&Pair::new(a, b), |link, _| link.hold());
    }

    pub(crate) fn release(&mut self, a: IpAddr, b: IpAddr) {
        self.with_link(&Pair::new(a, b), |link, now| link.release(now));
    }

    pub(crate) fn partition(&mut self, a: IpAddr, b: IpAddr) {
        self.with_link(&Pair::new(a, b), |link, _| link.explicit_partition());
    }

    pub(crate) fn partition_oneway(&mut self, a: IpAddr, b: IpAddr) {
        self.with_link(&Pair::new(a, b), |link, _| link.partition_oneway(a, b));
    }

    pub(crate) fn repair(&mut self, a: IpAddr, b: IpAddr) {
        self.with_link(&Pair::new(a, b), |link, _| link.explicit_repair());
    }

    pub(crate) fn repair_oneway(&mut self, a: IpAddr, b: IpAddr) {
        self.with_link(&Pair::new(a, b), |link, _| link.repair_oneway(a, b));
    }

    // Moves the network's time on, and on every link with messages in
    // flight those whose time has come to the messages ready for their
    // receivers. A link without messages in flight has none to move.
    pub(crate) fn tick_by(&mut self, duration: Duration) {
        let _ = self.rt.tick(duration);
        self.now = self.rt.now();
        for &index in &self.busy {
            let Some((pair, link)) = self.links.get_index_mut(index) else {
                continue;
            };
            if link.process_deliverables(self.now) {
                mark_ready(&mut self.ready, index, pair, link);
            }
        }
        let links = &self.links;
        self.busy
            .retain(|&index| links.get_index(index).is_some_and(|(_, l)| !l.sent.is_empty()));
    }

    pub(crate) fn iter_mut(&mut self) -> LinksIter<'_> {
        LinksIter {
            iter: self.links.iter_mut().enumerate(),
            now: self.now,
        }
    }

    pub(crate) fn busy_mut(&mut self) -> BusyLinksIter<'_> {
        BusyLinksIter {
            iter: self.links.iter_mut(),
            next: 0,
            busy: self.busy.iter(),
            now: self.now,
        }
    }

    /// Applies `f` to the link of `pair`, at the network's time, and
    /// tracks what it leaves on the link.
    fn with_link(&mut self, pair: &Pair, f: impl FnOnce(&mut Link, Instant)) {
        let (index, pair, link) = self
            .links
            .get_full_mut(pair)
            .unwrap_or_else(|| panic!("unable to find link between {pair:?}"));
        f(link, self.now);
        track(&mut self.busy, &mut self.ready, index, pair, link);
    }
}

/// Keeps `busy` and `ready` in step with the link at `index`, after a change
/// to its queues.
fn track(
    busy: &mut BTreeSet<usize>,
    ready: &mut IndexMap<IpAddr, BTreeSet<usize>>,
    index: usize,
    pair: &Pair,
    link: &Link,
) {
    if link.sent.is_empty() {
        busy.remove(&index);
    } else {
        busy.insert(index);
    }
    mark_ready(ready, index, pair, link);
}

/// Adds the link at `index` to the ready links of each end it holds
/// messages ready for.
fn mark_ready(
    ready: &mut IndexMap<IpAddr, BTreeSet<usize>>,
    index: usize,
    pair: &Pair,
    link: &Link,
) {
    for host in [pair.0, pair.1] {
        if link.deliverable.get(&host).is_some_and(|q| !q.is_empty()) {
            ready.entry(host).or_default().insert(index);
        }
    }
}

/// Represents a message sent between two hosts on the network.
struct Sent {
    src: SocketAddr,
    dst: SocketAddr,
    status: DeliveryStatus,
    protocol: Protocol,
}

impl Sent {
    fn deliver(&mut self, now: Instant) {
        self.status = DeliveryStatus::DeliverAfter(now);
    }
}

enum DeliveryStatus {
    DeliverAfter(Instant),
    Hold,
}

impl Link {
    fn new() -> Link {
        Link {
            state_a_b: State::Healthy,
            state_b_a: State::Healthy,
            config: config::Link::default(),
            sent: VecDeque::new(),
            deliverable: IndexMap::default(),
        }
    }

    fn enqueue_message(
        &mut self,
        global_config: &config::Link,
        rand: &mut dyn RngCore,
        now: Instant,
        src: SocketAddr,
        dst: SocketAddr,
        message: Protocol,
    ) -> Result<()> {
        tracing::trace!(target: TRACING_TARGET, ?src, ?dst, protocol = %message, "Send");

        self.rand_partition_or_repair(global_config, rand, now);
        let result = self.enqueue(global_config, rand, now, src, dst, message);
        self.process_deliverables(now);
        result
    }

    fn get_state_for_message(&self, src: IpAddr, dst: IpAddr) -> State {
        // Between each pair of ip-addresses there can exist a link.
        // A link between two such pairs can be constructed as
        // Pair::new(x,y) or Pair::new(y,x). These two expressions create
        // the exact same link. We denote one of the ends of the link as 'a',
        // and the other 'b'. 'a' is always the one that compares smaller, i.e,
        // `a < b` holds.
        if src < dst {
            self.state_a_b
        } else {
            self.state_b_a
        }
    }

    // src -> link -> dst
    //        ^-- you are here!
    //
    // Messages may be dropped, sit on the link for a while (due to latency, or
    // because the link has stalled), or be delivered immediately.
    // Link will return an Err(()) if the socket buffer is full.
    fn enqueue(
        &mut self,
        global_config: &config::Link,
        rand: &mut dyn RngCore,
        now: Instant,
        src: SocketAddr,
        dst: SocketAddr,
        message: Protocol,
    ) -> Result<()> {
        let state = self.get_state_for_message(src.ip(), dst.ip());
        let status = match state {
            State::Healthy => {
                let delay = self.delay(global_config.latency(), rand);
                DeliveryStatus::DeliverAfter(now + delay)
            }
            // Only A->B is blocked, so B can send, so we can send if src is B
            State::Hold => {
                tracing::trace!(target: TRACING_TARGET,?src, ?dst, protocol = %message, "Hold");
                DeliveryStatus::Hold
            }
            _ => {
                tracing::trace!(target: TRACING_TARGET,?src, ?dst, protocol = %message, "Drop");
                return Ok(());
            }
        };

        let sent = Sent {
            src,
            dst,
            status,
            protocol: message,
        };

        self.sent.push_back(sent);
        Ok(())
    }

    // Moves the messages whose time has come by `now` to the messages ready
    // for their receivers. Returns whether it moved any.
    fn process_deliverables(&mut self, now: Instant) -> bool {
        // TODO: `drain_filter` is not yet stable, and so we have a low quality
        // implementation here that avoids clones.
        let mut deliverable = 0;
        for i in 0..self.sent.len() {
            let index = i - deliverable;
            let sent = &self.sent[index];
            if let DeliveryStatus::DeliverAfter(time) = sent.status {
                if time <= now {
                    let sent = self.sent.remove(index).unwrap();
                    let envelope = Envelope {
                        src: sent.src,
                        dst: sent.dst,
                        message: sent.protocol,
                    };
                    self.deliverable
                        .entry(sent.dst.ip())
                        .or_default()
                        .push_back(envelope);
                    deliverable += 1;
                }
            }
        }
        deliverable > 0
    }

    // FIXME: This implementation does not respect message delivery order. If
    // host A and host B are ordered (by addr), and B sends before A, then this
    // method will deliver A's message before B's.
    fn deliver_messages(
        &mut self,
        global_config: &config::Link,
        rand: &mut dyn RngCore,
        now: Instant,
        host: &mut Host,
    ) {
        let deliverable = self
            .deliverable
            .entry(host.addr)
            .or_default()
            .drain(..)
            .collect::<Vec<Envelope>>();

        for message in deliverable {
            let (src, dst) = (message.src, message.dst);
            if let Err(message) = host.receive_from_network(message) {
                let _ = self.enqueue_message(global_config, rand, now, dst, src, message);
            }
        }
    }

    // Randomly break or repair this link.
    fn rand_partition_or_repair(
        &mut self,
        global_config: &config::Link,
        rand: &mut dyn RngCore,
        now: Instant,
    ) {
        let do_rand = self.rand_partition(global_config.message_loss(), rand);
        match (self.state_a_b, self.state_b_a) {
            (State::Healthy, _) | (_, State::Healthy) if do_rand => {
                self.state_a_b = State::RandPartition;
                self.state_b_a = State::RandPartition;

                self.sent.clear();
            }
            (State::RandPartition, _) | (_, State::RandPartition)
                if self.rand_repair(global_config.message_loss(), rand) =>
            {
                self.release(now);
            }
            _ => {}
        }
    }

    fn hold(&mut self) {
        self.state_a_b = State::Hold;
        self.state_b_a = State::Hold;

        for sent in &mut self.sent {
            sent.status = DeliveryStatus::Hold;
        }
    }

    // This link becomes healthy, and any held messages are scheduled for delivery.
    fn release(&mut self, now: Instant) {
        self.state_a_b = State::Healthy;
        self.state_b_a = State::Healthy;
        for sent in &mut self.sent {
            if let DeliveryStatus::Hold = sent.status {
                sent.deliver(now);
            }
        }
    }

    fn explicit_partition(&mut self) {
        self.state_a_b = State::ExplicitPartition;
        self.state_b_a = State::ExplicitPartition;

        self.sent.clear();
    }

    fn partition_oneway(&mut self, from: IpAddr, to: IpAddr) {
        if from < to {
            self.state_a_b = State::ExplicitPartition;
        } else {
            self.state_b_a = State::ExplicitPartition;
        }

        self.sent.retain(|sent| sent.src.ip() != from);
    }

    fn repair_oneway(&mut self, from: IpAddr, to: IpAddr) {
        if from < to {
            self.state_a_b = State::Healthy;
        } else {
            self.state_b_a = State::Healthy;
        }
    }

    // Repair the link, without releasing any held messages.
    fn explicit_repair(&mut self) {
        self.state_a_b = State::Healthy;
        self.state_b_a = State::Healthy;
    }

    /// Should the link be randomly partitioned
    fn rand_partition(&self, global: &config::MessageLoss, rand: &mut dyn RngCore) -> bool {
        let config = self.config.message_loss.as_ref().unwrap_or(global);
        let fail_rate = config.fail_rate;
        fail_rate > 0.0 && rand.random_bool(fail_rate)
    }

    fn rand_repair(&self, global: &config::MessageLoss, rand: &mut dyn RngCore) -> bool {
        let config = self.config.message_loss.as_ref().unwrap_or(global);
        let repair_rate = config.repair_rate;
        repair_rate > 0.0 && rand.random_bool(repair_rate)
    }

    fn delay(&self, global: &config::Latency, rand: &mut dyn RngCore) -> Duration {
        let config = self.config.latency.as_ref().unwrap_or(global);

        let mult = config.latency_distribution.sample(rand);
        let range = (config.max_message_latency - config.min_message_latency).as_millis() as f64;
        let delay = config.min_message_latency + Duration::from_millis((range * mult) as _);

        std::cmp::min(delay, config.max_message_latency)
    }

    fn latency(&mut self, global: &config::Latency) -> &mut config::Latency {
        self.config.latency.get_or_insert_with(|| global.clone())
    }

    fn message_loss(&mut self, global: &config::MessageLoss) -> &mut config::MessageLoss {
        self.config
            .message_loss
            .get_or_insert_with(|| global.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::config::Link;

    use super::*;

    #[test]
    #[should_panic(expected = "unable to find link between Pair(10.0.0.1, 192.168.0.1)")]
    fn link_access_gives_useful_panic_message() {
        let mut topo = Topology::new(Link::default());
        topo.hold("10.0.0.1".parse().unwrap(), "192.168.0.1".parse().unwrap());
    }
}

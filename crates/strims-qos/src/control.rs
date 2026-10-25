// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The H-PFQ scheduler.
//!
//! [`Control`] is one H-PFQ scheduler (Bennett & Zhang, "Hierarchical packet
//! fair queueing algorithms", 1997) with a host-wide [`Hlb`] rate limit. It
//! schedules *send opportunities* and never holds packet bytes. Owners
//! report whether their session has something to send ([`Control::set_backlog`]),
//! the driver asks which session goes next ([`Control::dequeue`]), the owner
//! fills one frame, and the driver reports the bytes actually sent
//! ([`Control::complete`]).
//!
//! # Scheduler model
//!
//! Every busy class holds a *logical head*: the session its subtree will
//! send next. The class sits in its parent's SEFF queue tagged with virtual
//! start and finish times for that head. Selection is WF2Q+: among children
//! whose start tag is at most the class's virtual time, pick the smallest
//! finish tag.
//!
//! - A node's weight `w` is its share, at full resolution: weights are
//!   clamped to [`MIN_WEIGHT`]`..=`[`MAX_WEIGHT`], and a packet of `L` bytes
//!   advances the node's finish tag by `L * VT_SCALE / w` in 128-bit fixed
//!   point. ppspp's reciprocity needs weights near the top of the range
//!   (950 vs 1000) to discriminate.
//! - A class's virtual time advances by `L * VT_SCALE / Σw` over its
//!   children for every `L` bytes it sends, rounded down, and never lags its
//!   smallest queued start tag. It can't outrun a continuously backlogged
//!   child, so a newcomer starts level with long-running siblings.
//! - Tags are computed from the length hint when a head is chosen.
//!   [`Control::complete`] re-tags the committed path with the actual
//!   length, so byte shares follow weights even when hints are only upper
//!   bounds. The rate limit checks the hint at `dequeue` and charges the
//!   actual length at `complete`.
//! - `complete` clears the committed path's heads, and the next
//!   [`Control::dequeue`] chooses them (`settle`), so an owner that refills
//!   its backlog right after `complete` still competes. Settling inside
//!   `complete` would raise a class's virtual time to the other children's
//!   start tags while a one-frame-at-a-time owner is momentarily idle; it
//!   would then rejoin past its last finish tag and lose service it is owed.
//!   [`Control::poll_timeout`] stays exact in between by replaying `settle`
//!   read-only (`next_head`).
//! - A class's head is final only once in flight. A child that joins (or
//!   gets a smaller finish tag) and beats the class's current pick (eligible,
//!   with a smaller finish tag; ties keep the pick) takes it over
//!   (`maybe_preempt`), and so on up the tree. Nothing was sent, so the class
//!   keeps its start tag. Picks on the in-flight session's path are frozen
//!   until `complete`.
//! - A blocked session leaves its parent's queue but stays backlogged,
//!   parked with the start tag of its next frame. On unblock it rejoins at
//!   `max(start, V - cost(L, w))`: it keeps its place in line and banks at
//!   most one frame of credit, as much as a continuously backlogged WF2Q+
//!   session can have. A session that goes idle rejoins at `max(finish, V)`.
//! - Virtual times and tags are `u128` and saturate. A class advances at
//!   most `VT_SCALE` ≈ 2^42 per byte, so it lasts about 2^86 bytes.

use slotmap::{SlotMap, new_key_type};
use strims_base::time::Time;

use crate::Error;
use crate::hlb::Hlb;
use crate::seff::{self, Links, VTime};

/// The largest share; larger weights count as this.
pub const MAX_WEIGHT: u64 = 1000;

/// The smallest share; 0 counts as this.
pub const MIN_WEIGHT: u64 = 1;

/// The rate limit of [`Control::new`] in bytes per second: 1 TiB/s,
/// effectively none.
pub const DEFAULT_RATE_LIMIT: u64 = 1 << 40;

/// Virtual time units per byte at weight 1. A multiple of [`MAX_WEIGHT`], so
/// the cost of weights that divide 1000 is exact; the factor 2^32 keeps the
/// rounding error of any other weight below one part in 4 × 10^9 per frame.
const VT_SCALE: VTime = (MAX_WEIGHT as VTime) << 32;

/// The integer inverse weight `MAX_WEIGHT / w`. Weights above
/// [`MAX_WEIGHT`] map to 1 and 0 maps to 1000. It quantizes: every weight
/// from 501 to 1000 maps to 1. The scheduler doesn't use it.
#[deprecated(note = "shares are proportional to the weight; see Control::weight_of")]
pub fn inverse_weight(w: u64) -> u64 {
    if w > MAX_WEIGHT {
        1
    } else {
        MAX_WEIGHT / w.max(1)
    }
}

/// The weight the scheduler uses for `w`.
fn effective_weight(w: u64) -> u64 {
    w.clamp(MIN_WEIGHT, MAX_WEIGHT)
}

/// Virtual time of `len` bytes at weight `w` (or at a parent's weight sum).
fn cost(len: u64, w: u64) -> VTime {
    VTime::from(len) * VT_SCALE / VTime::from(w.max(1))
}

new_key_type! {
    struct NodeKey;
}

/// A class in the scheduling tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClassId(NodeKey);

/// A leaf of the scheduling tree, one per frame writer (a port on a link).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionId(NodeKey);

/// Either kind of tree node, for [`Control::set_weight`] and
/// [`Control::remove`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NodeId {
    Class(ClassId),
    Session(SessionId),
}

impl From<ClassId> for NodeId {
    fn from(c: ClassId) -> NodeId {
        NodeId::Class(c)
    }
}

impl From<SessionId> for NodeId {
    fn from(s: SessionId) -> NodeId {
        NodeId::Session(s)
    }
}

/// A logical head packet: the session that sends it and its length hint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Head {
    session: NodeKey,
    len: u64,
}

/// Where a non-root node is in its parent's scheduling state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Place {
    /// Not competing: no head, or a class on the committed path awaiting
    /// [`Control::settle`].
    Idle,
    /// In the parent's SEFF queue.
    Queued,
    /// The parent's active child: its head is the parent's head.
    Active,
}

/// A class or a session. Each kind leaves the other's fields unused.
#[derive(Debug)]
struct Node {
    parent: Option<NodeKey>,
    first_child: Option<NodeKey>,
    prev_sibling: Option<NodeKey>,
    next_sibling: Option<NodeKey>,
    is_session: bool,

    /// The effective weight, in `MIN_WEIGHT..=MAX_WEIGHT`.
    weight: u64,
    head: Option<Head>,
    stime: VTime,
    ftime: VTime,
    place: Place,
    seff: Links<NodeKey>,

    // classes
    /// Root of this class's SEFF queue of children.
    queue: Option<NodeKey>,
    active_child: Option<NodeKey>,
    vtime: VTime,
    busy: bool,
    /// Sum of the children's weights.
    weight_sum: u64,

    // sessions
    /// Length hint of the next frame, if the owner has something to send.
    backlog: Option<u64>,
    /// The session's link can't take a frame right now.
    blocked: bool,
    /// Left the queue while backlogged because it was blocked. `stime` is
    /// then the start tag of its next frame, kept for the unblock.
    parked: bool,
}

impl Node {
    fn new(parent: Option<NodeKey>, is_session: bool, weight: u64) -> Node {
        Node {
            parent,
            first_child: None,
            prev_sibling: None,
            next_sibling: None,
            is_session,
            weight: effective_weight(weight),
            head: None,
            stime: 0,
            ftime: 0,
            place: Place::Idle,
            seff: Links::default(),
            queue: None,
            active_child: None,
            vtime: 0,
            busy: false,
            weight_sum: 0,
            backlog: None,
            blocked: false,
            parked: false,
        }
    }

    /// The length hint this session competes with, if it competes.
    fn wants(&self) -> Option<u64> {
        if self.blocked { None } else { self.backlog }
    }

    fn finish_for(&self, len: u64) -> VTime {
        self.stime.saturating_add(cost(len, self.weight))
    }
}

impl seff::Arena<NodeKey> for SlotMap<NodeKey, Node> {
    fn links(&self, k: NodeKey) -> &Links<NodeKey> {
        // invariant: tree links only ever hold keys of live nodes; nodes
        // leave their parent's queue before they are deleted.
        #[allow(clippy::indexing_slicing)]
        &self[k].seff
    }

    fn links_mut(&mut self, k: NodeKey) -> &mut Links<NodeKey> {
        // invariant: as above.
        #[allow(clippy::indexing_slicing)]
        &mut self[k].seff
    }
}

/// The host-wide egress scheduler.
///
/// # Egress contract
///
/// 1. Build the tree with [`Control::add_class`] and [`Control::add_session`]
///    under [`Control::root`]. A session is one frame writer, which is one
///    (link, port) pair, and the driver keeps that mapping.
/// 2. While a session's owner has data, keep it backlogged with
///    [`Control::set_backlog`]`(s, hint)`, where `hint` is the expected size
///    of its next frame including the frame header. An upper bound such as
///    the link MTU is best: the rate-limit check uses the hint, so a hint
///    that is too small lets a larger frame overshoot the bucket. Repeating
///    the call with the same hint is a cheap no-op. Call
///    [`Control::clear_backlog`] when the owner has nothing left. A session
///    that stays backlogged across frames gets its weighted share. One that
///    goes idle and returns rejoins at the current virtual time, so it banks
///    no credit while idle.
/// 3. When a link can't take a frame, call [`Control::set_blocked`]`(s, true)`
///    on each of its sessions, and `false` when it can again. A blocked
///    session keeps its backlog and its place in line: on unblock it resumes
///    from its old start tag, but never more than one frame behind the
///    current virtual time, so a long block banks at most one frame of credit.
/// 4. Loop: `while let Some((s, hint)) = qos.dequeue(now)`, have the owner
///    fill at most one frame for `s` (its link is writable, since it isn't
///    blocked), then call [`Control::complete`]`(s, frame_len, now)` and
///    update the backlog for what remains (before or after `complete`; the
///    next head is chosen at the next `dequeue`). Exactly one session is in
///    flight between `dequeue` and `complete`; until `complete`, `dequeue`
///    returns the same session again. A `frame_len` of 0 means the owner had
///    nothing and clears the session's backlog.
/// 5. When `dequeue` returns `None`, wake at [`Control::poll_timeout`] (the
///    rate limit's release time), or on the next `set_backlog` or unblock.
///    `poll_timeout` is exact at any point, including between `complete`
///    and the next `dequeue`.
///
/// Weights (see [`Control::set_weight`]) split a class among its competing
/// children in proportion, at full resolution. They shape throughput only
/// where this scheduler is the bottleneck: under the host rate limit, or
/// when the driver paces `dequeue` to a shared bottleneck. Otherwise each
/// unblocked link drains at its own rate and weights only order frames. Until a session is in
/// flight, which session goes next reflects every call so far: a session
/// that starts competing and would finish first takes over its classes'
/// picks. Calls that change a session's backlog, blocking or weight while it
/// is in flight take effect at `complete`. Removing it while in flight
/// cancels the send, and `complete` then returns [`Error::UnknownSession`].
#[derive(Debug)]
pub struct Control {
    /// Bounded by the classes and sessions the host creates (frame writers
    /// per link and port); peers can't add nodes directly.
    nodes: SlotMap<NodeKey, Node>,
    root: NodeKey,
    hlb: Hlb,
    in_flight: Option<NodeKey>,
    /// After `complete`: the lowest class on the committed path, whose heads
    /// up to the root are chosen at the next `dequeue` (`settle`).
    pending: Option<NodeKey>,
    /// Unique insertion number for SEFF keys (FIFO tie-break).
    seq: u64,
}

impl Default for Control {
    fn default() -> Self {
        Control::new()
    }
}

impl Control {
    /// A scheduler limited to [`DEFAULT_RATE_LIMIT`].
    pub fn new() -> Control {
        Control::with_rate_limit(DEFAULT_RATE_LIMIT)
    }

    /// A scheduler limited to `limit` bytes per second.
    pub fn with_rate_limit(limit: u64) -> Control {
        let mut nodes = SlotMap::with_key();
        // The root never sits in a queue, so its weight is unused.
        let root = nodes.insert(Node::new(None, false, MAX_WEIGHT));
        Control {
            nodes,
            root,
            hlb: Hlb::new(limit),
            in_flight: None,
            pending: None,
            seq: 0,
        }
    }

    /// Bytes per second; 0 stops all egress.
    pub fn set_rate_limit(&mut self, limit: u64) {
        self.hlb.set_limit(limit);
    }

    pub fn rate_limit(&self) -> u64 {
        self.hlb.limit()
    }

    /// The root class.
    pub fn root(&self) -> ClassId {
        ClassId(self.root)
    }

    /// Adds a class under `parent`.
    pub fn add_class(&mut self, parent: ClassId, weight: u64) -> Result<ClassId, Error> {
        let p = self.class(parent)?;
        Ok(ClassId(self.add_node(p, false, weight)))
    }

    /// Adds a session under `parent`. The session starts idle.
    pub fn add_session(&mut self, parent: ClassId, weight: u64) -> Result<SessionId, Error> {
        let p = self.class(parent)?;
        Ok(SessionId(self.add_node(p, true, weight)))
    }

    /// Siblings share their parent in proportion to their weights, clamped
    /// to [`MIN_WEIGHT`]`..=`[`MAX_WEIGHT`]. The new weight applies from the
    /// node's next tag; current tags are kept.
    pub fn set_weight(&mut self, id: impl Into<NodeId>, weight: u64) -> Result<(), Error> {
        let k = self.node_key(id.into())?;
        let new = effective_weight(weight);
        let node = self.n_mut(k);
        let old = node.weight;
        node.weight = new;
        if let Some(p) = node.parent {
            let p = self.n_mut(p);
            p.weight_sum = p.weight_sum.saturating_sub(old).saturating_add(new);
        }
        Ok(())
    }

    /// The weight in use for a node: the last one set, clamped to
    /// [`MIN_WEIGHT`]`..=`[`MAX_WEIGHT`].
    pub fn weight_of(&self, id: impl Into<NodeId>) -> Result<u64, Error> {
        let k = self.node_key(id.into())?;
        Ok(self.n(k).weight)
    }

    /// The quantized inverse ([`inverse_weight`]) of the node's weight.
    #[deprecated(note = "the scheduler uses the full-resolution weight; see weight_of")]
    pub fn inverse_weight_of(&self, id: impl Into<NodeId>) -> Result<u64, Error> {
        #[allow(deprecated)]
        self.weight_of(id).map(inverse_weight)
    }

    /// Removes a session, or a class with everything under it. The ids of
    /// removed nodes become invalid.
    pub fn remove(&mut self, id: impl Into<NodeId>) -> Result<(), Error> {
        let k = self.node_key(id.into())?;
        let (parent, place, weight) = {
            let n = self.n(k);
            (n.parent, n.place, n.weight)
        };
        // Only the root has no parent.
        let Some(parent) = parent else {
            return Err(Error::RemoveRoot);
        };

        match place {
            Place::Queued => self.dequeue_child(parent, k),
            Place::Active => self.n_mut(parent).active_child = None,
            Place::Idle => {}
        }

        let (prev, next) = {
            let n = self.n(k);
            (n.prev_sibling, n.next_sibling)
        };
        match prev {
            Some(p) => self.n_mut(p).next_sibling = next,
            None => self.n_mut(parent).first_child = next,
        }
        if let Some(nx) = next {
            self.n_mut(nx).prev_sibling = prev;
        }
        let p = self.n_mut(parent);
        p.weight_sum = p.weight_sum.saturating_sub(weight);

        // Delete the subtree. Its queues hold only its own nodes.
        let mut cancelled = false;
        let mut stack = vec![k];
        while let Some(d) = stack.pop() {
            if let Some(n) = self.nodes.remove(d) {
                let mut c = n.first_child;
                while let Some(ck) = c {
                    stack.push(ck);
                    c = self.n(ck).next_sibling;
                }
            }
            if self.in_flight == Some(d) {
                self.in_flight = None;
                cancelled = true;
            }
            if self.pending == Some(d) {
                // The removed node was on the committed path; its parent is
                // the lowest class left on it.
                self.pending = Some(parent);
            }
        }

        if place == Place::Active {
            // The removed node carried the parent's head (and, if a send was
            // in flight under it, every head up to the root, which were
            // frozen and must all choose again).
            self.reselect(parent, cancelled);
        }
        Ok(())
    }

    /// Reports that the session has a frame of about `len_hint` bytes to
    /// send (see the egress contract on [`Control`]). A hint of 0 counts as 1.
    pub fn set_backlog(&mut self, s: SessionId, len_hint: usize) -> Result<(), Error> {
        let k = self.session(s)?;
        let len = u64::try_from(len_hint).unwrap_or(u64::MAX).max(1);
        self.n_mut(k).backlog = Some(len);
        self.refresh(k);
        Ok(())
    }

    /// Reports that the session has nothing to send.
    pub fn clear_backlog(&mut self, s: SessionId) -> Result<(), Error> {
        let k = self.session(s)?;
        self.n_mut(k).backlog = None;
        self.refresh(k);
        Ok(())
    }

    /// Marks the session's link as unable (or able again) to take a frame.
    /// See the egress contract on [`Control`].
    pub fn set_blocked(&mut self, s: SessionId, blocked: bool) -> Result<(), Error> {
        let k = self.session(s)?;
        self.n_mut(k).blocked = blocked;
        self.refresh(k);
        Ok(())
    }

    pub fn is_backlogged(&self, s: SessionId) -> Result<bool, Error> {
        Ok(self.n(self.session(s)?).backlog.is_some())
    }

    pub fn is_blocked(&self, s: SessionId) -> Result<bool, Error> {
        Ok(self.n(self.session(s)?).blocked)
    }

    /// The session's length hint of its next frame, if it is backlogged.
    pub fn backlog_of(&self, s: SessionId) -> Result<Option<u64>, Error> {
        Ok(self.n(self.session(s)?).backlog)
    }

    /// Whether the session left the queue while backlogged because its
    /// link was blocked.
    pub fn is_parked(&self, s: SessionId) -> Result<bool, Error> {
        Ok(self.n(self.session(s)?).parked)
    }

    /// Whether `id` refers to a live node.
    pub fn contains(&self, id: impl Into<NodeId>) -> bool {
        self.node_key(id.into()).is_ok()
    }

    /// The node's parent class; `None` for the root.
    pub fn parent(&self, id: impl Into<NodeId>) -> Result<Option<ClassId>, Error> {
        let k = self.node_key(id.into())?;
        Ok(self.n(k).parent.map(ClassId))
    }

    /// The session dequeued and not yet completed, if any.
    pub fn in_flight(&self) -> Option<SessionId> {
        self.in_flight.map(SessionId)
    }

    /// The next session allowed to send a frame, in H-PFQ order and within
    /// the rate limit, with its length hint.
    ///
    /// Commits nothing: the send is accounted by [`Control::complete`], and
    /// until then this returns the same session again.
    pub fn dequeue(&mut self, now: Time) -> Option<(SessionId, usize)> {
        #[cfg(test)]
        let predicted = self.next_head();
        self.settle();
        #[cfg(test)]
        assert_eq!(
            predicted,
            self.n(self.root).head,
            "next_head disagrees with settle"
        );
        let head = self.n(self.root).head?;
        if self.in_flight.is_none() {
            self.hlb.advance(now);
            if !self.hlb.admits(head.len) {
                return None;
            }
            self.in_flight = Some(head.session);
        }
        Some((
            SessionId(head.session),
            usize::try_from(head.len).unwrap_or(usize::MAX),
        ))
    }

    /// Accounts the frame of `len` bytes sent for the in-flight session.
    /// `len` may differ from the hint; virtual times and the rate limit are
    /// charged with `len`. A `len` of 0 means nothing was sent, and clears
    /// the session's backlog.
    pub fn complete(&mut self, s: SessionId, len: usize, now: Time) -> Result<(), Error> {
        let k = self.session(s)?;
        if self.in_flight != Some(k) {
            return Err(Error::NotInFlight);
        }
        self.in_flight = None;
        let len = u64::try_from(len).unwrap_or(u64::MAX);
        self.hlb.advance(now);
        self.hlb.charge(len);
        if len == 0 {
            self.n_mut(k).backlog = None;
        }
        self.commit(k, len);
        Ok(())
    }

    /// When [`Control::dequeue`] will next return a session, or `None` if no
    /// session is eligible (none backlogged and unblocked, or limit 0). A
    /// time at or before `now` means right away. Exact if nothing changes in
    /// between: `dequeue(t)` returns a session iff `t` is at or after it
    /// (for `t` no earlier than the last time passed in). While a session is
    /// in flight, `dequeue` returns it at any time.
    pub fn poll_timeout(&self) -> Option<Time> {
        if self.in_flight.is_some() {
            return Some(self.hlb.last());
        }
        let head = self.next_head()?;
        self.hlb.ready_at(head.len)
    }

    // --- internals ------------------------------------------------------

    fn n(&self, k: NodeKey) -> &Node {
        // invariant: internal keys (root, parent/child/sibling links, tree
        // links, heads, in_flight, pending) always refer to live nodes;
        // `remove` unlinks a subtree before deleting it. Public ids are
        // checked by `class`, `session` and `node_key` first.
        #[allow(clippy::indexing_slicing)]
        &self.nodes[k]
    }

    fn n_mut(&mut self, k: NodeKey) -> &mut Node {
        // invariant: as in `n`.
        #[allow(clippy::indexing_slicing)]
        &mut self.nodes[k]
    }

    fn class(&self, c: ClassId) -> Result<NodeKey, Error> {
        match self.nodes.get(c.0) {
            Some(n) if !n.is_session => Ok(c.0),
            _ => Err(Error::UnknownClass),
        }
    }

    fn session(&self, s: SessionId) -> Result<NodeKey, Error> {
        match self.nodes.get(s.0) {
            Some(n) if n.is_session => Ok(s.0),
            _ => Err(Error::UnknownSession),
        }
    }

    fn node_key(&self, id: NodeId) -> Result<NodeKey, Error> {
        match id {
            NodeId::Class(c) => self.class(c),
            NodeId::Session(s) => self.session(s),
        }
    }

    fn add_node(&mut self, parent: NodeKey, is_session: bool, weight: u64) -> NodeKey {
        let next = self.n(parent).first_child;
        let mut node = Node::new(Some(parent), is_session, weight);
        node.next_sibling = next;
        let weight = node.weight;
        let k = self.nodes.insert(node);
        if let Some(nx) = next {
            self.n_mut(nx).prev_sibling = Some(k);
        }
        let p = self.n_mut(parent);
        p.first_child = Some(k);
        p.weight_sum = p.weight_sum.saturating_add(weight);
        k
    }

    /// Inserts `child` into `parent`'s queue with its current tags.
    fn enqueue(&mut self, parent: NodeKey, child: NodeKey) {
        self.seq += 1;
        let seq = self.seq;
        let (s, f) = {
            let c = self.n(child);
            (c.stime, c.ftime)
        };
        let root = self.n(parent).queue;
        let root = seff::insert(&mut self.nodes, root, child, (s, seq), (f, seq));
        self.n_mut(parent).queue = Some(root);
        self.n_mut(child).place = Place::Queued;
    }

    /// Takes a queued `child` out of `parent`'s queue.
    fn dequeue_child(&mut self, parent: NodeKey, child: NodeKey) {
        let root = self.n(parent).queue;
        let root = seff::remove(&mut self.nodes, root, child);
        self.n_mut(parent).queue = root;
        self.n_mut(child).place = Place::Idle;
    }

    /// Takes the WF2Q+ pick out of `p`'s queue, after raising the class's
    /// virtual time to its smallest queued start tag (`max(V, min S)`), so a
    /// non-empty queue always yields a child.
    fn select_next(&mut self, p: NodeKey) -> Option<NodeKey> {
        let root = self.n(p).queue;
        let min_start = seff::min_start(&self.nodes, root)?;
        let node = self.n_mut(p);
        node.vtime = node.vtime.max(min_start);
        let vtime = node.vtime;
        let c = seff::select(&self.nodes, root, vtime)?;
        self.dequeue_child(p, c);
        Some(c)
    }

    /// Tags `k` for its head. The start tag continues from its finish tag
    /// with `use_ftime` (a busy class), and is otherwise `max(finish, V)`.
    fn update_start_finish(&mut self, k: NodeKey, use_ftime: bool) {
        let parent_vtime = self.n(k).parent.map(|p| self.n(p).vtime);
        let node = self.n_mut(k);
        node.stime = match parent_vtime {
            Some(v) if !use_ftime => node.ftime.max(v),
            _ => node.ftime,
        };
        node.ftime = node.finish_for(node.head.map_or(0, |h| h.len));
    }

    /// `p` has no head and is in no queue. Picks
    /// its next head, queues it in its parent, and walks up while the parent
    /// has no head either: idle parents always, and with `settling`, the
    /// classes of the committed path (busy but headless). A parent that
    /// already has a head may switch to `p` (`maybe_preempt`).
    fn restart(&mut self, mut p: NodeKey, settling: bool) {
        loop {
            let parent = self.n(p).parent;
            if let Some(c) = self.select_next(p) {
                let head = self.n(c).head;
                self.n_mut(c).place = Place::Active;
                let busy = self.n(p).busy;
                let node = self.n_mut(p);
                node.active_child = Some(c);
                node.head = head;
                if let Some(gp) = parent {
                    self.update_start_finish(p, busy);
                    self.enqueue(gp, p);
                }
                self.n_mut(p).busy = true;
            } else {
                self.n_mut(p).busy = false;
            }
            match parent {
                Some(gp) if self.n(gp).head.is_none() && (settling || !self.n(gp).busy) => p = gp,
                Some(gp) if self.n(p).head.is_some() => {
                    self.maybe_preempt(gp, p);
                    return;
                }
                _ => return,
            }
        }
    }

    /// Chooses the heads of the path committed by the last `complete`.
    fn settle(&mut self) {
        if let Some(p) = self.pending.take() {
            self.restart(p, true);
        }
    }

    /// The root's head after `settle`, without changing anything: a
    /// read-only replay of `restart(pending, true)`, so `poll_timeout` is
    /// exact between `complete` and the next `dequeue`. Each class on the
    /// committed path chooses among its queue plus the class below it,
    /// which `restart` would queue last with start tag = its old finish tag
    /// (it is busy) and finish tag for its new head. `dequeue` checks the
    /// replay against `settle` in unit tests.
    fn next_head(&self) -> Option<Head> {
        let Some(mut p) = self.pending else {
            return self.n(self.root).head;
        };
        // (start, finish, head) of the class below, as queued in `p`.
        let mut below: Option<(VTime, VTime, Head)> = None;
        loop {
            let node = self.n(p);
            let min_start = match (seff::min_start(&self.nodes, node.queue), below) {
                (Some(q), Some((s, _, _))) => Some(q.min(s)),
                (q, b) => q.or(b.map(|(s, _, _)| s)),
            };
            let head = min_start.and_then(|ms| {
                let v = node.vtime.max(ms);
                let queued = seff::select(&self.nodes, node.queue, v).map(|c| {
                    let c = self.n(c);
                    (c.ftime, c.head)
                });
                let below = below.filter(|(s, _, _)| *s <= v);
                match (queued, below) {
                    // Queued last, the class below loses ties.
                    (Some((qf, _)), Some((_, bf, bh))) if bf < qf => Some(bh),
                    (Some((_, qh)), _) => qh,
                    (None, b) => b.map(|(_, _, bh)| bh),
                }
            });
            let Some(gp) = node.parent else {
                return head;
            };
            below = head.map(|h| {
                (
                    node.ftime,
                    node.ftime.saturating_add(cost(h.len, node.weight)),
                    h,
                )
            });
            p = gp;
        }
    }

    /// `p`'s active child stopped competing, changed its head or lost to a
    /// sibling, and the caller has put it back in `p`'s queue (or not) and
    /// cleared `p.active_child`. Picks a new head for `p`, keeping `p`'s
    /// start tag (nothing was sent), and repeats for each ancestor that
    /// carried the old head and whose tags change, or with `all`, every
    /// ancestor that carried it (their picks were frozen while it was in
    /// flight).
    fn reselect(&mut self, mut p: NodeKey, all: bool) {
        loop {
            let (place, parent) = {
                let n = self.n(p);
                (n.place, n.parent)
            };
            let picked = self.select_next(p);
            let head = picked.and_then(|c| self.n(c).head);
            if let Some(c) = picked {
                self.n_mut(c).place = Place::Active;
            }
            let node = self.n_mut(p);
            node.active_child = picked;
            node.head = head;
            node.busy = head.is_some();
            let Some(gp) = parent else {
                return;
            };
            let ftime = node.finish_for(head.map_or(0, |h| h.len));
            if !all && head.is_some() && ftime == node.ftime {
                // Same tags: `p` keeps its place in `gp`, and the ancestors
                // that carried the old head keep their choice.
                if place == Place::Active {
                    self.carry_head_up(gp, head);
                }
                return;
            }
            if place == Place::Queued {
                self.dequeue_child(gp, p);
            }
            self.n_mut(p).ftime = ftime;
            if head.is_some() {
                self.enqueue(gp, p);
            } else {
                self.n_mut(p).place = Place::Idle;
            }
            if place != Place::Active {
                if head.is_some() {
                    self.maybe_preempt(gp, p);
                }
                return;
            }
            self.n_mut(gp).active_child = None;
            p = gp;
        }
    }

    /// Replaces the head of `p` and of each ancestor that carries it with
    /// `head`, a session whose frame has the same length as the old head's,
    /// so no tags change.
    fn carry_head_up(&mut self, mut p: NodeKey, head: Option<Head>) {
        loop {
            let node = self.n_mut(p);
            node.head = head;
            match (node.place, node.parent) {
                (Place::Active, Some(gp)) => p = gp,
                _ => return,
            }
        }
    }

    /// `c` just joined `p`'s queue, or got a smaller finish tag there. If `p`
    /// has a head that isn't in flight and `c` beats it (eligible, with a
    /// smaller finish tag), `p` re-chooses, so its pick stays WF2Q+'s over
    /// every child competing now. Classes without a head (idle, or on the committed path) choose
    /// at the next `dequeue` anyway. Ties keep the current head (FIFO).
    fn maybe_preempt(&mut self, p: NodeKey, c: NodeKey) {
        let node = self.n(p);
        let (Some(head), Some(a)) = (node.head, node.active_child) else {
            return;
        };
        if self.in_flight == Some(head.session) {
            return;
        }
        let (cn, an) = (self.n(c), self.n(a));
        // `p`'s virtual time already covers the active child's start tag
        // (`select_next`), so it needs no raising here.
        if cn.stime > node.vtime || cn.ftime >= an.ftime {
            return;
        }
        self.enqueue(p, a);
        self.n_mut(p).active_child = None;
        self.reselect(p, false);
    }

    /// Queues a session that starts competing. The start tag is `max(finish, vtime)` (WF2Q+), or for a parked session its kept
    /// start tag, held to at most one frame behind `vtime`.
    fn arrive(&mut self, s: NodeKey) {
        let Some(len) = self.n(s).wants() else {
            return;
        };
        // invariant: sessions always have a parent.
        let Some(parent) = self.n(s).parent else {
            return;
        };
        let vtime = self.n(parent).vtime;
        let node = self.n_mut(s);
        node.head = Some(Head { session: s, len });
        let floor = if node.parked {
            vtime.saturating_sub(cost(len, node.weight))
        } else {
            vtime
        };
        node.parked = false;
        node.stime = node.ftime.max(floor);
        node.ftime = node.finish_for(len);
        self.enqueue(parent, s);
        if self.n(parent).busy {
            self.maybe_preempt(parent, s);
        } else {
            self.restart(parent, false);
        }
    }

    /// Brings a session's scheduling state in line with its backlog and
    /// blocked flags.
    fn refresh(&mut self, s: NodeKey) {
        if self.in_flight == Some(s) {
            return; // applied by `commit`
        }
        let node = self.n_mut(s);
        if node.backlog.is_none() {
            node.parked = false;
        }
        let (place, want, head_len, parent) = (
            node.place,
            node.wants(),
            node.head.map(|h| h.len),
            node.parent,
        );
        // invariant: sessions always have a parent.
        let Some(parent) = parent else {
            return;
        };
        match (place, want) {
            (Place::Idle, None) => {}
            (Place::Idle, Some(_)) => self.arrive(s),
            (_, Some(len)) if head_len == Some(len) => {}
            (Place::Queued, want) => {
                self.dequeue_child(parent, s);
                self.retag_unsent(s, want);
                if want.is_some() {
                    self.enqueue(parent, s);
                    self.maybe_preempt(parent, s);
                }
            }
            (Place::Active, want) => {
                self.retag_unsent(s, want);
                if want.is_some() {
                    self.enqueue(parent, s);
                } else {
                    self.n_mut(s).place = Place::Idle;
                }
                self.n_mut(parent).active_child = None;
                self.reselect(parent, false);
            }
        }
    }

    /// Re-tags a session whose head was never sent: same start tag, finish
    /// for the new hint. If it stops competing, its finish goes back to the
    /// start tag, and a blocked session is parked.
    fn retag_unsent(&mut self, s: NodeKey, want: Option<u64>) {
        let node = self.n_mut(s);
        node.head = want.map(|len| Head { session: s, len });
        node.ftime = node.finish_for(want.unwrap_or(0));
        node.parked = want.is_none() && node.backlog.is_some();
    }

    /// Accounts the `len` bytes `leaf`, the root's head, actually sent.
    /// Choosing the path's next heads is left to [`Control::settle`].
    fn commit(&mut self, leaf: NodeKey, len: u64) {
        // Re-tag the path with the real length, advance each class's
        // virtual time, and clear the path's heads.
        let mut k = leaf;
        loop {
            let node = self.n_mut(k);
            let parent = node.parent;
            if parent.is_some() {
                node.ftime = node.finish_for(len);
            }
            if !node.is_session {
                // Rounded down, so virtual time never runs ahead of the
                // fluid model.
                node.vtime = node.vtime.saturating_add(cost(len, node.weight_sum));
                node.head = None;
                node.active_child = None;
            }
            node.place = Place::Idle;
            match parent {
                Some(p) => k = p,
                None => break,
            }
        }

        // invariant: sessions always have a parent.
        let Some(parent) = self.n(leaf).parent else {
            return;
        };
        let node = self.n_mut(leaf);
        node.head = node.wants().map(|len| Head { session: leaf, len });
        if node.head.is_some() {
            // Continuously backlogged: start where the last frame finished.
            node.parked = false;
            self.update_start_finish(leaf, true);
            self.enqueue(parent, leaf);
        } else if node.backlog.is_some() {
            // Blocked with more to send: park at the next frame's start.
            node.parked = true;
            node.stime = node.ftime;
        } else {
            node.parked = false;
        }
        self.pending = Some(parent);
    }

    /// Checks structural invariants. Test-only.
    #[cfg(test)]
    pub(crate) fn check_invariants(&self) {
        // Classes on the committed path awaiting `settle`.
        let mut pending_path = Vec::new();
        let mut p = self.pending;
        while let Some(k) = p {
            pending_path.push(k);
            p = self.n(k).parent;
        }
        for (k, n) in &self.nodes {
            let mut sum = 0;
            let mut c = n.first_child;
            let mut prev = None;
            while let Some(ck) = c {
                let cn = self.n(ck);
                assert_eq!(cn.parent, Some(k));
                assert_eq!(cn.prev_sibling, prev);
                assert!((MIN_WEIGHT..=MAX_WEIGHT).contains(&cn.weight));
                sum += cn.weight;
                prev = c;
                c = cn.next_sibling;
            }
            assert_eq!(n.weight_sum, sum, "weight_sum");
            if n.is_session {
                assert!(n.first_child.is_none());
                let competing = n.place != Place::Idle;
                if self.in_flight != Some(k) {
                    assert_eq!(competing, n.wants().is_some(), "session competing state");
                }
                assert_eq!(competing, n.head.is_some());
                if let Some(h) = n.head {
                    assert_eq!(h.session, k);
                }
                if n.parked {
                    assert!(n.backlog.is_some() && !competing);
                }
                continue;
            }
            // A class's queue holds exactly its Queued children, and its
            // head is its active child's.
            let queued = seff::check(&self.nodes, n.queue);
            let mut want_queued = 0;
            let mut active = None;
            let mut c = n.first_child;
            while let Some(ck) = c {
                let cn = self.n(ck);
                match cn.place {
                    Place::Queued => want_queued += 1,
                    Place::Active => {
                        assert!(active.is_none(), "two active children");
                        active = Some(ck);
                    }
                    Place::Idle => {}
                }
                c = cn.next_sibling;
            }
            assert_eq!(queued, want_queued, "queue size");
            assert_eq!(n.active_child, active);
            assert_eq!(n.head, active.and_then(|a| self.n(a).head));
            if pending_path.contains(&k) {
                assert!(
                    n.head.is_none() && n.busy && n.place == Place::Idle,
                    "pending path"
                );
                continue;
            }
            assert_eq!(n.busy, n.head.is_some());
            if n.parent.is_some() {
                assert_eq!(n.place != Place::Idle, n.head.is_some());
            }
            // Work conservation: a class with queued children is busy.
            if queued > 0 {
                assert!(n.head.is_some(), "idle class with queued children");
            }
            // The head is the WF2Q+ pick among the children competing now,
            // unless it is in flight (`maybe_preempt`).
            if let (Some(h), Some(a)) = (n.head, n.active_child)
                && self.in_flight != Some(h.session)
            {
                let an = self.n(a);
                assert!(an.stime <= n.vtime, "ineligible head");
                if let Some(c) = seff::select(&self.nodes, n.queue, n.vtime) {
                    assert!(self.n(c).ftime >= an.ftime, "stale head");
                }
            }
        }
        if let Some(s) = self.in_flight {
            assert!(self.pending.is_none());
            assert_eq!(self.n(self.root).head.map(|h| h.session), Some(s));
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use rand_core::Rng;

    fn t(ms: i64) -> Time {
        Time::from_unix_millis(ms)
    }

    #[test]
    #[allow(deprecated)]
    fn inverse_weight_quantizes() {
        assert_eq!(inverse_weight(1), 1000);
        assert_eq!(inverse_weight(3), 333);
        assert_eq!(inverse_weight(50), 20);
        assert_eq!(inverse_weight(500), 2);
        assert_eq!(inverse_weight(501), 1);
        assert_eq!(inverse_weight(1000), 1);
        assert_eq!(inverse_weight(5000), 1);
        assert_eq!(inverse_weight(0), 1000);
    }

    /// Costs are inversely proportional to the weight at full resolution:
    /// no two weights in range cost the same, and ratios are exact to well
    /// under one part in 10^9.
    #[test]
    fn costs_have_full_resolution() {
        for w in MIN_WEIGHT..MAX_WEIGHT {
            assert!(cost(1500, w) > cost(1500, w + 1), "{w}");
        }
        // Weights dividing 1000 cost exactly their inverse weight × 2^32.
        assert_eq!(cost(1000, 1000), 1000 << 32);
        assert_eq!(cost(1000, 50), (20 * 1000) << 32);
        for (a, b) in [(600, 900), (501, 1000), (950, 1000), (1, 1000), (50, 1000)] {
            let ratio = cost(1500, a) as f64 / cost(1500, b) as f64;
            assert!((ratio - b as f64 / a as f64).abs() < 1e-9, "{a}:{b}");
        }
        assert_eq!(effective_weight(0), MIN_WEIGHT);
        assert_eq!(effective_weight(5000), MAX_WEIGHT);
    }

    #[test]
    fn single_session_flow() {
        let mut q = Control::new();
        let s = q.add_session(q.root(), 1).unwrap();
        assert_eq!(q.dequeue(t(0)), None);
        assert_eq!(q.poll_timeout(), None);

        q.set_backlog(s, 1200).unwrap();
        q.check_invariants();
        assert!(q.poll_timeout().unwrap() <= t(0));
        assert_eq!(q.dequeue(t(0)), Some((s, 1200)));
        // Idempotent until complete.
        assert_eq!(q.dequeue(t(0)), Some((s, 1200)));
        assert_eq!(q.in_flight(), Some(s));
        q.check_invariants();

        q.complete(s, 1000, t(0)).unwrap();
        q.check_invariants();
        assert_eq!(q.complete(s, 1000, t(0)), Err(Error::NotInFlight));
        // Still backlogged with the same hint.
        assert_eq!(q.dequeue(t(0)), Some((s, 1200)));
        q.clear_backlog(s).unwrap(); // deferred while in flight
        q.check_invariants();
        q.complete(s, 900, t(0)).unwrap();
        assert_eq!(q.dequeue(t(0)), None);
        assert_eq!(q.poll_timeout(), None);
        q.check_invariants();
    }

    #[test]
    fn zero_length_complete_clears_backlog() {
        let mut q = Control::new();
        let s = q.add_session(q.root(), 1).unwrap();
        q.set_backlog(s, 100).unwrap();
        assert!(q.dequeue(t(0)).is_some());
        q.complete(s, 0, t(0)).unwrap();
        assert!(!q.is_backlogged(s).unwrap());
        assert_eq!(q.dequeue(t(0)), None);
        q.check_invariants();
    }

    #[test]
    fn blocked_sessions_are_skipped() {
        let mut q = Control::new();
        let a = q.add_session(q.root(), 1).unwrap();
        let b = q.add_session(q.root(), 1).unwrap();
        q.set_backlog(a, 100).unwrap();
        q.set_backlog(b, 100).unwrap();
        q.set_blocked(a, true).unwrap();
        q.check_invariants();
        for _ in 0..5 {
            let (s, _) = q.dequeue(t(0)).unwrap();
            assert_eq!(s, b);
            q.complete(s, 100, t(0)).unwrap();
            q.check_invariants();
        }
        q.set_blocked(b, true).unwrap();
        assert_eq!(q.dequeue(t(0)), None);
        assert_eq!(q.poll_timeout(), None);
        assert!(q.is_backlogged(a).unwrap());
        q.set_blocked(a, false).unwrap();
        q.check_invariants();
        assert_eq!(q.dequeue(t(0)).unwrap().0, a);
    }

    #[test]
    fn blocked_in_flight_session_parks_at_complete() {
        let mut q = Control::new();
        let a = q.add_session(q.root(), 1).unwrap();
        q.set_backlog(a, 100).unwrap();
        assert_eq!(q.dequeue(t(0)).unwrap().0, a);
        q.set_blocked(a, true).unwrap();
        q.complete(a, 100, t(0)).unwrap();
        q.check_invariants();
        assert_eq!(q.dequeue(t(0)), None);
        q.set_blocked(a, false).unwrap();
        assert_eq!(q.dequeue(t(0)).unwrap().0, a);
    }

    #[test]
    fn head_invalidation_through_classes() {
        // root -> c1 -> c2 -> {a, b}; block the session that is the head at
        // every level.
        let mut q = Control::new();
        let c1 = q.add_class(q.root(), 1).unwrap();
        let c2 = q.add_class(c1, 1).unwrap();
        let a = q.add_session(c2, 1).unwrap();
        let b = q.add_session(c2, 1).unwrap();
        let other = q.add_session(q.root(), 1).unwrap();
        q.set_backlog(a, 100).unwrap();
        q.check_invariants();
        q.set_backlog(b, 100).unwrap();
        q.set_backlog(other, 100).unwrap();
        q.check_invariants();
        let (head, _) = q.dequeue(t(0)).unwrap();
        q.complete(head, 100, t(0)).unwrap();
        let (head, _) = q.dequeue(t(0)).unwrap();
        q.complete(head, 100, t(0)).unwrap();
        assert!(q.dequeue(t(0)).is_some());
        let (head, _) = q.dequeue(t(0)).unwrap();
        q.complete(head, 100, t(0)).unwrap();
        q.dequeue(t(0)).unwrap(); // settle so heads are chosen
        let (head, _) = q.dequeue(t(0)).unwrap();
        q.complete(head, 100, t(0)).unwrap();
        for s in [a, b, other] {
            q.set_blocked(s, true).unwrap();
            q.check_invariants();
            q.set_backlog(s, 300).unwrap();
            q.check_invariants();
        }
        assert_eq!(q.dequeue(t(0)), None);
        q.set_blocked(b, false).unwrap();
        q.check_invariants();
        assert_eq!(q.dequeue(t(0)), Some((b, 300)));
    }

    #[test]
    fn remove_in_flight_and_classes() {
        let mut q = Control::new();
        let c = q.add_class(q.root(), 1).unwrap();
        let a = q.add_session(c, 1).unwrap();
        let b = q.add_session(c, 1).unwrap();
        let d = q.add_session(q.root(), 1).unwrap();
        for s in [a, b, d] {
            q.set_backlog(s, 100).unwrap();
        }
        let (s, _) = q.dequeue(t(0)).unwrap();
        q.remove(s).unwrap();
        q.check_invariants();
        assert_eq!(q.complete(s, 100, t(0)), Err(Error::UnknownSession));
        let (s, _) = q.dequeue(t(0)).unwrap();
        q.complete(s, 100, t(0)).unwrap();
        q.check_invariants();

        // Remove a class while the committed path through it is pending.
        q.remove(c).unwrap();
        q.check_invariants();
        assert!(!q.contains(a) && !q.contains(b) && !q.contains(c));
        assert_eq!(q.set_backlog(a, 1), Err(Error::UnknownSession));
        assert_eq!(q.add_session(c, 1), Err(Error::UnknownClass));
        assert_eq!(q.remove(q.root()), Err(Error::RemoveRoot));
        for _ in 0..3 {
            let (s, _) = q.dequeue(t(0)).unwrap();
            assert_eq!(s, d);
            q.complete(s, 100, t(0)).unwrap();
        }
    }

    /// Heads on the path of the session in flight are frozen, so a sibling
    /// that arrives meanwhile can't take them. Removing the session cancels
    /// the send, and every frozen pick is made again.
    #[test]
    fn cancelled_send_unfreezes_picks() {
        // root -> { c(1) -> {a(1), b(1)}, x(1000) }
        let mut q = Control::new();
        let c = q.add_class(q.root(), 1).unwrap();
        let a = q.add_session(c, 1).unwrap();
        let b = q.add_session(c, 1).unwrap();
        let x = q.add_session(q.root(), 1000).unwrap();
        q.set_backlog(a, 1000).unwrap();
        q.set_backlog(b, 1000).unwrap();
        assert_eq!(q.dequeue(t(0)).unwrap().0, a);
        q.set_backlog(x, 1000).unwrap(); // finishes first, but a is in flight
        q.check_invariants();
        assert_eq!(q.dequeue(t(0)).unwrap().0, a);
        // c re-picks b with the same tags; the root must still re-pick.
        q.remove(a).unwrap();
        q.check_invariants();
        assert_eq!(q.dequeue(t(0)).unwrap().0, x);
    }

    #[test]
    fn hint_change_while_queued_and_active() {
        let mut q = Control::new();
        let a = q.add_session(q.root(), 1).unwrap();
        let b = q.add_session(q.root(), 1).unwrap();
        q.set_backlog(a, 100).unwrap();
        q.set_backlog(b, 100).unwrap();
        // a is the root's head, b is queued.
        q.set_backlog(a, 5000).unwrap();
        q.check_invariants();
        q.set_backlog(b, 50).unwrap();
        q.check_invariants();
        let (s, hint) = q.dequeue(t(0)).unwrap();
        // Same start tags; b now finishes first.
        assert_eq!((s, hint), (b, 50));
    }

    #[test]
    fn rate_limited_dequeue_and_timeout() {
        let mut q = Control::with_rate_limit(10_000);
        let s = q.add_session(q.root(), 1).unwrap();
        q.set_backlog(s, 4000).unwrap();
        let mut now = t(0);
        let mut sent = 0;
        while let Some((s, hint)) = q.dequeue(now) {
            q.complete(s, hint, now).unwrap();
            sent += hint;
        }
        // 4000 + 4000 < 10000, then 8000 + 4000 >= 10000.
        assert_eq!(sent, 8000);
        let wake = q.poll_timeout().unwrap();
        assert!(wake > now);
        assert_eq!(
            q.dequeue(wake - strims_base::time::Duration::NANOSECOND),
            None
        );
        now = wake;
        assert!(q.dequeue(now).is_some());
        q.set_rate_limit(0);
        q.complete(s, 4000, now).unwrap();
        assert_eq!(q.dequeue(now + strims_base::time::Duration::HOUR), None);
        assert_eq!(q.poll_timeout(), None);
    }

    #[test]
    fn set_weight_updates_share_sum() {
        let mut q = Control::new();
        let c = q.add_class(q.root(), 1).unwrap();
        let a = q.add_session(c, 1).unwrap();
        q.set_weight(a, 1000).unwrap();
        q.set_weight(c, 3).unwrap();
        assert_eq!(q.weight_of(a), Ok(1000));
        assert_eq!(q.weight_of(c), Ok(3));
        q.set_weight(a, 0).unwrap();
        assert_eq!(q.weight_of(a), Ok(MIN_WEIGHT));
        q.set_weight(a, 5000).unwrap();
        assert_eq!(q.weight_of(a), Ok(MAX_WEIGHT));
        #[allow(deprecated)]
        {
            assert_eq!(q.inverse_weight_of(a), Ok(1));
            assert_eq!(q.inverse_weight_of(c), Ok(333));
        }
        q.check_invariants();
        assert_eq!(q.parent(a), Ok(Some(c)));
        assert_eq!(q.parent(q.root()), Ok(None));
    }

    /// Random calls in random order, checking the scheduler's invariants
    /// after every call, that every class's head is its current WF2Q+ pick,
    /// and that `poll_timeout` predicts `dequeue` (and, inside `dequeue`,
    /// that `next_head` agrees with `settle`). Seeds 40 and up draw weights
    /// and lengths from small sets, so tags tie often.
    #[test]
    fn random_calls_keep_invariants() {
        for seed in 0..80 {
            let tied = seed >= 40;
            let mut rng = strims_base::rng::det_rng(seed);
            let mut r = |n: u32| rng.next_u32() % n.max(1);
            let weight = |r: &mut dyn FnMut(u32) -> u32| {
                if tied {
                    [1, 2, 1000][r(3) as usize]
                } else {
                    u64::from(r(1500))
                }
            };
            let len = |r: &mut dyn FnMut(u32) -> u32| {
                if tied {
                    [0, 500, 1000, 1000][r(4) as usize]
                } else {
                    r(3000) as usize
                }
            };
            let mut q = Control::with_rate_limit(50_000);
            let mut classes = vec![q.root()];
            let mut sessions: Vec<SessionId> = Vec::new();
            let mut now = t(0);
            for _ in 0..3000 {
                match r(14) {
                    0 if classes.len() < 8 => {
                        let p = classes[r(classes.len() as u32) as usize];
                        classes.push(q.add_class(p, weight(&mut r)).unwrap());
                    }
                    1 if sessions.len() < 24 => {
                        let p = classes[r(classes.len() as u32) as usize];
                        sessions.push(q.add_session(p, weight(&mut r)).unwrap());
                    }
                    2 if !sessions.is_empty() => {
                        let s = sessions.swap_remove(r(sessions.len() as u32) as usize);
                        q.remove(s).unwrap();
                    }
                    3 if classes.len() > 1 => {
                        let c = classes.swap_remove(1 + r(classes.len() as u32 - 1) as usize);
                        q.remove(c).unwrap();
                        classes.retain(|c| q.contains(*c));
                        sessions.retain(|s| q.contains(*s));
                    }
                    4 if !classes.is_empty() => {
                        let c = classes[r(classes.len() as u32) as usize];
                        q.set_weight(c, weight(&mut r)).unwrap();
                    }
                    5 | 6 if !sessions.is_empty() => {
                        let s = sessions[r(sessions.len() as u32) as usize];
                        q.set_backlog(s, len(&mut r)).unwrap();
                    }
                    7 if !sessions.is_empty() => {
                        let s = sessions[r(sessions.len() as u32) as usize];
                        q.clear_backlog(s).unwrap();
                    }
                    8 if !sessions.is_empty() => {
                        let s = sessions[r(sessions.len() as u32) as usize];
                        q.set_blocked(s, r(2) == 0).unwrap();
                    }
                    9 => now += strims_base::time::Duration::from_millis(i64::from(r(50))),
                    10 | 11 => {
                        let due = q.poll_timeout();
                        let idle = q.in_flight().is_none();
                        let got = q.dequeue(now);
                        if idle {
                            assert_eq!(got.is_some(), due.is_some_and(|t| t <= now));
                        }
                    }
                    _ => {
                        if let Some(s) = q.in_flight() {
                            q.complete(s, len(&mut r), now).unwrap();
                        }
                    }
                }
                q.check_invariants();
            }
        }
    }
}

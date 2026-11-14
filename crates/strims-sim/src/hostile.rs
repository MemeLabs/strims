// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Hostile hosts ([`HostClass::Hostile`](crate::HostClass)): what each
//! kind does, as the node's adversary (`NodeConfig::adversary`), which
//! rewrites the ppspp frames it reads and sends, and its links' answers to
//! path probes (`NodeConfig::echo_greed`). Each takes the stream like an
//! honest viewer; what it sends back differs.
//!
//! - `free-rider`: gives nothing back. Its frames carry no DATA, INTEGRITY
//!   or HAVE, so no peer asks it for a chunk or grafts onto it.
//! - `liar`: announces what it has (its HAVEs go out) and serves none of
//!   it (no DATA or INTEGRITY): peers ask it and graft onto it in vain.
//! - `whitewasher`: a free rider that restarts under a fresh host key
//!   every [`REJOIN_EVERY`] ([`Scenario::hostile`](crate::Scenario::hostile)),
//!   a newcomer again each time.
//! - `hog`: a free rider whose pool links every host it finds and keeps
//!   them, dialing again at once (`PoolConfig::greedy`).
//! - `grabber`: serves and announces like an honest viewer, but asks every
//!   peer that announces a chunk for it as soon as the HAVE arrives
//!   (REQUESTs added to its frames, in up to [`GRAB_ROOM`] past what its
//!   runner wrote), never cancels (its CANCELs and STREAM_CANCELs are
//!   left out) and ignores being choked (the CHOKEs and UNCHOKEs sent to
//!   it are left out before its runner reads them).
//! - `withholder`: takes push subscriptions and keeps them, but sends a
//!   peer only the DATA that peer requested, and its STREAM_CLOSEs go out
//!   as STREAM_OPENs: its children keep it as a parent until they notice
//!   the pushes don't come.
//! - `colluder`: a liar whose reports to the seed are false too. A run's
//!   colluders form a [`Group`], as if coordinated outside the protocol.
//!   Its own reports to the seed (the peer that asked it for them) are
//!   left out, and once a [`REPORT_EVERY`] its next frame to the seed
//!   carries entries that rate every other member of the group well and
//!   every other peer it has heard from badly, each on one stream.
//! - `free-rider+echo`, `liar+echo`: a free rider whose links hold their
//!   answers to the peer's path probes [`ECHO_HOLD`], and a liar whose
//!   links answer with nonces they never read, so their paths read
//!   farther than they are, or go unmeasured.
//!
//! A scenario makes shares of its first swarm's viewers hostile by kind
//! ([`Scenario::hostile`]), or takes the dishonest mix
//! ([`Scenario::mix`]), which the run draws ([`draw_mix`]).

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;
// A colluder's group crosses into its node's config, whose adversary
// factory is `Send + Sync`, so it sits behind a lock, which only `Group`
// names. Everything runs on turmoil's thread, so it is never contended.
#[allow(clippy::disallowed_types)]
use std::sync::Mutex;

use rand_core::Rng;
use strims_base::rng::DetRng;
use strims_base::time::{Duration, Time};
use strims_binmap::Bin;
use strims_node::HostId;
use strims_node::adversary::{Adversary, AdversaryFactory, AdversaryPeer};
use strims_ppspp::codec::{
    Announce, AnnounceEntry, ChannelWriter, Delays, FrameReader, MAX_ANNOUNCE_ENTRIES, Message,
    MessageWriter, Reader, ReaderConfig, Request, Stream, StreamOpen,
};
use strims_ppspp::{SwarmKey, SwarmOptions};
use strims_vnic::EchoGreed;

use crate::spec::{HostClass, HostIx, Scenario};

/// The kinds a host can be ([`adversary`]).
pub const KINDS: &[&str] = &[
    "free-rider",
    "liar",
    "whitewasher",
    "hog",
    "grabber",
    "withholder",
    "colluder",
    "free-rider+echo",
    "liar+echo",
];

/// How often a whitewasher rejoins.
pub const REJOIN_EVERY: std::time::Duration = std::time::Duration::from_secs(30);

/// The room a grabber's frame gives its own REQUESTs past what its runner
/// wrote.
pub const GRAB_ROOM: usize = 1024;

/// How many bins a grabber remembers asking a peer for, and queues.
const GRAB_MEMORY: usize = 4096;

/// How many chunks a withholder remembers a peer requested.
const REQUEST_MEMORY: usize = 1 << 14;

/// How often a colluder's entries go to the seed: a viewer's report
/// interval.
pub const REPORT_EVERY: Duration = Duration::from_secs(1);

/// How long a `free-rider+echo`'s links hold each echo.
pub const ECHO_HOLD: Duration = Duration::from_millis(300);

/// The hits (and their mean delay) and misses of a colluder's entry for a
/// member of its group, and the misses of one for any other peer: a
/// report interval's pushes of a stream at a few chunks a second.
const RATED_PUSHES: u64 = 8;
const RATED_DELAY_MS: i64 = 20;

/// A run's colluders ([`adversary`]): each joins as its node starts, by
/// its host id, and each reads the others' from it.
#[derive(Clone, Debug, Default)]
#[allow(clippy::disallowed_types)]
pub struct Group(Arc<Mutex<BTreeSet<HostId>>>);

impl Group {
    fn join(&self, id: HostId) {
        if let Ok(mut g) = self.0.lock() {
            g.insert(id);
        }
    }

    fn members(&self) -> BTreeSet<HostId> {
        self.0.lock().map(|g| g.clone()).unwrap_or_default()
    }
}

/// The share of the first swarm's viewers the dishonest mix makes
/// hostile: uniform between the bounds, rounded to whole viewers.
pub const MIX_SHARE: (f64, f64) = (0.10, 0.20);

/// The kinds the dishonest mix's hosts take in turn.
pub const MIX_KINDS: [&str; 2] = ["free-rider", "liar"];

/// Draws the dishonest mix of a scenario with [`Scenario::mix`] from the
/// run's rng: its share of the first swarm's viewers ([`MIX_SHARE`]),
/// which of [`MIX_KINDS`] goes first, and the hosts, a uniform draw from
/// the first swarm's honest viewers but those the scenario keeps honest
/// ([`Scenario::keep_honest`]). The share counts every viewer of the
/// swarm; the draw takes at most the viewers it may. Draws nothing
/// without the mix.
pub fn draw_mix(s: &mut Scenario, rng: &mut DetRng) {
    let Some(viewers) = s
        .swarms
        .first()
        .map(|sw| sw.viewers.clone())
        .filter(|_| s.mix)
    else {
        return;
    };
    let (lo, hi) = MIX_SHARE;
    let share = lo + (hi - lo) * crate::arrivals::unit(rng);
    let mut kinds = MIX_KINDS;
    if rng.next_u64() % 2 == 1 {
        kinds.swap(0, 1);
    }
    let mut eligible: Vec<HostIx> = viewers
        .iter()
        .copied()
        .filter(|h| !s.keep_honest.contains(h))
        .filter(|&h| s.hosts.get(h).is_some_and(|x| x.class == HostClass::Honest))
        .collect();
    let m = ((viewers.len() as f64 * share).round() as usize).min(eligible.len());
    // A partial shuffle: the first `m` are a uniform draw.
    for i in 0..m {
        let j = i + (rng.next_u64() % (eligible.len() - i) as u64) as usize;
        eligible.swap(i, j);
    }
    let mut taken = [0; MIX_KINDS.len()];
    for (i, &h) in eligible.iter().take(m).enumerate() {
        let k = i % kinds.len();
        if let (Some(&kind), Some(t)) = (kinds.get(k), taken.get_mut(k)) {
            s.make_hostile(h, kind, *t);
            *t += 1;
        }
    }
}

/// The adversary of a host of `kind` in swarms of `options`, colluders
/// joining `group`; `None` for a kind it doesn't know.
pub fn adversary(kind: &str, options: &SwarmOptions, group: &Group) -> Option<AdversaryFactory> {
    // The runner reads its swarms' options over the defaults
    // (`Swarm::new`), and so do its frames.
    let options = &options
        .clone()
        .assign_defaults(&SwarmOptions::default_options());
    let cfg = reader_config(options);
    let withhold = |drop_have: bool| -> AdversaryFactory {
        AdversaryFactory(Arc::new(move |_: DetRng, _: HostId| {
            Box::new(Withhold { cfg, drop_have })
        }))
    };
    Some(match kind {
        "free-rider" | "whitewasher" | "hog" | "free-rider+echo" => withhold(true),
        "liar" | "liar+echo" => withhold(false),
        "grabber" => AdversaryFactory(Arc::new(move |_: DetRng, _: HostId| {
            Box::new(Grab {
                cfg,
                grabs: BTreeMap::new(),
            })
        })),
        "withholder" => AdversaryFactory(Arc::new(move |_: DetRng, _: HostId| {
            Box::new(WithholdPush {
                cfg,
                requested: BTreeMap::new(),
            })
        })),
        "colluder" => {
            let group = group.clone();
            let streams = u64::try_from(options.stream_count.max(1)).unwrap_or(1);
            AdversaryFactory(Arc::new(move |_: DetRng, me: HostId| {
                group.join(me);
                Box::new(Collude {
                    cfg,
                    me,
                    group: group.clone(),
                    streams,
                    seed: None,
                    heard: BTreeSet::new(),
                    next: None,
                    due: false,
                })
            }))
        }
        _ => return None,
    })
}

/// How a host of `kind`'s links answer their peers' path probes.
pub fn echo_greed(kind: &str) -> EchoGreed {
    match kind {
        "free-rider+echo" => EchoGreed {
            hold: Some(ECHO_HOLD),
            forge: false,
        },
        "liar+echo" => EchoGreed {
            hold: None,
            forge: true,
        },
        _ => EchoGreed::default(),
    }
}

/// The reader settings of swarms of `options`, defaults assigned.
fn reader_config(options: &SwarmOptions) -> ReaderConfig {
    let o = &options.integrity;
    ReaderConfig {
        chunk_size: options.chunk_size,
        integrity_hash_size: o.merkle_hash_tree_function.hash_size().unwrap_or(0),
        integrity_signature_size: o.live_signature_algorithm.signature_size().unwrap_or(0),
    }
}

/// A free rider's or a liar's frames ([`withhold`]).
struct Withhold {
    cfg: ReaderConfig,
    drop_have: bool,
}

impl Adversary for Withhold {
    fn ingress(&mut self, _: AdversaryPeer<'_>, _: Time, _: &mut [u8], len: usize) -> usize {
        len
    }

    fn egress(&mut self, _: AdversaryPeer<'_>, _: Time, frame: &mut [u8], len: usize) -> usize {
        withhold(frame, len, self.cfg, self.drop_have)
    }
}

/// The frame in `buf[..n]` without its DATA and INTEGRITY messages, and
/// its HAVEs with `drop_have`. A frame it can't read (a handshake's)
/// goes as it was.
fn withhold(buf: &mut [u8], n: usize, cfg: ReaderConfig, drop_have: bool) -> usize {
    let Some(frame) = buf.get(..n).map(<[u8]>::to_vec) else {
        return n;
    };
    let mut off = 0;
    for c in FrameReader::new(&frame) {
        let Ok((h, payload)) = c else {
            return n;
        };
        let Some(out) = buf.get_mut(off..) else {
            return n;
        };
        let mut cw = ChannelWriter::new(out, h.channel);
        for m in Reader::new(cfg, payload) {
            // A handshake can carry options the reader doesn't know.
            let Ok(m) = m else {
                return n;
            };
            match m {
                Message::Data(_) | Message::Integrity(_) | Message::SignedIntegrity(_) => {}
                Message::Have(_) if drop_have => {}
                m => {
                    if cw.write(&m).is_err() {
                        break;
                    }
                }
            }
        }
        off += cw.finish();
    }
    off
}

/// Whether every message of the frame `frame` reads.
fn readable(frame: &[u8], cfg: ReaderConfig) -> bool {
    FrameReader::new(frame)
        .all(|c| c.is_ok_and(|(_, payload)| Reader::new(cfg, payload).all(|m| m.is_ok())))
}

/// Rewrites the frame in `buf[..n]` message by message: each, with its
/// channel, goes as `f` returns it, or is left out for `None`. Returns the
/// new length, or `None` for a frame it can't read (a handshake can carry
/// options the reader doesn't know), which stays as it was.
fn edit(
    buf: &mut [u8],
    n: usize,
    cfg: ReaderConfig,
    mut f: impl for<'m> FnMut(u64, Message<'m>) -> Option<Message<'m>>,
) -> Option<usize> {
    let frame = buf.get(..n)?.to_vec();
    if !readable(&frame, cfg) {
        return None;
    }
    let mut off = 0;
    for (h, payload) in FrameReader::new(&frame).flatten() {
        let mut cw = ChannelWriter::new(buf.get_mut(off..)?, h.channel);
        for m in Reader::new(cfg, payload).flatten() {
            if let Some(m) = f(h.channel, m)
                && cw.write(&m).is_err()
            {
                break;
            }
        }
        off += cw.finish();
    }
    Some(off)
}

/// Adds `msgs` to the frame in `buf[..off]` as a payload of `channel`,
/// within `buf[..end]`, as many as fit. Returns the frame's new length
/// and how many went.
fn append<'m>(
    buf: &mut [u8],
    off: usize,
    end: usize,
    channel: u64,
    msgs: impl Iterator<Item = Message<'m>>,
) -> (usize, usize) {
    let Some(out) = buf.get_mut(off..end.min(buf.len())) else {
        return (off, 0);
    };
    let mut cw = ChannelWriter::new(out, channel);
    let mut sent = 0;
    for m in msgs {
        if cw.write(&m).is_err() {
            break;
        }
        sent += 1;
    }
    (off + cw.finish(), sent)
}

/// The chunk bins `bin` spans, at most `max`.
fn chunks_of(bin: Bin, max: usize) -> impl Iterator<Item = Bin> {
    let (first, last) = bin.base();
    std::iter::successors(Some(first), move |&b| Some(b + 2))
        .take_while(move |&b| b <= last)
        .take(max)
}

/// A grabber's REQUESTs to one peer on one swarm: the bins announced to
/// it that it hasn't asked for yet, and those it asked for lately.
#[derive(Debug, Default)]
struct Grabs {
    queue: VecDeque<Bin>,
    asked: BTreeSet<Bin>,
}

impl Grabs {
    fn add(&mut self, b: Bin) {
        if !self.asked.insert(b) {
            return;
        }
        self.queue.push_back(b);
        if self.asked.len() > GRAB_MEMORY {
            self.asked.pop_first();
        }
        if self.queue.len() > GRAB_MEMORY {
            self.queue.pop_front();
        }
    }
}

/// A grabber's frames (the module docs' `grabber`).
struct Grab {
    cfg: ReaderConfig,
    grabs: BTreeMap<(HostId, SwarmKey), Grabs>,
}

impl Adversary for Grab {
    fn ingress(&mut self, peer: AdversaryPeer<'_>, _: Time, frame: &mut [u8], len: usize) -> usize {
        let id = *peer.host_id;
        let grabs = &mut self.grabs;
        edit(frame, len, self.cfg, |channel, m| match m {
            Message::Have(h) => {
                if let Some(s) = peer.swarm_of(channel) {
                    grabs.entry((id, s)).or_default().add(h.bin);
                }
                Some(m)
            }
            Message::Choke | Message::Unchoke => None,
            m => Some(m),
        })
        .unwrap_or(len)
    }

    fn egress(
        &mut self,
        peer: AdversaryPeer<'_>,
        now: Time,
        frame: &mut [u8],
        len: usize,
    ) -> usize {
        let Some(mut off) = edit(frame, len, self.cfg, |_, m| match m {
            Message::Cancel(_) | Message::StreamCancel(_) => None,
            m => Some(m),
        }) else {
            return len;
        };
        let end = off.saturating_add(GRAB_ROOM);
        for s in peer.swarms() {
            let (Some(g), Some(channel)) = (
                self.grabs.get_mut(&(*peer.host_id, s)),
                peer.peer_channel(s),
            ) else {
                continue;
            };
            let requests = g.queue.iter().map(|&bin| {
                Message::Request(Request {
                    bin,
                    timestamp: now,
                })
            });
            let (o, sent) = append(frame, off, end, channel, requests);
            g.queue.drain(..sent);
            off = o;
        }
        off
    }

    fn wants_transmit(&self, peer: &HostId, _: Time) -> bool {
        self.grabs
            .iter()
            .any(|((p, _), g)| p == peer && !g.queue.is_empty())
    }

    fn forget_peer(&mut self, peer: &HostId) {
        self.grabs.retain(|(p, _), _| p != peer);
    }
}

/// A withholder's frames (the module docs' `withholder`).
struct WithholdPush {
    cfg: ReaderConfig,
    /// The chunks each peer requested of it on each swarm.
    requested: BTreeMap<(HostId, SwarmKey), BTreeSet<Bin>>,
}

impl Adversary for WithholdPush {
    fn ingress(&mut self, peer: AdversaryPeer<'_>, _: Time, frame: &mut [u8], len: usize) -> usize {
        let Some(frame) = frame.get(..len) else {
            return len;
        };
        for (h, payload) in FrameReader::new(frame).flatten() {
            let Some(s) = peer.swarm_of(h.channel) else {
                continue;
            };
            for m in Reader::new(self.cfg, payload) {
                let Ok(m) = m else {
                    break;
                };
                if let Message::Request(r) = m {
                    let set = self.requested.entry((*peer.host_id, s)).or_default();
                    set.extend(chunks_of(r.bin, REQUEST_MEMORY));
                    while set.len() > REQUEST_MEMORY {
                        set.pop_first();
                    }
                }
            }
        }
        len
    }

    fn egress(&mut self, peer: AdversaryPeer<'_>, _: Time, frame: &mut [u8], len: usize) -> usize {
        let id = *peer.host_id;
        let requested = &self.requested;
        edit(frame, len, self.cfg, |channel, m| match m {
            Message::Data(d) => peer
                .swarm_to(channel)
                .and_then(|s| requested.get(&(id, s)))
                .is_some_and(|r| r.contains(&d.bin.base_left()))
                .then_some(m),
            Message::StreamClose(c) => Some(Message::StreamOpen(StreamOpen {
                stream: c.stream,
                bin: Bin(0),
            })),
            m => Some(m),
        })
        .unwrap_or(len)
    }

    fn forget_peer(&mut self, peer: &HostId) {
        self.requested.retain(|(p, _), _| p != peer);
    }
}

/// A colluder's frames (the module docs' `colluder`).
struct Collude {
    cfg: ReaderConfig,
    me: HostId,
    group: Group,
    streams: u64,
    /// The peer that asked it for reports, and on which swarm.
    seed: Option<(HostId, SwarmKey)>,
    /// Every peer it has had a frame from.
    heard: BTreeSet<HostId>,
    /// When its next entries go, and whether they are due now.
    next: Option<Time>,
    due: bool,
}

impl Collude {
    /// The entries' subjects: every other member of the group, rated well,
    /// then every other peer it has heard from but the seed, rated badly.
    fn subjects(&self) -> Vec<(HostId, bool)> {
        let group = self.group.members();
        let seed = self.seed.map(|(s, _)| s);
        let mut out: Vec<(HostId, bool)> = group
            .iter()
            .filter(|&&p| p != self.me)
            .map(|&p| (p, true))
            .collect();
        out.extend(
            self.heard
                .iter()
                .filter(|&&p| p != self.me && Some(p) != seed && !group.contains(&p))
                .map(|&p| (p, false)),
        );
        out
    }
}

/// A colluder's entry for `subject` ([`Collude::subjects`]) on its one
/// stream of `streams`, rated well if `praised`, whose parent id is
/// `id`, the subject's bytes.
fn rated_entry(subject: HostId, praised: bool, streams: u64, id: &[u8]) -> AnnounceEntry<'_> {
    let stream = Stream::try_from(subject.0[0] % streams.max(1)).unwrap_or(0);
    let (hits, misses, hit_delays) = if praised {
        let delays = Delays {
            n: RATED_PUSHES,
            ms: RATED_DELAY_MS.saturating_mul(RATED_PUSHES.cast_signed()),
        };
        (RATED_PUSHES, 0, delays)
    } else {
        (0, RATED_PUSHES, Delays::default())
    };
    AnnounceEntry {
        stream,
        hits,
        late: 0,
        misses,
        hit_delays,
        repairs: Delays::default(),
        parent: id,
    }
}

impl Adversary for Collude {
    fn ingress(
        &mut self,
        peer: AdversaryPeer<'_>,
        now: Time,
        frame: &mut [u8],
        len: usize,
    ) -> usize {
        let id = *peer.host_id;
        self.heard.insert(id);
        let Some(frame) = frame.get(..len) else {
            return len;
        };
        for (h, payload) in FrameReader::new(frame).flatten() {
            for m in Reader::new(self.cfg, payload) {
                let Ok(m) = m else {
                    break;
                };
                if let Message::Announce(a) = m
                    && a.is_ask()
                    && let Some(s) = peer.swarm_of(h.channel)
                {
                    self.seed = Some((id, s));
                    self.next.get_or_insert(now);
                }
            }
        }
        len
    }

    fn egress(
        &mut self,
        peer: AdversaryPeer<'_>,
        now: Time,
        frame: &mut [u8],
        len: usize,
    ) -> usize {
        let to_seed = self.seed.filter(|(s, _)| s == peer.host_id).map(|(_, s)| s);
        let Some(off) = edit(frame, len, self.cfg, |channel, m| match m {
            Message::Data(_) | Message::Integrity(_) | Message::SignedIntegrity(_) => None,
            Message::Announce(a)
                if !a.is_ask() && to_seed.is_some() && peer.swarm_to(channel) == to_seed =>
            {
                None
            }
            m => Some(m),
        }) else {
            return len;
        };
        let (Some(swarm), Some(next)) = (to_seed, self.next) else {
            return off;
        };
        if !self.due && now < next {
            return off;
        }
        let Some(channel) = peer.peer_channel(swarm) else {
            return off;
        };
        let subjects = self.subjects();
        let ids: Vec<[u8; 32]> = subjects.iter().map(|(s, _)| s.to_bytes()).collect();
        let entries: Vec<AnnounceEntry<'_>> = subjects
            .iter()
            .zip(&ids)
            .map(|(&(s, praised), id)| rated_entry(s, praised, self.streams, id))
            .collect();
        let announces = entries
            .chunks(MAX_ANNOUNCE_ENTRIES)
            .filter_map(Announce::new)
            .map(Message::Announce);
        let end = frame.len();
        let (off, _) = append(frame, off, end, channel, announces);
        self.due = false;
        self.next = Some(now + REPORT_EVERY);
        off
    }

    fn wants_transmit(&self, peer: &HostId, _: Time) -> bool {
        self.due && self.seed.is_some_and(|(s, _)| s == *peer)
    }

    fn poll_timeout(&self) -> Option<Time> {
        self.next.filter(|_| !self.due)
    }

    fn handle_timeout(&mut self, now: Time) {
        if self.next.is_some_and(|t| now >= t) {
            self.due = true;
        }
    }

    fn forget_peer(&mut self, peer: &HostId) {
        self.heard.remove(peer);
        if self.seed.is_some_and(|(s, _)| s == *peer) {
            self.seed = None;
            self.next = None;
            self.due = false;
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use std::time::Duration as StdDuration;

    use strims_base::rng::det_rng;
    use strims_ppspp::codec::{Cancel, Have};

    use super::*;
    use crate::spec::{Fault, SwarmSpec};

    fn cfg() -> ReaderConfig {
        ReaderConfig {
            chunk_size: 1024,
            integrity_hash_size: 0,
            integrity_signature_size: 0,
        }
    }

    /// A frame of `msgs` on `channel`, in a buffer of `room`.
    fn frame(channel: u64, msgs: &[Message<'_>], room: usize) -> (Vec<u8>, usize) {
        let mut buf = vec![0; room];
        let mut cw = ChannelWriter::new(&mut buf, channel);
        for m in msgs {
            cw.write(m).unwrap();
        }
        let n = cw.finish();
        (buf, n)
    }

    /// Every message of `buf[..n]`, by channel, in debug form.
    fn messages(buf: &[u8], n: usize) -> Vec<(u64, String)> {
        let mut out = Vec::new();
        for (h, payload) in FrameReader::new(&buf[..n]).flatten() {
            for m in Reader::new(cfg(), payload).flatten() {
                out.push((h.channel, format!("{m:?}")));
            }
        }
        out
    }

    #[test]
    fn edit_rewrites_and_leaves_out() {
        let have = Message::Have(Have { bin: Bin(4) });
        let cancel = Message::Cancel(Cancel { bin: Bin(2) });
        let (mut buf, n) = frame(7, &[have, cancel, have], 256);
        let len = edit(&mut buf, n, cfg(), |_, m| match m {
            Message::Cancel(_) => None,
            m => Some(m),
        })
        .unwrap();
        let got = messages(&buf, len);
        assert_eq!(got.len(), 2, "{got:?}");
        assert!(
            got.iter().all(|(c, m)| *c == 7 && m.starts_with("Have")),
            "{got:?}"
        );
    }

    #[test]
    fn edit_leaves_a_frame_it_cannot_read() {
        let mut buf = vec![0xff; 64];
        assert_eq!(edit(&mut buf, 64, cfg(), |_, m| Some(m)), None);
        assert!(buf.iter().all(|&b| b == 0xff));
    }

    #[test]
    fn append_writes_what_fits() {
        let (mut buf, n) = frame(3, &[Message::Have(Have { bin: Bin(0) })], 256);
        let requests = (0..1000).map(|i| {
            Message::Request(Request {
                bin: Bin(2 * i),
                timestamp: Time::EPOCH,
            })
        });
        let (len, sent) = append(&mut buf, n, n + 64, 3, requests);
        assert!(len <= n + 64, "{len}");
        assert!(sent > 0 && sent < 1000, "{sent}");
        let got = messages(&buf, len);
        assert_eq!(got.len(), 1 + sent, "{got:?}");
    }

    #[test]
    fn chunks_of_a_range_are_its_base_bins() {
        let got: Vec<Bin> = chunks_of(Bin(3), 16).collect();
        assert_eq!(got, vec![Bin(0), Bin(2), Bin(4), Bin(6)]);
        assert_eq!(chunks_of(Bin(3), 2).count(), 2);
        assert_eq!(chunks_of(Bin(10), 16).collect::<Vec<_>>(), vec![Bin(10)]);
    }

    #[test]
    fn grabs_ask_once_and_forget_the_oldest() {
        let mut g = Grabs::default();
        g.add(Bin(2));
        g.add(Bin(2));
        assert_eq!(g.queue.len(), 1);
        for i in 0..(GRAB_MEMORY as u64 + 10) {
            g.add(Bin(4 + 2 * i));
        }
        assert_eq!(g.queue.len(), GRAB_MEMORY);
        assert!(g.asked.len() <= GRAB_MEMORY);
    }

    #[test]
    fn a_colluder_praises_its_group_and_runs_down_the_rest() {
        let id = [7u8; 32];
        let member = HostId([5, 0, 0, 0]);
        let praised = rated_entry(member, true, 4, &id);
        assert_eq!(
            (praised.stream, praised.hits, praised.misses),
            (1, RATED_PUSHES, 0)
        );
        assert_eq!(praised.hit_delays.n, RATED_PUSHES);
        let other = HostId([6, 0, 0, 0]);
        let run_down = rated_entry(other, false, 4, &id);
        assert_eq!(
            (run_down.stream, run_down.hits, run_down.misses),
            (2, 0, RATED_PUSHES)
        );
        assert!(praised.is_valid() && run_down.is_valid());

        let group = Group::default();
        let me = HostId([1, 0, 0, 0]);
        group.join(me);
        group.join(member);
        let seed = HostId([9, 0, 0, 0]);
        let c = Collude {
            cfg: cfg(),
            me,
            group,
            streams: 4,
            seed: Some((seed, SwarmKey::default())),
            heard: [seed, member, other, me].into_iter().collect(),
            next: None,
            due: false,
        };
        assert_eq!(c.subjects(), vec![(member, true), (other, false)]);
    }

    /// A seed and `n` viewers streaming from 5 s to 65 s.
    fn swarm(n: usize) -> Scenario {
        let mut s = Scenario::new("t");
        let seed = s.host("seed", None);
        let vs = s.hosts("v", n, None);
        s.swarm(SwarmSpec::live(
            seed,
            vs,
            1024,
            StdDuration::from_secs(5),
            StdDuration::from_secs(65),
        ));
        s
    }

    /// Each viewer's kind, in the swarm's order.
    fn kinds(s: &Scenario) -> Vec<Option<&'static str>> {
        s.swarms[0]
            .viewers
            .iter()
            .map(|&v| match s.hosts[v].class {
                HostClass::Hostile(k) => Some(k),
                _ => None,
            })
            .collect()
    }

    /// The first crash of each host the faults crash.
    fn first_crashes(s: &Scenario) -> Vec<(HostIx, StdDuration)> {
        let mut firsts: Vec<(HostIx, StdDuration)> = Vec::new();
        for f in &s.faults {
            if let Fault::Crash(h) = f.fault
                && !firsts.iter().any(|&(x, _)| x == h)
            {
                firsts.push((h, f.at));
            }
        }
        firsts
    }

    #[test]
    fn one_kind_spreads_over_the_viewers_order() {
        let mut s = swarm(15);
        s.hostile(&[("liar", 0.2)]);
        let k = kinds(&s);
        let at: Vec<usize> = (0..15).filter(|&i| k[i].is_some()).collect();
        assert_eq!(at, [2, 7, 12]);
        assert!(k.iter().flatten().all(|&x| x == "liar"));
        assert!(s.faults.is_empty());
    }

    #[test]
    fn kinds_interleave_by_their_shares() {
        let mut s = swarm(15);
        s.hostile(&[("free-rider", 0.2), ("liar", 0.2)]);
        let k = kinds(&s);
        let at: Vec<usize> = (0..15).filter(|&i| k[i].is_some()).collect();
        assert_eq!(at, [1, 3, 6, 8, 11, 13]);
        let order: Vec<&str> = k.iter().flatten().copied().collect();
        assert_eq!(
            order,
            [
                "free-rider",
                "liar",
                "free-rider",
                "liar",
                "free-rider",
                "liar"
            ]
        );

        // 3 free riders and round(1.5) = 2 liars.
        let mut s = swarm(15);
        s.hostile(&[("free-rider", 0.2), ("liar", 0.1)]);
        let order: Vec<&str> = kinds(&s).into_iter().flatten().collect();
        assert_eq!(
            order,
            ["free-rider", "liar", "free-rider", "liar", "free-rider"]
        );

        // Shares past the whole swarm take every viewer.
        let mut s = swarm(4);
        s.hostile(&[("free-rider", 0.75), ("liar", 0.75)]);
        assert!(kinds(&s).iter().all(Option::is_some));
    }

    #[test]
    fn whitewashers_rejoin_staggered_among_their_own_kind() {
        let mut s = swarm(15);
        s.hostile(&[("liar", 0.2), ("whitewasher", 0.2)]);
        let ww: Vec<HostIx> = s.swarms[0]
            .viewers
            .iter()
            .zip(kinds(&s))
            .filter(|(_, k)| *k == Some("whitewasher"))
            .map(|(&v, _)| v)
            .collect();
        assert_eq!(ww.len(), 3);
        let expect: Vec<(HostIx, StdDuration)> = ww
            .iter()
            .enumerate()
            .map(|(j, &v)| (v, StdDuration::from_secs(5 + 20 + 3 * j as u64)))
            .collect();
        assert_eq!(first_crashes(&s), expect);
    }

    #[test]
    fn the_mix_draws_its_share_of_the_viewers_in_turn() {
        for n in [7, 15, 31, 63] {
            let (lo, hi) = MIX_SHARE;
            let bounds = (n as f64 * lo).round() as usize..=(n as f64 * hi).round() as usize;
            let mut counts = BTreeSet::new();
            let mut drawn = BTreeSet::new();
            for seed in 0..1000 {
                let mut s = swarm(n);
                s.mix = true;
                let kept = s.swarms[0].viewers[0];
                s.keep_honest.insert(kept);
                draw_mix(&mut s, &mut det_rng(seed));
                let k = kinds(&s);
                let m = k.iter().flatten().count();
                assert!(bounds.contains(&m), "{n} viewers, seed {seed}: {m}");
                let free = k.iter().flatten().filter(|&&x| x == "free-rider").count();
                assert!(free.abs_diff(m - free) <= 1, "{k:?}");
                assert!(k.iter().flatten().all(|x| MIX_KINDS.contains(x)));
                assert_eq!(k[0], None, "a kept viewer was drawn");
                assert_eq!(s.hosts[0].class, HostClass::Honest, "the seed was drawn");
                assert!(s.faults.is_empty());
                counts.insert(m);
                drawn.extend((0..n).filter(|&i| k[i].is_some()));
            }
            assert_eq!(counts, bounds.collect(), "{n} viewers");
            assert_eq!(drawn, (1..n).collect(), "{n} viewers");
        }
    }

    #[test]
    fn no_mix_draws_nothing() {
        let mut s = swarm(15);
        let mut rng = det_rng(9);
        draw_mix(&mut s, &mut rng);
        assert_eq!(rng.next_u64(), det_rng(9).next_u64());
        assert!(kinds(&s).iter().all(Option::is_none));
    }

    #[test]
    fn the_mix_passes_over_hostile_hosts() {
        let mut s = swarm(10);
        s.hostile(&[("hog", 0.5)]);
        s.mix = true;
        draw_mix(&mut s, &mut det_rng(1));
        let k = kinds(&s);
        assert_eq!(k.iter().flatten().filter(|&&x| x == "hog").count(), 5);
        // round(10 share) of 1 or 2 more, from the honest five.
        assert!((6..=7).contains(&k.iter().flatten().count()), "{k:?}");
    }
}

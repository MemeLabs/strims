// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The directory in an event-queue simulation with a virtual clock: one
//! [`Server`] fed by users with one to three hosts each, whose calls cross
//! a lossy channel with seeded latency and are retried; snippet publishers
//! that stream signed deltas with loss; and [`View`]s that read the
//! broadcasts through chunkstream framing in a windowed log, each from a
//! random frame-aligned start, with injected underruns that reset them.
//!
//! A run has three phases: random operations (publish, join, part,
//! unpublish, pings, snippet updates, host restarts, renames, moderation),
//! a quiet phase with pings and snippet streams only, and a frozen phase
//! with no input for a refresh interval and more. The server's and views'
//! invariants hold after every step, and at the end every view holds the
//! server's projection.
//!
//! - `STRIMS_DIRECTORY_SIM_SEEDS=<n>` runs seeds `1..=n`;
//! - `STRIMS_DIRECTORY_SIM_SEED=<s>` runs only seed `s`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

use prost::Message;
use rand_core::Rng;
use strims_base::rng::{DetRng, det_rng, fork};
use strims_base::time::{Duration, Time};
use strims_chunkstream::{RecordDecoder, RecordEncoder};
use strims_crypto::{
    DEFAULT_CERT_TTL, SignableMessage, generate_key, host_certificate, new_certificate_request,
    sign_certificate_request_with_network,
};
use strims_dao::schema::directory::{listing_key, new_user_record};
use strims_ppspp::{SwarmId, Uri, UriOptions};
use strims_proto::network::v1::directory::listing::{Chat, Content, Embed, Media, Service};
use strims_proto::network::v1::directory::listing_query::Query;
use strims_proto::network::v1::directory::listing_snippet_image::SourceOneof;
use strims_proto::network::v1::directory::{
    Listing, ListingModeration, ListingQuery, ListingRecord, ListingSnippet, ListingSnippetDelta,
    ListingSnippetImage, ServerConfig, UserModeration, UserRecord,
};
use strims_proto::types::{Certificate, Image, Key, KeyUsage};

use crate::moderation::{is_admin, is_moderator, merge_listing, merge_user};
use crate::snippet::{diff, is_empty, split};
use crate::view::Projection;
use crate::{
    Caller, DirectoryError, EVENT_FRAME, Limits, LocalSnippets, Server, ServerAction, SubId, View,
    ViewEvent, ViewLimits, listing,
};

const T0: Time = Time::from_unix_millis(1_700_000_000_000);
const NET_ID: u64 = 7;
/// The event swarm's window.
const WINDOW: usize = 2 * 1024 * 1024;
const MAX_RECORD: usize = 2 * 1024 * 1024;
const LOSS_PERCENT: u64 = 5;
const READERS: usize = 4;
const USERS: usize = 5;
const SWARMS: usize = 6;
const ATTEMPT_TIMEOUT: Duration = Duration::SECOND;
const RETRIES: u32 = 3;

/// The run's phases.
#[derive(Clone, Copy, Debug)]
struct Plan {
    ops: Duration,
    quiet: Duration,
    frozen: Duration,
}

impl Plan {
    fn full() -> Plan {
        Plan {
            ops: Duration::MINUTE * 30,
            quiet: Duration::MINUTE * 15,
            frozen: Duration::from_secs(95),
        }
    }

    fn short() -> Plan {
        Plan {
            ops: Duration::MINUTE * 5,
            quiet: Duration::MINUTE * 3,
            frozen: Duration::from_secs(95),
        }
    }
}

fn server_config() -> ServerConfig {
    ServerConfig {
        broadcast_interval: 1,
        refresh_interval: 60,
        session_timeout: 600,
        min_ping_interval: 60,
        max_ping_interval: 120,
        load_media_embed_timeout: 10,
        publish_quota: 4,
        join_quota: 6,
        ..ServerConfig::default()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Ops,
    Quiet,
    Frozen,
}

#[derive(Clone, Debug)]
enum Call {
    Publish(Listing),
    Unpublish(u64),
    Join(ListingQuery),
    Part(u64),
    Ping,
    ModerateListing(u64, ListingModeration),
    ModerateUser(Vec<u8>, UserModeration),
}

#[derive(Clone, Debug)]
enum Ev {
    Request {
        host: usize,
        call: u64,
        req: Call,
    },
    Reply {
        host: usize,
        epoch: u64,
        call: u64,
        result: Result<u64, DirectoryError>,
    },
    Retry {
        host: usize,
        epoch: u64,
        call: u64,
        attempt: u32,
    },
    Delta {
        sub: SubId,
        delta: Box<ListingSnippetDelta>,
    },
    Ended {
        sub: SubId,
    },
    Read {
        reader: usize,
    },
    Ping {
        host: usize,
        epoch: u64,
    },
    Up {
        host: usize,
    },
    Op,
}

struct UserSim {
    profile: Key,
    alias: String,
    membership: Certificate,
    hosts: Vec<usize>,
}

struct HostSim {
    user: usize,
    key: Key,
    caller: Caller,
    up: bool,
    epoch: u64,
    pending: Option<(u64, Call)>,
    published: BTreeSet<u64>,
    joined: BTreeSet<u64>,
    swarms: Vec<usize>,
    snippets: LocalSnippets,
}

struct SwarmSim {
    id: SwarmId,
    key: Key,
    version: u32,
}

/// A publisher's snippet stream to the server.
struct Stream {
    host: usize,
    swarm: usize,
    sent: ListingSnippet,
    last: Time,
}

/// The event swarm: records in frames, keeping the last [`WINDOW`]
/// bytes.
#[derive(Default)]
struct Log {
    records: VecDeque<(u64, Vec<u8>)>,
    start: u64,
    end: u64,
    bytes: usize,
}

impl Log {
    fn append(&mut self, frames: Vec<u8>) {
        let n = frames.len();
        self.records.push_back((self.end, frames));
        self.end += n as u64;
        self.bytes += n;
        while self.bytes > WINDOW {
            let (_, r) = self.records.pop_front().unwrap();
            self.bytes -= r.len();
            self.start = self.records.front().map_or(self.end, |(o, _)| *o);
        }
    }

    /// A random frame-aligned offset in the window.
    fn random_start(&self, rng: &mut DetRng) -> u64 {
        let frames = (self.end - self.start) / EVENT_FRAME as u64;
        self.start + (rng.next_u64() % (frames + 1)) * EVENT_FRAME as u64
    }

    /// Bytes from `offset` to the end of its record, at most `max`; none
    /// if `offset` left the window.
    fn read(&self, offset: u64, max: usize) -> Option<&[u8]> {
        if offset < self.start {
            return None;
        }
        let i = self.records.partition_point(|(o, _)| *o <= offset);
        let (o, r) = self.records.get(i.checked_sub(1)?)?;
        let from = usize::try_from(offset - o).unwrap();
        let to = r.len().min(from.saturating_add(max));
        r.get(from..to)
    }
}

struct Reader {
    view: View,
    decoder: RecordDecoder,
    offset: u64,
    started: bool,
}

/// A rolling hash of the trace, and its tail.
#[derive(Default)]
struct Trace {
    hash: u64,
    lines: u64,
    tail: VecDeque<String>,
}

impl Trace {
    fn push(&mut self, line: String) {
        if self.lines == 0 {
            self.hash = 0xcbf2_9ce4_8422_2325;
        }
        for b in line.bytes().chain(*b"\n") {
            self.hash ^= u64::from(b);
            self.hash = self.hash.wrapping_mul(0x0100_0000_01b3);
        }
        self.lines += 1;
        self.tail.push_back(line);
        if self.tail.len() > 60 {
            self.tail.pop_front();
        }
    }
}

#[derive(Debug, Default)]
struct Stats {
    calls: u64,
    refused: BTreeMap<String, u64>,
    deltas_sent: u64,
    deltas_lost: u64,
    records: u64,
    underruns: u64,
    restarts: u64,
    renames: u64,
    moves: u64,
    lagged: u64,
}

struct Sim {
    plan: Plan,
    now: Time,
    phase: Phase,
    rng: DetRng,
    server: Server,
    queue: BTreeMap<(Time, u64), Ev>,
    seq: u64,
    next_call: u64,
    network: Key,
    users: Vec<UserSim>,
    hosts: Vec<HostSim>,
    swarms: Vec<SwarmSim>,
    store_users: BTreeMap<Vec<u8>, UserRecord>,
    store_listings: BTreeMap<Vec<u8>, ListingRecord>,
    next_record: u64,
    streams: BTreeMap<SubId, Stream>,
    log: Log,
    encoder: RecordEncoder,
    readers: Vec<Reader>,
    trace: Trace,
    stats: Stats,
}

struct Report {
    hash: u64,
    lines: u64,
    listings: u64,
    shown: u64,
    summary: String,
}

impl Sim {
    fn new(seed: u64, plan: Plan) -> Sim {
        let mut rng = det_rng(seed);
        let network = generate_key(&mut rng);
        let limits = Limits {
            max_sessions_per_user: 2,
            ..Limits::default()
        };
        let server = Server::new(limits, Some(&server_config()), fork(&mut rng), T0);
        let mut sim = Sim {
            plan,
            now: T0,
            phase: Phase::Ops,
            rng,
            server,
            queue: BTreeMap::new(),
            seq: 0,
            next_call: 1,
            network,
            users: Vec::new(),
            hosts: Vec::new(),
            swarms: Vec::new(),
            store_users: BTreeMap::new(),
            store_listings: BTreeMap::new(),
            next_record: 1,
            streams: BTreeMap::new(),
            log: Log::default(),
            encoder: RecordEncoder::new(EVENT_FRAME).unwrap(),
            readers: Vec::new(),
            trace: Trace::default(),
            stats: Stats::default(),
        };
        for u in 0..USERS {
            let profile = generate_key(&mut sim.rng);
            let alias = format!("user{u}");
            let membership = sim.membership(&profile, &alias);
            sim.users.push(UserSim {
                profile,
                alias,
                membership,
                hosts: Vec::new(),
            });
            for _ in 0..=sim.rng.next_u64() % 3 {
                let h = sim.hosts.len();
                let key = generate_key(&mut sim.rng);
                let caller = sim.caller(u, &key);
                sim.users[u].hosts.push(h);
                sim.hosts.push(HostSim {
                    user: u,
                    key,
                    caller,
                    up: true,
                    epoch: 0,
                    pending: None,
                    published: BTreeSet::new(),
                    joined: BTreeSet::new(),
                    swarms: Vec::new(),
                    snippets: LocalSnippets::new(),
                });
            }
        }
        // the owner is the directory's admin
        let admin = sim.users[0].profile.public.clone();
        sim.store_users.insert(
            admin.clone(),
            new_user_record(
                0,
                NET_ID,
                &admin,
                UserModeration {
                    is_admin: Some(true),
                    is_moderator: Some(true),
                    ..Default::default()
                },
            ),
        );
        for s in 0..SWARMS {
            let key = generate_key(&mut sim.rng);
            sim.swarms.push(SwarmSim {
                id: SwarmId::new(&key.public),
                key,
                version: 0,
            });
            let owners = 1 + sim.rng.next_u64() % 2;
            for _ in 0..owners {
                let h = sim.pick(sim.hosts.len());
                if !sim.hosts[h].swarms.contains(&s) {
                    sim.hosts[h].swarms.push(s);
                    let snippet = sim.make_snippet(s);
                    let id = sim.swarms[s].id.clone();
                    sim.hosts[h].snippets.push(id, snippet).unwrap();
                }
            }
        }
        for i in 0..READERS {
            sim.readers.push(Reader {
                view: View::new(ViewLimits::default()),
                decoder: RecordDecoder::new(EVENT_FRAME, MAX_RECORD).unwrap(),
                offset: 0,
                started: false,
            });
            let start = if i == 0 {
                T0
            } else {
                T0 + Duration::from_millis(sim.range(0, 10 * 60 * 1000))
            };
            sim.at(start, Ev::Read { reader: i });
        }
        for h in 0..sim.hosts.len() {
            let t = T0 + Duration::from_millis(sim.range(0, 5000));
            sim.at(t, Ev::Ping { host: h, epoch: 0 });
        }
        sim.at(T0, Ev::Op);
        sim
    }

    fn membership(&mut self, profile: &Key, alias: &str) -> Certificate {
        let csr = new_certificate_request(
            profile,
            KeyUsage::Peer as i32 | KeyUsage::Sign as i32,
            alias,
        )
        .unwrap();
        sign_certificate_request_with_network(
            &csr,
            &self.network,
            "net",
            DEFAULT_CERT_TTL,
            T0,
            &mut self.rng,
        )
        .unwrap()
    }

    fn caller(&self, user: usize, key: &Key) -> Caller {
        let u = &self.users[user];
        let cert = host_certificate(&key.public, &u.profile, &u.membership).unwrap();
        Caller::from_certificate(&cert).unwrap()
    }

    fn pick(&mut self, n: usize) -> usize {
        usize::try_from(self.rng.next_u64() % n.max(1) as u64).unwrap()
    }

    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + i64::try_from(self.rng.next_u64() % u64::try_from(hi - lo + 1).unwrap()).unwrap()
    }

    fn lost(&mut self) -> bool {
        self.rng.next_u64() % 100 < LOSS_PERCENT
    }

    fn latency(&mut self) -> Duration {
        Duration::from_millis(self.range(5, 200))
    }

    fn at(&mut self, t: Time, ev: Ev) {
        self.seq += 1;
        self.queue.insert((t, self.seq), ev);
    }

    fn make_snippet(&mut self, s: usize) -> ListingSnippet {
        self.swarms[s].version += 1;
        let v = self.swarms[s].version;
        let image = |rng: &mut DetRng, max: u64| {
            let mut data = vec![0; usize::try_from(1 + rng.next_u64() % max).unwrap()];
            rng.fill_bytes(&mut data);
            ListingSnippetImage {
                source_oneof: Some(SourceOneof::Image(Image {
                    r#type: 4,
                    data,
                    ..Default::default()
                })),
            }
        };
        let tags = (0..self.rng.next_u64() % 3)
            .map(|i| format!("t{i}"))
            .collect();
        let thumbnail = match self.rng.next_u64() % 4 {
            0 => None,
            _ => Some(image(&mut self.rng, 3000)),
        };
        let channel_logo = match self.rng.next_u64() % 3 {
            0 => None,
            1 => Some(ListingSnippetImage {
                source_oneof: Some(SourceOneof::Url(format!("https://x/{s}/{v}.png"))),
            }),
            _ => Some(image(&mut self.rng, 1500)),
        };
        let mut snippet = ListingSnippet {
            title: format!("swarm {s} v{v}"),
            live: self.rng.next_u64().is_multiple_of(2),
            user_count: self.rng.next_u64() % 100,
            tags,
            thumbnail,
            channel_logo,
            ..Default::default()
        };
        snippet.sign_message(&self.swarms[s].key).unwrap();
        snippet
    }

    fn media(&self, s: usize) -> Listing {
        Listing {
            content: Some(Content::Media(Media {
                mime_type: "video/mp4".into(),
                swarm_uri: Uri::new(self.swarms[s].id.clone(), UriOptions::default()).to_string(),
            })),
        }
    }

    fn pool_listing(&mut self, h: usize) -> Listing {
        match self.rng.next_u64() % 20 {
            0..=9 => chat(self.pick(10)),
            10..=12 => Listing {
                content: Some(Content::Service(Service {
                    r#type: "svc".into(),
                    swarm_uri: format!("svc-{}", self.pick(2)),
                })),
            },
            13..=18 => {
                let swarms = self.hosts[h].swarms.clone();
                let s = if swarms.is_empty() {
                    self.pick(SWARMS)
                } else {
                    swarms[self.pick(swarms.len())]
                };
                self.media(s)
            }
            _ => Listing {
                content: Some(Content::Embed(Embed {
                    service: 1,
                    id: "x".into(),
                    ..Default::default()
                })),
            },
        }
    }

    fn some_id(&mut self, believed: &BTreeSet<u64>) -> u64 {
        if !believed.is_empty() && !self.rng.next_u64().is_multiple_of(4) {
            let i = self.pick(believed.len());
            return *believed.iter().nth(i).unwrap();
        }
        let ids = self.server.listing_ids();
        if !ids.is_empty() && !self.rng.next_u64().is_multiple_of(3) {
            return ids[self.pick(ids.len())];
        }
        1 + self.rng.next_u64() % 40
    }

    // --- the loop -------------------------------------------------------

    fn run(mut self) -> Report {
        let ops_end = T0 + self.plan.ops;
        let quiet_end = ops_end + self.plan.quiet;
        let end = quiet_end + self.plan.frozen;
        loop {
            let tq = self.queue.first_key_value().map(|((t, _), _)| *t);
            let ts = self.server.poll_timeout();
            let next = match (tq, ts) {
                (Some(a), Some(b)) => a.min(b),
                (a, b) => a.or(b).unwrap(),
            };
            if next > end {
                break;
            }
            self.now = next;
            self.phase = if next < ops_end {
                Phase::Ops
            } else if next < quiet_end {
                Phase::Quiet
            } else {
                Phase::Frozen
            };
            if ts == Some(next) {
                self.server.handle_timeout(next);
                self.drain();
                if (next - T0).as_millis() % 1000 == 0 {
                    self.check_bans();
                }
            } else {
                let (_, ev) = self.queue.pop_first().unwrap();
                self.handle(ev);
            }
            self.server.check().unwrap();
            self.check_moderation();
        }
        self.now = end;
        self.finish()
    }

    fn handle(&mut self, ev: Ev) {
        match ev {
            Ev::Request { host, call, req } => {
                if self.phase == Phase::Frozen {
                    return;
                }
                let result = self.serve(host, &req);
                self.trace.push(format!(
                    "{} serve h{host} c{call} {} -> {result:?}",
                    self.stamp(),
                    describe(&req)
                ));
                self.drain();
                if let Err(e) = &result {
                    *self.stats.refused.entry(format!("{e:?}")).or_default() += 1;
                }
                if !self.lost() {
                    let t = self.now + self.latency();
                    let epoch = self.hosts[host].epoch;
                    self.at(
                        t,
                        Ev::Reply {
                            host,
                            epoch,
                            call,
                            result,
                        },
                    );
                }
            }
            Ev::Reply {
                host,
                epoch,
                call,
                result,
            } => self.replied(host, epoch, call, result),
            Ev::Retry {
                host,
                epoch,
                call,
                attempt,
            } => {
                let h = &mut self.hosts[host];
                if h.epoch != epoch || h.pending.as_ref().map(|p| p.0) != Some(call) {
                    return;
                }
                if attempt > RETRIES {
                    h.pending = None;
                    return;
                }
                let req = h.pending.as_ref().unwrap().1.clone();
                self.send_request(host, call, req);
                let t = self.now + ATTEMPT_TIMEOUT;
                self.at(
                    t,
                    Ev::Retry {
                        host,
                        epoch,
                        call,
                        attempt: attempt + 1,
                    },
                );
            }
            Ev::Delta { sub, delta } => {
                if self.phase == Phase::Frozen || !self.streams.contains_key(&sub) {
                    return;
                }
                self.server.snippet_delta(self.now, sub, &delta);
                self.drain();
            }
            Ev::Ended { sub } => {
                if self.phase == Phase::Frozen {
                    return;
                }
                self.server.snippet_ended(self.now, sub);
                self.drain();
            }
            Ev::Read { reader } => {
                let max = 1 + usize::try_from(self.rng.next_u64() % 16_384).unwrap();
                self.read(reader, max);
                let t = self.now + Duration::from_millis(self.range(100, 1000));
                self.at(t, Ev::Read { reader });
            }
            Ev::Ping { host, epoch } => {
                let h = &self.hosts[host];
                if self.phase == Phase::Frozen || !h.up || h.epoch != epoch {
                    return;
                }
                if h.pending.is_some() {
                    self.at(self.now + Duration::SECOND, Ev::Ping { host, epoch });
                    return;
                }
                self.call(host, Call::Ping);
                let t = self.now + Duration::from_millis(self.range(60_000, 110_000));
                self.at(t, Ev::Ping { host, epoch });
            }
            Ev::Up { host } => {
                self.hosts[host].up = true;
                let epoch = self.hosts[host].epoch;
                let t = self.now + Duration::from_millis(self.range(0, 5000));
                self.at(t, Ev::Ping { host, epoch });
                self.trace.push(format!("{} up h{host}", self.stamp()));
            }
            Ev::Op => {
                if self.phase != Phase::Ops {
                    return;
                }
                self.op();
                let t = self.now + Duration::from_millis(self.range(100, 2000));
                self.at(t, Ev::Op);
            }
        }
    }

    fn stamp(&self) -> i64 {
        (self.now - T0).as_millis()
    }

    fn call(&mut self, host: usize, req: Call) {
        if !self.hosts[host].up || self.hosts[host].pending.is_some() {
            return;
        }
        let call = self.next_call;
        self.next_call += 1;
        self.stats.calls += 1;
        self.hosts[host].pending = Some((call, req.clone()));
        self.send_request(host, call, req);
        let epoch = self.hosts[host].epoch;
        let t = self.now + ATTEMPT_TIMEOUT;
        self.at(
            t,
            Ev::Retry {
                host,
                epoch,
                call,
                attempt: 1,
            },
        );
    }

    fn send_request(&mut self, host: usize, call: u64, req: Call) {
        if self.lost() {
            return;
        }
        let t = self.now + self.latency();
        self.at(t, Ev::Request { host, call, req });
    }

    fn replied(&mut self, host: usize, epoch: u64, call: u64, result: Result<u64, DirectoryError>) {
        let h = &mut self.hosts[host];
        if h.epoch != epoch || h.pending.as_ref().map(|p| p.0) != Some(call) {
            return;
        }
        let (_, req) = h.pending.take().unwrap();
        match (req, result) {
            (Call::Publish(_), Ok(id)) => {
                h.published.insert(id);
            }
            (Call::Join(_), Ok(id)) => {
                h.joined.insert(id);
            }
            (Call::Unpublish(id), _) => {
                h.published.remove(&id);
            }
            (Call::Part(id), _) => {
                h.joined.remove(&id);
            }
            (_, Err(DirectoryError::SessionNotFound)) => {
                h.published.clear();
                h.joined.clear();
            }
            _ => {}
        }
    }

    /// What the server's control does for a call.
    fn serve(&mut self, host: usize, req: &Call) -> Result<u64, DirectoryError> {
        let c = self.hosts[host].caller.clone();
        let record = self.store_users.get(&c.peer_key).cloned();
        let user = record.as_ref().and_then(|r| r.moderation);
        let now = self.now;
        match req {
            Call::Publish(l) => {
                let key = listing_key(NET_ID, Some(l));
                let lr = self
                    .store_listings
                    .get(&key)
                    .and_then(|r| r.moderation.clone());
                self.server
                    .publish(now, &c, l.clone(), user.as_ref(), lr.as_ref())
            }
            Call::Join(q) => self.server.join(now, &c, q, user.as_ref()),
            Call::Part(id) => self.server.part(now, &c, *id).map(|()| 0),
            Call::Unpublish(id) => self.server.unpublish(now, &c, *id).map(|()| 0),
            Call::Ping => self.server.ping(now, &c).map(|()| 0),
            Call::ModerateListing(id, m) => {
                if !is_moderator(record.as_ref()) {
                    return Err(DirectoryError::PermissionDenied);
                }
                listing::validate_moderation(m)?;
                let l = self
                    .server
                    .listing(*id)
                    .cloned()
                    .ok_or(DirectoryError::ListingNotFound)?;
                let key = listing_key(NET_ID, Some(&l));
                let existing = self.store_listings.get(&key).cloned();
                let r = merge_listing(existing, Some(m), self.next_record, NET_ID, &l);
                self.next_record += 1;
                self.store_listings.insert(key, r.clone());
                self.server.listing_moderated(now, *id, r.moderation);
                Ok(0)
            }
            Call::ModerateUser(peer, m) => {
                if !is_moderator(record.as_ref()) {
                    return Err(DirectoryError::PermissionDenied);
                }
                let existing = self.store_users.get(peer).cloned();
                let r = merge_user(
                    existing,
                    Some(m),
                    is_admin(record.as_ref()),
                    self.next_record,
                    NET_ID,
                    peer,
                );
                self.next_record += 1;
                self.store_users.insert(peer.clone(), r.clone());
                self.server
                    .user_moderated(now, peer, &r.moderation.unwrap_or_default());
                Ok(0)
            }
        }
    }

    fn drain(&mut self) {
        while let Some(a) = self.server.poll_action() {
            match a {
                ServerAction::Broadcast(records) => {
                    for r in records {
                        let payload = r.encode_to_vec();
                        assert!(payload.len() <= MAX_RECORD);
                        let n = self.encoder.encoded_len(payload.len()).unwrap();
                        let mut frames = vec![0; n];
                        self.encoder.encode(&payload, &mut frames).unwrap();
                        self.log.append(frames);
                        self.stats.records += 1;
                    }
                }
                ServerAction::Subscribe {
                    sub,
                    listing_id,
                    host,
                    swarm_id,
                } => {
                    self.trace.push(format!(
                        "{} subscribe {} l{listing_id} {host}",
                        self.stamp(),
                        sub.0
                    ));
                    self.subscribe(sub, host, &swarm_id);
                }
                ServerAction::Unsubscribe { sub } => {
                    self.trace
                        .push(format!("{} unsubscribe {}", self.stamp(), sub.0));
                    self.streams.remove(&sub);
                }
            }
        }
    }

    fn subscribe(&mut self, sub: SubId, host: strims_kademlia::ID, swarm_id: &SwarmId) {
        if self.phase == Phase::Frozen {
            return;
        }
        let h = self.hosts.iter().position(|h| h.caller.host == host);
        let snippet = h.and_then(|h| {
            let hs = &self.hosts[h];
            let swarm = self.swarms.iter().position(|s| &s.id == swarm_id)?;
            let (s, _) = hs.snippets.get(swarm_id)?;
            hs.up.then(|| (h, swarm, s.clone()))
        });
        let Some((h, swarm, snippet)) = snippet else {
            let t = self.now + self.latency();
            self.at(t, Ev::Ended { sub });
            return;
        };
        self.streams.insert(
            sub,
            Stream {
                host: h,
                swarm,
                sent: ListingSnippet::default(),
                last: self.now,
            },
        );
        self.stream_to(sub, snippet);
    }

    /// Sends stream `sub` the deltas to `snippet`.
    fn stream_to(&mut self, sub: SubId, snippet: ListingSnippet) {
        let Some(st) = self.streams.get(&sub) else {
            return;
        };
        let deltas = split(diff(&st.sent, &snippet));
        let mut last = st.last;
        for delta in deltas.into_iter().filter(|d| !is_empty(d)) {
            self.stats.deltas_sent += 1;
            if self.lost() {
                self.stats.deltas_lost += 1;
                continue;
            }
            let t = (self.now + self.latency()).max(last + Duration::MILLISECOND);
            last = t;
            self.at(
                t,
                Ev::Delta {
                    sub,
                    delta: Box::new(delta),
                },
            );
        }
        if let Some(st) = self.streams.get_mut(&sub) {
            st.sent = snippet;
            st.last = last;
        }
    }

    fn read(&mut self, i: usize, max: usize) {
        let inject = self.phase != Phase::Frozen && self.rng.next_u64() % 1000 < 4;
        let Sim {
            readers,
            log,
            rng,
            trace,
            stats,
            ..
        } = self;
        let r = &mut readers[i];
        if !r.started {
            r.started = true;
            r.offset = log.random_start(rng);
            r.decoder.reset(r.offset);
        }
        if inject || r.offset < log.start {
            stats.underruns += 1;
            r.view.reset();
            let from = r.offset.max(log.start);
            let frames = (log.end - from) / EVENT_FRAME as u64;
            r.offset = from + (rng.next_u64() % (frames + 1)) * EVENT_FRAME as u64;
            if r.offset < from {
                r.offset = log.random_start(rng);
            }
            r.offset = r.offset - r.offset % EVENT_FRAME as u64;
            r.decoder.reset(r.offset);
        }
        let mut left = max;
        while left > 0 {
            let Some(bytes) = log.read(r.offset, left) else {
                break;
            };
            if bytes.is_empty() {
                break;
            }
            let mut input = bytes;
            while !input.is_empty() {
                let d = r.decoder.decode(input);
                input = &input[d.consumed..];
                if d.record {
                    r.view.handle_record(r.decoder.record());
                }
            }
            r.offset += bytes.len() as u64;
            left -= bytes.len();
        }
        r.view.check().unwrap();
        while let Some(e) = r.view.poll_event() {
            if e == ViewEvent::Lagged {
                stats.lagged += 1;
            }
            trace.push(format!("v{i} {}", event_kind(&e)));
        }
    }

    // --- operations -----------------------------------------------------

    fn op(&mut self) {
        let up: Vec<usize> = (0..self.hosts.len())
            .filter(|h| self.hosts[*h].up)
            .collect();
        if up.is_empty() {
            return;
        }
        let h = up[self.pick(up.len())];
        let r = self.rng.next_u64() % 100;
        match r {
            0..=24 => {
                let l = self.pool_listing(h);
                self.call(h, Call::Publish(l));
            }
            25..=41 => {
                let q = if self.rng.next_u64() % 5 < 3 {
                    let id = self.some_id(&BTreeSet::new());
                    Query::Id(id)
                } else {
                    Query::Listing(self.pool_listing(h))
                };
                self.call(h, Call::Join(ListingQuery { query: Some(q) }));
            }
            42..=51 => {
                let joined = self.hosts[h].joined.clone();
                let id = self.some_id(&joined);
                self.call(h, Call::Part(id));
            }
            52..=61 => {
                let published = self.hosts[h].published.clone();
                let id = self.some_id(&published);
                self.call(h, Call::Unpublish(id));
            }
            62..=71 => self.update_snippet(),
            72..=74 => self.restart(h),
            75 => self.rename(self.hosts[h].user),
            76 => self.move_host(h),
            77..=87 => {
                let by = self.moderator(h);
                let id = self.some_id(&BTreeSet::new());
                let m = ListingModeration {
                    is_banned: match self.rng.next_u64() % 10 {
                        0 => Some(true),
                        1..=3 => Some(false),
                        _ => None,
                    },
                    is_mature: self
                        .rng
                        .next_u64()
                        .is_multiple_of(2)
                        .then(|| self.rng.next_u64().is_multiple_of(2)),
                    category: self
                        .rng
                        .next_u64()
                        .is_multiple_of(3)
                        .then(|| format!("c{}", self.rng.next_u64() % 3)),
                };
                self.call(by, Call::ModerateListing(id, m));
            }
            88..=94 => {
                let by = self.moderator(h);
                let target = self.pick(USERS);
                let peer = self.users[target].profile.public.clone();
                let flag = |rng: &mut DetRng| match rng.next_u64() % 6 {
                    0 => Some(true),
                    1..=3 => Some(false),
                    _ => None,
                };
                let m = UserModeration {
                    disable_join: flag(&mut self.rng),
                    disable_publish: flag(&mut self.rng),
                    is_moderator: flag(&mut self.rng),
                    is_admin: None,
                };
                self.call(by, Call::ModerateUser(peer, m));
            }
            _ => self.call(h, Call::Ping),
        }
    }

    /// The host that moderates: mostly the admin's, else `h`.
    fn moderator(&mut self, h: usize) -> usize {
        let admins: Vec<usize> = self.users[0]
            .hosts
            .iter()
            .copied()
            .filter(|a| self.hosts[*a].up)
            .collect();
        if !admins.is_empty() && self.rng.next_u64() % 10 < 6 {
            admins[self.pick(admins.len())]
        } else {
            h
        }
    }

    fn update_snippet(&mut self) {
        let owners: Vec<(usize, usize)> = (0..self.hosts.len())
            .flat_map(|h| self.hosts[h].swarms.iter().map(move |s| (h, *s)))
            .collect();
        if owners.is_empty() {
            return;
        }
        let (h, s) = owners[self.pick(owners.len())];
        let snippet = self.make_snippet(s);
        let id = self.swarms[s].id.clone();
        self.hosts[h].snippets.push(id, snippet.clone()).unwrap();
        let subs: Vec<SubId> = self
            .streams
            .iter()
            .filter(|(_, st)| st.host == h && st.swarm == s)
            .map(|(sub, _)| *sub)
            .collect();
        self.trace.push(format!(
            "{} snippet h{h} s{s} v{} to {subs:?}",
            self.stamp(),
            self.swarms[s].version
        ));
        for sub in subs {
            self.stream_to(sub, snippet.clone());
        }
    }

    fn restart(&mut self, h: usize) {
        self.stats.restarts += 1;
        let host = &mut self.hosts[h];
        host.up = false;
        host.epoch += 1;
        host.pending = None;
        host.published.clear();
        host.joined.clear();
        let subs: Vec<SubId> = self
            .streams
            .iter()
            .filter(|(_, st)| st.host == h)
            .map(|(sub, _)| *sub)
            .collect();
        for sub in subs {
            self.streams.remove(&sub);
            let t = self.now + self.latency();
            self.at(t, Ev::Ended { sub });
        }
        // back before the quiet phase ends, so no session expires frozen
        let latest = T0 + self.plan.ops + self.plan.quiet - Duration::MINUTE * 5;
        let down = Duration::from_millis(self.range(10_000, 15 * 60 * 1000));
        let t = (self.now + down).min(latest).max(self.now);
        self.at(t, Ev::Up { host: h });
        self.trace.push(format!("{} restart h{h}", self.stamp()));
    }

    /// Host `h` starts calling as another user.
    fn move_host(&mut self, h: usize) {
        let from = self.hosts[h].user;
        let to = (from + 1 + self.pick(USERS - 1)) % USERS;
        self.users[from].hosts.retain(|x| *x != h);
        self.users[to].hosts.push(h);
        self.hosts[h].user = to;
        let key = self.hosts[h].key.clone();
        self.hosts[h].caller = self.caller(to, &key);
        self.hosts[h].published.clear();
        self.hosts[h].joined.clear();
        self.stats.moves += 1;
        self.trace
            .push(format!("{} move h{h} u{from} -> u{to}", self.stamp()));
    }

    fn rename(&mut self, u: usize) {
        self.stats.renames += 1;
        let alias = format!("user{u}.{}", self.stats.renames);
        let profile = self.users[u].profile.clone();
        self.users[u].membership = self.membership(&profile, &alias);
        self.users[u].alias = alias;
        for h in self.users[u].hosts.clone() {
            let key = self.hosts[h].key.clone();
            self.hosts[h].caller = self.caller(u, &key);
        }
    }

    // --- checks ---------------------------------------------------------

    /// Right after a broadcast no listing with a banned record is live.
    fn check_bans(&self) {
        for id in self.server.listing_ids() {
            let l = self.server.listing(id).unwrap();
            let banned = self
                .store_listings
                .get(&listing_key(NET_ID, Some(l)))
                .and_then(|r| r.moderation.as_ref())
                .and_then(|m| m.is_banned);
            assert_ne!(banned, Some(true), "listing {id} is banned but live");
        }
    }

    /// Users whose joins or publishing are disabled hold no such
    /// memberships.
    fn check_moderation(&self) {
        for (peer, r) in &self.store_users {
            let Some(m) = &r.moderation else {
                continue;
            };
            for (published, viewed) in self.server.user_sessions(peer).values() {
                if m.disable_publish == Some(true) {
                    assert!(
                        published.is_empty(),
                        "a user with publishing disabled publishes"
                    );
                }
                if m.disable_join == Some(true) {
                    assert!(viewed.is_empty(), "a user with joins disabled views");
                }
            }
        }
    }

    fn finish(mut self) -> Report {
        for i in 0..READERS {
            self.read(i, usize::MAX);
        }
        assert!(self.server.settled(), "the server has unsent changes");
        let expected = self.server.projection();
        for (i, r) in self.readers.iter().enumerate() {
            let got = r.view.projection();
            if got != expected {
                panic!(
                    "view {i} differs from the server: {}",
                    differences(&expected, &got)
                );
            }
            assert_eq!(r.view.metrics().malformed_events, 0);
        }
        assert_eq!(self.stats.lagged, 0);
        let m = self.server.metrics();
        let shown = expected
            .listings
            .values()
            .filter(|l| l.snippet.as_ref().is_some_and(|s| !s.title.is_empty()))
            .count();
        let summary = format!(
            "{} listings ({shown} with snippets), {} users; {} calls, refused {:?}; \
             server: {} broadcasts, {} records, {} refreshes, {} sessions expired, {} evicted, \
             {} subscriptions, {} deltas ({} lost), {} snippets shown, {} unverified, \
             {} timeouts, {} ended, {} sessions displaced; views: {} underruns; {} restarts, \
             {} renames, {} moves; {} log bytes",
            expected.listings.len(),
            expected.users.len(),
            self.stats.calls,
            self.stats.refused,
            m.broadcasts,
            m.records,
            m.refreshes,
            m.sessions_expired,
            m.sessions_evicted,
            m.subscriptions,
            self.stats.deltas_sent,
            self.stats.deltas_lost,
            m.snippets_shown,
            m.snippets_unverified,
            m.snippet_timeouts,
            m.snippets_ended,
            m.sessions_displaced,
            self.stats.underruns,
            self.stats.restarts,
            self.stats.renames,
            self.stats.moves,
            self.log.end,
        );
        Report {
            hash: self.trace.hash,
            lines: self.trace.lines,
            listings: m.listings_created,
            shown: m.snippets_shown,
            summary,
        }
    }
}

fn chat(n: usize) -> Listing {
    let n = u8::try_from(n).unwrap();
    Listing {
        content: Some(Content::Chat(Chat {
            key: vec![n; 32],
            name: format!("chat {n}"),
        })),
    }
}

fn describe(c: &Call) -> String {
    match c {
        Call::Publish(l) => format!("publish {:x?}", listing::key(l)[8..].get(..6)),
        Call::Unpublish(id) => format!("unpublish {id}"),
        Call::Join(q) => match &q.query {
            Some(Query::Id(id)) => format!("join {id}"),
            _ => "join by listing".into(),
        },
        Call::Part(id) => format!("part {id}"),
        Call::Ping => "ping".into(),
        Call::ModerateListing(id, m) => format!("moderate listing {id} {m:?}"),
        Call::ModerateUser(peer, m) => format!("moderate user {:x?} {m:?}", &peer[..4]),
    }
}

fn event_kind(e: &ViewEvent) -> String {
    match e {
        ViewEvent::ListingChanged(l) => format!("changed {}", l.id),
        ViewEvent::ListingUnpublished(l) => format!("unpublished {}", l.id),
        ViewEvent::UserCountChanged {
            id,
            user_count,
            recent_user_count,
        } => format!("count {id} {user_count}/{recent_user_count}"),
        ViewEvent::UserJoined { listing, user } => format!("join {listing} {}", user.id),
        ViewEvent::UserParted { listing, user } => format!("part {listing} {}", user.id),
        ViewEvent::UserRenamed { listing, user } => {
            format!("rename {listing} {} {}", user.id, user.alias)
        }
        ViewEvent::Lagged => "lagged".into(),
    }
}

fn differences(want: &Projection, got: &Projection) -> String {
    let mut out = Vec::new();
    for (id, l) in &want.listings {
        match got.listings.get(id) {
            None => out.push(format!("listing {id} missing")),
            Some(g) if g != l => out.push(format!("listing {id}: want {l:?}, got {g:?}")),
            Some(_) => {}
        }
    }
    for id in got.listings.keys() {
        if !want.listings.contains_key(id) {
            out.push(format!("listing {id} extra"));
        }
    }
    for (id, u) in &want.users {
        match got.users.get(id) {
            None => out.push(format!("user {id} missing")),
            Some(g) if g != u => out.push(format!("user {id}: want {u:?}, got {g:?}")),
            Some(_) => {}
        }
    }
    for id in got.users.keys() {
        if !want.users.contains_key(id) {
            out.push(format!("user {id} extra"));
        }
    }
    out.join("; ")
}

/// Reads a setting of the test run (never of the simulated nodes).
#[allow(clippy::disallowed_methods)]
fn env(name: &str) -> Option<u64> {
    std::env::var(name).ok().and_then(|s| s.trim().parse().ok())
}

fn seeds() -> Vec<u64> {
    if let Some(s) = env("STRIMS_DIRECTORY_SIM_SEED") {
        return vec![s];
    }
    let n = env("STRIMS_DIRECTORY_SIM_SEEDS").unwrap_or(if cfg!(debug_assertions) { 3 } else { 8 });
    (1..=n.max(1)).collect()
}

/// Runs `seed`, printing the replay command if it fails.
fn run(test: &str, seed: u64, plan: Plan) -> Report {
    let result = catch_unwind(AssertUnwindSafe(|| Sim::new(seed, plan).run()));
    match result {
        Ok(r) => r,
        Err(p) => {
            eprintln!(
                "{test} failed with seed {seed}\nreplay: STRIMS_DIRECTORY_SIM_SEED={seed} \
                 cargo test -p strims-directory {test} -- --exact --nocapture"
            );
            resume_unwind(p)
        }
    }
}

#[test]
fn sim_views_converge_to_the_server() {
    for seed in seeds() {
        let r = run(
            "sim_test::sim_views_converge_to_the_server",
            seed,
            Plan::full(),
        );
        eprintln!(
            "seed {seed}: trace {:016x} ({} lines): {}",
            r.hash, r.lines, r.summary
        );
        assert!(
            r.listings > 0 && r.shown > 0,
            "seed {seed} published nothing"
        );
    }
}

#[test]
fn sim_runs_are_deterministic() {
    let test = "sim_test::sim_runs_are_deterministic";
    let a = run(test, 101, Plan::short());
    let b = run(test, 101, Plan::short());
    let c = run(test, 102, Plan::short());
    assert_eq!(
        (a.hash, a.lines),
        (b.hash, b.lines),
        "seed 101 replayed differently\nreplay: cargo test -p strims-directory {test} -- \
         --exact --nocapture"
    );
    assert_ne!(a.hash, c.hash, "seeds 101 and 102 gave the same trace");
    eprintln!(
        "seed 101: trace {:016x} ({} lines), seed 102: {:016x}",
        a.hash, a.lines, c.hash
    );
}

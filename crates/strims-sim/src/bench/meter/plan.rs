// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A meter bench condition and the traffic it drives, drawn from one seed.
//!
//! [`Condition::draw`] draws every parameter of a class's condition from
//! the seed's rng, and [`Traffic::new`] the stream and the neighbours'
//! requests from the same rng after it, so every transport and meter run
//! on one seed sees the same units, requests and events.

use std::fmt;

use strims_base::rng::{DetRng, det_rng};
use strims_base::time::Duration;
use strims_wan::live::{CHUNK_HEADER, Deadline, Encoder, Mode, Signer, plan};
use strims_wan::sample::unit;

use crate::wan::SctpProfile;

/// A stream chunk's bytes.
pub const CHUNK: u64 = 4096;
/// Chunks per signed unit.
pub const UNIT_CHUNKS: usize = 8;
/// When a unit that hasn't filled is signed, after its first byte.
pub const SIGN_DEADLINE: Duration = Duration::from_millis(200);
/// A chunk's frame beyond its stream bytes: its DATA message, the unit's
/// integrity and the frame's headers.
pub const CHUNK_FRAME_OVERHEAD: u64 = 120;
/// A data-less chunk's record (a DATA of zeros), in its unit's last frame.
pub const DATALESS_BYTES: u64 = 16;
/// A unit's signature, in its first frame.
pub const SIGNATURE_BYTES: u64 = 96;
/// The control records' sizes on the wire.
pub const STATE_BYTES: u64 = 64;
pub const HAVE_BYTES: u64 = 24;
pub const REQUEST_BYTES: u64 = 24;
pub const NOT_YET_BYTES: u64 = 20;
pub const PING_BYTES: u64 = 24;
/// How often each side of a link sends a STATE ...
pub const STATE_EVERY: Duration = Duration::from_millis(250);
/// ... and a ping.
pub const PING_EVERY: Duration = Duration::SECOND;
/// The stream starts this long into the run, so links settle.
pub const WARM: Duration = Duration::from_secs(1);

/// What a condition varies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Class {
    /// Nothing changes.
    Steady,
    /// The uplink halves, then recovers.
    Halve,
    /// The uplink doubles.
    Rise,
    /// A bulk CUBIC upload shares the uplink for a while.
    Compete,
    /// Every link's latency steps up, then back.
    Latency,
    /// A third of the links change route: their latency moves and they
    /// stall for a moment.
    Route,
    /// Random loss on the access link.
    Loss,
    /// LTE's and Starlink's burst loss.
    Lte,
    Starlink,
    /// Wi-Fi's jitter.
    Wifi,
    /// One to three early-ranked neighbours whose downlinks can't take a
    /// copy.
    Slow,
    /// A quarter of the neighbours delay echoes, lie or go silent.
    Byzantine,
    /// Neighbours join and leave.
    Churn,
    /// A relay on a 2-4 Mbit/s uplink that receives a copy.
    Thin,
    /// The byzantine class's conditions with every neighbour honest: the
    /// pair that scores a meter's sensitivity to the liars.
    Byzclean,
}

impl Class {
    pub const ALL: [Class; 15] = [
        Class::Steady,
        Class::Halve,
        Class::Rise,
        Class::Compete,
        Class::Latency,
        Class::Route,
        Class::Loss,
        Class::Lte,
        Class::Starlink,
        Class::Wifi,
        Class::Slow,
        Class::Byzantine,
        Class::Churn,
        Class::Thin,
        Class::Byzclean,
    ];

    /// The class whose draws it takes.
    fn draws_as(self) -> Class {
        match self {
            Class::Byzclean => Class::Byzantine,
            c => c,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Class::Steady => "steady",
            Class::Halve => "halve",
            Class::Rise => "rise",
            Class::Compete => "compete",
            Class::Latency => "latency",
            Class::Route => "route",
            Class::Loss => "loss",
            Class::Lte => "lte",
            Class::Starlink => "starlink",
            Class::Wifi => "wifi",
            Class::Slow => "slow",
            Class::Byzantine => "byzantine",
            Class::Churn => "churn",
            Class::Thin => "thin",
            Class::Byzclean => "byzclean",
        }
    }

    pub fn parse(s: &str) -> Result<Class, String> {
        Class::ALL
            .into_iter()
            .find(|c| c.name() == s)
            .ok_or_else(|| format!("no class {s}"))
    }
}

impl fmt::Display for Class {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// What carries the links.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Transport {
    Tcp,
    /// The window model's data channels, the node under test sending as
    /// that stack sends.
    Channel(Stack),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Stack {
    Chromium,
    Firefox,
    Str0m,
}

impl Transport {
    pub fn name(self) -> &'static str {
        match self {
            Transport::Tcp => "tcp",
            Transport::Channel(Stack::Chromium) => "chromium",
            Transport::Channel(Stack::Firefox) => "firefox",
            Transport::Channel(Stack::Str0m) => "str0m",
        }
    }

    pub fn parse(s: &str) -> Result<Transport, String> {
        Ok(match s {
            "tcp" => Transport::Tcp,
            "chromium" => Transport::Channel(Stack::Chromium),
            "firefox" => Transport::Channel(Stack::Firefox),
            "str0m" => Transport::Channel(Stack::Str0m),
            _ => return Err(format!("no transport {s}")),
        })
    }

    pub fn profile(self) -> Option<SctpProfile> {
        match self {
            Transport::Tcp => None,
            Transport::Channel(Stack::Chromium) => Some(SctpProfile::CHROMIUM),
            Transport::Channel(Stack::Firefox) => Some(SctpProfile::FIREFOX),
            Transport::Channel(Stack::Str0m) => Some(SctpProfile::STR0M),
        }
    }
}

/// How a byzantine neighbour misbehaves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Byz {
    /// Holds each echo this long.
    Delay(Duration),
    /// Reports each unit's spread at this multiple of what it saw.
    Lie(f64),
    /// Sends no echo and no report.
    Silent,
}

/// One neighbour of the node under test.
#[derive(Clone, Debug, PartialEq)]
pub struct Neighbour {
    /// The link's one-way base latency.
    pub delay: Duration,
    /// Its downlink, bytes per second, when it is narrow.
    pub down: Option<u64>,
    pub byz: Option<Byz>,
    /// When it takes part, from the run's start; a link stays up while
    /// its neighbour is away, carrying nothing.
    pub present: Vec<(Duration, Duration)>,
    /// What its clock reads over the run's.
    pub clock_offset: Duration,
}

impl Neighbour {
    pub fn present_at(&self, t: Duration) -> bool {
        self.present.iter().any(|&(a, b)| a <= t && t < b)
    }
}

/// Whether the node under test seeds the stream or relays a copy it
/// receives from a feeder.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    Seed,
    Relay,
}

/// A step of some links' latency.
#[derive(Clone, Debug, PartialEq)]
pub struct LatencyStep {
    pub at: Duration,
    pub links: Vec<usize>,
    /// Each link's new one-way base latency.
    pub delays: Vec<Duration>,
    /// The links stall this long first (a route change), in both
    /// directions.
    pub freeze: Duration,
}

/// Loss in bursts on the node's access link.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BurstLoss {
    pub every: Duration,
    pub phase: Option<Duration>,
    pub length: Duration,
    pub loss: f64,
}

/// One run's condition.
#[derive(Clone, Debug, PartialEq)]
pub struct Condition {
    pub class: Class,
    pub seed: u64,
    pub secs: u64,
    pub role: Role,
    pub transport: Transport,
    /// Main's traffic mix: a HAVE to every neighbour with each unit.
    pub have: bool,
    /// The node's uplink at the start, bytes per second ...
    pub up: u64,
    /// ... and from given times on.
    pub up_steps: Vec<(Duration, u64)>,
    /// Every access queue, in transmission time.
    pub queue: Duration,
    pub loss: f64,
    pub bursts: Option<BurstLoss>,
    /// The node's access link's mean jitter, each way.
    pub jitter: Duration,
    /// The stream's encoder, and its scenes' log-normal spread of
    /// complexity.
    pub kbit: u32,
    pub scene_sigma: f64,
    pub neighbours: Vec<Neighbour>,
    /// Bulk uploads on the node's uplink: start and length, and the sink's
    /// round trip.
    pub compete: Vec<(Duration, Duration)>,
    pub compete_rtt: Duration,
    pub latency_steps: Vec<LatencyStep>,
    /// The share of a unit's chunks a neighbour that isn't pushed to asks
    /// the node for, and the share of those requests that come before the
    /// unit is signed (answered NOT_YET).
    pub pull_share: f64,
    pub early_share: f64,
    /// Open loop: the copies (children) from given times on; `None`, the
    /// driving meter's budget sets them.
    pub copies: Option<Vec<(Duration, usize)>>,
    /// The feeder's latency to a relay under test.
    pub feeder_delay: Duration,
}

fn between(rng: &mut DetRng, lo: f64, hi: f64) -> f64 {
    lo + (hi - lo) * unit(rng)
}

fn log_between(rng: &mut DetRng, lo: f64, hi: f64) -> f64 {
    (lo.ln() + (hi.ln() - lo.ln()) * unit(rng)).exp()
}

fn ms(x: f64) -> Duration {
    Duration::from_micros((x * 1000.0) as i64)
}

fn secs(x: f64) -> Duration {
    Duration::from_micros((x * 1e6) as i64)
}

/// Bytes per second in one Mbit/s.
pub const MBIT: f64 = 125_000.0;

impl Condition {
    /// `class`'s condition at `seed`, its links on `transport`.
    pub fn draw(class: Class, seed: u64, transport: Transport, have: bool) -> Condition {
        let named = class;
        let class = class.draws_as();
        let mut rng = det_rng(seed ^ 0x4d45_5445_5200 ^ ((class as u64) << 32));
        let rng = &mut rng;
        let secs_total: u64 = 90;
        let role = if class == Class::Thin {
            Role::Relay
        } else {
            Role::Seed
        };
        let up_mbit = if class == Class::Thin {
            between(rng, 2.0, 4.0)
        } else {
            log_between(rng, 1.0, 50.0)
        };
        let up = (up_mbit * MBIT) as u64;
        let queue = ms(log_between(rng, 20.0, 500.0));
        // A copy is a fifth to a half of the uplink at 1 Mbit/s, and up to
        // a sixteenth past 10: the budget pays for 1 to 11 copies.
        let share = if class == Class::Thin {
            between(rng, 0.25, 0.45)
        } else {
            log_between(rng, 0.06, 0.5)
        };
        let kbit = ((up_mbit * share * 1000.0) as u32).clamp(200, 8000);
        let n = if class == Class::Thin {
            1 + (unit(rng) * 8.0) as usize
        } else {
            let x = log_between(rng, 1.0, 30.0).round() as usize;
            x.clamp(1, 30)
        };
        let nut_mile = between(rng, 2.0, 20.0);
        let end = secs(secs_total as f64);
        let mut neighbours: Vec<Neighbour> = (0..n)
            .map(|_| Neighbour {
                delay: ms(nut_mile + log_between(rng, 3.0, 80.0)),
                down: None,
                byz: None,
                present: vec![(Duration::ZERO, end)],
                clock_offset: ms(between(rng, -2000.0, 2000.0)),
            })
            .collect();
        let mut c = Condition {
            class,
            seed,
            secs: secs_total,
            role,
            transport,
            have,
            up,
            up_steps: Vec::new(),
            queue,
            loss: 0.0,
            bursts: None,
            jitter: Duration::ZERO,
            kbit,
            scene_sigma: 0.0,
            neighbours: Vec::new(),
            compete: Vec::new(),
            compete_rtt: ms(between(rng, 15.0, 80.0)),
            latency_steps: Vec::new(),
            // Answers take up to 0.6 of a copy in all.
            pull_share: between(rng, 0.0, 0.6) / n as f64,
            early_share: between(rng, 0.0, 0.5),
            copies: None,
            feeder_delay: ms(between(rng, 5.0, 40.0)),
        };
        match class {
            Class::Steady | Class::Thin | Class::Byzclean => {}
            Class::Halve => {
                c.up_steps = vec![(secs(30.0), up / 2), (secs(60.0), up)];
            }
            Class::Rise => {
                c.up = up / 2;
                c.up_steps = vec![(secs(30.0), up)];
            }
            Class::Compete => {
                c.compete = vec![(secs(30.0), secs(30.0))];
            }
            Class::Latency => {
                let step = ms(log_between(rng, 20.0, 150.0));
                let links: Vec<usize> = (0..n).collect();
                c.latency_steps = vec![
                    LatencyStep {
                        at: secs(30.0),
                        links: links.clone(),
                        delays: neighbours.iter().map(|nb| nb.delay + step).collect(),
                        freeze: Duration::ZERO,
                    },
                    LatencyStep {
                        at: secs(60.0),
                        links,
                        delays: neighbours.iter().map(|nb| nb.delay).collect(),
                        freeze: Duration::ZERO,
                    },
                ];
            }
            Class::Route => {
                let k = n.div_ceil(3);
                let links: Vec<usize> = (0..n).filter(|_| unit(rng) < 0.5).take(k).collect();
                let links = if links.is_empty() { vec![0] } else { links };
                let delays = links
                    .iter()
                    .map(|&l| {
                        let d = neighbours.get(l).map_or(Duration::ZERO, |nb| nb.delay);
                        let moved = d.as_secs_f64() * 1000.0 + between(rng, -30.0, 80.0);
                        ms(moved.max(nut_mile + 2.0))
                    })
                    .collect();
                c.latency_steps = vec![LatencyStep {
                    at: secs(30.0),
                    links,
                    delays,
                    freeze: ms(between(rng, 100.0, 300.0)),
                }];
            }
            Class::Loss => c.loss = log_between(rng, 0.005, 0.02),
            Class::Lte => {
                c.loss = 0.000_5;
                c.bursts = Some(BurstLoss {
                    every: Duration::from_secs(4),
                    phase: None,
                    length: Duration::from_millis(50),
                    loss: 0.12,
                });
            }
            Class::Starlink => {
                c.loss = 0.000_5;
                c.bursts = Some(BurstLoss {
                    every: Duration::from_secs(15),
                    phase: Some(Duration::from_secs(12)),
                    length: Duration::from_millis(120),
                    loss: 0.3,
                });
            }
            Class::Wifi => c.jitter = ms(log_between(rng, 2.0, 15.0)),
            Class::Slow => {
                let k = (1 + (unit(rng) * 3.0) as usize).min(n);
                let rate = kbit as f64 * 125.0;
                for nb in neighbours.iter_mut().take(k) {
                    nb.down = Some((rate * between(rng, 0.3, 0.9)) as u64);
                    // The nearest, so the budget's first copies go to them.
                    nb.delay = ms(nut_mile + between(rng, 1.0, 3.0));
                }
            }
            Class::Byzantine => {
                let k = n.div_ceil(4).min(n.saturating_sub(1).max(1));
                for nb in neighbours.iter_mut().take(k) {
                    let x = unit(rng);
                    nb.byz = Some(if x < 0.34 {
                        Byz::Delay(ms(between(rng, 50.0, 300.0)))
                    } else if x < 0.67 {
                        Byz::Lie(if unit(rng) < 0.5 { 0.25 } else { 4.0 })
                    } else {
                        Byz::Silent
                    });
                    // Among the nearest, so their links weigh in.
                    nb.delay = ms(nut_mile + between(rng, 1.0, 5.0));
                }
            }
            Class::Churn => {
                for nb in neighbours.iter_mut() {
                    let join = if unit(rng) < 0.5 {
                        0.0
                    } else {
                        between(rng, 5.0, 60.0)
                    };
                    let mut present = Vec::new();
                    if unit(rng) < 0.4 {
                        let leave = between(rng, join + 5.0, secs_total as f64);
                        present.push((secs(join), secs(leave)));
                        if unit(rng) < 0.5 {
                            let back = between(rng, leave, secs_total as f64);
                            present.push((secs(back), end));
                        }
                    } else {
                        present.push((secs(join), end));
                    }
                    nb.present = present;
                }
            }
        }
        if named == Class::Byzclean {
            for nb in &mut neighbours {
                nb.byz = None;
            }
        }
        c.scene_sigma = log_between(rng, 0.1, 0.7);
        c.class = named;
        c.neighbours = neighbours;
        c
    }

    /// The node's uplink at `t`.
    pub fn up_at(&self, t: Duration) -> u64 {
        self.up_steps
            .iter()
            .rev()
            .find(|&&(at, _)| at <= t)
            .map_or(self.up, |&(_, r)| r)
    }

    /// The condition as a JSON object, for the scorer.
    pub fn json(&self) -> String {
        let d = |x: Duration| format!("{:.3}", x.as_secs_f64());
        let steps: Vec<String> = self
            .up_steps
            .iter()
            .map(|&(t, r)| format!("[{},{r}]", d(t)))
            .collect();
        let compete: Vec<String> = self
            .compete
            .iter()
            .map(|&(t, l)| format!("[{},{}]", d(t), d(t + l)))
            .collect();
        let lat: Vec<String> = self
            .latency_steps
            .iter()
            .map(|s| format!("[{},{}]", d(s.at), s.links.len()))
            .collect();
        let nbs: Vec<String> = self
            .neighbours
            .iter()
            .map(|nb| {
                let byz = match nb.byz {
                    None => "null".to_owned(),
                    Some(Byz::Delay(x)) => format!(r#""delay {}""#, x.as_millis()),
                    Some(Byz::Lie(f)) => format!(r#""lie {f}""#),
                    Some(Byz::Silent) => r#""silent""#.to_owned(),
                };
                let present: Vec<String> = nb
                    .present
                    .iter()
                    .map(|&(a, b)| format!("[{},{}]", d(a), d(b)))
                    .collect();
                format!(
                    r#"{{"delay_ms":{:.1},"down":{},"byz":{byz},"present":[{}]}}"#,
                    nb.delay.as_secs_f64() * 1000.0,
                    nb.down.map_or("null".to_owned(), |x| x.to_string()),
                    present.join(","),
                )
            })
            .collect();
        format!(
            r#"{{"class":"{}","seed":{},"secs":{},"role":"{}","transport":"{}","have":{},"up":{},"up_steps":[{}],"queue_ms":{:.1},"loss":{},"bursts":{},"jitter_ms":{:.2},"kbit":{},"scene_sigma":{:.2},"compete":[{}],"latency_steps":[{}],"pull_share":{:.3},"neighbours":[{}]}}"#,
            self.class,
            self.seed,
            self.secs,
            match self.role {
                Role::Seed => "seed",
                Role::Relay => "relay",
            },
            self.transport.name(),
            self.have,
            self.up,
            steps.join(","),
            self.queue.as_secs_f64() * 1000.0,
            self.loss,
            self.bursts.is_some(),
            self.jitter.as_secs_f64() * 1000.0,
            self.kbit,
            self.scene_sigma,
            compete.join(","),
            lat.join(","),
            self.pull_share,
            nbs.join(","),
        )
    }
}

/// One signed unit: when it was signed, from the run's start, and its
/// frames' bytes (a frame a chunk with data; the data-less chunks' records
/// ride the last).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unit {
    pub at: Duration,
    pub frames: Vec<u32>,
}

impl Unit {
    pub fn bytes(&self) -> u64 {
        self.frames.iter().map(|&f| u64::from(f)).sum()
    }
}

/// A neighbour's request for one chunk of a unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    pub at: Duration,
    pub unit: u32,
    pub frame: u16,
}

/// The stream and the requests, drawn after the condition.
#[derive(Clone, Debug)]
pub struct Traffic {
    pub units: Vec<Unit>,
    /// Per neighbour, its requests in time order.
    pub requests: Vec<Vec<Request>>,
}

impl Traffic {
    pub fn new(c: &Condition) -> Traffic {
        let mut rng = det_rng(c.seed ^ 0x5452_4146 ^ ((c.class.draws_as() as u64) << 40));
        let rng = &mut rng;
        let span = Duration::from_secs(c.secs as i64) - WARM - Duration::from_secs(2);
        let mut enc = Encoder::new(c.kbit, Mode::Vbr);
        enc.scene_sigma = c.scene_sigma;
        let trace = enc.trace(rng, span);
        let signer = Signer::new(
            Deadline::Fixed(SIGN_DEADLINE),
            UNIT_CHUNKS,
            CHUNK as usize,
            CHUNK_HEADER,
        );
        let p = plan(&trace, signer);
        let unit_bytes = CHUNK * UNIT_CHUNKS as u64;
        let mut units = Vec::new();
        // Each unit's chunks: their stream bytes less the padding and
        // data-less ranges.
        let n_units = p.stream_bytes / unit_bytes;
        for u in 0..n_units {
            let first = u * UNIT_CHUNKS as u64;
            let Some(&at) = p.signed_at.get(first as usize) else {
                break;
            };
            let mut frames = Vec::new();
            let mut dataless = 0u64;
            for c in first..first + UNIT_CHUNKS as u64 {
                let lo = c * CHUNK;
                let hi = lo + CHUNK;
                let zeros: u64 = p
                    .pads
                    .iter()
                    .filter(|r| r.start < hi && r.end > lo)
                    .map(|r| r.end.min(hi) - r.start.max(lo))
                    .sum();
                let sent = CHUNK - zeros;
                if sent == 0 {
                    dataless += 1;
                } else {
                    frames.push(sent + CHUNK_FRAME_OVERHEAD);
                }
            }
            if frames.is_empty() {
                frames.push(CHUNK_FRAME_OVERHEAD);
            }
            if let Some(f) = frames.first_mut() {
                *f += SIGNATURE_BYTES;
            }
            if let Some(f) = frames.last_mut() {
                *f += dataless * DATALESS_BYTES;
            }
            units.push(Unit {
                at: WARM + at,
                frames: frames.into_iter().map(|f| f as u32).collect(),
            });
        }
        let requests = c
            .neighbours
            .iter()
            .map(|nb| {
                let mut out = Vec::new();
                for (u, unit_) in units.iter().enumerate() {
                    for f in 0..unit_.frames.len() {
                        if unit(rng) >= c.pull_share {
                            continue;
                        }
                        let early = unit(rng) < c.early_share;
                        let at = if early {
                            unit_.at - nb.delay - ms(between(rng, 0.0, 150.0))
                        } else {
                            unit_.at + nb.delay + ms(between(rng, 5.0, 250.0))
                        };
                        out.push(Request {
                            at: at.max(WARM),
                            unit: u as u32,
                            frame: f as u16,
                        });
                    }
                }
                out.sort_by_key(|r| r.at);
                out
            })
            .collect();
        Traffic { units, requests }
    }

    /// The stream's mean rate in bytes per second over the units.
    pub fn rate(&self) -> f64 {
        let (Some(a), Some(b)) = (self.units.first(), self.units.last()) else {
            return 0.0;
        };
        let span = (b.at - a.at).as_secs_f64().max(1.0);
        self.units.iter().map(Unit::bytes).sum::<u64>() as f64 / span
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;

    #[test]
    fn a_condition_and_its_traffic_follow_from_the_seed() {
        let a = Condition::draw(Class::Halve, 3, Transport::Tcp, false);
        let b = Condition::draw(Class::Halve, 3, Transport::Tcp, false);
        assert_eq!(a, b);
        let ta = Traffic::new(&a);
        let tb = Traffic::new(&b);
        assert_eq!(ta.units, tb.units);
        assert!(!ta.units.is_empty());
        let rate = ta.rate() * 8.0 / 1000.0;
        // The stream's bytes on the wire run near the encoder's rate.
        assert!(
            rate > a.kbit as f64 * 0.7 && rate < a.kbit as f64 * 1.6,
            "{rate} {}",
            a.kbit
        );
        let c = Condition::draw(Class::Halve, 4, Transport::Tcp, false);
        assert_ne!(a, c);
    }

    #[test]
    fn units_are_bursts_of_chunk_frames() {
        let c = Condition::draw(Class::Steady, 1, Transport::Tcp, false);
        let t = Traffic::new(&c);
        for u in &t.units {
            assert!(!u.frames.is_empty() && u.frames.len() <= UNIT_CHUNKS);
            assert!(u.frames.iter().all(|&f| u64::from(f) <= CHUNK + 1024));
        }
        assert!(t.units.windows(2).all(|w| w[0].at <= w[1].at));
    }
}

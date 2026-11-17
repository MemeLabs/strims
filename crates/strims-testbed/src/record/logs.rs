// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The scheduler's logs as a node's records carry them: the strims-ppspp
//! records strims-sim's log tables take (`forecast`, `arrivals`, `haves`,
//! `asks`, `seed_decisions`, `seed_sends`), written and read back whole.
//!
//! A node whose config keeps the logs (`NodeConfig::forecast_log`, the
//! forecast log and the held log; `NodeConfig::request_log`, the held,
//! HAVE, request and seed logs) hands them over with each metrics
//! snapshot, as a simulated node's sample does. [`Logs::of`] takes one
//! snapshot's, of one swarm, each record's hosts by their testbed index;
//! [`Logs::json`] writes them for a line's records and [`Logs::parse`]
//! reads them back as the same records, which strims-sim's testbed module
//! hands its tables as a simulated node's.
//!
//! Each record is a JSON array of its fields in declaration order (a
//! struct's fields, a tuple's items), an `Option` `null` or its value, an
//! enum of unit variants its variant's name, and an enum with data
//! `[name, fields...]`. Times are nanoseconds from the line's origin (the
//! stream's start), durations nanoseconds, bins their number and a
//! runner's peer key its FFI form. A field list is the struct's own,
//! matched whole on writing and on reading, so a field the records gain
//! fails the build until it is written here.

use std::fmt::Write as _;

use serde_json::Value;
use slotmap::{Key as _, KeyData};
use strims_base::time::{Duration, Time};
use strims_binmap::Bin;
use strims_node_core::{HostId, NodeMetrics, SwarmKey};
use strims_ppspp::{
    AdmitRule, CancelOpen, HaveRecord, Held, HeldHow, HolderLeft, LagForecast, LagSnapshot,
    PeerKey, RequestChannel, RequestEnd, RequestRecord, SeedBar, SeedCheck, SeedClose,
    SeedDecision, SeedDecisionKind, SeedRequest, SeedSend, SeedWindow, SendFate, SendHow,
    WindowPhase,
};

/// One value of a log's records as a line writes it: a JSON value, its
/// times from `origin`.
pub trait Field: Sized {
    fn put(&self, out: &mut String, origin: Time);
    fn get(v: &Value, origin: Time) -> Option<Self>;
}

/// A JSON array being written: its items, comma-separated.
struct List<'a> {
    out: &'a mut String,
    n: usize,
}

impl<'a> List<'a> {
    fn open(out: &'a mut String) -> List<'a> {
        out.push('[');
        List { out, n: 0 }
    }

    fn sep(&mut self) {
        if self.n > 0 {
            self.out.push(',');
        }
        self.n += 1;
    }

    fn item<T: Field>(&mut self, x: &T, origin: Time) {
        self.sep();
        x.put(self.out, origin);
    }

    fn name(&mut self, name: &str) {
        self.sep();
        let _ = write!(self.out, "\"{name}\"");
    }

    fn close(self) {
        self.out.push(']');
    }
}

/// A JSON array being read: its items in order.
struct Items<'a> {
    it: std::slice::Iter<'a, Value>,
    origin: Time,
}

impl<'a> Items<'a> {
    fn of(v: &'a Value, origin: Time) -> Option<Items<'a>> {
        Some(Items {
            it: v.as_array()?.iter(),
            origin,
        })
    }

    fn next<T: Field>(&mut self) -> Option<T> {
        T::get(self.it.next()?, self.origin)
    }

    fn name(&mut self) -> Option<&'a str> {
        self.it.next()?.as_str()
    }
}

macro_rules! unsigned {
    ($($t:ty),*) => {$(
        impl Field for $t {
            fn put(&self, out: &mut String, _: Time) {
                let _ = write!(out, "{self}");
            }
            fn get(v: &Value, _: Time) -> Option<Self> {
                <$t>::try_from(v.as_u64()?).ok()
            }
        }
    )*};
}
unsigned!(u8, u16, u32, u64, usize);

impl Field for bool {
    fn put(&self, out: &mut String, _: Time) {
        out.push_str(if *self { "true" } else { "false" });
    }
    fn get(v: &Value, _: Time) -> Option<Self> {
        v.as_bool()
    }
}

impl Field for Time {
    fn put(&self, out: &mut String, origin: Time) {
        let _ = write!(out, "{}", (*self - origin).as_nanos());
    }
    fn get(v: &Value, origin: Time) -> Option<Self> {
        Some(origin + Duration::from_nanos(v.as_i64()?))
    }
}

impl Field for Duration {
    fn put(&self, out: &mut String, _: Time) {
        let _ = write!(out, "{}", self.as_nanos());
    }
    fn get(v: &Value, _: Time) -> Option<Self> {
        Some(Duration::from_nanos(v.as_i64()?))
    }
}

impl Field for Bin {
    fn put(&self, out: &mut String, o: Time) {
        self.0.put(out, o);
    }
    fn get(v: &Value, o: Time) -> Option<Self> {
        Some(Bin(u64::get(v, o)?))
    }
}

impl Field for PeerKey {
    fn put(&self, out: &mut String, o: Time) {
        self.data().as_ffi().put(out, o);
    }
    fn get(v: &Value, o: Time) -> Option<Self> {
        Some(KeyData::from_ffi(u64::get(v, o)?).into())
    }
}

impl<T: Field> Field for Option<T> {
    fn put(&self, out: &mut String, o: Time) {
        match self {
            Some(x) => x.put(out, o),
            None => out.push_str("null"),
        }
    }
    fn get(v: &Value, o: Time) -> Option<Self> {
        if v.is_null() {
            return Some(None);
        }
        T::get(v, o).map(Some)
    }
}

impl<T: Field> Field for Vec<T> {
    fn put(&self, out: &mut String, o: Time) {
        let mut l = List::open(out);
        for x in self {
            l.item(x, o);
        }
        l.close();
    }
    fn get(v: &Value, o: Time) -> Option<Self> {
        v.as_array()?.iter().map(|x| T::get(x, o)).collect()
    }
}

macro_rules! tuple {
    ($($t:ident $x:ident),*) => {
        impl<$($t: Field),*> Field for ($($t,)*) {
            fn put(&self, out: &mut String, o: Time) {
                let ($($x,)*) = self;
                let mut l = List::open(out);
                $(l.item($x, o);)*
                l.close();
            }
            fn get(v: &Value, o: Time) -> Option<Self> {
                let mut r = Items::of(v, o)?;
                Some(($(r.next::<$t>()?,)*))
            }
        }
    };
}
tuple!(A a, B b);
tuple!(A a, B b, C c);
tuple!(A a, B b, C c, D d);

/// A struct as the array of its fields: every field, by its own name.
macro_rules! fields {
    ($t:ident { $($f:ident),* $(,)? }) => {
        impl Field for $t {
            fn put(&self, out: &mut String, o: Time) {
                let $t { $($f),* } = self;
                let mut l = List::open(out);
                $(l.item($f, o);)*
                l.close();
            }
            fn get(v: &Value, o: Time) -> Option<Self> {
                let mut r = Items::of(v, o)?;
                Some($t { $($f: r.next()?),* })
            }
        }
    };
}

/// An enum of unit variants as its variant's name.
macro_rules! units {
    ($t:ident { $($v:ident),* $(,)? }) => {
        impl Field for $t {
            fn put(&self, out: &mut String, _: Time) {
                let name = match self {
                    $($t::$v => stringify!($v),)*
                };
                let _ = write!(out, "\"{name}\"");
            }
            fn get(v: &Value, _: Time) -> Option<Self> {
                match v.as_str()? {
                    $(stringify!($v) => Some($t::$v),)*
                    _ => None,
                }
            }
        }
    };
}

fields!(LagForecast {
    at,
    peer,
    stream,
    bin,
    sample,
    estimate,
    lazy,
    first_sight,
    channel_open,
    run,
});
fields!(LagSnapshot {
    mean,
    horizon,
    samples,
    fresh
});
fields!(Held {
    bin,
    at,
    peer,
    how,
    parent,
    expected,
    released,
    asked,
    asks,
    last_ask,
    rtt,
});
units!(HeldHow {
    Own,
    Push,
    Answer,
    Other
});
fields!(HaveRecord {
    at,
    peer,
    stream,
    bin,
    channel_open,
});
fields!(RequestRecord {
    at,
    stream,
    bin,
    timeout,
    released,
    asked,
    rivals,
    end,
    ended,
});
fields!(RequestChannel {
    peer,
    announced,
    predicted,
    rtt,
    channel_open,
});
units!(RequestEnd {
    Answered,
    Expired,
    Elsewhere,
    Closed
});
fields!(SeedDecision { at, peer, kind });
fields!(SeedRequest {
    stream,
    granted,
    rule,
    subscribers,
    holders,
    copies,
    held,
    per_holder,
});
units!(AdmitRule {
    Subscribed,
    Pin,
    SeedHolder,
    SeedBarred,
    SeedNoCopy,
    SeedStreamsHeld,
    Unmeasured,
    Measured,
    Silent,
});
units!(HolderLeft {
    Silent,
    Unserved,
    PastCopies,
    Idle,
    Closed,
});
units!(SeedBar { Silent, Unserved });
units!(CancelOpen {
    None,
    Queued,
    Written
});
fields!(SeedCheck {
    satellite,
    streams,
    due,
    stream,
    judged,
    frontier,
    hole,
    state_at,
    silent,
    queue,
    delivered,
    wait,
    stale,
    rtt,
});
fields!(SeedWindow {
    phase,
    budget,
    measured,
    capacity,
    copies,
    streams,
    holders,
    pushed,
    behind,
    overloaded,
    in_use,
    saturated,
    unconstrained,
});
units!(WindowPhase {
    Check,
    FirstWave,
    Ramp,
    RampEnd
});
fields!(SeedSend {
    peer,
    stream,
    bin,
    how,
    queued,
    ended,
    fate,
    bytes,
    cancel,
    declared,
    subscribed,
});
units!(SendHow { Push, Answer });
units!(SendFate {
    Written,
    Stale,
    Cancelled,
    Suppressed,
    Refused,
    Dropped,
    Closed,
});

/// A holder's leaving ([`SeedClose::Left`]) as `["Left", why]`, the rest
/// as their names.
impl Field for SeedClose {
    fn put(&self, out: &mut String, o: Time) {
        let name = match self {
            SeedClose::Left(why) => {
                let mut l = List::open(out);
                l.name("Left");
                l.item(why, o);
                l.close();
                return;
            }
            SeedClose::Silent => "Silent",
            SeedClose::Spill => "Spill",
            SeedClose::Pin => "Pin",
            SeedClose::Saturated => "Saturated",
            SeedClose::Declined => "Declined",
            SeedClose::Channel => "Channel",
        };
        let _ = write!(out, "\"{name}\"");
    }
    fn get(v: &Value, o: Time) -> Option<Self> {
        if let Some(name) = v.as_str() {
            return Some(match name {
                "Silent" => SeedClose::Silent,
                "Spill" => SeedClose::Spill,
                "Pin" => SeedClose::Pin,
                "Saturated" => SeedClose::Saturated,
                "Declined" => SeedClose::Declined,
                "Channel" => SeedClose::Channel,
                _ => return None,
            });
        }
        let mut r = Items::of(v, o)?;
        (r.name()? == "Left").then_some(())?;
        Some(SeedClose::Left(r.next()?))
    }
}

/// A decision's kind as `[name, fields...]`, its name its table's
/// ([`SeedDecisionKind::name`]), its fields in declaration order.
impl Field for SeedDecisionKind {
    fn put(&self, out: &mut String, o: Time) {
        use SeedDecisionKind as K;
        let mut l = List::open(out);
        l.name(self.name());
        match self {
            K::Request(q) => l.item(q, o),
            K::Deferred { stream } => l.item(stream, o),
            K::Released {
                stream,
                subscribers,
                kept,
            } => {
                l.item(stream, o);
                l.item(subscribers, o);
                l.item(kept, o);
            }
            K::Closed {
                stream,
                why,
                subscribers,
            } => {
                l.item(stream, o);
                l.item(why, o);
                l.item(subscribers, o);
            }
            K::HolderJoined {
                satellite,
                rtt,
                holders,
                copies,
            } => {
                l.item(satellite, o);
                l.item(rtt, o);
                l.item(holders, o);
                l.item(copies, o);
            }
            K::HolderLeft {
                why,
                rtt,
                holders,
                copies,
            } => {
                l.item(why, o);
                l.item(rtt, o);
                l.item(holders, o);
                l.item(copies, o);
            }
            K::Barred {
                why,
                until,
                bars,
                reporters,
            } => {
                l.item(why, o);
                l.item(until, o);
                l.item(bars, o);
                l.item(reporters, o);
            }
            K::Check(c) => l.item(c, o),
            K::Cancel { stream, bin, open } => {
                l.item(stream, o);
                l.item(bin, o);
                l.item(open, o);
            }
            K::Window(w) => l.item(w, o),
            K::AnswerRate {
                rate,
                reading,
                engaged,
            } => {
                l.item(rate, o);
                l.item(reading, o);
                l.item(engaged, o);
            }
        }
        l.close();
    }
    fn get(v: &Value, o: Time) -> Option<Self> {
        use SeedDecisionKind as K;
        let mut r = Items::of(v, o)?;
        Some(match r.name()? {
            "request" => K::Request(r.next()?),
            "deferred" => K::Deferred { stream: r.next()? },
            "released" => K::Released {
                stream: r.next()?,
                subscribers: r.next()?,
                kept: r.next()?,
            },
            "closed" => K::Closed {
                stream: r.next()?,
                why: r.next()?,
                subscribers: r.next()?,
            },
            "holder_joined" => K::HolderJoined {
                satellite: r.next()?,
                rtt: r.next()?,
                holders: r.next()?,
                copies: r.next()?,
            },
            "holder_left" => K::HolderLeft {
                why: r.next()?,
                rtt: r.next()?,
                holders: r.next()?,
                copies: r.next()?,
            },
            "barred" => K::Barred {
                why: r.next()?,
                until: r.next()?,
                bars: r.next()?,
                reporters: r.next()?,
            },
            "check" => K::Check(r.next()?),
            "cancel" => K::Cancel {
                stream: r.next()?,
                bin: r.next()?,
                open: r.next()?,
            },
            "window" => K::Window(r.next()?),
            "answer_rate" => K::AnswerRate {
                rate: r.next()?,
                reading: r.next()?,
                engaged: r.next()?,
            },
            _ => return None,
        })
    }
}

/// A chunk the node first held ([`Logs::held`]): the hosts of its source,
/// of the stream's parent then and of the peer last asked for it, by
/// testbed index, and its record.
pub type HeldEntry = (Option<usize>, Option<usize>, Option<usize>, Held);

/// A request the node made ([`Logs::asks`]): the asked peer's and its
/// rivals' testbed indexes, in the record's order, and the record.
pub type AskEntry = (Option<usize>, Vec<Option<usize>>, RequestRecord);

/// The records the logs' caps left out since the last snapshot: the
/// forecast log's, the held log's, the HAVE log's, the request log's (a
/// request with its rivals counting one) and the seed log's
/// (`NodeMetrics::forecasts_lost`, `held_lost`, `haves_lost`,
/// `requests_lost`, `seed_log_lost`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Lost {
    pub forecasts: u64,
    pub held: u64,
    pub haves: u64,
    pub requests: u64,
    pub seed_log: u64,
}

fields!(Lost {
    forecasts,
    held,
    haves,
    requests,
    seed_log
});

/// One metrics snapshot's logs of a swarm (see the [module docs](self)):
/// each record with its hosts' testbed indexes (`None`: a peer without
/// one, the relay, or none).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Logs {
    pub forecasts: Vec<(Option<usize>, LagForecast)>,
    pub held: Vec<HeldEntry>,
    pub haves: Vec<(Option<usize>, HaveRecord)>,
    pub asks: Vec<AskEntry>,
    pub seed_decisions: Vec<(Option<usize>, SeedDecision)>,
    pub seed_sends: Vec<(Option<usize>, SeedSend)>,
    pub lost: Lost,
}

fields!(Logs {
    forecasts,
    held,
    haves,
    asks,
    seed_decisions,
    seed_sends,
    lost,
});

impl Logs {
    /// Snapshot `m`'s logs of `swarm`, its hosts by `index`.
    pub fn of(m: &NodeMetrics, swarm: SwarmKey, index: impl Fn(&HostId) -> Option<usize>) -> Logs {
        let host = |id: &Option<HostId>| id.as_ref().and_then(&index);
        Logs {
            forecasts: m
                .forecasts
                .iter()
                .filter(|x| x.swarm == swarm)
                .map(|x| (host(&x.peer), x.forecast))
                .collect(),
            held: m
                .held
                .iter()
                .filter(|x| x.swarm == swarm)
                .map(|x| (host(&x.peer), host(&x.parent), host(&x.asked), x.held))
                .collect(),
            haves: m
                .haves
                .iter()
                .filter(|x| x.swarm == swarm)
                .map(|x| (host(&x.peer), x.have))
                .collect(),
            asks: m
                .requests
                .iter()
                .filter(|x| x.swarm == swarm)
                .map(|x| {
                    let rivals = x.rivals.iter().map(host).collect();
                    (host(&x.peer), rivals, x.record.clone())
                })
                .collect(),
            seed_decisions: m
                .seed_decisions
                .iter()
                .filter(|x| x.swarm == swarm)
                .map(|x| (host(&x.peer), x.record))
                .collect(),
            seed_sends: m
                .seed_sends
                .iter()
                .filter(|x| x.swarm == swarm)
                .map(|x| (host(&x.peer), x.record))
                .collect(),
            lost: Lost {
                forecasts: m.forecasts_lost,
                held: m.held_lost,
                haves: m.haves_lost,
                requests: m.requests_lost,
                seed_log: m.seed_log_lost,
            },
        }
    }

    /// Whether the snapshot logged nothing and its caps left nothing out:
    /// a node whose logs are off.
    pub fn is_empty(&self) -> bool {
        *self == Logs::default()
    }

    /// The logs as a line's records write them, times from `origin`.
    pub fn json(&self, origin: Time) -> String {
        let mut out = String::new();
        self.put(&mut out, origin);
        out
    }

    /// The logs [`Logs::json`] wrote, times from `origin`; `None` for
    /// anything else.
    pub fn parse(v: &Value, origin: Time) -> Option<Logs> {
        Logs::get(v, origin)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;

    fn key(n: u64) -> PeerKey {
        KeyData::from_ffi((1 << 32) | n).into()
    }

    /// Logs that hold every kind of record, every decision kind and every
    /// variant of the enums they carry, with times before and after the
    /// origin.
    fn every(o: Time) -> Logs {
        let at = |ms: i64| o + Duration::from_millis(ms);
        let d = Duration::from_micros;
        let snap = LagSnapshot {
            mean: d(1500),
            horizon: d(9001),
            samples: 7,
            fresh: true,
        };
        let forecast = |n: u32, estimate| LagForecast {
            at: at(1234),
            peer: key(2),
            stream: 3,
            bin: Bin(88),
            sample: d(4321),
            estimate,
            lazy: n.is_multiple_of(2),
            first_sight: n == 0,
            channel_open: at(-4000),
            run: n,
        };
        let held = |how| Held {
            bin: Bin(16),
            at: at(77),
            peer: Some(key(1)),
            how,
            parent: None,
            expected: Some(at(70)),
            released: None,
            asked: Some(at(71)),
            asks: 2,
            last_ask: Some((key(3), at(72))),
            rtt: d(30_500),
        };
        let channel = |p, predicted| RequestChannel {
            peer: key(p),
            announced: Some(at(10)),
            predicted,
            rtt: d(20_000),
            channel_open: at(-1),
        };
        let ask = |end| RequestRecord {
            at: at(100),
            stream: 1,
            bin: Bin(40),
            timeout: d(250_000),
            released: Some(at(99)),
            asked: channel(1, None),
            rivals: vec![channel(2, Some((at(101), at(140)))), channel(3, None)],
            end,
            ended: at(180),
        };
        let check = SeedCheck {
            satellite: true,
            streams: 2,
            due: Some((Bin(8), at(50))),
            stream: Some(5),
            judged: true,
            frontier: Some(Bin(10)),
            hole: None,
            state_at: Some(at(49)),
            silent: 1,
            queue: Some(d(5000)),
            delivered: Some(123_456),
            wait: None,
            stale: true,
            rtt: Some(d(40_000)),
        };
        let window = |phase| SeedWindow {
            phase,
            budget: 12,
            measured: true,
            capacity: Some(1_000_000),
            copies: 3,
            streams: None,
            holders: 4,
            pushed: 2,
            behind: 1,
            overloaded: 0,
            in_use: 4,
            saturated: false,
            unconstrained: true,
        };
        use SeedDecisionKind as K;
        let mut kinds = vec![
            K::Deferred { stream: 2 },
            K::Released {
                stream: 1,
                subscribers: 3,
                kept: true,
            },
            K::HolderJoined {
                satellite: false,
                rtt: Some(d(25_000)),
                holders: 2,
                copies: 2,
            },
            K::Barred {
                why: SeedBar::Silent,
                until: at(9000),
                bars: 2,
                reporters: 3,
            },
            K::Barred {
                why: SeedBar::Unserved,
                until: at(9000),
                bars: 1,
                reporters: 0,
            },
            K::Check(check),
            K::AnswerRate {
                rate: 777,
                reading: None,
                engaged: true,
            },
        ];
        for rule in [
            AdmitRule::Subscribed,
            AdmitRule::Pin,
            AdmitRule::SeedHolder,
            AdmitRule::SeedBarred,
            AdmitRule::SeedNoCopy,
            AdmitRule::SeedStreamsHeld,
            AdmitRule::Unmeasured,
            AdmitRule::Measured,
            AdmitRule::Silent,
        ] {
            kinds.push(K::Request(SeedRequest {
                stream: 4,
                granted: rule != AdmitRule::Silent,
                rule,
                subscribers: 1,
                holders: Some(2),
                copies: None,
                held: Some(1),
                per_holder: Some(3),
            }));
        }
        let lefts = [
            HolderLeft::Silent,
            HolderLeft::Unserved,
            HolderLeft::PastCopies,
            HolderLeft::Idle,
            HolderLeft::Closed,
        ];
        for why in lefts {
            kinds.push(K::HolderLeft {
                why,
                rtt: None,
                holders: 1,
                copies: 2,
            });
            kinds.push(K::Closed {
                stream: 1,
                why: SeedClose::Left(why),
                subscribers: 0,
            });
        }
        for why in [
            SeedClose::Silent,
            SeedClose::Spill,
            SeedClose::Pin,
            SeedClose::Saturated,
            SeedClose::Declined,
            SeedClose::Channel,
        ] {
            kinds.push(K::Closed {
                stream: 2,
                why,
                subscribers: 1,
            });
        }
        for open in [CancelOpen::None, CancelOpen::Queued, CancelOpen::Written] {
            kinds.push(K::Cancel {
                stream: 0,
                bin: Bin(6),
                open,
            });
        }
        for phase in [
            WindowPhase::Check,
            WindowPhase::FirstWave,
            WindowPhase::Ramp,
            WindowPhase::RampEnd,
        ] {
            kinds.push(K::Window(window(phase)));
        }
        let send = |how, fate| SeedSend {
            peer: key(5),
            stream: 2,
            bin: Bin(30),
            how,
            queued: at(-20),
            ended: at(15),
            fate,
            bytes: 4096,
            cancel: Some(at(10)),
            declared: true,
            subscribed: false,
        };
        let mut sends = Vec::new();
        for how in [SendHow::Push, SendHow::Answer] {
            for fate in [
                SendFate::Written,
                SendFate::Stale,
                SendFate::Cancelled,
                SendFate::Suppressed,
                SendFate::Refused,
                SendFate::Dropped,
                SendFate::Closed,
            ] {
                sends.push((Some(1), send(how, fate)));
            }
        }
        Logs {
            forecasts: vec![
                (Some(2), forecast(0, None)),
                (None, forecast(1, Some(snap))),
            ],
            held: [HeldHow::Own, HeldHow::Push, HeldHow::Answer, HeldHow::Other]
                .into_iter()
                .map(|how| (Some(1), None, Some(3), held(how)))
                .collect(),
            haves: vec![(
                Some(4),
                HaveRecord {
                    at: at(-3),
                    peer: key(4),
                    stream: 0,
                    bin: Bin(2),
                    channel_open: at(-5000),
                },
            )],
            asks: [
                RequestEnd::Answered,
                RequestEnd::Expired,
                RequestEnd::Elsewhere,
                RequestEnd::Closed,
            ]
            .into_iter()
            .map(|end| (Some(1), vec![Some(2), None], ask(end)))
            .collect(),
            seed_decisions: kinds
                .into_iter()
                .enumerate()
                .map(|(i, kind)| {
                    let peer = (i % 3 != 0).then(|| key(i as u64));
                    (
                        peer.map(|_| i),
                        SeedDecision {
                            at: at(i as i64),
                            peer,
                            kind,
                        },
                    )
                })
                .collect(),
            seed_sends: sends,
            lost: Lost {
                forecasts: 1,
                held: 2,
                haves: 3,
                requests: 4,
                seed_log: 5,
            },
        }
    }

    /// Every record reads back as it was written, to the nanosecond, from
    /// a line's JSON as strims-sim's reader parses it.
    #[test]
    fn the_logs_read_back_as_the_records_they_wrote() {
        let o = Time::from_unix_millis(1_800_000_000_000) + Duration::from_nanos(123);
        let logs = every(o);
        let text = logs.json(o);
        let v: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(Logs::parse(&v, o), Some(logs.clone()));
        // A line's records hold them beside the rest.
        let line: Value =
            serde_json::from_str(&format!("{{\"rec\":{{\"logs\":{text}}}}}")).unwrap();
        assert_eq!(Logs::parse(&line["rec"]["logs"], o), Some(logs));
        assert!(Logs::default().is_empty() && !every(o).is_empty());
    }

    /// A value that isn't what a log writes reads as none: an unknown
    /// variant, a short record, a field of the wrong kind.
    #[test]
    fn what_no_log_wrote_reads_as_none() {
        let o = Time::EPOCH;
        let parse = |s: &str| SeedDecisionKind::get(&serde_json::from_str(s).unwrap(), o);
        assert_eq!(
            parse(r#"["deferred",2]"#),
            Some(SeedDecisionKind::Deferred { stream: 2 })
        );
        assert_eq!(parse(r#"["teleport",2]"#), None);
        assert_eq!(parse(r#"["released",1,3]"#), None);
        assert_eq!(parse(r#"["deferred","2"]"#), None);
        assert_eq!(HeldHow::get(&Value::from("Stolen"), o), None);
        assert_eq!(u8::get(&Value::from(300), o), None);
    }
}

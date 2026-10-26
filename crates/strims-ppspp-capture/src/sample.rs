// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Samples of the state below ppspp: what sat in a link's queues and its
//! transport, and in a simulation, what its model held. A node's driver
//! takes them at passes it already makes; they go into sample records,
//! named by these sources (sizes bytes, rates bytes per second, durations
//! nanoseconds, -1 unknown):
//!
//! | source | name | fields |
//! |---|---|---|
//! | 1 | `Queues` | `runner_pushes`, `runner_answers`, `qos_backlog`, `qos_blocked`, `qos_parked`, `qos_weight`, `link_pending`, `link_capacity`, `link_blocked`, `write_held`, `write_max` |
//! | 2 | `Tcp` | `ws`, `unsent`, `unacked`, `cwnd`, `srtt`, `rttvar`, `retrans`, `delivery_rate` |
//! | 3 | `WebRtc` | `buffered` |
//! | 4 | `WebSocket` | `buffered` |
//! | 5 | `SimConn` | `unsent`, `lowat`, `cwnd`, `ssthresh`, `in_flight`, `pending`, `uplink_queue`, `uplink_rate` |
//! | 6 | `SimUplink` | `free_in`, `rate`, `vtime`, `weight` |
//! | 8 | `Path` | `rtt`, `base`, `recent_min`, `in_flight`, `rate`, `queued`, `written_since`, `samples`, `age`, `hold`, `unanswered`, `unknown` |
//!
//! [`Sample`] documents each field.

use strims_base::path::PathReading;
use strims_base::time::{Duration, Time};

use crate::capture::{KindInfo, num};

/// The sources this crate writes.
pub const SAMPLE_KINDS: &[KindInfo] = &[
    KindInfo::new(
        1,
        "Queues",
        &[
            num("runner_pushes"),
            num("runner_answers"),
            num("qos_backlog"),
            num("qos_blocked"),
            num("qos_parked"),
            num("qos_weight"),
            num("link_pending"),
            num("link_capacity"),
            num("link_blocked"),
            num("write_held"),
            num("write_max"),
        ],
    ),
    KindInfo::new(
        2,
        "Tcp",
        &[
            num("ws"),
            num("unsent"),
            num("unacked"),
            num("cwnd"),
            num("srtt"),
            num("rttvar"),
            num("retrans"),
            num("delivery_rate"),
        ],
    ),
    KindInfo::new(3, "WebRtc", &[num("buffered")]),
    KindInfo::new(4, "WebSocket", &[num("buffered")]),
    KindInfo::new(
        5,
        "SimConn",
        &[
            num("unsent"),
            num("lowat"),
            num("cwnd"),
            num("ssthresh"),
            num("in_flight"),
            num("pending"),
            num("uplink_queue"),
            num("uplink_rate"),
        ],
    ),
    KindInfo::new(
        6,
        "SimUplink",
        &[num("free_in"), num("rate"), num("vtime"), num("weight")],
    ),
    KindInfo::new(
        8,
        "Path",
        &[
            num("rtt"),
            num("base"),
            num("recent_min"),
            num("in_flight"),
            num("rate"),
            num("queued"),
            num("written_since"),
            num("samples"),
            num("age"),
            num("hold"),
            num("unanswered"),
            num("unknown"),
        ],
    ),
];

/// The source with `code`.
pub fn sample_kind(code: u32) -> Option<&'static KindInfo> {
    SAMPLE_KINDS.iter().find(|k| k.code == code)
}

/// One reading below ppspp.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sample {
    /// The node's own queues for a link: the runner's data queue for the
    /// peer (entries of pushes and of answers to requests), the ppspp
    /// writer's QoS session (the length of its next frame, `None` with
    /// nothing to send; whether its link is blocked, and whether it left
    /// the queue for that; its weight), vnic's link buffer (bytes pending,
    /// its size, whether its writers are blocked) and the driver's write
    /// buffers (held by the transport's writer, and the bytes each holds at
    /// most).
    Queues {
        runner_pushes: u64,
        runner_answers: u64,
        qos_backlog: Option<u64>,
        qos_blocked: bool,
        qos_parked: bool,
        qos_weight: u64,
        link_pending: u64,
        link_capacity: u64,
        link_blocked: bool,
        write_held: u64,
        write_max: u64,
    },
    /// The kernel's view of a link's TCP socket (a WebSocket's when `ws`):
    /// bytes not yet sent and sent but unacknowledged, the congestion
    /// window in bytes, the smoothed round trip and its variation, the
    /// connection's retransmitted segments, and its delivery rate.
    Tcp {
        ws: bool,
        unsent: u64,
        unacked: u64,
        cwnd: u64,
        srtt: Duration,
        rttvar: Duration,
        retrans: u64,
        delivery_rate: u64,
    },
    /// A WebRTC data channel's buffered amount: bytes written and not yet
    /// acknowledged.
    WebRtc { buffered: u64 },
    /// A browser WebSocket's `bufferedAmount`, as of its latest send.
    WebSocket { buffered: u64 },
    /// A simulation's model of a link's TCP connection: bytes written and
    /// not yet admitted to the uplink, the socket's `TCP_NOTSENT_LOWAT`,
    /// the window model's congestion window, slow start threshold, bytes
    /// in flight and bytes waiting for the window (`None` without it), and
    /// the host's uplink queue as the time it takes to drain, and the
    /// uplink's rate (`None` without the WAN model).
    SimConn {
        unsent: u64,
        lowat: Option<u64>,
        cwnd: Option<u64>,
        ssthresh: Option<u64>,
        in_flight: Option<u64>,
        pending: Option<u64>,
        uplink_queue: Option<Duration>,
        uplink_rate: Option<u64>,
    },
    /// A simulation's model of the node's uplink as it sends a frame: the
    /// time until the uplink is free, its rate, and the fair queue's
    /// virtual time and weight of the frame's link.
    SimUplink {
        free_in: Duration,
        rate: u64,
        vtime: u64,
        weight: u64,
    },
    /// A link's path measure ([`strims_base::path`]) as of its latest
    /// echo: the round trip, the base and the recent minimum, the bytes in
    /// flight, the delivery rate and the bytes queued, the bytes written
    /// since, the echoes so far and the reading's age (all unknown before
    /// the first echo; base, recent minimum, rate and queued may be on
    /// their own). Then what the node holds itself: how long its latest
    /// echo to the peer waited, its probes the peer hasn't answered, and
    /// the peer's echoes of unknown probes.
    Path {
        rtt: Option<Duration>,
        base: Option<Duration>,
        recent_min: Option<Duration>,
        in_flight: Option<u64>,
        rate: Option<u64>,
        queued: Option<u64>,
        written_since: Option<u64>,
        samples: u64,
        age: Option<Duration>,
        hold: Duration,
        unanswered: u64,
        unknown: u64,
    },
}

impl Sample {
    /// A [`Sample::Path`] of `reading` at `now`.
    pub fn path(
        now: Time,
        reading: Option<&PathReading>,
        hold: Duration,
        unanswered: u64,
        unknown: u64,
    ) -> Sample {
        Sample::Path {
            rtt: reading.map(|r| r.rtt),
            base: reading.and_then(|r| r.base),
            recent_min: reading.and_then(|r| r.recent_min),
            in_flight: reading.map(|r| r.in_flight),
            rate: reading.and_then(|r| r.rate),
            queued: reading.and_then(|r| r.queued),
            written_since: reading.map(|r| r.written_since),
            samples: reading.map_or(0, |r| r.samples),
            age: reading.map(|r| r.age(now)),
            hold,
            unanswered,
            unknown,
        }
    }

    /// The source code and fields, in [`SAMPLE_KINDS`] order.
    pub fn encode(&self) -> (u32, Vec<i64>) {
        let u = |v: u64| i64::try_from(v).unwrap_or(i64::MAX);
        let b = i64::from;
        let opt = |v: Option<u64>| v.map_or(-1, u);
        match *self {
            Sample::Queues {
                runner_pushes,
                runner_answers,
                qos_backlog,
                qos_blocked,
                qos_parked,
                qos_weight,
                link_pending,
                link_capacity,
                link_blocked,
                write_held,
                write_max,
            } => (
                1,
                vec![
                    u(runner_pushes),
                    u(runner_answers),
                    opt(qos_backlog),
                    b(qos_blocked),
                    b(qos_parked),
                    u(qos_weight),
                    u(link_pending),
                    u(link_capacity),
                    b(link_blocked),
                    u(write_held),
                    u(write_max),
                ],
            ),
            Sample::Tcp {
                ws,
                unsent,
                unacked,
                cwnd,
                srtt,
                rttvar,
                retrans,
                delivery_rate,
            } => (
                2,
                vec![
                    b(ws),
                    u(unsent),
                    u(unacked),
                    u(cwnd),
                    srtt.as_nanos(),
                    rttvar.as_nanos(),
                    u(retrans),
                    u(delivery_rate),
                ],
            ),
            Sample::WebRtc { buffered } => (3, vec![u(buffered)]),
            Sample::WebSocket { buffered } => (4, vec![u(buffered)]),
            Sample::SimConn {
                unsent,
                lowat,
                cwnd,
                ssthresh,
                in_flight,
                pending,
                uplink_queue,
                uplink_rate,
            } => (
                5,
                vec![
                    u(unsent),
                    opt(lowat),
                    opt(cwnd),
                    opt(ssthresh),
                    opt(in_flight),
                    opt(pending),
                    uplink_queue.map_or(-1, Duration::as_nanos),
                    opt(uplink_rate),
                ],
            ),
            Sample::SimUplink {
                free_in,
                rate,
                vtime,
                weight,
            } => (6, vec![free_in.as_nanos(), u(rate), u(vtime), u(weight)]),
            Sample::Path {
                rtt,
                base,
                recent_min,
                in_flight,
                rate,
                queued,
                written_since,
                samples,
                age,
                hold,
                unanswered,
                unknown,
            } => {
                let d = |v: Option<Duration>| v.map_or(-1, Duration::as_nanos);
                (
                    8,
                    vec![
                        d(rtt),
                        d(base),
                        d(recent_min),
                        opt(in_flight),
                        opt(rate),
                        opt(queued),
                        opt(written_since),
                        u(samples),
                        d(age),
                        hold.as_nanos(),
                        u(unanswered),
                        u(unknown),
                    ],
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic)]

    use super::*;

    #[test]
    fn every_sample_encodes_its_table_fields() {
        let d = Duration::from_millis(3);
        let all = [
            Sample::Queues {
                runner_pushes: 1,
                runner_answers: 2,
                qos_backlog: None,
                qos_blocked: true,
                qos_parked: false,
                qos_weight: 1,
                link_pending: 3,
                link_capacity: 4,
                link_blocked: false,
                write_held: 5,
                write_max: 6,
            },
            Sample::Tcp {
                ws: false,
                unsent: 1,
                unacked: 2,
                cwnd: 3,
                srtt: d,
                rttvar: d,
                retrans: 4,
                delivery_rate: 5,
            },
            Sample::WebRtc { buffered: 1 },
            Sample::WebSocket { buffered: 1 },
            Sample::SimConn {
                unsent: 1,
                lowat: None,
                cwnd: Some(2),
                ssthresh: None,
                in_flight: None,
                pending: None,
                uplink_queue: Some(d),
                uplink_rate: None,
            },
            Sample::SimUplink {
                free_in: d,
                rate: 1,
                vtime: 2,
                weight: 3,
            },
            Sample::path(Time::EPOCH, None, d, 1, 0),
        ];
        for s in all {
            let (code, fields) = s.encode();
            let k = sample_kind(code).unwrap_or_else(|| panic!("{s:?}"));
            assert_eq!(k.fields.len(), fields.len(), "{s:?}");
        }
        assert_eq!(SAMPLE_KINDS.len(), all.len());
    }
}

// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! [`ProxyRtc`]: an [`RtcEndpoint`] whose peer connections live outside the
//! node task, behind commands and signals.
//!
//! A browser worker has no `RTCPeerConnection`; the window owns them. The
//! proxy keeps each connection's signalling state (its role, its
//! descriptions and its deadlines) and sends what the connection must do
//! as [`RtcCommand`]s. The driver carries them to the real connection and
//! reports back with [`RtcInput`]s, which the node passes to
//! [`RtcEndpoint::handle_signal`]. The data channel never passes through
//! the proxy: the driver attaches it as a stream link
//! (`Input::Connected { rtc: Some(conn), .. }`), so the proxy emits only
//! [`RtcEvent::LocalDescription`] and [`RtcEvent::Closed`].
//!
//! One connection's life:
//!
//! | node or driver | command | then |
//! |---|---|---|
//! | `create_connection` | `Create` | the connect deadline starts |
//! | first `poll_local_description` | `Describe` (offer or answer) | pending, with a describe deadline |
//! | `Signal(LocalDescription)` | | `LocalDescription` event; the next poll is ready |
//! | `set_remote_description` | `SetRemote` | the action is decided at once |
//! | `Signal(Connected)` | | the connect deadline is lifted |
//! | `close`, `close_all`, a deadline, `Signal(Closed)` | `Close` | one `Closed` event |
//!
//! Roles and misuse follow the native endpoint: the first
//! `poll_local_description` picks offer or answer with
//! [`WebRtcSignaling`], and `set_remote_description` refuses what the
//! native endpoint refuses, with the same messages. Session descriptions
//! are not parsed here; the connection rejects a bad one, and its
//! `Signal(Closed)` closes it.
//!
//! **Close reasons.** `Signal(Closed(reason))` closes with
//! [`RtcCloseReason::Remote`] when `reason` is one of [`CLOSED_BY_PEER`]
//! (the connection or its channel ended without an error), and with
//! [`RtcCloseReason::Error`] carrying `reason` otherwise. Both deadlines
//! close with [`RtcCloseReason::ConnectTimeout`].
//!
//! **Ids.** Connection ids are `id_base + 1`, `id_base + 2`, ..., up to
//! [`MAX_CONN_ID`], so they survive a trip through a JS number. A driver
//! that runs several proxies against one signalling peer gives each its own
//! `id_base`.
//!
//! **Determinism.** The proxy draws no randomness, and iterates its
//! connections in id order.

use std::collections::{BTreeMap, VecDeque};
use std::task::Poll;

use strims_base::time::{Duration, Time};
use strims_vnic::webrtc::{RemoteDescriptionAction, SdpType, SessionDescription, WebRtcSignaling};
use tokio::sync::mpsc;

use super::{RtcCloseReason, RtcConn, RtcEndpoint, RtcEvent, RtcFactory, RtcInput, RtcSignal};
use crate::config::WebRtcConfig;
use crate::event::DriverMetrics;

/// The largest connection id: `Number.MAX_SAFE_INTEGER`, the largest
/// integer a JS number holds exactly.
pub const MAX_CONN_ID: u64 = (1 << 53) - 1;

/// How long a requested local description may take: creating it plus ICE
/// gathering, which the connection bounds by the gather timeout it was
/// created with.
pub const DESCRIBE_TIMEOUT: Duration = Duration::from_secs(10);

/// `Signal(Closed)` reasons that mean the connection or its data channel
/// ended without an error; they close with [`RtcCloseReason::Remote`].
pub const CLOSED_BY_PEER: [&str; 2] = ["connection closed", "data channel closed"];

// The native endpoint's messages for the same misuse (`strims_webrtc::Error`).
const UNKNOWN_CONNECTION: &str = "unknown webrtc connection";
const TOO_MANY_CONNECTIONS: &str = "too many webrtc connections";
const REMOTE_DESCRIPTION_SET: &str = "remote description already set";
const UNEXPECTED_OFFER: &str = "received an offer after creating one";
const NO_PENDING_OFFER: &str = "received an answer without a pending offer";

/// What a proxied connection must do, in the order the proxy decided it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RtcCommand {
    /// Make the connection and its data channel. `gather_timeout` bounds
    /// ICE gathering for each local description.
    Create {
        conn: RtcConn,
        ice_servers: Vec<String>,
        gather_timeout: Duration,
    },
    /// Make the local description (`Offer` or `Answer`), apply it, gather,
    /// and report it with `Signal(LocalDescription)`.
    Describe { conn: RtcConn, sdp_type: SdpType },
    /// Apply the peer's description.
    SetRemote {
        conn: RtcConn,
        desc: SessionDescription,
    },
    /// Close the connection. Nothing more is reported for it.
    Close { conn: RtcConn },
}

#[derive(Debug)]
enum Local {
    NotRequested,
    Pending { sdp_type: SdpType, deadline: Time },
    Ready(SessionDescription),
}

#[derive(Debug)]
struct Conn {
    signaling: WebRtcSignaling,
    local: Local,
    /// Until `Signal(Connected)`; `None` without a connect timeout.
    connect_deadline: Option<Time>,
}

impl Conn {
    fn deadline(&self) -> Option<Time> {
        let describe = match self.local {
            Local::Pending { deadline, .. } => Some(deadline),
            _ => None,
        };
        match (describe, self.connect_deadline) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }
}

/// An [`RtcEndpoint`] that proxies its connections to the driver (see the
/// [module docs](self)).
#[derive(Debug)]
pub struct ProxyRtc {
    /// To the driver. Unbounded, and bounded in practice: a connection
    /// sends at most four commands (`Create`, `Describe`, `SetRemote`,
    /// `Close`), and at most `EndpointConfig::max_connections` are live,
    /// so while the driver keeps up the queue holds at most four per
    /// connection made since it last drained.
    commands: mpsc::UnboundedSender<RtcCommand>,
    ice_servers: Vec<String>,
    gather_timeout: Duration,
    connect_timeout: Option<Duration>,
    max_connections: usize,
    id_base: u64,
    /// Ids handed out so far.
    issued: u64,
    /// At most `max_connections`.
    conns: BTreeMap<RtcConn, Conn>,
    /// Drained by the node after every call.
    events: VecDeque<RtcEvent>,
    /// Scratch for `handle_timeout`.
    due: Vec<RtcConn>,
}

impl ProxyRtc {
    /// A proxy with `config`'s ICE servers, gather and connect timeouts and
    /// connection limit, sending its commands to `commands`. Its
    /// connection ids start after `id_base`.
    pub fn new(
        config: &WebRtcConfig,
        id_base: u64,
        commands: mpsc::UnboundedSender<RtcCommand>,
    ) -> ProxyRtc {
        let e = &config.endpoint;
        ProxyRtc {
            commands,
            ice_servers: e.ice_server_urls(),
            gather_timeout: e.stun_gather_timeout,
            connect_timeout: e.connect_timeout,
            max_connections: e.max_connections,
            id_base,
            issued: 0,
            conns: BTreeMap::new(),
            events: VecDeque::new(),
            due: Vec::new(),
        }
    }

    /// The node's endpoint factory for [`ProxyRtc::new`].
    pub fn factory(
        config: &WebRtcConfig,
        id_base: u64,
        commands: mpsc::UnboundedSender<RtcCommand>,
    ) -> RtcFactory {
        let proxy = ProxyRtc::new(config, id_base, commands);
        Box::new(move |_, _| Box::new(proxy))
    }

    fn send(&self, c: RtcCommand) {
        // A driver that stopped listening leaves its connections to the
        // deadlines.
        let _ = self.commands.send(c);
    }

    /// Ends a live connection: tells the driver and reports it once.
    fn end(&mut self, conn: RtcConn, reason: RtcCloseReason) {
        if self.conns.remove(&conn).is_some() {
            self.send(RtcCommand::Close { conn });
            self.events.push_back(RtcEvent::Closed { conn, reason });
        }
    }
}

fn close_reason(reason: String) -> RtcCloseReason {
    if CLOSED_BY_PEER.contains(&reason.as_str()) {
        RtcCloseReason::Remote
    } else {
        RtcCloseReason::Error(reason)
    }
}

impl RtcEndpoint for ProxyRtc {
    fn create_connection(&mut self, now: Time) -> Result<RtcConn, String> {
        if self.conns.len() >= self.max_connections {
            return Err(TOO_MANY_CONNECTIONS.to_owned());
        }
        if self.commands.is_closed() {
            return Err("webrtc signalling has stopped".to_owned());
        }
        let id = self
            .id_base
            .checked_add(self.issued.saturating_add(1))
            .filter(|&id| id <= MAX_CONN_ID)
            .ok_or_else(|| "webrtc connection ids are exhausted".to_owned())?;
        self.issued += 1;
        let conn = RtcConn(id);
        self.conns.insert(
            conn,
            Conn {
                signaling: WebRtcSignaling::new(),
                local: Local::NotRequested,
                connect_deadline: self.connect_timeout.map(|d| now + d),
            },
        );
        self.send(RtcCommand::Create {
            conn,
            ice_servers: self.ice_servers.clone(),
            gather_timeout: self.gather_timeout,
        });
        Ok(conn)
    }

    fn poll_local_description(
        &mut self,
        now: Time,
        conn: RtcConn,
    ) -> Poll<Result<SessionDescription, String>> {
        let Some(c) = self.conns.get_mut(&conn) else {
            return Poll::Ready(Err(UNKNOWN_CONNECTION.to_owned()));
        };
        match &c.local {
            Local::NotRequested => {
                let sdp_type = c.signaling.create_local_description();
                c.local = Local::Pending {
                    sdp_type,
                    deadline: now + DESCRIBE_TIMEOUT,
                };
                self.send(RtcCommand::Describe { conn, sdp_type });
                Poll::Pending
            }
            Local::Pending { .. } => Poll::Pending,
            Local::Ready(d) => Poll::Ready(Ok(d.clone())),
        }
    }

    fn set_remote_description(
        &mut self,
        _now: Time,
        conn: RtcConn,
        desc: &SessionDescription,
    ) -> Result<RemoteDescriptionAction, String> {
        let Some(c) = self.conns.get_mut(&conn) else {
            return Err(UNKNOWN_CONNECTION.to_owned());
        };
        if c.signaling.has_remote_description() {
            return Err(REMOTE_DESCRIPTION_SET.to_owned());
        }
        match desc.sdp_type {
            SdpType::Offer => {
                if !matches!(c.local, Local::NotRequested) {
                    return Err(UNEXPECTED_OFFER.to_owned());
                }
            }
            SdpType::Answer => {
                // An offer is pending once it was made and handed out.
                let offered = matches!(
                    &c.local,
                    Local::Ready(SessionDescription {
                        sdp_type: SdpType::Offer,
                        ..
                    })
                );
                if !offered {
                    return Err(NO_PENDING_OFFER.to_owned());
                }
            }
            t => return Err(format!("unsupported session description type {t:?}")),
        }
        let action = c.signaling.set_remote_description();
        self.send(RtcCommand::SetRemote {
            conn,
            desc: desc.clone(),
        });
        Ok(action)
    }

    fn close(&mut self, _now: Time, conn: RtcConn) {
        self.end(conn, RtcCloseReason::Local);
    }

    fn close_all(&mut self, _now: Time) {
        while let Some((&conn, _)) = self.conns.first_key_value() {
            self.end(conn, RtcCloseReason::Local);
        }
    }

    fn poll_event(&mut self) -> Option<RtcEvent> {
        self.events.pop_front()
    }

    fn poll_timeout(&self) -> Option<Time> {
        self.conns.values().filter_map(Conn::deadline).min()
    }

    fn handle_timeout(&mut self, now: Time) {
        let mut due = std::mem::take(&mut self.due);
        due.clear();
        due.extend(
            self.conns
                .iter()
                .filter(|(_, c)| c.deadline().is_some_and(|d| d <= now))
                .map(|(&k, _)| k),
        );
        for &conn in &due {
            self.end(conn, RtcCloseReason::ConnectTimeout);
        }
        self.due = due;
    }

    /// Commands go out as they are made, so there is never anything to
    /// flush.
    fn flush(&mut self, _metrics: &mut DriverMetrics) -> bool {
        false
    }

    /// Signals for connections that are unknown or closed are dropped, and
    /// so is a local description nobody asked for.
    fn handle_signal(&mut self, _now: Time, input: RtcInput) {
        let RtcInput { conn, signal } = input;
        let Some(c) = self.conns.get_mut(&conn) else {
            return;
        };
        match signal {
            RtcSignal::LocalDescription(d) => match c.local {
                Local::Pending { sdp_type, .. } if sdp_type == d.sdp_type => {
                    c.local = Local::Ready(d);
                    self.events.push_back(RtcEvent::LocalDescription(conn));
                }
                Local::Pending { sdp_type, .. } => {
                    let e = format!(
                        "the local description is an {}, not an {}",
                        d.sdp_type.as_str(),
                        sdp_type.as_str()
                    );
                    self.end(conn, RtcCloseReason::Error(e));
                }
                Local::NotRequested | Local::Ready(_) => {}
            },
            RtcSignal::Connected => c.connect_deadline = None,
            RtcSignal::Closed(reason) => self.end(conn, close_reason(reason)),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::config::WebRtcConfig;

    fn epoch() -> Time {
        Time::from_unix_millis(1_700_000_000_000)
    }

    fn sdp(sdp_type: SdpType) -> SessionDescription {
        SessionDescription {
            sdp_type,
            sdp: format!("v=0\r\ns={}\r\n", sdp_type.as_str()),
        }
    }

    struct Fixture {
        rtc: ProxyRtc,
        commands: mpsc::UnboundedReceiver<RtcCommand>,
    }

    impl Fixture {
        fn new(config: &WebRtcConfig) -> Fixture {
            let (tx, commands) = mpsc::unbounded_channel();
            Fixture {
                rtc: ProxyRtc::new(config, 1000, tx),
                commands,
            }
        }

        fn command(&mut self) -> Option<RtcCommand> {
            self.commands.try_recv().ok()
        }

        fn events(&mut self) -> Vec<RtcEvent> {
            std::iter::from_fn(|| self.rtc.poll_event()).collect()
        }

        fn signal(&mut self, conn: RtcConn, signal: RtcSignal) {
            self.rtc.handle_signal(epoch(), RtcInput { conn, signal });
        }

        /// A connection whose local description of `sdp_type` is ready.
        fn described(&mut self, remote_first: bool) -> RtcConn {
            let conn = self.rtc.create_connection(epoch()).unwrap();
            let sdp_type = if remote_first {
                self.rtc
                    .set_remote_description(epoch(), conn, &sdp(SdpType::Offer))
                    .unwrap();
                SdpType::Answer
            } else {
                SdpType::Offer
            };
            assert!(self.rtc.poll_local_description(epoch(), conn).is_pending());
            self.signal(conn, RtcSignal::LocalDescription(sdp(sdp_type)));
            assert_eq!(self.events(), [RtcEvent::LocalDescription(conn)]);
            while self.command().is_some() {}
            conn
        }
    }

    fn fixture() -> Fixture {
        Fixture::new(&WebRtcConfig {
            enabled: true,
            ..WebRtcConfig::default()
        })
    }

    fn closed(conn: RtcConn, reason: RtcCloseReason) -> RtcEvent {
        RtcEvent::Closed { conn, reason }
    }

    /// The offerer: create, describe an offer once gathering reports it,
    /// then the answer awaits the link.
    #[test]
    fn an_offer_is_described_then_answered() {
        let mut f = fixture();
        let conn = f.rtc.create_connection(epoch()).unwrap();
        assert_eq!(conn, RtcConn(1001));
        assert_eq!(
            f.command(),
            Some(RtcCommand::Create {
                conn,
                ice_servers: strims_vnic::webrtc::DEFAULT_ICE_SERVERS
                    .iter()
                    .map(|s| (*s).to_owned())
                    .collect(),
                gather_timeout: Duration::from_secs(5),
            })
        );

        assert!(f.rtc.poll_local_description(epoch(), conn).is_pending());
        assert_eq!(
            f.command(),
            Some(RtcCommand::Describe {
                conn,
                sdp_type: SdpType::Offer
            })
        );
        // Polling again asks for nothing more.
        assert!(f.rtc.poll_local_description(epoch(), conn).is_pending());
        assert_eq!(f.command(), None);
        assert!(f.events().is_empty());

        let offer = sdp(SdpType::Offer);
        f.signal(conn, RtcSignal::LocalDescription(offer.clone()));
        assert_eq!(f.events(), [RtcEvent::LocalDescription(conn)]);
        assert_eq!(
            f.rtc.poll_local_description(epoch(), conn),
            Poll::Ready(Ok(offer.clone()))
        );
        // A second description isn't asked for, and changes nothing.
        f.signal(conn, RtcSignal::LocalDescription(sdp(SdpType::Offer)));
        assert!(f.events().is_empty());

        let answer = sdp(SdpType::Answer);
        assert_eq!(
            f.rtc.set_remote_description(epoch(), conn, &answer),
            Ok(RemoteDescriptionAction::AwaitLink)
        );
        assert_eq!(
            f.command(),
            Some(RtcCommand::SetRemote { conn, desc: answer })
        );
        f.signal(conn, RtcSignal::Connected);
        assert_eq!(f.rtc.poll_timeout(), None, "connected and described");
        assert_eq!(f.command(), None);
        assert!(f.events().is_empty());
    }

    /// The answerer: the offer arrives first, and the description asked
    /// for next is the answer.
    #[test]
    fn an_offer_received_is_answered_in_the_background() {
        let mut f = fixture();
        let conn = f.rtc.create_connection(epoch()).unwrap();
        f.command();
        let offer = sdp(SdpType::Offer);
        assert_eq!(
            f.rtc.set_remote_description(epoch(), conn, &offer),
            Ok(RemoteDescriptionAction::AwaitInBackground)
        );
        assert_eq!(
            f.command(),
            Some(RtcCommand::SetRemote { conn, desc: offer })
        );
        assert!(f.rtc.poll_local_description(epoch(), conn).is_pending());
        assert_eq!(
            f.command(),
            Some(RtcCommand::Describe {
                conn,
                sdp_type: SdpType::Answer
            })
        );
        let answer = sdp(SdpType::Answer);
        f.signal(conn, RtcSignal::LocalDescription(answer.clone()));
        assert_eq!(f.events(), [RtcEvent::LocalDescription(conn)]);
        assert_eq!(
            f.rtc.poll_local_description(epoch(), conn),
            Poll::Ready(Ok(answer))
        );
    }

    /// A description that takes longer than `DESCRIBE_TIMEOUT` closes its
    /// connection, before the connect deadline.
    #[test]
    fn a_slow_description_times_out() {
        let mut f = fixture();
        let conn = f.rtc.create_connection(epoch()).unwrap();
        let later = epoch() + Duration::from_secs(1);
        assert!(f.rtc.poll_local_description(later, conn).is_pending());
        let deadline = later + DESCRIBE_TIMEOUT;
        assert_eq!(f.rtc.poll_timeout(), Some(deadline));
        while f.command().is_some() {}

        f.rtc.handle_timeout(deadline - Duration::MILLISECOND);
        assert!(f.events().is_empty());
        f.rtc.handle_timeout(deadline);
        assert_eq!(f.events(), [closed(conn, RtcCloseReason::ConnectTimeout)]);
        assert_eq!(f.command(), Some(RtcCommand::Close { conn }));
        assert_eq!(f.rtc.poll_timeout(), None);
    }

    /// A connection that isn't connected within the connect timeout of its
    /// creation closes; one that connected in time stays.
    #[test]
    fn a_connection_that_never_connects_times_out() {
        let mut f = fixture();
        let slow = f.described(false);
        let fast = f.described(true);
        f.signal(fast, RtcSignal::Connected);
        let deadline = epoch() + Duration::from_secs(20);
        assert_eq!(f.rtc.poll_timeout(), Some(deadline));
        f.rtc.handle_timeout(deadline - Duration::MILLISECOND);
        assert!(f.events().is_empty());
        f.rtc.handle_timeout(deadline);
        assert_eq!(f.events(), [closed(slow, RtcCloseReason::ConnectTimeout)]);
        assert_eq!(f.command(), Some(RtcCommand::Close { conn: slow }));
        assert_eq!(f.rtc.poll_timeout(), None);
        assert!(
            f.rtc.poll_local_description(deadline, fast).is_ready(),
            "the connected one lives"
        );
    }

    /// No connect deadline without a connect timeout.
    #[test]
    fn without_a_connect_timeout_only_descriptions_expire() {
        let mut config = WebRtcConfig::default();
        config.endpoint.connect_timeout = None;
        let mut f = Fixture::new(&config);
        let conn = f.described(false);
        assert_eq!(f.rtc.poll_timeout(), None);
        f.rtc.handle_timeout(epoch() + Duration::from_secs(3600));
        assert!(f.events().is_empty());
        assert!(f.rtc.poll_local_description(epoch(), conn).is_ready());
    }

    /// At most `max_connections` live at once; closing one makes room.
    #[test]
    fn connections_are_limited() {
        let mut config = WebRtcConfig::default();
        config.endpoint.max_connections = 2;
        let mut f = Fixture::new(&config);
        let a = f.rtc.create_connection(epoch()).unwrap();
        let b = f.rtc.create_connection(epoch()).unwrap();
        assert_ne!(a, b);
        assert_eq!(
            f.rtc.create_connection(epoch()),
            Err("too many webrtc connections".to_owned())
        );
        f.rtc.close(epoch(), a);
        let c = f.rtc.create_connection(epoch()).unwrap();
        assert!(c != a && c != b, "ids are never reused");
    }

    /// `close` and `close_all` report each connection closed once, and
    /// the driver hears `Close` once.
    #[test]
    fn a_connection_closes_once() {
        let mut f = fixture();
        let a = f.described(false);
        let b = f.described(true);
        let c = f.rtc.create_connection(epoch()).unwrap();
        f.command();

        f.rtc.close(epoch(), a);
        f.rtc.close(epoch(), a);
        assert_eq!(f.events(), [closed(a, RtcCloseReason::Local)]);
        assert_eq!(f.command(), Some(RtcCommand::Close { conn: a }));
        assert_eq!(f.command(), None);

        f.rtc.close_all(epoch());
        f.rtc.close_all(epoch());
        assert_eq!(
            f.events(),
            [
                closed(b, RtcCloseReason::Local),
                closed(c, RtcCloseReason::Local)
            ]
        );
        assert_eq!(f.command(), Some(RtcCommand::Close { conn: b }));
        assert_eq!(f.command(), Some(RtcCommand::Close { conn: c }));
        assert_eq!(f.command(), None);

        // A closed connection is unknown, to signals and calls alike.
        f.signal(b, RtcSignal::Closed("connection failed".to_owned()));
        f.signal(c, RtcSignal::Connected);
        assert!(f.events().is_empty());
        assert_eq!(
            f.rtc.poll_local_description(epoch(), a),
            Poll::Ready(Err("unknown webrtc connection".to_owned()))
        );
        assert_eq!(f.rtc.poll_timeout(), None);
    }

    /// Signals for connections the proxy never made change nothing.
    #[test]
    fn signals_for_unknown_connections_are_dropped() {
        let mut f = fixture();
        let conn = f.rtc.create_connection(epoch()).unwrap();
        f.command();
        let other = RtcConn(conn.0 + 1);
        f.signal(other, RtcSignal::LocalDescription(sdp(SdpType::Offer)));
        f.signal(other, RtcSignal::Connected);
        f.signal(other, RtcSignal::Closed("connection closed".to_owned()));
        assert!(f.events().is_empty());
        assert_eq!(f.command(), None);
        assert!(f.rtc.poll_timeout().is_some(), "the real one still waits");
    }

    /// The connection's own close reasons: an ended connection or channel
    /// is the peer's doing, anything else an error.
    #[test]
    fn closed_signals_pick_their_reason() {
        let mut f = fixture();
        let cases = [
            ("connection closed", RtcCloseReason::Remote),
            ("data channel closed", RtcCloseReason::Remote),
            (
                "connection failed",
                RtcCloseReason::Error("connection failed".to_owned()),
            ),
            (
                "InvalidAccessError: bad sdp",
                RtcCloseReason::Error("InvalidAccessError: bad sdp".to_owned()),
            ),
        ];
        for (reason, want) in cases {
            let conn = f.rtc.create_connection(epoch()).unwrap();
            f.command();
            f.signal(conn, RtcSignal::Closed(reason.to_owned()));
            assert_eq!(f.events(), [closed(conn, want)], "{reason}");
            assert_eq!(f.command(), Some(RtcCommand::Close { conn }));
        }
    }

    /// A local description of the wrong type is an error; one that wasn't
    /// asked for is ignored.
    #[test]
    fn a_mismatched_local_description_closes() {
        let mut f = fixture();
        let conn = f.rtc.create_connection(epoch()).unwrap();
        f.signal(conn, RtcSignal::LocalDescription(sdp(SdpType::Offer)));
        assert!(f.events().is_empty(), "not asked for yet");
        assert!(f.rtc.poll_local_description(epoch(), conn).is_pending());
        f.signal(conn, RtcSignal::LocalDescription(sdp(SdpType::Answer)));
        assert_eq!(
            f.events(),
            [closed(
                conn,
                RtcCloseReason::Error(
                    "the local description is an answer, not an offer".to_owned()
                )
            )]
        );
    }

    /// The same misuse fails with the native endpoint's messages.
    #[test]
    fn misuse_fails_like_the_native_endpoint() {
        let mut f = fixture();
        let unknown = RtcConn(1);
        assert_eq!(
            f.rtc
                .set_remote_description(epoch(), unknown, &sdp(SdpType::Offer)),
            Err(UNKNOWN_CONNECTION.to_owned())
        );
        assert_eq!(
            f.rtc.poll_local_description(epoch(), unknown),
            Poll::Ready(Err(UNKNOWN_CONNECTION.to_owned()))
        );

        // An answer before any offer was made, then while it gathers.
        let offerer = f.rtc.create_connection(epoch()).unwrap();
        assert_eq!(
            f.rtc
                .set_remote_description(epoch(), offerer, &sdp(SdpType::Answer)),
            Err(NO_PENDING_OFFER.to_owned())
        );
        assert!(f.rtc.poll_local_description(epoch(), offerer).is_pending());
        assert_eq!(
            f.rtc
                .set_remote_description(epoch(), offerer, &sdp(SdpType::Answer)),
            Err(NO_PENDING_OFFER.to_owned())
        );
        // An offer after making one.
        assert_eq!(
            f.rtc
                .set_remote_description(epoch(), offerer, &sdp(SdpType::Offer)),
            Err(UNEXPECTED_OFFER.to_owned())
        );
        for t in [SdpType::Pranswer, SdpType::Rollback] {
            assert_eq!(
                f.rtc.set_remote_description(epoch(), offerer, &sdp(t)),
                Err(format!("unsupported session description type {t:?}"))
            );
        }

        // A second remote description.
        let answerer = f.rtc.create_connection(epoch()).unwrap();
        f.rtc
            .set_remote_description(epoch(), answerer, &sdp(SdpType::Offer))
            .unwrap();
        while f.command().is_some() {}
        for t in [SdpType::Offer, SdpType::Answer, SdpType::Pranswer] {
            assert_eq!(
                f.rtc.set_remote_description(epoch(), answerer, &sdp(t)),
                Err(REMOTE_DESCRIPTION_SET.to_owned())
            );
        }
        assert_eq!(f.command(), None, "refused descriptions aren't sent");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn misuse_messages_are_the_native_endpoints() {
        use strims_webrtc::Error;
        assert_eq!(UNKNOWN_CONNECTION, Error::UnknownConnection.to_string());
        assert_eq!(TOO_MANY_CONNECTIONS, Error::TooManyConnections.to_string());
        assert_eq!(
            REMOTE_DESCRIPTION_SET,
            Error::RemoteDescriptionSet.to_string()
        );
        assert_eq!(UNEXPECTED_OFFER, Error::UnexpectedOffer.to_string());
        assert_eq!(NO_PENDING_OFFER, Error::NoPendingOffer.to_string());
        assert_eq!(
            format!(
                "unsupported session description type {:?}",
                SdpType::Pranswer
            ),
            Error::UnsupportedSdpType(SdpType::Pranswer).to_string()
        );
    }

    /// Ids stay JS-safe integers: the last one is `MAX_CONN_ID`.
    #[test]
    fn ids_end_at_the_largest_safe_integer() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut rtc = ProxyRtc::new(&WebRtcConfig::default(), MAX_CONN_ID - 2, tx);
        assert_eq!(rtc.create_connection(epoch()), Ok(RtcConn(MAX_CONN_ID - 1)));
        assert_eq!(rtc.create_connection(epoch()), Ok(RtcConn(MAX_CONN_ID)));
        assert_eq!(
            rtc.create_connection(epoch()),
            Err("webrtc connection ids are exhausted".to_owned())
        );
    }

    /// Without a driver listening, no connection can make progress.
    #[test]
    fn connections_need_a_listening_driver() {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut rtc = ProxyRtc::new(&WebRtcConfig::default(), 0, tx);
        drop(rx);
        assert_eq!(
            rtc.create_connection(epoch()),
            Err("webrtc signalling has stopped".to_owned())
        );
    }

    #[test]
    fn the_factory_builds_a_proxy() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let factory = ProxyRtc::factory(&WebRtcConfig::default(), 7, tx);
        let mut ep = factory(epoch(), strims_base::rng::det_rng(0));
        assert_eq!(ep.create_connection(epoch()), Ok(RtcConn(8)));
        assert!(matches!(rx.try_recv(), Ok(RtcCommand::Create { .. })));
    }
}

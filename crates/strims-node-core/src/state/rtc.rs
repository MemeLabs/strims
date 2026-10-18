// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The WebRTC interface inside the node task: the endpoint
//! ([`RtcEndpoint`]) and the node's bookkeeping of its connections.
//!
//! The endpoint is a sans-IO state machine like vnic, so it lives in
//! [`State`] and is driven the same way:
//!
//! - **Driver input.** Datagrams, resolved STUN servers and signals go to
//!   the endpoint; after every call into it, [`State::drain_rtc`] flushes
//!   its output and handles its events.
//! - **Timers.** Its deadline is part of [`State::poll_timeout`].
//! - **Links.** An in-process channel's `Open` adds a vnic link
//!   (`LinkConfig::webrtc().with_mtu`); `Data` is read like a stream
//!   link's bytes; vnic's egress is written into the channel
//!   (`write_channel`) until it is full, and `Writable` resumes it. A
//!   channel the driver owns is attached as a stream link instead
//!   (`Input::Connected` with `rtc`). Either way the link's slot names its
//!   connection: an endpoint `Closed` removes the vnic link (the transport
//!   went away), and vnic's `LinkClosed` or the transport failing closes
//!   the connection.
//! - **Signalling.** Every peer exchange gets a connection. Its candidate
//!   (`crate::exchange`) is filled in from here: the gathered local
//!   description, the applied remote description and the role it implies,
//!   and the outcome (`Linked` at `PeerAdded`, or a failure). When the
//!   exchange ends, a connection that never opened is closed; an open one
//!   carries a link that outlives the exchange.

use std::collections::BTreeMap;
use std::task::Poll;

use strims_base::time::Time;
use strims_vnic::webrtc::RemoteDescriptionAction;
use strims_vnic::{CandidateError, LinkCandidatePool, LinkConfig, LinkKind, PeerKey as LinkKey};
use strims_vpn::ExchangeId;

use super::{DialError, LinkSlot, LinkTransport, State, VnicHost};
use crate::exchange::{Candidate, CandidateState};
use crate::rtc::{RtcConn, RtcEndpoint, RtcEvent, RtcInput};

/// An in-process data channel's write state.
#[derive(Debug)]
pub(crate) struct DataChannel {
    /// The channel's send buffer filled up: wait for `Writable`.
    blocked: bool,
    /// The buffered amount reached `max_buffered_amount`: wait until it
    /// drains to half.
    capped: bool,
}

impl DataChannel {
    /// The channel takes no more bytes for now.
    pub(crate) fn blocked(&self) -> bool {
        self.blocked || self.capped
    }
}

/// The node's view of one connection.
#[derive(Debug)]
struct ConnSlot {
    /// The exchange and candidate index it was made for, until the
    /// exchange ends.
    exchange: Option<(ExchangeId, usize)>,
    /// The vnic link, once the data channel opened.
    link: Option<LinkKey>,
    /// We offered (the answer was applied with `AwaitLink`).
    offerer: bool,
}

/// The endpoint and its connections.
#[derive(Debug)]
pub(crate) struct Rtc {
    pub(crate) ep: Box<dyn RtcEndpoint>,
    /// `WebRtcConfig::max_buffered_amount`.
    max_buffered: usize,
    /// Bounded by the endpoint's connection limit.
    conns: BTreeMap<RtcConn, ConnSlot>,
}

impl Rtc {
    pub(super) fn new(ep: Box<dyn RtcEndpoint>, max_buffered: usize) -> Rtc {
        Rtc {
            ep,
            max_buffered,
            conns: BTreeMap::new(),
        }
    }
}

/// Writes the link's pending bytes into its in-process data channel until
/// vnic has no more, the channel is full, or its buffered amount reaches
/// `max_buffered`. Returns the bytes taken.
///
/// A capped channel resumes once its buffered amount is down to half the
/// cap. Acknowledgements arrive as driver input, and the pass after every
/// input calls this, so the check needs no event of its own.
pub(super) fn write_channel(
    rtc: &mut Rtc,
    conn: RtcConn,
    dc: &mut DataChannel,
    vnic: &mut VnicHost,
    link: LinkKey,
) -> usize {
    if dc.capped && rtc.ep.buffered_amount(conn) > rtc.max_buffered / 2 {
        return 0;
    }
    dc.capped = false;
    let mut total = 0;
    while !dc.blocked {
        let room = rtc
            .max_buffered
            .saturating_sub(rtc.ep.buffered_amount(conn));
        if room == 0 {
            dc.capped = true;
            break;
        }
        let data = match vnic.poll_link_transmit(link) {
            Ok(d) if !d.is_empty() => d,
            _ => break,
        };
        let data = data.get(..room.min(data.len())).unwrap_or_default();
        let len = data.len();
        match rtc.ep.write(conn, data) {
            Ok(n) => {
                if n > 0 {
                    let _ = vnic.advance_link_transmit(link, n);
                    total += n;
                }
                // Short: the send buffer is full; `Writable` follows.
                dc.blocked = n < len;
            }
            Err(e) => {
                // Not open (yet, or any more): `Closed` follows.
                tracing::trace!(error = %e, "data channel write");
                break;
            }
        }
    }
    total
}

/// Records the local descriptions of the pool's WebRTC candidates that
/// finished gathering. The endpoint's first poll of a connection decides
/// offer or answer, so this runs only once vpn asked for descriptions.
pub(super) fn poll_local_descriptions(rtc: &mut Rtc, candidates: &mut [Candidate], now: Time) {
    for c in candidates {
        let Some(conn) = c.conn else {
            continue;
        };
        if !c.local_pending() {
            continue;
        }
        match rtc.ep.poll_local_description(now, conn) {
            Poll::Ready(Ok(d)) => c.set_local(Ok(Some(d.to_link_description()))),
            Poll::Ready(Err(e)) => c.set_local(Err(CandidateError::Failed(e))),
            Poll::Pending => {}
        }
    }
}

/// Applies a WebRTC candidate's remote description (`Describe`): an answer
/// to our offer awaits the link, an offer is answered and linked in the
/// background.
pub(super) fn apply_remote_description(rtc: Option<&mut Rtc>, c: &mut Candidate, now: Time) {
    let CandidateState::Describe(desc) = std::mem::replace(&mut c.state, CandidateState::Idle)
    else {
        return;
    };
    let (Some(rtc), Some(conn)) = (rtc, c.conn) else {
        c.state =
            CandidateState::Failed(CandidateError::Failed("webrtc is not enabled".to_owned()));
        return;
    };
    c.state = match rtc.ep.set_remote_description(now, conn, &desc) {
        Ok(RemoteDescriptionAction::AwaitLink) => {
            if let Some(rc) = rtc.conns.get_mut(&conn) {
                rc.offerer = true;
            }
            CandidateState::AwaitLink
        }
        Ok(RemoteDescriptionAction::AwaitInBackground) => CandidateState::Background,
        Err(e) => CandidateState::Failed(CandidateError::Failed(e)),
    };
}

impl State {
    /// A new connection and its candidate for exchange `x`, at index `i`
    /// of its pool. `None` when WebRTC is off or the endpoint refused.
    pub(super) fn rtc_candidate(
        &mut self,
        x: ExchangeId,
        i: usize,
        now: Time,
    ) -> Option<Candidate> {
        let rtc = self.rtc.as_mut()?;
        match rtc.ep.create_connection(now) {
            Ok(conn) => {
                self.metrics.webrtc_connections += 1;
                rtc.conns.insert(
                    conn,
                    ConnSlot {
                        exchange: Some((x, i)),
                        link: None,
                        offerer: false,
                    },
                );
                Some(Candidate::webrtc(conn))
            }
            Err(e) => {
                tracing::debug!(error = %e, "webrtc candidate");
                None
            }
        }
    }

    /// A datagram the driver read from UDP socket `socket`.
    pub(crate) fn handle_datagram(
        &mut self,
        socket: usize,
        from: std::net::SocketAddr,
        data: &[u8],
        now: Time,
    ) {
        let Some(rtc) = &mut self.rtc else {
            return;
        };
        if rtc
            .ep
            .handle_datagram(now, socket, from, data, &mut self.metrics)
        {
            self.process_events(now);
        }
    }

    pub(crate) fn return_udp_buf(&mut self, socket: usize, buf: Vec<u8>) {
        if let Some(rtc) = &mut self.rtc {
            rtc.ep.return_udp_buffer(socket, buf);
        }
    }

    /// The STUN servers, resolved. Connections created afterwards gather
    /// server-reflexive candidates from them.
    pub(crate) fn set_stun_servers(&mut self, addrs: Vec<std::net::SocketAddr>) {
        if let Some(rtc) = &mut self.rtc {
            rtc.ep.set_stun_servers(addrs);
        }
    }

    /// A signal about a connection outside the node task.
    pub(crate) fn handle_rtc_signal(&mut self, input: RtcInput, now: Time) {
        let Some(rtc) = &mut self.rtc else {
            return;
        };
        rtc.ep.handle_signal(now, input);
        self.process_events(now);
    }

    /// Flushes the endpoint's output and handles its events until it has
    /// none left. Returns whether there were any. Not re-entered: a handler
    /// that drains events (a data-channel read) leaves the endpoint's to
    /// this loop.
    pub(super) fn drain_rtc(&mut self, now: Time) -> bool {
        if self.rtc.is_none() || self.rtc_draining {
            return false;
        }
        self.rtc_draining = true;
        let mut any = false;
        loop {
            let mut progress = match &mut self.rtc {
                Some(rtc) => rtc.ep.flush(&mut self.metrics),
                None => false,
            };
            if let Some(e) = self.rtc.as_mut().and_then(|r| r.ep.poll_event()) {
                progress = true;
                self.on_rtc_event(e, now);
            }
            if !progress {
                break;
            }
            any = true;
        }
        self.rtc_draining = false;
        any
    }

    fn rtc_conn(&self, conn: RtcConn) -> Option<&ConnSlot> {
        self.rtc.as_ref()?.conns.get(&conn)
    }

    fn on_rtc_event(&mut self, e: RtcEvent, now: Time) {
        match e {
            RtcEvent::LocalDescription(conn) => {
                if let Some((x, _)) = self.rtc_conn(conn).and_then(|c| c.exchange) {
                    self.gather(x, now);
                }
            }
            RtcEvent::Open { conn, mtu } => self.rtc_open(conn, mtu, now),
            RtcEvent::Data { conn, mut data } => {
                if let Some(link) = self.rtc_conn(conn).and_then(|c| c.link) {
                    self.handle_read(link, now, &mut data);
                }
            }
            RtcEvent::Writable(conn) => {
                if let Some(link) = self.rtc_conn(conn).and_then(|c| c.link)
                    && let Some(LinkSlot {
                        io: LinkTransport::DataChannel(dc),
                        ..
                    }) = self.links.get_mut(link)
                {
                    dc.blocked = false;
                }
            }
            RtcEvent::Closed { conn, reason } => {
                self.metrics.webrtc_closed += 1;
                let Some(rc) = self.rtc.as_mut().and_then(|r| r.conns.remove(&conn)) else {
                    return;
                };
                if let Some(link) = rc.link
                    && self.links.remove(link).is_some()
                {
                    // The transport went away.
                    tracing::debug!(?reason, "webrtc link down");
                    self.metrics.transport_failures += 1;
                    self.vnic.remove_link(link);
                }
                if let Some((x, i)) = rc.exchange {
                    let e = CandidateError::Failed(format!("webrtc: {reason:?}"));
                    self.fail_candidate(x, i, e, now);
                }
            }
        }
    }

    /// An in-process data channel opened: add the vnic link. There is no
    /// transport handshake before `PeerInit`.
    fn rtc_open(&mut self, conn: RtcConn, mtu: usize, now: Time) {
        let State {
            rtc,
            vnic,
            links,
            metrics,
            ..
        } = self;
        let Some(rtc) = rtc else {
            return;
        };
        let Some(rc) = rtc.conns.get_mut(&conn) else {
            return;
        };
        metrics.webrtc_opened += 1;
        match vnic.add_link(now, LinkConfig::webrtc().with_mtu(mtu)) {
            Ok(link) => {
                rc.link = Some(link);
                links.insert(
                    link,
                    LinkSlot {
                        io: LinkTransport::DataChannel(DataChannel {
                            blocked: false,
                            capped: false,
                        }),
                        kind: LinkKind::WebRtc,
                        dial: None,
                        dialed: rc.offerer,
                        rtc: Some(conn),
                    },
                );
            }
            Err(e) => {
                tracing::debug!(error = %e, "adding a webrtc link");
                self.rtc_link_failed(conn, e.to_string(), now);
            }
        }
    }

    /// Whether a channel the driver attaches for `conn` can become its
    /// link: the connection is live and has no link yet. Counts the
    /// channel as opened when it can.
    pub(crate) fn rtc_accepts_stream(&mut self, conn: RtcConn) -> bool {
        let open = self.rtc_conn(conn).is_some_and(|c| c.link.is_none());
        if open {
            self.metrics.webrtc_opened += 1;
        }
        open
    }

    /// Ties a link the driver attached to its connection. Returns whether
    /// this node offered.
    pub(super) fn rtc_stream_linked(&mut self, conn: RtcConn, link: LinkKey) -> bool {
        match self.rtc.as_mut().and_then(|r| r.conns.get_mut(&conn)) {
            Some(rc) => {
                rc.link = Some(link);
                rc.offerer
            }
            None => false,
        }
    }

    /// A connection's channel couldn't become a link: close the
    /// connection, and fail its candidate.
    pub(crate) fn rtc_link_failed(&mut self, conn: RtcConn, error: String, now: Time) {
        let Some(rtc) = &mut self.rtc else {
            return;
        };
        rtc.ep.close(now, conn);
        if let Some((x, i)) = rtc.conns.get(&conn).and_then(|c| c.exchange) {
            self.fail_candidate(x, i, CandidateError::Failed(error), now);
        }
    }

    /// vnic's `PeerAdded` for a WebRTC link: an offerer's candidate is
    /// linked.
    pub(super) fn rtc_peer_added(&mut self, conn: RtcConn, now: Time) {
        let Some((x, i)) = self.rtc_conn(conn).and_then(|c| c.exchange) else {
            return;
        };
        let Some(c) = self
            .exchanges
            .get_mut(&x)
            .and_then(|ex| ex.pool.candidates_mut().get_mut(i))
        else {
            return;
        };
        if c.state == CandidateState::AwaitLink {
            c.state = CandidateState::Linked;
            self.poll_exchange(x, now);
        }
    }

    /// A WebRTC link closed (vnic: handshake failure, replaced, closed
    /// locally; or its stream transport failed): close its connection, and
    /// fail an offerer's candidate still waiting for it.
    pub(super) fn rtc_link_closed(&mut self, conn: RtcConn, e: DialError, now: Time) {
        let Some(rtc) = &mut self.rtc else {
            return;
        };
        rtc.ep.close(now, conn);
        let Some(rc) = rtc.conns.get_mut(&conn) else {
            return;
        };
        rc.link = None;
        if let Some((x, i)) = rc.exchange {
            let e = match e {
                DialError::PeerInit(m) => CandidateError::PeerInit(m),
                e @ DialError::OwnHost => CandidateError::PeerInit(e.message().to_owned()),
                DialError::Connect(m) => CandidateError::Failed(m),
            };
            self.fail_candidate(x, i, e, now);
        }
    }

    /// An exchange ended: its connections that never opened are closed;
    /// the others carry links that outlive it.
    pub(super) fn rtc_exchange_closed(&mut self, pool: &LinkCandidatePool<Candidate>, now: Time) {
        let Some(rtc) = &mut self.rtc else {
            return;
        };
        for conn in pool.candidates().iter().filter_map(|c| c.conn) {
            let Some(rc) = rtc.conns.get_mut(&conn) else {
                continue;
            };
            rc.exchange = None;
            if rc.link.is_none() {
                rtc.ep.close(now, conn);
            }
        }
    }

    /// Closes every connection (shutdown).
    pub(super) fn rtc_close_all(&mut self, now: Time) {
        if let Some(rtc) = &mut self.rtc {
            rtc.ep.close_all(now);
        }
    }

    /// A WebRTC candidate's connection failed: whatever the pool waits for
    /// from it (the remote outcome, the local description) is an error.
    fn fail_candidate(&mut self, x: ExchangeId, i: usize, e: CandidateError, now: Time) {
        let Some(c) = self
            .exchanges
            .get_mut(&x)
            .and_then(|ex| ex.pool.candidates_mut().get_mut(i))
        else {
            return;
        };
        let remote = c.state.pending();
        let local = c.local_pending();
        if remote {
            c.state = CandidateState::Failed(e.clone());
        }
        if local {
            c.set_local(Err(e));
        }
        if remote {
            self.poll_exchange(x, now);
        }
        if local {
            self.gather(x, now);
        }
    }
}

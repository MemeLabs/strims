// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Peer exchange link candidates: the node's side of vnic's
//! `LinkCandidatePool`.
//!
//! vpn runs the exchange and reports `GatherLocalDescriptions` and
//! `DialRemoteDescriptions`; the node keeps one pool per `ExchangeId`
//! with one candidate per interface its driver has, in the order TCP,
//! WebSocket, WebRTC, which is the order their descriptions are advertised
//! in. The candidates are plain state: the node does the work they ask for
//! and records the outcome, then polls the pool again.
//!
//! - **TCP and WebSocket** advertise their listener URI and dial the
//!   remote's. A dial is asynchronous: the candidate reports `Pending`
//!   until the dialed link completes its vnic handshake (`Linked`) or
//!   fails.
//! - **WebRTC** owns one connection of the node's endpoint. Its local
//!   description is `Pending` until the node has polled the endpoint and
//!   ICE gathering finished (an offer, or the answer once the remote offer
//!   was applied). A remote description is parsed here (`Describe`) and
//!   applied by the node, which then knows the role:
//!   `AwaitInBackground` (we answered) reports `false` at once and the link
//!   comes up when the channel opens;
//!   `AwaitLink` (we offered) stays `Pending` until the channel opened and
//!   vnic's `PeerAdded` arrived (`Linked`).

use std::task::Poll;

use strims_vnic::webrtc::SessionDescription;
use strims_vnic::{CandidateError, DialTarget, LinkCandidate, LinkDescription, LinkInterface};

use crate::rtc::RtcConn;
use crate::state::DialId;

/// Where a candidate's remote attempt is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CandidateState {
    Idle,
    /// `set_remote_description` accepted a URI; the driver starts the dial.
    Requested(DialTarget),
    Dialing(DialId),
    /// WebRTC: the remote session description, for the driver to apply to
    /// the connection.
    Describe(SessionDescription),
    /// WebRTC, we offered: waiting for the data channel and `PeerAdded`.
    AwaitLink,
    /// WebRTC, we answered: report not connected; the link comes up in the
    /// background.
    Background,
    Linked,
    Failed(CandidateError),
}

impl CandidateState {
    /// The driver still owes this candidate an outcome.
    pub(crate) fn pending(&self) -> bool {
        matches!(
            self,
            CandidateState::Requested(_)
                | CandidateState::Dialing(_)
                | CandidateState::Describe(_)
                | CandidateState::AwaitLink
        )
    }
}

#[derive(Debug)]
pub(crate) struct Candidate {
    iface: LinkInterface,
    /// This candidate's local description: a listener URI (or nothing) at
    /// once for TCP and WebSocket; for WebRTC, `Pending` until the driver
    /// records the gathered offer or answer.
    local: Poll<Result<Option<LinkDescription>, CandidateError>>,
    allow_insecure_ws: bool,
    /// The WebRTC candidate's connection.
    pub(crate) conn: Option<RtcConn>,
    pub(crate) state: CandidateState,
}

impl Candidate {
    /// A TCP or WebSocket candidate.
    pub(crate) fn new(
        iface: LinkInterface,
        local: Option<LinkDescription>,
        allow_insecure_ws: bool,
    ) -> Candidate {
        Candidate {
            iface,
            local: Poll::Ready(Ok(local)),
            allow_insecure_ws,
            conn: None,
            state: CandidateState::Idle,
        }
    }

    /// A WebRTC candidate on `conn`.
    pub(crate) fn webrtc(conn: RtcConn) -> Candidate {
        Candidate {
            iface: LinkInterface::WebRtc,
            local: Poll::Pending,
            allow_insecure_ws: false,
            conn: Some(conn),
            state: CandidateState::Idle,
        }
    }

    /// Whether the local description is still to be recorded.
    pub(crate) fn local_pending(&self) -> bool {
        self.local.is_pending()
    }

    /// Records the WebRTC local description (or why there is none).
    pub(crate) fn set_local(&mut self, r: Result<Option<LinkDescription>, CandidateError>) {
        self.local = Poll::Ready(r);
    }
}

impl LinkCandidate for Candidate {
    fn interface(&self) -> &str {
        self.iface.name()
    }

    fn poll_local_description(&mut self) -> Poll<Result<Option<LinkDescription>, CandidateError>> {
        self.local.clone()
    }

    fn set_remote_description(&mut self, d: &LinkDescription) -> Result<(), CandidateError> {
        if self.iface == LinkInterface::WebRtc {
            let desc = SessionDescription::from_link_description(d)
                .map_err(|e| CandidateError::Failed(e.to_string()))?;
            self.state = CandidateState::Describe(desc);
            return Ok(());
        }
        let target = DialTarget::parse(&d.description, self.allow_insecure_ws)
            .map_err(|e| CandidateError::Failed(e.to_string()))?;
        let matches = matches!(
            (self.iface, &target),
            (LinkInterface::Tcp, DialTarget::Tcp(_)) | (LinkInterface::Ws, DialTarget::Ws(_))
        );
        if !matches {
            return Err(CandidateError::Failed(format!(
                "{} description for the {} interface",
                d.description,
                self.iface.name()
            )));
        }
        self.state = CandidateState::Requested(target);
        Ok(())
    }

    fn poll_remote_description(&mut self) -> Poll<Result<bool, CandidateError>> {
        match std::mem::replace(&mut self.state, CandidateState::Idle) {
            CandidateState::Idle | CandidateState::Background => Poll::Ready(Ok(false)),
            CandidateState::Linked => Poll::Ready(Ok(true)),
            CandidateState::Failed(e) => Poll::Ready(Err(e)),
            s @ (CandidateState::Requested(_)
            | CandidateState::Dialing(_)
            | CandidateState::Describe(_)
            | CandidateState::AwaitLink) => {
                self.state = s;
                Poll::Pending
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

    use super::*;

    fn desc(iface: &str, d: &str) -> LinkDescription {
        LinkDescription {
            interface: iface.to_owned(),
            description: d.to_owned(),
        }
    }

    #[test]
    fn candidates_dial_their_own_scheme() {
        let key = "00".repeat(32);
        let mut tcp = Candidate::new(LinkInterface::Tcp, None, false);
        assert_eq!(tcp.poll_local_description(), Poll::Ready(Ok(None)));
        tcp.set_remote_description(&desc("tcp", &format!("tcp://127.0.0.1:1/{key}")))
            .unwrap();
        assert!(matches!(
            tcp.state,
            CandidateState::Requested(DialTarget::Tcp(_))
        ));
        assert_eq!(tcp.poll_remote_description(), Poll::Pending);
        tcp.state = CandidateState::Linked;
        assert_eq!(tcp.poll_remote_description(), Poll::Ready(Ok(true)));
        assert_eq!(tcp.state, CandidateState::Idle);

        // ws:// needs allow_insecure_ws; a TCP URI is no WebSocket target.
        let mut ws = Candidate::new(LinkInterface::Ws, None, false);
        assert!(
            ws.set_remote_description(&desc("ws", "ws://a:1/x"))
                .is_err()
        );
        assert!(
            ws.set_remote_description(&desc("ws", &format!("tcp://127.0.0.1:1/{key}")))
                .is_err()
        );
        let mut ws = Candidate::new(LinkInterface::Ws, None, true);
        ws.set_remote_description(&desc("ws", "ws://a:1/x"))
            .unwrap();
        ws.state = CandidateState::Failed(CandidateError::Failed("x".into()));
        assert!(matches!(ws.poll_remote_description(), Poll::Ready(Err(_))));
    }

    #[test]
    fn webrtc_candidates_wait_for_the_driver() {
        let mut c = Candidate::webrtc(RtcConn(1));
        assert_eq!(c.interface(), "webrtc");
        assert!(c.local_pending());
        assert_eq!(c.poll_local_description(), Poll::Pending);
        let offer = SessionDescription {
            sdp_type: strims_vnic::webrtc::SdpType::Offer,
            sdp: "v=0\r\n".to_owned(),
        };
        c.set_local(Ok(Some(offer.to_link_description())));
        assert!(!c.local_pending());
        assert_eq!(
            c.poll_local_description(),
            Poll::Ready(Ok(Some(offer.to_link_description())))
        );

        // A remote description waits for the driver to apply it.
        c.set_remote_description(&offer.to_link_description())
            .unwrap();
        assert_eq!(c.state, CandidateState::Describe(offer.clone()));
        assert_eq!(c.poll_remote_description(), Poll::Pending);
        // Answering: not connected now, the link comes up later.
        c.state = CandidateState::Background;
        assert_eq!(c.poll_remote_description(), Poll::Ready(Ok(false)));
        // Offering: pending until the link is up.
        c.state = CandidateState::AwaitLink;
        assert_eq!(c.poll_remote_description(), Poll::Pending);
        c.state = CandidateState::Linked;
        assert_eq!(c.poll_remote_description(), Poll::Ready(Ok(true)));
        // Garbage is this candidate's error.
        assert!(c.set_remote_description(&desc("webrtc", "{")).is_err());
    }
}

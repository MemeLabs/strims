// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! WebRTC link formats and signalling. The driver owns the peer connection.
//!
//! - **Signalling.** Peer exchange (vpn) carries a `LinkDescription` with
//!   interface `"webrtc"` whose description is the JSON session description
//!   `{"type":"offer"|"answer","sdp":"..."}` ([`SessionDescription`]). ICE
//!   isn't trickled: a local description is taken only after candidate
//!   gathering completes, so the SDP carries every candidate.
//! - **Roles.** [`WebRtcSignaling`] decides offer or answer and whether
//!   applying the remote description should wait for the link.
//! - **Data channel.** One pre-negotiated channel, label `"data"`, id 1,
//!   ordered and reliable ([`DATA_CHANNEL_INIT`]). Once it opens it is the
//!   vnic link: each write is one message of at most [`DATA_CHANNEL_MTU`]
//!   bytes, and the reader concatenates messages into a byte stream. No
//!   handshake runs before `PeerInit`.
//!
//! The driver runs one peer connection per candidate, and:
//!
//! - uses the ICE servers in [`WebRtcOptions::ice_servers`], else
//!   [`DEFAULT_ICE_SERVERS`]; UDP4 and UDP6, plus TCP when an ICE TCP mux is
//!   configured; [`WebRtcOptions::host_ip`] as the NAT 1:1 host candidate
//!   IP; and an SCTP receive buffer of [`SCTP_MAX_RECEIVE_BUFFER_SIZE`];
//! - creates the data channel before the offer or answer;
//! - when the channel opens, adds the link with
//!   [`crate::LinkConfig::webrtc`], lowered with
//!   [`crate::LinkConfig::with_mtu`] when the stack reports a smaller
//!   maximum message size;
//! - closes the peer connection when the candidate fails or is dropped, and
//!   on [`crate::Event::LinkClosed`].

use crate::interface::LinkDescription;
use crate::json::{JsonError, decode_session_description, encode_session_description};

/// The interface name in `LinkDescription`s.
pub const WEBRTC_INTERFACE: &str = "webrtc";

/// The largest data channel message, and so the link MTU unless the stack
/// reports a smaller maximum.
pub const DATA_CHANNEL_MTU: usize = 64 * 1024;

/// The SCTP receive buffer size.
pub const SCTP_MAX_RECEIVE_BUFFER_SIZE: usize = 8 * 1024 * 1024;

/// Used when no ICE servers are configured.
pub const DEFAULT_ICE_SERVERS: [&str; 5] = [
    "stun:stun.l.google.com:19302",
    "stun:stun1.l.google.com:19302",
    "stun:stun2.l.google.com:19302",
    "stun:stun3.l.google.com:19302",
    "stun:stun4.l.google.com:19302",
];

/// Data channel parameters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DataChannelInit {
    pub label: &'static str,
    /// The SCTP stream id both sides use.
    pub id: u16,
    /// Created on both sides without the in-band open handshake (DCEP).
    pub negotiated: bool,
    pub ordered: bool,
    pub max_retransmits: Option<u16>,
    pub max_packet_life_time: Option<u16>,
    pub protocol: &'static str,
}

/// The only data channel a vnic WebRTC link uses. Ordered, with no
/// retransmit or lifetime limit: the link is a reliable byte stream.
pub const DATA_CHANNEL_INIT: DataChannelInit = DataChannelInit {
    label: "data",
    id: 1,
    negotiated: true,
    ordered: true,
    max_retransmits: None,
    max_packet_life_time: None,
    protocol: "",
};

/// Peer connection settings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WebRtcOptions {
    /// STUN/TURN URLs; empty means [`DEFAULT_ICE_SERVERS`].
    pub ice_servers: Vec<String>,
    /// Ephemeral UDP port range when no UDP mux is used.
    pub port_min: u16,
    pub port_max: u16,
    /// Advertise this IP for host candidates (NAT 1:1).
    pub host_ip: String,
}

/// A session description's type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SdpType {
    Offer,
    Pranswer,
    Answer,
    Rollback,
}

impl SdpType {
    pub fn as_str(self) -> &'static str {
        match self {
            SdpType::Offer => "offer",
            SdpType::Pranswer => "pranswer",
            SdpType::Answer => "answer",
            SdpType::Rollback => "rollback",
        }
    }

    /// Case-insensitive.
    pub fn parse(s: &str) -> Option<SdpType> {
        [
            SdpType::Offer,
            SdpType::Pranswer,
            SdpType::Answer,
            SdpType::Rollback,
        ]
        .into_iter()
        .find(|t| t.as_str().eq_ignore_ascii_case(s))
    }
}

/// A session description.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionDescription {
    pub sdp_type: SdpType,
    pub sdp: String,
}

impl SessionDescription {
    /// The JSON form; see [`crate::json`].
    pub fn to_json(&self) -> String {
        encode_session_description(self)
    }

    /// Parses the JSON form; see [`crate::json`].
    pub fn from_json(s: &str) -> Result<SessionDescription, JsonError> {
        decode_session_description(s)
    }

    /// The `LinkDescription` peer exchange sends.
    pub fn to_link_description(&self) -> LinkDescription {
        LinkDescription {
            interface: WEBRTC_INTERFACE.to_owned(),
            description: self.to_json(),
        }
    }

    /// Parses a peer's `LinkDescription`.
    pub fn from_link_description(d: &LinkDescription) -> Result<SessionDescription, JsonError> {
        SessionDescription::from_json(&d.description)
    }
}

/// What to do after applying a remote description.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteDescriptionAction {
    /// This side already sent its description (it offered and this is the
    /// answer): wait for the data channel, add the link, and report the
    /// candidate connected.
    AwaitLink,
    /// This side hasn't sent a description (this is an offer): report not
    /// connected yet, send the local description (an answer), and add the
    /// link when the data channel opens.
    AwaitInBackground,
}

/// The offer/answer state of one WebRTC link candidate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WebRtcSignaling {
    have_remote: bool,
    have_local: bool,
}

impl WebRtcSignaling {
    pub fn new() -> WebRtcSignaling {
        WebRtcSignaling::default()
    }

    /// Whether to create an offer or an answer for the local description,
    /// which also marks the local description as set.
    pub fn create_local_description(&mut self) -> SdpType {
        self.have_local = true;
        if self.have_remote {
            SdpType::Answer
        } else {
            SdpType::Offer
        }
    }

    /// Records the remote description.
    pub fn set_remote_description(&mut self) -> RemoteDescriptionAction {
        self.have_remote = true;
        if self.have_local {
            RemoteDescriptionAction::AwaitLink
        } else {
            RemoteDescriptionAction::AwaitInBackground
        }
    }

    pub fn has_local_description(&self) -> bool {
        self.have_local
    }

    pub fn has_remote_description(&self) -> bool {
        self.have_remote
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn offerer_awaits_the_link_after_the_answer() {
        // Offerer: local first, then the answer.
        let mut a = WebRtcSignaling::new();
        assert_eq!(a.create_local_description(), SdpType::Offer);
        assert_eq!(
            a.set_remote_description(),
            RemoteDescriptionAction::AwaitLink
        );

        // Answerer: remote first, then its local answer.
        let mut b = WebRtcSignaling::new();
        assert_eq!(
            b.set_remote_description(),
            RemoteDescriptionAction::AwaitInBackground
        );
        assert_eq!(b.create_local_description(), SdpType::Answer);
    }

    #[test]
    fn link_description_round_trip() {
        let d = SessionDescription {
            sdp_type: SdpType::Answer,
            sdp: "v=0\r\n".to_owned(),
        };
        let ld = d.to_link_description();
        assert_eq!(ld.interface, "webrtc");
        assert_eq!(ld.description, r#"{"type":"answer","sdp":"v=0\r\n"}"#);
        assert_eq!(SessionDescription::from_link_description(&ld).unwrap(), d);
        assert_eq!(SdpType::parse("OFFER"), Some(SdpType::Offer));
        assert_eq!(SdpType::parse("unknown"), None);
    }
}

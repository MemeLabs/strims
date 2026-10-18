// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Why a video control operation failed.

use strims_directory::SnippetError;
use strims_proto::service::Error as RpcError;

use crate::config::ConfigError;
use crate::hls::RouteError;

/// Why a video control operation failed. A frontend answers with
/// [`VideoError::to_rpc`]: code 0 and this error's message.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum VideoError {
    /// The profile has no channel with the id.
    #[error("video channel not found")]
    ChannelNotFound,
    /// The profile has [`MAX_CHANNELS`](crate::MAX_CHANNELS) channels.
    #[error("too many video channels")]
    TooManyChannels,
    /// The key names none of the profile's networks.
    #[error("network not found")]
    NetworkNotFound,
    /// The channel's snippet is over its bounds
    /// ([`strims_directory::snippet::check`]).
    #[error(transparent)]
    Snippet(#[from] SnippetError),
    /// The channel is another key holder's (`LocalShare`), or has no
    /// owner, so the profile can't make its stream key.
    #[error("cannot generate stream key for channel")]
    NoStreamKey,
    /// The profile has no key to sign stream keys with.
    #[error("profile has no signing key")]
    NoProfileKey,
    /// A config field is malformed or over its bound.
    #[error(transparent)]
    Config(#[from] ConfigError),
    /// The store failed.
    #[error(transparent)]
    Store(#[from] strims_dao::Error),
    /// A capture's MIME type is empty, over
    /// [`CaptureSettings::max_mime_bytes`](crate::CaptureSettings::max_mime_bytes),
    /// or not printable ASCII.
    #[error("invalid mime type")]
    MimeType,
    /// A capture names no networks, or a capture or stream more than its
    /// bound.
    #[error("wrong number of network keys")]
    NetworkKeyCount,
    /// A stream's network key is over
    /// [`EgressSettings::max_network_key_bytes`](crate::EgressSettings::max_network_key_bytes).
    #[error("network key too long")]
    NetworkKeyTooLong,
    /// The node runs [`CaptureSettings::max_captures`](crate::CaptureSettings::max_captures)
    /// captures.
    #[error("too many captures")]
    TooManyCaptures,
    /// No capture of this node, or of this frontend connection, has the
    /// id.
    #[error("capture not found")]
    CaptureNotFound,
    /// A capture record ended in a frame that holds the end of the record
    /// before it, so it was dropped. The capture goes on.
    #[error("record dropped: it ended too soon after the one before it")]
    RecordDropped,
    /// The swarm URI is over
    /// [`EgressSettings::max_uri_bytes`](crate::EgressSettings::max_uri_bytes).
    #[error("swarm uri too long")]
    UriTooLong,
    /// The swarm URI doesn't parse.
    #[error("invalid swarm uri: {0}")]
    Uri(#[from] strims_ppspp::UriError),
    /// A swarm option of the URI is outside the egress bounds
    /// ([`EgressSettings`](crate::EgressSettings)) or unsupported.
    #[error("unsupported swarm option: {0}")]
    SwarmOption(&'static str),
    /// The node reads [`EgressSettings::max_swarms`](crate::EgressSettings::max_swarms)
    /// swarms for streams.
    #[error("too many swarms")]
    TooManySwarms,
    /// The node has [`EgressSettings::max_streams`](crate::EgressSettings::max_streams)
    /// streams open.
    #[error("too many streams")]
    TooManyStreams,
    /// The stream's swarm was removed, or couldn't be added.
    #[error("stream closed")]
    StreamClosed,
    /// A swarm read failed.
    #[error("reading the stream failed: {0}")]
    StreamRead(String),
    /// A swarm write failed; the capture writes no more.
    #[error("writing the stream failed: {0}")]
    StreamWrite(String),
    /// A node call failed.
    #[error("node error: {0}")]
    Host(String),
    /// Signing a snippet failed.
    #[error("signing failed: {0}")]
    Sign(#[from] strims_crypto::Error),
    /// The driver doesn't serve HLS
    /// ([`HlsSettings::serve`](crate::HlsSettings::serve)).
    #[error("HLS egress is not supported")]
    HlsNotSupported,
    /// The profile's HLS streams can't be routed.
    #[error("HLS egress is unavailable: {0}")]
    HlsUnavailable(RouteError),
    /// The device's HLS egress config is disabled.
    #[error("HLS egress is disabled")]
    HlsDisabled,
    /// Neither the HLS config's public server address nor
    /// [`HlsSettings::base_url`](crate::HlsSettings::base_url) is set.
    #[error("HLS egress has no server address")]
    HlsNoServerAddr,
    /// The node serves [`HlsSettings::max_streams`](crate::HlsSettings::max_streams)
    /// HLS streams.
    #[error("too many HLS streams")]
    TooManyHlsStreams,
    /// The node has no HLS stream of the transfer.
    #[error("HLS stream not found")]
    HlsStreamNotFound,
    /// The device has no autoseed rule with the id.
    #[error("autoseed rule not found")]
    AutoseedRuleNotFound,
    /// The device has
    /// [`AutoseedSettings::max_rules`](crate::AutoseedSettings::max_rules)
    /// autoseed rules.
    #[error("too many autoseed rules")]
    TooManyAutoseedRules,
    /// A field of an autoseed rule is empty or over its bound; see
    /// [`autoseed`](crate::autoseed#rules).
    #[error("invalid autoseed rule: {0}")]
    AutoseedRule(&'static str),
}

impl VideoError {
    /// The RPC error a frontend answers with; see the type docs.
    pub fn to_rpc(&self) -> RpcError {
        RpcError::new(0, self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_answer_with_their_messages() {
        assert_eq!(
            VideoError::ChannelNotFound.to_rpc(),
            RpcError::new(0, "video channel not found")
        );
        assert_eq!(
            VideoError::Snippet(SnippetError::TooLarge).to_rpc().message,
            "snippet too large"
        );
        assert_eq!(
            VideoError::Config(ConfigError::ServerAddr).to_rpc().message,
            "server address must be host:port"
        );
        assert_eq!(
            VideoError::Store(strims_dao::Error::NotFound)
                .to_rpc()
                .message,
            "record not found"
        );
    }
}

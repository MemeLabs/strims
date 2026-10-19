// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Why a debug control operation failed.

use strims_proto::service::Error as RpcError;

/// Why a debug control operation failed. A frontend answers with
/// [`DebugError::to_rpc`]: code 0 and this error's message.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum DebugError {
    /// The node writes [`Config::max_mock_streams`](crate::Config::max_mock_streams)
    /// mock streams.
    #[error("too many mock streams")]
    TooManyMockStreams,
    /// The bitrate is over
    /// [`Config::max_bitrate_kbps`](crate::Config::max_bitrate_kbps).
    #[error("bitrate out of range")]
    BitrateOutOfRange,
    /// The segment interval is outside
    /// [`Config::min_segment_interval`](crate::Config::min_segment_interval)
    /// to [`Config::max_segment_interval`](crate::Config::max_segment_interval).
    #[error("segment interval out of range")]
    IntervalOutOfRange,
    /// The timeout is over [`Config::max_timeout`](crate::Config::max_timeout).
    #[error("timeout out of range")]
    TimeoutOutOfRange,
    /// The bitrate's segments at the interval would encode to more than
    /// [`MAX_SEGMENT_BYTES`](crate::MAX_SEGMENT_BYTES).
    #[error("segment too large for the bitrate and interval")]
    SegmentTooLarge,
    /// The key names none of the profile's networks.
    #[error("network not found")]
    NetworkNotFound,
    /// The store failed.
    #[error(transparent)]
    Store(#[from] strims_dao::Error),
    /// A node call failed.
    #[error("node error: {0}")]
    Host(String),
}

impl DebugError {
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
            DebugError::TooManyMockStreams.to_rpc(),
            RpcError::new(0, "too many mock streams")
        );
        assert_eq!(
            DebugError::Store(strims_dao::Error::NotFound)
                .to_rpc()
                .message,
            "record not found"
        );
        assert_eq!(
            DebugError::Host("gone".into()).to_rpc().message,
            "node error: gone"
        );
    }
}

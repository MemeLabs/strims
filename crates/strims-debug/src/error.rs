// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Why a debug control operation failed.

use strims_proto::ErrorCoded;
use strims_proto::debug::v1::errors::ErrorCode;

/// Why a debug control operation failed. A frontend answers with its
/// [`ErrorCoded::to_rpc`]: its code and message.
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
    /// A node call failed: the node error's code, and its message.
    #[error("node error: {1}")]
    Host(i32, String),
}

impl ErrorCoded for DebugError {
    fn code(&self) -> i32 {
        let code = match self {
            DebugError::TooManyMockStreams => ErrorCode::TooManyMockStreams,
            DebugError::BitrateOutOfRange => ErrorCode::BitrateOutOfRange,
            DebugError::IntervalOutOfRange => ErrorCode::IntervalOutOfRange,
            DebugError::TimeoutOutOfRange => ErrorCode::TimeoutOutOfRange,
            DebugError::SegmentTooLarge => ErrorCode::SegmentTooLarge,
            DebugError::NetworkNotFound => ErrorCode::NetworkNotFound,
            DebugError::Store(e) => return e.code(),
            DebugError::Host(code, _) => return *code,
        };
        code.into()
    }
}

#[cfg(test)]
mod tests {
    use strims_proto::service::Error as RpcError;

    use super::*;

    #[test]
    fn errors_answer_with_their_codes_and_messages() {
        assert_eq!(
            DebugError::TooManyMockStreams.to_rpc(),
            RpcError::new(ErrorCode::TooManyMockStreams, "too many mock streams")
        );
        assert_eq!(
            DebugError::Store(strims_dao::Error::NotFound)
                .to_rpc()
                .message,
            "record not found"
        );
        assert_eq!(
            DebugError::Host(3007, "gone".into()).to_rpc(),
            RpcError::new(3007, "node error: gone")
        );
    }
}

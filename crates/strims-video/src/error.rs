// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Why a video control operation failed.

use strims_directory::SnippetError;
use strims_proto::service::Error as RpcError;

use crate::config::ConfigError;

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

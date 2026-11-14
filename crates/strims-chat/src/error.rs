// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The chat server's refusals.

use strims_proto::ErrorCoded;
use strims_proto::chat::v1::errors::ErrorCode;

/// Why a chat server refused a call. Callers receive it as the RPC error,
/// with its code ([`ErrorCoded`]), which a node reads back with
/// [`ChatError::from_code`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ChatError {
    /// The caller already posted the running combo's emote.
    #[error("user has already participated in combo")]
    ComboDuplicate,
    /// Mute and unmute need the server's owner or one of its admins.
    #[error("operation requires moderator role")]
    NotModerator,
    /// The caller's mute hasn't expired.
    #[error("cannot send messages while muted")]
    Muted,
    /// The body is over the server's bound.
    #[error("message too long")]
    MessageTooLong,
    /// The caller is over its send rate.
    #[error("sending too fast")]
    TooFast,
    /// A key, alias or mute message of the request is over its bound.
    #[error("request too large")]
    TooLarge,
}

impl ChatError {
    /// Every error.
    const ALL: [ChatError; 6] = [
        ChatError::ComboDuplicate,
        ChatError::NotModerator,
        ChatError::Muted,
        ChatError::MessageTooLong,
        ChatError::TooFast,
        ChatError::TooLarge,
    ];

    /// The error whose code is `code`.
    pub fn from_code(code: i32) -> Option<ChatError> {
        ChatError::ALL.into_iter().find(|e| e.code() == code)
    }
}

impl ErrorCoded for ChatError {
    fn code(&self) -> i32 {
        let code = match self {
            ChatError::ComboDuplicate => ErrorCode::ComboDuplicate,
            ChatError::NotModerator => ErrorCode::NotModerator,
            ChatError::Muted => ErrorCode::Muted,
            ChatError::MessageTooLong => ErrorCode::MessageTooLong,
            ChatError::TooFast => ErrorCode::TooFast,
            ChatError::TooLarge => ErrorCode::TooLarge,
        };
        code.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_messages_match_the_wire() {
        assert_eq!(
            ChatError::ComboDuplicate.to_string(),
            "user has already participated in combo"
        );
        assert_eq!(
            ChatError::NotModerator.to_string(),
            "operation requires moderator role"
        );
        assert_eq!(
            ChatError::Muted.to_string(),
            "cannot send messages while muted"
        );
        assert_eq!(ChatError::MessageTooLong.to_string(), "message too long");
        assert_eq!(ChatError::TooFast.to_string(), "sending too fast");
        assert_eq!(ChatError::TooLarge.to_string(), "request too large");
    }

    #[test]
    fn every_error_is_found_by_its_code() {
        // A code two errors shared would find only the first of them.
        for e in ChatError::ALL {
            assert_eq!(ChatError::from_code(e.code()), Some(e));
        }
        assert_eq!(ChatError::from_code(0), None);
        assert_eq!(ChatError::from_code(ErrorCode::WhisperRefused.into()), None);
    }
}

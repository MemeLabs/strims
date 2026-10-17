// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The chat server's refusals.

/// Why a chat server refused a call. Callers receive the message as the
/// RPC error, so the messages are part of the wire format.
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

    /// The prefixes that mark a muted refusal, with either spelling of
    /// "messages" taken; any text may follow them.
    const MUTED_PREFIXES: [&str; 2] = [
        "cannot send messages while muted",
        "cannot send mesasges while muted",
    ];

    /// The error whose message is `message`. A muted refusal is matched
    /// by its prefix.
    pub fn from_message(message: &str) -> Option<ChatError> {
        if ChatError::MUTED_PREFIXES
            .iter()
            .any(|p| message.starts_with(p))
        {
            return Some(ChatError::Muted);
        }
        ChatError::ALL
            .into_iter()
            .find(|e| e.to_string() == message)
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
    fn every_error_is_found_by_its_message() {
        for e in ChatError::ALL {
            assert_eq!(ChatError::from_message(&e.to_string()), Some(e));
        }
        assert_eq!(ChatError::from_message("sending too fast!"), None);
        assert_eq!(ChatError::from_message(""), None);
    }

    #[test]
    fn a_muted_refusal_may_carry_its_expiry() {
        assert_eq!(
            ChatError::from_message(
                "cannot send mesasges while muted. mute expires: 2026-10-17 12:00:00 +0000 UTC"
            ),
            Some(ChatError::Muted)
        );
        assert_eq!(
            ChatError::from_message("cannot send messages while muted. mute expires: soon"),
            Some(ChatError::Muted)
        );
        assert_eq!(ChatError::from_message("cannot send"), None);
    }
}

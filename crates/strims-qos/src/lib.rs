// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Hierarchical packet fair queueing and rate limiting for vnic egress.
//!
//! [`Control`] is a sans-IO, pull-based H-PFQ scheduler with a host-wide
//! rate limit ([`Hlb`]). It picks which session may send the next frame and
//! never touches packet bytes: a session's owner fills a frame only when the
//! session is chosen. The egress loop in brief (the full contract is on
//! [`Control`]):
//!
//! 1. Keep each session that has data backlogged
//!    ([`Control::set_backlog`]), and mark sessions whose link can't take a
//!    frame blocked ([`Control::set_blocked`]).
//! 2. `while let Some((s, hint)) = qos.dequeue(now)`, fill one frame for
//!    `s`, then report the bytes sent with [`Control::complete`].
//! 3. When `dequeue` returns `None`, wake at [`Control::poll_timeout`], or
//!    on the next backlog or unblock.
//!
//! `set_backlog`, `set_blocked`, `dequeue` and `complete` don't allocate:
//! the SEFF queues are intrusive. There is one node per class and session,
//! all created by the host, so peers can't grow the scheduler.

mod control;
mod hlb;
mod seff;

pub use control::{
    ClassId, Control, DEFAULT_RATE_LIMIT, MAX_WEIGHT, MIN_WEIGHT, NodeId, SessionId,
};
// TODO: remove `inverse_weight` and `Control::inverse_weight_of`; nothing
// calls them.
#[allow(deprecated)]
pub use control::inverse_weight;
pub use hlb::Hlb;

/// Errors from [`Control`] calls with stale or mismatched ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("unknown qos class")]
    UnknownClass,
    #[error("unknown qos session")]
    UnknownSession,
    #[error("the root class can't be removed")]
    RemoveRoot,
    #[error("qos session is not in flight")]
    NotInFlight,
}
